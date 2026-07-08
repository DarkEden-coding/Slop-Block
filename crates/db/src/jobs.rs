use serde_json::Value;
use time::OffsetDateTime;

use crate::connection::{PgPool, Result};
use crate::models::Job;

pub async fn enqueue_job(
    pool: &PgPool,
    kind: &str,
    payload: Value,
    run_at: Option<OffsetDateTime>,
    max_attempts: i32,
) -> Result<Job> {
    enqueue_job_deduped(pool, kind, payload, run_at, max_attempts, None, 100).await
}

pub async fn enqueue_job_deduped(
    pool: &PgPool,
    kind: &str,
    payload: Value,
    run_at: Option<OffsetDateTime>,
    max_attempts: i32,
    dedupe_key: Option<&str>,
    priority: i32,
) -> Result<Job> {
    sqlx::query_as::<_, Job>("INSERT INTO jobs (kind,payload,run_at,max_attempts,dedupe_key,priority) VALUES ($1,$2,COALESCE($3, now()),$4,$5,$6) ON CONFLICT (dedupe_key) WHERE dedupe_key IS NOT NULL AND status IN ('queued','running') DO UPDATE SET payload=EXCLUDED.payload, run_at=LEAST(jobs.run_at, EXCLUDED.run_at), priority=LEAST(jobs.priority, EXCLUDED.priority), updated_at=now() RETURNING *")
        .bind(kind).bind(payload).bind(run_at).bind(max_attempts).bind(dedupe_key).bind(priority).fetch_one(pool).await
}

pub async fn claim_job(
    pool: &PgPool,
    worker: &str,
    stale_after_seconds: i64,
) -> Result<Option<Job>> {
    sqlx::query_as::<_, Job>("UPDATE jobs SET status='running', attempts=attempts+1, locked_by=$1, locked_at=now(), updated_at=now() WHERE id = (SELECT id FROM jobs WHERE (status='queued' OR (status='running' AND locked_at < now() - ($2::text || ' seconds')::interval)) AND run_at <= now() AND attempts < max_attempts ORDER BY priority, run_at, id FOR UPDATE SKIP LOCKED LIMIT 1) RETURNING *")
        .bind(worker).bind(stale_after_seconds).fetch_optional(pool).await
}

pub async fn complete_job(pool: &PgPool, id: i64) -> Result<Option<Job>> {
    sqlx::query_as::<_, Job>("UPDATE jobs SET status='completed', completed_at=now(), locked_by=NULL, locked_at=NULL, updated_at=now() WHERE id=$1 RETURNING *").bind(id).fetch_optional(pool).await
}

pub async fn get_app_setting(pool: &PgPool, key: &str) -> Result<Option<Value>> {
    let row: Option<(Value,)> = sqlx::query_as("SELECT value FROM app_settings WHERE key=$1")
        .bind(key)
        .fetch_optional(pool)
        .await?;
    Ok(row.map(|(value,)| value))
}

pub async fn upsert_app_setting(pool: &PgPool, key: &str, value: Value) -> Result<Value> {
    let (stored,): (Value,) = sqlx::query_as(
        "INSERT INTO app_settings (key, value) VALUES ($1, $2) ON CONFLICT (key) DO UPDATE SET value=EXCLUDED.value, updated_at=now() RETURNING value",
    )
    .bind(key)
    .bind(value)
    .fetch_one(pool)
    .await?;
    Ok(stored)
}

pub async fn fail_job(
    pool: &PgPool,
    id: i64,
    error: &str,
    retry_at: Option<OffsetDateTime>,
) -> Result<Option<Job>> {
    sqlx::query_as::<_, Job>("UPDATE jobs SET status=CASE WHEN attempts >= max_attempts THEN 'failed' ELSE 'queued' END, last_error=$2, run_at=COALESCE($3, now()), locked_by=NULL, locked_at=NULL, rate_limit_reset_at=$3, available_after_rate_limit=$3 IS NOT NULL, updated_at=now() WHERE id=$1 RETURNING *").bind(id).bind(error).bind(retry_at).fetch_optional(pool).await
}
