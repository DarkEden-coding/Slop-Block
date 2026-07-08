pub mod admin_auth;
pub mod backfill_jobs;
pub mod captcha_config;
pub mod captcha_routes;
pub mod config;
pub mod error;
pub mod github_helpers;
pub mod github_subjects;
pub mod github_tokens;
pub mod job_runner;
pub mod oauth;
pub mod policy_routes;
pub mod rate_limit;
pub mod secret_box;
pub mod web_util;
pub mod webhooks;

const SERVICE_NAME: &str = "github-human-auth";

pub use config::{Config, ConfigError};
pub use error::{ApiError, ErrorBody, ErrorDetail};

use axum::{
    extract::State,
    http::{header, HeaderValue},
    routing::get,
    Json, Router,
};
use db::PgPool;
use serde::Serialize;
use std::{sync::Arc, time::Duration};
use tower_http::{cors::CorsLayer, trace::TraceLayer};
use tracing::Span;

#[derive(Clone)]
pub struct AppState {
    pub config: Arc<Config>,
    pub db: Option<PgPool>,
    pub rate_limiter: Arc<rate_limit::RateLimiter>,
}

const RATE_LIMIT_MAX_REQUESTS: u32 = 60;
const RATE_LIMIT_WINDOW: Duration = Duration::from_secs(60);

impl AppState {
    pub fn new(config: Config, db: PgPool) -> Self {
        Self {
            config: Arc::new(config),
            db: Some(db),
            rate_limiter: Self::default_rate_limiter(),
        }
    }

    pub fn without_db(config: Config) -> Self {
        Self {
            config: Arc::new(config),
            db: None,
            rate_limiter: Self::default_rate_limiter(),
        }
    }

    fn default_rate_limiter() -> Arc<rate_limit::RateLimiter> {
        Arc::new(rate_limit::RateLimiter::new(
            RATE_LIMIT_MAX_REQUESTS,
            RATE_LIMIT_WINDOW,
        ))
    }
}

pub fn router(state: AppState) -> Router {
    let rate_limited = Router::new()
        .merge(admin_auth::routes())
        .merge(oauth::routes())
        .layer(axum::middleware::from_fn_with_state(
            state.clone(),
            rate_limit::rate_limit_middleware,
        ));
    Router::new()
        .route("/healthz", get(healthz))
        .route("/readyz", get(readyz))
        .merge(rate_limited)
        .merge(policy_routes::router())
        .merge(captcha_routes::router())
        .merge(webhooks::routes())
        .layer(cors_layer(&state.config))
        .layer(
            TraceLayer::new_for_http()
                .make_span_with(|request: &axum::http::Request<_>| {
                    let path = request.uri().path();
                    tracing::info_span!("request", method = %request.method(), path = %path)
                })
                .on_request(|_request: &axum::http::Request<_>, _span: &Span| {}),
        )
        .with_state(state)
}

fn cors_layer(config: &Config) -> CorsLayer {
    let origins = config
        .cors_allowed_origins
        .iter()
        .filter_map(|origin| origin.parse::<HeaderValue>().ok())
        .collect::<Vec<_>>();

    CorsLayer::new()
        .allow_origin(origins)
        .allow_methods([
            axum::http::Method::GET,
            axum::http::Method::POST,
            axum::http::Method::PUT,
            axum::http::Method::DELETE,
        ])
        .allow_headers([
            header::CONTENT_TYPE,
            header::AUTHORIZATION,
            header::HeaderName::from_static("x-requested-with"),
        ])
        .allow_credentials(true)
        .max_age(Duration::from_secs(3600))
}

#[derive(Serialize)]
struct StatusResponse {
    status: &'static str,
    service: &'static str,
}

async fn healthz() -> Json<StatusResponse> {
    Json(StatusResponse {
        status: "ok",
        service: SERVICE_NAME,
    })
}

async fn readyz(State(state): State<AppState>) -> Result<Json<StatusResponse>, ApiError> {
    let pool = state.db.as_ref().ok_or(ApiError::ServiceUnavailable(
        "database pool is not configured",
    ))?;
    sqlx::query("SELECT 1")
        .execute(pool)
        .await
        .map_err(|_| ApiError::ServiceUnavailable("database is not ready"))?;
    Ok(Json(StatusResponse {
        status: "ready",
        service: SERVICE_NAME,
    }))
}

#[cfg(test)]
mod tests {
    use super::*;
    use axum::{
        body::Body,
        http::{Request, StatusCode},
    };
    use std::collections::HashMap;
    use tower::ServiceExt;

    fn config_with(vars: &[(&str, &str)]) -> Result<Config, ConfigError> {
        let map = vars.iter().copied().collect::<HashMap<_, _>>();
        Config::from_getter(|key| map.get(key).map(|value| value.to_string()))
    }

    #[tokio::test]
    async fn healthz_works_without_db() {
        let cfg = config_with(&[("DATABASE_URL", "postgres://user:pass@localhost/db")]).unwrap();
        let app = router(AppState::without_db(cfg));
        let res = app
            .oneshot(
                Request::builder()
                    .uri("/healthz")
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();
        assert_eq!(res.status(), StatusCode::OK);
    }

    #[tokio::test]
    async fn readyz_fails_without_db() {
        let cfg = config_with(&[("DATABASE_URL", "postgres://user:pass@localhost/db")]).unwrap();
        let app = router(AppState::without_db(cfg));
        let res = app
            .oneshot(
                Request::builder()
                    .uri("/readyz")
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();
        assert_eq!(res.status(), StatusCode::SERVICE_UNAVAILABLE);
    }
}
