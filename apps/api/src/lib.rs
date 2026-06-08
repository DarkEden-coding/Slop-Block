pub mod job_runner;
pub mod oauth;
pub mod policy_routes;
pub mod webhooks;

use axum::{
    extract::State,
    http::{header, HeaderValue, StatusCode},
    response::{IntoResponse, Response},
    routing::get,
    Json, Router,
};
use db::PgPool;
use serde::{Deserialize, Serialize};
use std::{env, net::SocketAddr, sync::Arc, time::Duration};
use thiserror::Error;
use tower_http::{cors::CorsLayer, trace::TraceLayer};

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Config {
    pub host: String,
    pub port: u16,
    pub database_url: String,
    pub cors_allowed_origins: Vec<String>,
    pub cookie_secure: bool,
    pub session_cookie_name: String,
    pub github_webhook_secret: Option<String>,
    pub github_app_id: Option<String>,
    pub github_private_key: Option<String>,
    pub github_web_url: String,
    pub github_api_base: String,
    pub github_oauth_client_id: Option<String>,
    pub github_oauth_client_secret: Option<String>,
    pub api_base_url: String,
    pub web_base_url: String,
    pub turnstile_secret: Option<String>,
    pub turnstile_dev_bypass: bool,
}

impl Config {
    pub fn from_env() -> Result<Self, ConfigError> {
        Self::from_getter(|key| env::var(key).ok())
    }

    pub fn from_getter(get: impl Fn(&str) -> Option<String>) -> Result<Self, ConfigError> {
        let host = get("API_HOST").unwrap_or_else(|| "127.0.0.1".to_string());
        if host.trim().is_empty() {
            return Err(ConfigError::Invalid("API_HOST must not be empty"));
        }

        let port = match get("API_PORT") {
            Some(value) => value
                .parse::<u16>()
                .map_err(|_| ConfigError::Invalid("API_PORT must be a valid u16"))?,
            None => 8080,
        };

        let database_url = get("DATABASE_URL").ok_or(ConfigError::Missing("DATABASE_URL"))?;
        if !(database_url.starts_with("postgres://") || database_url.starts_with("postgresql://")) {
            return Err(ConfigError::Invalid(
                "DATABASE_URL must be a postgres:// or postgresql:// URL",
            ));
        }

        let cors_allowed_origins = get("CORS_ALLOWED_ORIGINS")
            .unwrap_or_else(|| "http://localhost:3000".to_string())
            .split(',')
            .map(str::trim)
            .filter(|origin| !origin.is_empty())
            .map(ToOwned::to_owned)
            .collect::<Vec<_>>();
        if cors_allowed_origins.is_empty() {
            return Err(ConfigError::Invalid(
                "CORS_ALLOWED_ORIGINS must include at least one origin",
            ));
        }

        let cookie_secure = get("COOKIE_SECURE")
            .map(|value| parse_bool("COOKIE_SECURE", &value))
            .transpose()?
            .unwrap_or(true);
        let session_cookie_name =
            get("SESSION_COOKIE_NAME").unwrap_or_else(|| "gho_session".into());
        if session_cookie_name.trim().is_empty() || session_cookie_name.contains(';') {
            return Err(ConfigError::Invalid("SESSION_COOKIE_NAME is invalid"));
        }

        let github_webhook_secret = get("GITHUB_WEBHOOK_SECRET").filter(|value| !value.is_empty());
        let github_app_id = get("GITHUB_APP_ID").filter(|value| !value.is_empty());
        let github_private_key = get("GITHUB_PRIVATE_KEY").filter(|value| !value.is_empty());
        let github_web_url = get("GITHUB_WEB_URL")
            .filter(|value| !value.is_empty())
            .unwrap_or_else(|| "http://localhost:3000".into());
        let github_api_base = get("GITHUB_API_BASE")
            .filter(|value| !value.is_empty())
            .unwrap_or_else(|| "https://api.github.com".into());
        let github_oauth_client_id =
            get("GITHUB_OAUTH_CLIENT_ID").filter(|value| !value.is_empty());
        let github_oauth_client_secret =
            get("GITHUB_OAUTH_CLIENT_SECRET").filter(|value| !value.is_empty());
        let api_base_url = get("API_BASE_URL")
            .filter(|value| !value.is_empty())
            .unwrap_or_else(|| format!("http://{host}:{port}"));
        let web_base_url = get("WEB_BASE_URL")
            .filter(|value| !value.is_empty())
            .unwrap_or_else(|| github_web_url.clone());
        let turnstile_secret = get("TURNSTILE_SECRET").filter(|value| !value.is_empty());
        let turnstile_dev_bypass = get("TURNSTILE_DEV_BYPASS")
            .map(|value| parse_bool("TURNSTILE_DEV_BYPASS", &value))
            .transpose()?
            .unwrap_or(false);

        Ok(Self {
            host,
            port,
            database_url,
            cors_allowed_origins,
            cookie_secure,
            session_cookie_name,
            github_webhook_secret,
            github_app_id,
            github_private_key,
            github_web_url,
            github_api_base,
            github_oauth_client_id,
            github_oauth_client_secret,
            api_base_url,
            web_base_url,
            turnstile_secret,
            turnstile_dev_bypass,
        })
    }

