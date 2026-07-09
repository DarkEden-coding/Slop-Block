use axum::{
    extract::{Path, Query, State},
    http::HeaderMap,
    Json,
};
use github::GitHubApi;
use serde::Deserialize;
use serde_json::json;

use super::guards::{ensure_installation_access, ensure_mutation_allowed};
use super::{InstallationResponse, PolicyRouteError, RepositorySummary};
use crate::{
    admin_auth, github_helpers::sync_user_installations_filtered, github_tokens::github_client,
    AppState,
};

#[derive(Debug, Deserialize)]
pub struct ListQuery {
    #[serde(default)]
    pub limit: Option<i64>,
    #[serde(default)]
    pub offset: Option<i64>,
}

pub(super) async fn list_installations(
    State(state): State<AppState>,
    headers: HeaderMap,
    Query(query): Query<ListQuery>,
) -> Result<Json<Vec<InstallationResponse>>, PolicyRouteError> {
    let pool = state.db.as_ref().ok_or(PolicyRouteError::NoDb)?;
    let limit = query
        .limit
        .unwrap_or(state.config.dashboard_list_page_size)
        .clamp(1, 500);
    let offset = query.offset.unwrap_or(0).max(0);
    let installations = if admin_auth::bearer_authorized(&state, &headers) {
        db::list_installations_page(pool, limit, offset).await?
    } else {
        let user = admin_auth::current_admin_user(&state, &headers)
            .ok_or(PolicyRouteError::Unauthorized)?;
        db::list_user_installations_page(pool, user.id as i64, limit, offset).await?
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

#[derive(Debug, serde::Serialize)]
pub struct SyncInstallationResponse {
    pub status: &'static str,
    pub installation_id: i64,
    pub repositories: Vec<RepositorySummary>,
}

pub(super) async fn sync_installation(
    State(state): State<AppState>,
    headers: HeaderMap,
    Path(installation_id): Path<i64>,
    Query(query): Query<ListQuery>,
) -> Result<Json<SyncInstallationResponse>, PolicyRouteError> {
    ensure_mutation_allowed(&state, &headers)?;
    ensure_installation_access(&state, &headers, installation_id).await?;
    let pool = state.db.as_ref().ok_or(PolicyRouteError::NoDb)?;
    let payload = json!({ "installation_id": installation_id });
    let key = format!("sync:installation:{installation_id}");
    jobs::enqueue_deduped(
        pool,
        jobs::JobKind::SyncInstallation,
        payload,
        None,
        8,
        Some(&key),
        20,
    )
    .await?;
    let limit = query
        .limit
        .unwrap_or(state.config.dashboard_list_page_size)
        .clamp(1, 500);
    let offset = query.offset.unwrap_or(0).max(0);
    let repositories = db::list_repositories_page(pool, installation_id, limit, offset).await?;
    Ok(Json(SyncInstallationResponse {
        status: "accepted",
        installation_id,
        repositories: repositories
            .into_iter()
            .map(RepositorySummary::from)
            .collect(),
    }))
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
    let gh = github_client(state);
    let installations = gh
        .user_installations(&token)
        .await
        .map_err(PolicyRouteError::GitHub)?;
    sync_user_installations_filtered(
        pool,
        &installations,
        github_user_id,
        login,
        "verified_claim",
        Some(installation_id),
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
