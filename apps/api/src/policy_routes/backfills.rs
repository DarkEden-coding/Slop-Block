use axum::{
    extract::{Path, State},
    http::HeaderMap,
    Json,
};

use super::guards::{ensure_mutation_allowed, find_repo_for_headers};
use super::{BackfillRequest, PolicyRouteError};
use crate::{admin_auth, AppState};

pub(super) async fn create_backfill(
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
    let user = admin_auth::current_admin_user(&state, &headers);
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

pub(super) async fn current_backfill(
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

pub(super) async fn cancel_backfill(
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