    pub fn addr(&self) -> Result<SocketAddr, ConfigError> {
        format!("{}:{}", self.host, self.port)
            .parse()
            .map_err(|_| ConfigError::Invalid("API_HOST/API_PORT do not form a socket address"))
    }
}

fn parse_bool(name: &'static str, value: &str) -> Result<bool, ConfigError> {
    match value.to_ascii_lowercase().as_str() {
        "1" | "true" | "yes" | "on" => Ok(true),
        "0" | "false" | "no" | "off" => Ok(false),
        _ => Err(ConfigError::InvalidBool(name)),
    }
}

#[derive(Debug, Error)]
pub enum ConfigError {
    #[error("missing required environment variable {0}")]
    Missing(&'static str),
    #[error("invalid configuration: {0}")]
    Invalid(&'static str),
    #[error("{0} must be a boolean")]
    InvalidBool(&'static str),
}

#[derive(Clone)]
pub struct AppState {
    pub config: Arc<Config>,
    pub db: Option<PgPool>,
}

impl AppState {
    pub fn new(config: Config, db: PgPool) -> Self {
        Self {
            config: Arc::new(config),
            db: Some(db),
        }
    }

    pub fn without_db(config: Config) -> Self {
        Self {
            config: Arc::new(config),
            db: None,
        }
    }
}

pub fn router(state: AppState) -> Router {
    Router::new()
        .route("/healthz", get(healthz))
        .route("/readyz", get(readyz))
        .merge(policy_routes::router())
        .merge(webhooks::routes())
        .merge(oauth::routes())
        .layer(cors_layer(&state.config))
        .layer(TraceLayer::new_for_http())
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
            axum::http::Method::DELETE,
        ])
        .allow_headers([header::CONTENT_TYPE, header::AUTHORIZATION])
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
        service: shared::SERVICE_NAME,
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
        service: shared::SERVICE_NAME,
    }))
}

#[derive(Debug, Error)]
pub enum ApiError {
    #[error("{0}")]
    ServiceUnavailable(&'static str),
    #[error("internal server error")]
    Internal,
}

#[derive(Serialize, Deserialize)]
pub struct ErrorBody {
    pub error: ErrorDetail,
}

#[derive(Serialize, Deserialize)]
pub struct ErrorDetail {
    pub code: String,
    pub message: String,
}

impl IntoResponse for ApiError {
    fn into_response(self) -> Response {
        let (status, code) = match self {
            ApiError::ServiceUnavailable(_) => {
                (StatusCode::SERVICE_UNAVAILABLE, "service_unavailable")
            }
            ApiError::Internal => (StatusCode::INTERNAL_SERVER_ERROR, "internal_error"),
        };
        let body = Json(ErrorBody {
            error: ErrorDetail {
                code: code.to_string(),
                message: self.to_string(),
            },
        });
        (status, body).into_response()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use axum::{body::Body, http::Request};
    use std::collections::HashMap;
    use tower::ServiceExt;

    fn config_with(vars: &[(&str, &str)]) -> Result<Config, ConfigError> {
        let map = vars.iter().copied().collect::<HashMap<_, _>>();
        Config::from_getter(|key| map.get(key).map(|value| value.to_string()))
    }

    #[test]
    fn parses_config_defaults() {
        let cfg = config_with(&[("DATABASE_URL", "postgres://user:pass@localhost/db")]).unwrap();
        assert_eq!(cfg.host, "127.0.0.1");
        assert_eq!(cfg.port, 8080);
        assert!(cfg.cookie_secure);
    }

    #[test]
    fn rejects_missing_database_url() {
        assert!(matches!(
            config_with(&[]),
            Err(ConfigError::Missing("DATABASE_URL"))
        ));
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
