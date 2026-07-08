use axum::{
    extract::{Path, State},
    http::HeaderMap,
    Json,
};
use serde::Serialize;
use serde_json::Value;

use super::guards::find_repo_for_headers;
use super::PolicyRouteError;
use crate::AppState;

#[derive(Debug, Serialize)]
pub struct QueueJobResponse {
    pub id: i64,
    pub kind: String,
    pub status: String,
    pub priority: i32,
    pub attempts: i32,
    pub max_attempts: i32,
    pub run_at: String,
    pub locked_by: Option<String>,
    pub last_error: Option<String>,
    pub subject_type: Option<String>,
    pub subject_number: Option<u64>,
    pub source: Option<String>,
    pub backfill_run_id: Option<i64>,
    pub available_after_rate_limit: bool,
    pub rate_limit_reset_at: Option<String>,
}

#[derive(Debug, Serialize)]
pub struct RateLimitPauseResponse {
    pub bucket: String,
    pub paused_until: String,
    pub remaining: Option<i32>,
    pub reset_at: Option<String>,
    pub last_error: Option<String>,
}

#[derive(Debug, Serialize)]
pub struct PropagationRunResponse {
    pub id: i64,
    pub github_user_id: Option<i64>,
    pub login: Option<String>,
    pub status: String,
    pub total_subjects: i32,
    pub processed_subjects: i32,
    pub current_subject_type: Option<String>,
    pub current_subject_id: Option<String>,
    pub last_error: Option<String>,
    pub started_at: String,
    pub completed_at: Option<String>,
}

#[derive(Debug, Serialize)]
pub struct RepoQueueResponse {
    pub jobs: Vec<QueueJobResponse>,
    pub backfill: Option<db::BackfillRun>,
    pub rate_limits: Vec<RateLimitPauseResponse>,
    pub propagations: Vec<PropagationRunResponse>,
    pub has_active_work: bool,
}

pub(super) async fn repo_queue(
    State(state): State<AppState>,
    headers: HeaderMap,
    Path(repo_id): Path<i64>,
) -> Result<Json<RepoQueueResponse>, PolicyRouteError> {
    let pool = state.db.as_ref().ok_or(PolicyRouteError::NoDb)?;
    let repo = find_repo_for_headers(&state, &headers, pool, repo_id).await?;

    let jobs = db::list_active_jobs_for_repo(pool, repo.repository_id).await?;
    let backfill = db::latest_backfill_run(pool, repo.repository_id).await?;
    let rate_limits =
        db::list_active_rate_limit_pauses_for_installation(pool, repo.installation_id).await?;
    let propagations = db::list_active_propagation_runs(pool, repo.repository_id).await?;

    let active_backfill = backfill
        .as_ref()
        .is_some_and(|run| matches!(run.status.as_str(), "queued" | "scanning" | "running"));

    let job_responses = jobs.into_iter().map(job_to_response).collect::<Vec<_>>();
    let has_active_work = !job_responses.is_empty()
        || active_backfill
        || !rate_limits.is_empty()
        || !propagations.is_empty();

    Ok(Json(RepoQueueResponse {
        jobs: job_responses,
        backfill,
        rate_limits: rate_limits
            .into_iter()
            .map(|pause| RateLimitPauseResponse {
                bucket: pause.bucket,
                paused_until: pause.paused_until.to_string(),
                remaining: pause.remaining,
                reset_at: pause.reset_at.map(|ts| ts.to_string()),
                last_error: pause.last_error,
            })
            .collect(),
        propagations: propagations
            .into_iter()
            .map(|run| PropagationRunResponse {
                id: run.id,
                github_user_id: run.github_user_id,
                login: run.login,
                status: run.status,
                total_subjects: run.total_subjects,
                processed_subjects: run.processed_subjects,
                current_subject_type: run.current_subject_type,
                current_subject_id: run.current_subject_id,
                last_error: run.last_error,
                started_at: run.started_at.to_string(),
                completed_at: run.completed_at.map(|ts| ts.to_string()),
            })
            .collect(),
        has_active_work,
    }))
}

fn job_to_response(job: db::Job) -> QueueJobResponse {
    let payload = &job.payload;
    QueueJobResponse {
        id: job.id,
        kind: job.kind,
        status: job.status,
        priority: job.priority,
        attempts: job.attempts,
        max_attempts: job.max_attempts,
        run_at: job.run_at.to_string(),
        locked_by: job.locked_by,
        last_error: job.last_error,
        subject_type: payload_string(payload, "subject_type"),
        subject_number: payload_u64(payload, "number"),
        source: payload_string(payload, "source"),
        backfill_run_id: payload_i64(payload, "backfill_run_id"),
        available_after_rate_limit: job.available_after_rate_limit,
        rate_limit_reset_at: job.rate_limit_reset_at.map(|ts| ts.to_string()),
    }
}

fn payload_string(payload: &Value, key: &str) -> Option<String> {
    payload.get(key).and_then(Value::as_str).map(str::to_owned)
}

fn payload_u64(payload: &Value, key: &str) -> Option<u64> {
    payload.get(key).and_then(|v| v.as_u64())
}

fn payload_i64(payload: &Value, key: &str) -> Option<i64> {
    payload.get(key).and_then(|v| v.as_i64())
}
