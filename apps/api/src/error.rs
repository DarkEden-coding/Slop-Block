use axum::{
    http::StatusCode,
    response::{IntoResponse, Response},
    Json,
};
use serde::{Deserialize, Serialize};
use thiserror::Error;

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

pub fn api_json_error(
    status: StatusCode,
    code: impl Into<String>,
    message: impl std::fmt::Display,
) -> Response {
    (
        status,
        Json(ErrorBody {
            error: ErrorDetail {
                code: code.into(),
                message: message.to_string(),
            },
        }),
    )
        .into_response()
}

impl IntoResponse for ApiError {
    fn into_response(self) -> Response {
        let (status, code) = match self {
            ApiError::ServiceUnavailable(_) => {
                (StatusCode::SERVICE_UNAVAILABLE, "service_unavailable")
            }
            ApiError::Internal => (StatusCode::INTERNAL_SERVER_ERROR, "internal_error"),
        };
        api_json_error(status, code, self)
    }
}
