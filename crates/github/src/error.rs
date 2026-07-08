use reqwest::StatusCode;
use time::OffsetDateTime;

#[derive(Debug, thiserror::Error)]
pub enum GitHubError {
    #[error("GitHub integration is not configured")]
    NotConfigured,
    #[error("invalid GitHub webhook signature")]
    InvalidSignature,
    #[error("JWT error: {0}")]
    Jwt(String),
    #[error("GitHub HTTP error: {0}")]
    Http(#[from] reqwest::Error),
    #[error("GitHub API returned status {0}")]
    ApiStatus(StatusCode),
    #[error("GitHub rate limited until {reset_at:?}: {message:?}")]
    RateLimited {
        status: StatusCode,
        reset_at: Option<OffsetDateTime>,
        retry_after_seconds: Option<u64>,
        remaining: Option<i64>,
        message: Option<String>,
    },
    #[error("GitHub secondary rate limited: {message:?}")]
    SecondaryRateLimited {
        retry_after_seconds: Option<u64>,
        message: Option<String>,
    },
}
