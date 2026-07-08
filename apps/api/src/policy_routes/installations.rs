use axum::{
    extract::{Path, State},
    http::HeaderMap,
    Json,
};
use github::GitHubApi;

use super::guards::{
    ensure_installation_access, ensure_mutation_allowed, policy_installation_token,
};
use super::{InstallationResponse, PolicyRouteError, RepositorySummary};
use crate::{admin_auth, github_helpers::sync_user_installations, AppState};

pub(super) async fn list_installations(
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
        let user = admin_auth::current_admin_user(&state, &headers)
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

pub(super) async fn claim_installation(
    State(state): State<AppState>,
    headers: HeaderMap,
    Path(installation_id): Path<i64>,
) -> Result<Json<InstallationResponse>, PolicyRouteError> {
    ensure_mutation_allowed(&state, &headers)?;
    let user =
        admin_auth::current_admin_user(&state, &headers).ok_or(PolicyRouteError::Unauthorized)?;
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

pub(super) async fn sync_installation(
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
    sync_user_installations(
        pool,
        &installations,
        github_user_id,
        login,
        "verified_claim",
    )
    .await?;
    if installations
        .iter()
        .any(|installation| installation.id as i64 == installation_id)
    {
        return Ok(());
    }
    Err(PolicyRouteError::Unauthorized)
}
