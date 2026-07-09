use axum::{
    extract::{Path, Query, State},
    http::{HeaderMap, StatusCode},
    Json,
};
use policy::VerificationPolicy;
use serde::Deserialize;

use super::guards::{ensure_mutation_allowed, find_repo_for_headers, list_trusted_users};
use super::{
    AllowlistRequest, PolicyRouteError, RepositoryPolicyResponse, RepositorySummary,
    TrustedUserResponse, UpsertPolicyRequest,
};
use crate::{admin_auth, AppState};

#[derive(Debug, Deserialize)]
pub struct ListQuery {
    #[serde(default)]
    pub limit: Option<i64>,
    #[serde(default)]
    pub offset: Option<i64>,
}

pub(super) async fn list_repositories(
    State(state): State<AppState>,
    headers: HeaderMap,
    Query(query): Query<ListQuery>,
) -> Result<Json<Vec<RepositorySummary>>, PolicyRouteError> {
    let pool = state.db.as_ref().ok_or(PolicyRouteError::NoDb)?;
    let limit = query
        .limit
        .unwrap_or(state.config.dashboard_list_page_size)
        .clamp(1, 500);
    let offset = query.offset.unwrap_or(0).max(0);
    let repositories = if admin_auth::bearer_authorized(&state, &headers) {
        db::list_repositories_for_user_page(pool, None, limit, offset).await?
    } else {
        let user = admin_auth::current_admin_user(&state, &headers)
            .ok_or(PolicyRouteError::Unauthorized)?;
        db::list_repositories_for_user_page(pool, Some(user.id as i64), limit, offset).await?
    };
    Ok(Json(
        repositories
            .into_iter()
            .map(RepositorySummary::from)
            .collect(),
    ))
}

pub(super) async fn get_repo_policy(
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

pub(super) async fn upsert_repo_policy(
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

pub(super) async fn add_allowlist_subject(
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

pub(super) async fn remove_allowlist_subject(
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
