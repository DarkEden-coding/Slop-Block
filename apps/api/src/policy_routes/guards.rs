use axum::http::HeaderMap;

use super::PolicyRouteError;
use crate::{admin_auth, AppState};

pub(crate) fn ensure_mutation_allowed(
    state: &AppState,
    headers: &HeaderMap,
) -> Result<(), PolicyRouteError> {
    if admin_auth::bearer_authorized(state, headers)
        || (admin_auth::current_admin_user(state, headers).is_some()
            && admin_auth::mutation_header_present(headers))
    {
        Ok(())
    } else {
        Err(PolicyRouteError::Unauthorized)
    }
}

pub(crate) async fn ensure_installation_access(
    state: &AppState,
    headers: &HeaderMap,
    installation_id: i64,
) -> Result<(), PolicyRouteError> {
    if admin_auth::bearer_authorized(state, headers) {
        return Ok(());
    }
    let pool = state.db.as_ref().ok_or(PolicyRouteError::NoDb)?;
    let user =
        admin_auth::current_admin_user(state, headers).ok_or(PolicyRouteError::Unauthorized)?;
    if db::user_can_manage_installation(pool, installation_id, user.id as i64).await? {
        Ok(())
    } else {
        Err(PolicyRouteError::Unauthorized)
    }
}

pub(crate) async fn find_repo_for_headers(
    state: &AppState,
    headers: &HeaderMap,
    pool: &db::PgPool,
    repo_id: i64,
) -> Result<db::GithubRepository, PolicyRouteError> {
    let repo = find_repo(pool, repo_id).await?;
    if admin_auth::bearer_authorized(state, headers) {
        return Ok(repo);
    }
    let user =
        admin_auth::current_admin_user(state, headers).ok_or(PolicyRouteError::Unauthorized)?;
    if db::user_can_manage_repo(pool, repo.repository_id, user.id as i64).await? {
        Ok(repo)
    } else {
        Err(PolicyRouteError::Unauthorized)
    }
}

#[allow(dead_code)]
pub(crate) async fn policy_installation_token(
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

pub(crate) async fn find_repo(
    pool: &db::PgPool,
    repo_id: i64,
) -> Result<db::GithubRepository, PolicyRouteError> {
    sqlx::query_as::<_, db::GithubRepository>(
        "SELECT * FROM github_repositories WHERE (repository_id=$1 OR id=$1) AND active=true ORDER BY (repository_id=$1) DESC LIMIT 1",
    )
    .bind(repo_id)
    .fetch_optional(pool)
    .await?
    .ok_or(PolicyRouteError::NotFound)
}

pub(crate) async fn list_trusted_users(
    pool: &db::PgPool,
    repository_id: i64,
) -> Result<Vec<super::TrustedUserResponse>, PolicyRouteError> {
    Ok(db::list_trusted_subjects(pool, repository_id)
        .await?
        .into_iter()
        .filter(|s| s.subject_type == "github_user")
        .map(super::TrustedUserResponse::from)
        .collect())
}
