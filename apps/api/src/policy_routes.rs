use axum::{
    extract::{Path, State},
    http::{HeaderMap, StatusCode},
    response::{IntoResponse, Response},
    routing::{delete, get, post},
    Json, Router,
};
use policy::VerificationPolicy;
use serde::{Deserialize, Serialize};
use thiserror::Error;

use crate::{admin_auth, AppState, ErrorBody, ErrorDetail};
use github::GitHubApi;

pub fn router() -> Router<AppState> {
    Router::new()
        .route("/api/installations", get(list_installations))
        .route(
            "/api/installations/:installation_id/claim",
            post(claim_installation),
        )
        .route(
            "/api/installations/:installation_id/sync",
            post(sync_installation),
        )
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
        .route("/api/repos/:repo_id/backfills", post(create_backfill))
        .route(
            "/api/repos/:repo_id/backfills/current",
            get(current_backfill),
        )
        .route(
            "/api/repos/:repo_id/backfills/:backfill_id/cancel",
            post(cancel_backfill),
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

async fn list_installations(
    State(state): State<AppState>,
    headers: HeaderMap,
) -> Result<Json<Vec<InstallationResponse>>, PolicyRouteError> {
    let pool = state.db.as_ref().ok_or(PolicyRouteError::NoDb)?;
    let installations = if admin_auth::bearer_authorized(&state, &headers) {
        sqlx::query_as::<_, db::GithubInstallation>(
            "SELECT * FROM github_installations WHERE deleted_at IS NULL ORDER BY account_login",
        )
        .fetch_all(pool)
        .await?
    } else {
        let user = crate::admin_auth::current_admin_user(&state, &headers)
            .ok_or(PolicyRouteError::Unauthorized)?;
        sqlx::query_as::<_, db::GithubInstallation>(
            "SELECT i.* FROM github_installations i JOIN installation_admins a ON a.installation_id=i.installation_id WHERE a.github_user_id=$1 AND i.deleted_at IS NULL ORDER BY i.account_login",
        )
        .bind(user.id as i64)
        .fetch_all(pool)
        .await?
    };
    Ok(Json(
        installations
            .into_iter()
            .map(InstallationResponse::from)
            .collect(),
    ))
}

async fn list_repositories(
    State(state): State<AppState>,
    headers: HeaderMap,
) -> Result<Json<Vec<RepositorySummary>>, PolicyRouteError> {
    let pool = state.db.as_ref().ok_or(PolicyRouteError::NoDb)?;
    let repositories = if admin_auth::bearer_authorized(&state, &headers) {
        sqlx::query_as::<_, db::GithubRepository>(
            "SELECT * FROM github_repositories WHERE active=true ORDER BY full_name",
        )
        .fetch_all(pool)
        .await?
    } else {
        let user = crate::admin_auth::current_admin_user(&state, &headers)
            .ok_or(PolicyRouteError::Unauthorized)?;
        sqlx::query_as::<_, db::GithubRepository>(
            "SELECT r.* FROM github_repositories r JOIN installation_admins a ON a.installation_id=r.installation_id WHERE a.github_user_id=$1 AND r.active=true ORDER BY r.full_name",
        )
        .bind(user.id as i64)
        .fetch_all(pool)
        .await?
    };
    Ok(Json(
        repositories
            .into_iter()
            .map(RepositorySummary::from)
            .collect(),
    ))
}

async fn claim_installation(
    State(state): State<AppState>,
    headers: HeaderMap,
    Path(installation_id): Path<i64>,
) -> Result<Json<InstallationResponse>, PolicyRouteError> {
    ensure_mutation_allowed(&state, &headers)?;
    let user = crate::admin_auth::current_admin_user(&state, &headers)
        .ok_or(PolicyRouteError::Unauthorized)?;
    let pool = state.db.as_ref().ok_or(PolicyRouteError::NoDb)?;
    verify_user_installation_access(&state, pool, user.id as i64, &user.login, installation_id)
        .await?;
    let installation = sqlx::query_as::<_, db::GithubInstallation>(
        "SELECT * FROM github_installations WHERE installation_id=$1 AND deleted_at IS NULL",
    )
    .bind(installation_id)
    .fetch_optional(pool)
    .await?
    .ok_or(PolicyRouteError::NotFound)?;
    Ok(Json(InstallationResponse::from(installation)))
}

async fn sync_installation(
    State(state): State<AppState>,
    headers: HeaderMap,
    Path(installation_id): Path<i64>,
) -> Result<Json<Vec<RepositorySummary>>, PolicyRouteError> {
    ensure_mutation_allowed(&state, &headers)?;
    ensure_installation_access(&state, &headers, installation_id).await?;
    let pool = state.db.as_ref().ok_or(PolicyRouteError::NoDb)?;
    let token = policy_installation_token(&state, installation_id as u64).await?;
    let client = github::ReqwestGitHubClient::with_base_url(&state.config.github_api_base);
    let repos = client
        .installation_repositories(&token)
        .await
        .map_err(PolicyRouteError::GitHub)?;
    for repo in repos {
        let owner = repo.owner.login;
        let name = repo.name;
        db::upsert_repository(
            pool,
            repo.id as i64,
            installation_id,
            &owner,
            &name,
            &repo.full_name,
            repo.private,
            repo.default_branch.as_deref(),
            serde_json::json!({}),
        )
        .await?;
    }
    let repositories = db::list_repositories(pool, installation_id).await?;
    Ok(Json(
        repositories
            .into_iter()
            .map(RepositorySummary::from)
            .collect(),
    ))
}

async fn get_repo_policy(
    State(state): State<AppState>,
    headers: HeaderMap,
    Path(repo_id): Path<i64>,
) -> Result<Json<RepositoryPolicyResponse>, PolicyRouteError> {
    let pool = state.db.as_ref().ok_or(PolicyRouteError::NoDb)?;
    let repo = find_repo_for_headers(&state, &headers, pool, repo_id).await?;

    let stored = db::get_policy(pool, repo.repository_id).await?;
    let (enabled, policy) = match stored {
        Some(stored) => (
            stored.enabled,
            serde_json::from_value(stored.policy).map_err(|_| PolicyRouteError::InvalidPolicy)?,
        ),
        None => (false, VerificationPolicy::default()),
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
    ensure_mutation_allowed(&state, &headers)?;
    let pool = state.db.as_ref().ok_or(PolicyRouteError::NoDb)?;
    let repo = find_repo_for_headers(&state, &headers, pool, repo_id).await?;

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
    ensure_mutation_allowed(&state, &headers)?;
    let pool = state.db.as_ref().ok_or(PolicyRouteError::NoDb)?;
    let repo = find_repo_for_headers(&state, &headers, pool, repo_id).await?;
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
    ensure_mutation_allowed(&state, &headers)?;
    let pool = state.db.as_ref().ok_or(PolicyRouteError::NoDb)?;
    let repo = find_repo_for_headers(&state, &headers, pool, repo_id).await?;
    db::revoke_subject(pool, repo.repository_id, "github_user", &user_id)
        .await?
        .ok_or(PolicyRouteError::NotFound)?;
    Ok(StatusCode::NO_CONTENT)
}

async fn create_backfill(
    State(state): State<AppState>,
    headers: HeaderMap,
    Path(repo_id): Path<i64>,
    Json(req): Json<BackfillRequest>,
) -> Result<Json<db::BackfillRun>, PolicyRouteError> {
    ensure_mutation_allowed(&state, &headers)?;
    if !req.include_issues && !req.include_pull_requests {
        return Err(PolicyRouteError::InvalidPolicy);
    }
    let pool = state.db.as_ref().ok_or(PolicyRouteError::NoDb)?;
    let repo = find_repo_for_headers(&state, &headers, pool, repo_id).await?;
    let user = crate::admin_auth::current_admin_user(&state, &headers);
    let run = db::create_backfill_run(
        pool,
        repo.repository_id,
        user.as_ref().map(|u| u.id as i64),
        user.as_ref().map(|u| u.login.as_str()),
        req.include_issues,
        req.include_pull_requests,
        req.notify_authors,
        req.force_new_comments,
    )
    .await?;
    let key = format!("backfill:{}:scan", run.id);
    jobs::enqueue_deduped(
        pool,
        jobs::JobKind::BackfillScan,
        serde_json::json!({"backfill_run_id": run.id, "repository_id": repo.repository_id}),
        None,
        8,
        Some(&key),
        40,
    )
    .await?;
    Ok(Json(run))
}

async fn current_backfill(
    State(state): State<AppState>,
    headers: HeaderMap,
    Path(repo_id): Path<i64>,
) -> Result<Json<Option<db::BackfillRun>>, PolicyRouteError> {
    let pool = state.db.as_ref().ok_or(PolicyRouteError::NoDb)?;
    let repo = find_repo_for_headers(&state, &headers, pool, repo_id).await?;
    Ok(Json(
        db::latest_backfill_run(pool, repo.repository_id).await?,
    ))
}

async fn cancel_backfill(
    State(state): State<AppState>,
    headers: HeaderMap,
    Path((repo_id, backfill_id)): Path<(i64, i64)>,
) -> Result<Json<db::BackfillRun>, PolicyRouteError> {
    ensure_mutation_allowed(&state, &headers)?;
    let pool = state.db.as_ref().ok_or(PolicyRouteError::NoDb)?;
    let _repo = find_repo_for_headers(&state, &headers, pool, repo_id).await?;
    let run = db::cancel_backfill_run(pool, backfill_id)
        .await?
        .ok_or(PolicyRouteError::NotFound)?;
    Ok(Json(run))
}

fn ensure_mutation_allowed(state: &AppState, headers: &HeaderMap) -> Result<(), PolicyRouteError> {
    if admin_auth::bearer_authorized(state, headers)
        || (crate::admin_auth::current_admin_user(state, headers).is_some()
            && crate::admin_auth::mutation_header_present(headers))
    {
        Ok(())
    } else {
        Err(PolicyRouteError::Unauthorized)
    }
}

async fn ensure_installation_access(
    state: &AppState,
    headers: &HeaderMap,
    installation_id: i64,
) -> Result<(), PolicyRouteError> {
    if admin_auth::bearer_authorized(state, headers) {
        return Ok(());
    }
    let pool = state.db.as_ref().ok_or(PolicyRouteError::NoDb)?;
    let user = crate::admin_auth::current_admin_user(state, headers)
        .ok_or(PolicyRouteError::Unauthorized)?;
    if db::user_can_manage_installation(pool, installation_id, user.id as i64).await? {
        Ok(())
    } else {
        Err(PolicyRouteError::Unauthorized)
    }
}

async fn find_repo_for_headers(
    state: &AppState,
    headers: &HeaderMap,
    pool: &db::PgPool,
    repo_id: i64,
) -> Result<db::GithubRepository, PolicyRouteError> {
    let repo = find_repo(pool, repo_id).await?;
    if admin_auth::bearer_authorized(state, headers) {
        return Ok(repo);
    }
    let user = crate::admin_auth::current_admin_user(state, headers)
        .ok_or(PolicyRouteError::Unauthorized)?;
    if db::user_can_manage_repo(pool, repo.repository_id, user.id as i64).await? {
        Ok(repo)
    } else {
        Err(PolicyRouteError::Unauthorized)
    }
}

async fn policy_installation_token(
    state: &AppState,
    installation_id: u64,
) -> Result<String, PolicyRouteError> {
    if state.config.github_app_id.is_none() || state.config.github_private_key.is_none() {
        return Err(PolicyRouteError::GitHubNotConfigured);
    }
    crate::github_tokens::installation_token(state, installation_id)
        .await
        .map_err(|err| {
            err.downcast::<github::GitHubError>()
                .map(PolicyRouteError::GitHub)
                .unwrap_or(PolicyRouteError::GitHubNotConfigured)
        })
}

async fn verify_user_installation_access(
    state: &AppState,
    pool: &db::PgPool,
    github_user_id: i64,
    login: &str,
    installation_id: i64,
) -> Result<(), PolicyRouteError> {
    let stored = db::get_dashboard_oauth_token(pool, github_user_id)
        .await?
        .ok_or(PolicyRouteError::ReauthRequired)?;
    let token = crate::secret_box::decrypt_field(&state.config, &stored.access_token_encrypted)
        .ok_or(PolicyRouteError::ReauthRequired)?;
    let gh = github::ReqwestGitHubClient::with_base_url(&state.config.github_api_base);
    let installations = gh
        .user_installations(&token)
        .await
        .map_err(PolicyRouteError::GitHub)?;
    for installation in installations {
        let account = installation.account;
        let account_login = account
            .as_ref()
            .map(|a| a.login.as_str())
            .unwrap_or("unknown");
        let account_id = account.as_ref().map(|a| a.id as i64);
        db::upsert_installation(
            pool,
            installation.id as i64,
            account_login,
            account_id,
            None,
            serde_json::json!({"source":"verified_claim"}),
        )
        .await?;
        db::upsert_installation_admin(pool, installation.id as i64, github_user_id, login).await?;
        if installation.id as i64 == installation_id {
            return Ok(());
        }
    }
    Err(PolicyRouteError::Unauthorized)
}

async fn find_repo(
    pool: &db::PgPool,
    repo_id: i64,
) -> Result<db::GithubRepository, PolicyRouteError> {
    // Prefer the GitHub repository id over the internal serial id so a collision
    // between the two can never resolve to the wrong repository.
    sqlx::query_as::<_, db::GithubRepository>(
        "SELECT * FROM github_repositories WHERE (repository_id=$1 OR id=$1) AND active=true ORDER BY (repository_id=$1) DESC LIMIT 1",
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
