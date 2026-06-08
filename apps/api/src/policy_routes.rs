use axum::{
    extract::{Path, State},
    http::{header, HeaderMap, StatusCode},
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
        .route("/api/installations", get(list_installations))
        .route("/api/repos", get(list_repositories))
        .route("/api/repos/:repo_id", get(get_repo_policy))
        .route(
            "/api/repos/:repo_id/policy",
            get(get_repo_policy).post(upsert_repo_policy),
        )
        .route("/api/repos/:repo_id/allowlist", post(add_allowlist_subject))
        .route(
            "/api/repos/:repo_id/allowlist/:user_id",
            delete(remove_allowlist_subject),
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

async fn list_installations(
    State(state): State<AppState>,
    headers: HeaderMap,
) -> Result<Json<Vec<InstallationResponse>>, PolicyRouteError> {
    ensure_admin(&state, &headers)?;
    let pool = state.db.as_ref().ok_or(PolicyRouteError::NoDb)?;
    let installations = sqlx::query_as::<_, db::GithubInstallation>(
        "SELECT * FROM github_installations ORDER BY account_login",
    )
    .fetch_all(pool)
    .await?
    .into_iter()
    .map(InstallationResponse::from)
    .collect();
    Ok(Json(installations))
}

async fn list_repositories(
    State(state): State<AppState>,
    headers: HeaderMap,
) -> Result<Json<Vec<RepositorySummary>>, PolicyRouteError> {
    ensure_admin(&state, &headers)?;
    let pool = state.db.as_ref().ok_or(PolicyRouteError::NoDb)?;
    let repositories = sqlx::query_as::<_, db::GithubRepository>(
        "SELECT * FROM github_repositories ORDER BY full_name",
    )
    .fetch_all(pool)
    .await?
    .into_iter()
    .map(RepositorySummary::from)
    .collect();
    Ok(Json(repositories))
}

async fn get_repo_policy(
    State(state): State<AppState>,
    headers: HeaderMap,
    Path(repo_id): Path<i64>,
) -> Result<Json<RepositoryPolicyResponse>, PolicyRouteError> {
    ensure_admin(&state, &headers)?;
    let pool = state.db.as_ref().ok_or(PolicyRouteError::NoDb)?;
    let repo = sqlx::query_as::<_, db::GithubRepository>(
        "SELECT * FROM github_repositories WHERE repository_id=$1 OR id=$1",
    )
    .bind(repo_id)
    .fetch_optional(pool)
    .await?
    .ok_or(PolicyRouteError::NotFound)?;

    let stored = db::get_policy(pool, repo.repository_id).await?;
    let (enabled, policy) = match stored {
        Some(stored) => (
            stored.enabled,
            serde_json::from_value(stored.policy).map_err(|_| PolicyRouteError::InvalidPolicy)?,
        ),
        None => (true, VerificationPolicy::default()),
    };

    let trusted_users = list_trusted_users(pool, repo.repository_id).await?;

    Ok(Json(RepositoryPolicyResponse {
        repository: RepositorySummary::from(repo),
        enabled,
        policy,
        trusted_users,
    }))
}

async fn upsert_repo_policy(
    State(state): State<AppState>,
    headers: HeaderMap,
    Path(repo_id): Path<i64>,
    Json(req): Json<UpsertPolicyRequest>,
) -> Result<Json<RepositoryPolicyResponse>, PolicyRouteError> {
    ensure_admin(&state, &headers)?;
    let pool = state.db.as_ref().ok_or(PolicyRouteError::NoDb)?;
    let repo = sqlx::query_as::<_, db::GithubRepository>(
        "SELECT * FROM github_repositories WHERE repository_id=$1 OR id=$1",
    )
    .bind(repo_id)
    .fetch_optional(pool)
    .await?
    .ok_or(PolicyRouteError::NotFound)?;

    let policy_value =
        serde_json::to_value(&req.policy).map_err(|_| PolicyRouteError::InvalidPolicy)?;
    let stored = db::upsert_policy(pool, repo.repository_id, policy_value, req.enabled).await?;
    let policy =
        serde_json::from_value(stored.policy).map_err(|_| PolicyRouteError::InvalidPolicy)?;

    let trusted_users = list_trusted_users(pool, repo.repository_id).await?;

    Ok(Json(RepositoryPolicyResponse {
        repository: RepositorySummary::from(repo),
        enabled: stored.enabled,
        policy,
        trusted_users,
    }))
}

async fn add_allowlist_subject(
    State(state): State<AppState>,
    headers: HeaderMap,
    Path(repo_id): Path<i64>,
    Json(req): Json<AllowlistRequest>,
) -> Result<Json<TrustedUserResponse>, PolicyRouteError> {
    ensure_admin(&state, &headers)?;
    let pool = state.db.as_ref().ok_or(PolicyRouteError::NoDb)?;
    let repo = find_repo(pool, repo_id).await?;
    let user = req.user.trim();
    if user.is_empty() {
        return Err(PolicyRouteError::InvalidAllowlistUser);
    }
    let github_user_id = user.parse::<i64>().ok();
    let login = github_user_id.is_none().then(|| user.to_string());
    let metadata = serde_json::json!({"source":"manual_allowlist","login": login});
    let subject = db::trust_subject(
        pool,
        repo.repository_id,
        "github_user",
        user,
        github_user_id,
        req.reason.as_deref(),
        None,
        metadata,
    )
    .await?;
    Ok(Json(TrustedUserResponse::from(subject)))
}

async fn remove_allowlist_subject(
    State(state): State<AppState>,
    headers: HeaderMap,
    Path((repo_id, user_id)): Path<(i64, String)>,
) -> Result<StatusCode, PolicyRouteError> {
    ensure_admin(&state, &headers)?;
    let pool = state.db.as_ref().ok_or(PolicyRouteError::NoDb)?;
    let repo = find_repo(pool, repo_id).await?;
    db::revoke_subject(pool, repo.repository_id, "github_user", &user_id)
        .await?
        .ok_or(PolicyRouteError::NotFound)?;
    Ok(StatusCode::NO_CONTENT)
}

fn ensure_admin(state: &AppState, headers: &HeaderMap) -> Result<(), PolicyRouteError> {
    let Some(expected) = state.config.admin_api_token.as_deref() else {
        return Ok(());
    };
    let Some(provided) = headers
        .get(header::AUTHORIZATION)
        .and_then(|value| value.to_str().ok())
        .and_then(|value| value.strip_prefix("Bearer "))
    else {
        return Err(PolicyRouteError::Unauthorized);
    };
    if constant_time_eq(provided.as_bytes(), expected.as_bytes()) {
        Ok(())
    } else {
        Err(PolicyRouteError::Unauthorized)
    }
}

fn constant_time_eq(a: &[u8], b: &[u8]) -> bool {
    if a.len() != b.len() {
        return false;
    }
    a.iter().zip(b).fold(0u8, |acc, (x, y)| acc | (x ^ y)) == 0
}

async fn find_repo(
    pool: &db::PgPool,
    repo_id: i64,
) -> Result<db::GithubRepository, PolicyRouteError> {
    sqlx::query_as::<_, db::GithubRepository>(
        "SELECT * FROM github_repositories WHERE repository_id=$1 OR id=$1",
    )
    .bind(repo_id)
    .fetch_optional(pool)
    .await?
    .ok_or(PolicyRouteError::NotFound)
}

async fn list_trusted_users(
    pool: &db::PgPool,
    repository_id: i64,
) -> Result<Vec<TrustedUserResponse>, PolicyRouteError> {
    Ok(db::list_trusted_subjects(pool, repository_id)
        .await?
        .into_iter()
        .filter(|s| s.subject_type == "github_user")
        .map(TrustedUserResponse::from)
        .collect())
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
