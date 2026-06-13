use axum::{
    extract::State,
    http::{HeaderMap, StatusCode},
    response::{IntoResponse, Response},
    routing::get,
    Json, Router,
};
use thiserror::Error;

use crate::captcha_config::{
    self, CaptchaPublicConfig, CaptchaSettings, CaptchaSettingsUpdate, SETTINGS_KEY,
};
use crate::{AppState, ErrorBody, ErrorDetail};

pub fn router() -> Router<AppState> {
    Router::new()
        .route("/api/captcha/config", get(get_public_config))
        .route(
            "/api/settings/captcha",
            get(get_settings).put(update_settings),
        )
}

async fn get_public_config(
    State(state): State<AppState>,
) -> Result<Json<CaptchaPublicConfig>, CaptchaRouteError> {
    let settings = captcha_config::load_settings(&state).await;
    Ok(Json(captcha_config::public_config(&settings)))
}

async fn get_settings(
    State(state): State<AppState>,
    headers: HeaderMap,
) -> Result<Json<CaptchaSettings>, CaptchaRouteError> {
    ensure_admin(&state, &headers)?;
    Ok(Json(captcha_config::load_settings(&state).await))
}

async fn update_settings(
    State(state): State<AppState>,
    headers: HeaderMap,
    Json(body): Json<CaptchaSettingsUpdate>,
) -> Result<Json<CaptchaSettings>, CaptchaRouteError> {
    ensure_admin_mutation(&state, &headers)?;
    let pool = state.db.as_ref().ok_or(CaptchaRouteError::NoDb)?;
    let existing = db::get_app_setting(pool, SETTINGS_KEY)
        .await
        .map_err(CaptchaRouteError::Db)?;
    captcha_config::validate_settings_update(&body, &state.config, existing.as_ref())
        .map_err(CaptchaRouteError::InvalidSettings)?;
    let value = captcha_config::merge_settings_update(existing, &body, &state.config)
        .map_err(CaptchaRouteError::InvalidSettings)?;
    db::upsert_app_setting(pool, SETTINGS_KEY, value)
        .await
        .map_err(CaptchaRouteError::Db)?;
    Ok(Json(captcha_config::load_settings(&state).await))
}

fn ensure_admin(state: &AppState, headers: &HeaderMap) -> Result<(), CaptchaRouteError> {
    if crate::admin_auth::authorize_admin(state, headers) {
        Ok(())
    } else {
        Err(CaptchaRouteError::Unauthorized)
    }
}

fn ensure_admin_mutation(state: &AppState, headers: &HeaderMap) -> Result<(), CaptchaRouteError> {
    if crate::admin_auth::authorize_admin_mutation(state, headers) {
        Ok(())
    } else {
        Err(CaptchaRouteError::Unauthorized)
    }
}

#[derive(Debug, Error)]
pub enum CaptchaRouteError {
    #[error("database pool is not configured")]
    NoDb,
    #[error("unauthorized")]
    Unauthorized,
    #[error("{0}")]
    InvalidSettings(String),
    #[error("database error")]
    Db(#[from] sqlx::Error),
}

impl IntoResponse for CaptchaRouteError {
    fn into_response(self) -> Response {
        let (status, code) = match &self {
            CaptchaRouteError::NoDb => (StatusCode::SERVICE_UNAVAILABLE, "service_unavailable"),
            CaptchaRouteError::Unauthorized => (StatusCode::UNAUTHORIZED, "unauthorized"),
            CaptchaRouteError::InvalidSettings(_) => (StatusCode::BAD_REQUEST, "invalid_settings"),
            CaptchaRouteError::Db(_) => (StatusCode::INTERNAL_SERVER_ERROR, "internal_error"),
        };
        (
            status,
            Json(ErrorBody {
                error: ErrorDetail {
                    code: code.into(),
                    message: self.to_string(),
                },
            }),
        )
            .into_response()
    }
}
