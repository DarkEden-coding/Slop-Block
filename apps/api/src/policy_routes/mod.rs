mod backfills;
mod guards;
mod installations;
mod repos;

use axum::{
    http::StatusCode,
    response::{IntoResponse, Response},
    routing::{delete, get, post},
    Json, Router,
};
use policy::VerificationPolicy;
use serde::{Deserialize, Serialize};
use thiserror::Error;

use crate::{AppState, ErrorBody, ErrorDetail};

pub fn router() -> Router<AppState> {
    Router::new()
        .route("/api/installations", get(installations::list_installations))
        .route(
            "/api/installations/:installation_id/claim",
            post(installations::claim_installation),
        )
        .route(
            "/api/installations/:installation_id/sync",
            post(installations::sync_installation),
        )
        .route("/api/repos", get(repos::list_repositories))
        .route("/api/repos/:repo_id", get(repos::get_repo_policy))
        .route(
            "/api/repos/:repo_id/policy",
            get(repos::get_repo_policy).post(repos::upsert_repo_policy),
        )
        .route(
            "/api/repos/:repo_id/allowlist",
            post(repos::add_allowlist_subject),
        )
        .route(
            "/api/repos/:repo_id/allowlist/:user_id",
            delete(repos::remove_allowlist_subject),
        )
        .route(
            "/api/repos/:repo_id/backfills",
            post(backfills::create_backfill),
        )
        .route(
            "/api/repos/:repo_id/backfills/current",
            get(backfills::current_backfill),
        )
        .route(
            "/api/repos/:repo_id/backfills/:backfill_id/cancel",
            post(backfills::cancel_backfill),
        )
}

#[derive(Debug, Serialize, Deserialize)]
pub struct InstallationResponse {
    pub id: i64,
    pub installation_id: i64,
    pub account_login: String,
    pub account_id: Option<i64>,
    pub account_type: Option<String>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct RepositoryPolicyResponse {
    pub repository: RepositorySummary,
    pub enabled: bool,
    pub policy: VerificationPolicy,
    #[serde(default)]
    pub trusted_users: Vec<TrustedUserResponse>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct TrustedUserResponse {
    pub id: i64,
    pub subject_id: String,
    pub github_user_id: Option<i64>,
    pub login: Option<String>,
    pub reason: Option<String>,
    pub trusted_at: String,
    pub expires_at: Option<String>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct AllowlistRequest {
    pub user: String,
    #[serde(default)]
    pub reason: Option<String>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct BackfillRequest {
    #[serde(default = "default_true")]
    pub include_issues: bool,
    #[serde(default = "default_true")]
    pub include_pull_requests: bool,
    #[serde(default = "default_true")]
    pub notify_authors: bool,
    #[serde(default)]
    pub force_new_comments: bool,
}

fn default_true() -> bool {
    true
}

#[derive(Debug, Serialize, Deserialize)]
pub struct RepositorySummary {
    pub id: i64,
    pub repository_id: i64,
    pub owner: String,
    pub name: String,
    pub full_name: String,
    pub private: bool,
    pub default_branch: Option<String>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct UpsertPolicyRequest {
    #[serde(default = "default_enabled")]
    pub enabled: bool,
    #[serde(default)]
    pub policy: VerificationPolicy,
}

fn default_enabled() -> bool {
    true
}

impl From<db::GithubInstallation> for InstallationResponse {
    fn from(installation: db::GithubInstallation) -> Self {
        Self {
            id: installation.id,
            installation_id: installation.installation_id,
            account_login: installation.account_login,
            account_id: installation.account_id,
            account_type: installation.account_type,
        }
    }
}

impl From<db::TrustedSubject> for TrustedUserResponse {
    fn from(subject: db::TrustedSubject) -> Self {
        let login = subject
            .metadata
            .get("login")
            .and_then(|v| v.as_str())
            .map(ToOwned::to_owned);
        Self {
            id: subject.id,
            subject_id: subject.subject_id,
            github_user_id: subject.github_user_id,
            login,
            reason: subject.reason,
            trusted_at: subject.trusted_at.to_string(),
            expires_at: subject.expires_at.map(|ts| ts.to_string()),
        }
    }
}

impl From<db::GithubRepository> for RepositorySummary {
    fn from(repo: db::GithubRepository) -> Self {
        Self {
            id: repo.id,
            repository_id: repo.repository_id,
            owner: repo.owner,
            name: repo.name,
            full_name: repo.full_name,
            private: repo.private,
            default_branch: repo.default_branch,
        }
    }
}

#[derive(Debug, Error)]
pub enum PolicyRouteError {
    #[error("database pool is not configured")]
    NoDb,
    #[error("unauthorized")]
    Unauthorized,
    #[error("repository not found")]
    NotFound,
    #[error("invalid policy document")]
    InvalidPolicy,
    #[error("allowlist user must be a GitHub login or numeric user id")]
    InvalidAllowlistUser,
    #[error("reauthentication required to verify GitHub installation access")]
    ReauthRequired,
    #[error("GitHub app credentials are not configured")]
    GitHubNotConfigured,
    #[error("GitHub API error")]
    GitHub(#[from] github::GitHubError),
    #[error("database error")]
    Db(#[from] sqlx::Error),
}

impl IntoResponse for PolicyRouteError {
    fn into_response(self) -> Response {
        let (status, code) = match self {
            PolicyRouteError::NoDb => (StatusCode::SERVICE_UNAVAILABLE, "service_unavailable"),
            PolicyRouteError::Unauthorized => (StatusCode::UNAUTHORIZED, "unauthorized"),
            PolicyRouteError::NotFound => (StatusCode::NOT_FOUND, "not_found"),
            PolicyRouteError::InvalidPolicy => (StatusCode::BAD_REQUEST, "invalid_policy"),
            PolicyRouteError::InvalidAllowlistUser => {
                (StatusCode::BAD_REQUEST, "invalid_allowlist_user")
            }
            PolicyRouteError::ReauthRequired => (StatusCode::UNAUTHORIZED, "reauth_required"),
            PolicyRouteError::GitHubNotConfigured => {
                (StatusCode::SERVICE_UNAVAILABLE, "github_not_configured")
            }
            PolicyRouteError::GitHub(_) => (StatusCode::BAD_GATEWAY, "github_error"),
            PolicyRouteError::Db(_) => (StatusCode::INTERNAL_SERVER_ERROR, "internal_error"),
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
