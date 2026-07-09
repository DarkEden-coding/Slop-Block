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
    sqlx::query_as::<_, Job>(
        r#"
        UPDATE jobs
        SET status='running',
            attempts=attempts+1,
            locked_by=$1,
            locked_at=now(),
            updated_at=now()
        WHERE id = (
            SELECT j.id
            FROM jobs j
            WHERE (
                j.status='queued'
                OR (
                    j.status='running'
                    AND j.locked_at < now() - ($2::text || ' seconds')::interval
                )
            )
              AND j.run_at <= now()
              AND j.attempts < j.max_attempts
              AND (
                NOT j.available_after_rate_limit
                OR j.rate_limit_reset_at IS NULL
                OR j.rate_limit_reset_at <= now()
              )
              AND NOT EXISTS (
                SELECT 1
                FROM github_rate_limits rl
                WHERE rl.paused_until > now()
                  AND (
                    (
                      (j.payload->>'installation_id') ~ '^[0-9]+$'
                      AND rl.bucket = 'installation:' || (j.payload->>'installation_id') || ':core'
                    )
                    OR (
                      (j.payload->>'repository_id') ~ '^[0-9]+$'
                      AND rl.bucket = (
                        SELECT 'installation:' || r.installation_id::text || ':core'
                        FROM github_repositories r
                        WHERE r.repository_id = (j.payload->>'repository_id')::bigint
                        LIMIT 1
                      )
                    )
                  )
              )
            ORDER BY j.priority, j.run_at, j.id
            FOR UPDATE SKIP LOCKED
            LIMIT 1
        )
        RETURNING *
        "#,
    )
    .bind(worker)
    .bind(stale_after_seconds)
    .fetch_optional(pool)
    .await
}

pub async fn complete_job(pool: &PgPool, id: i64) -> Result<Option<Job>> {
    sqlx::query_as::<_, Job>(
        "UPDATE jobs SET status='completed', completed_at=now(), locked_by=NULL, locked_at=NULL, available_after_rate_limit=false, rate_limit_reset_at=NULL, updated_at=now() WHERE id=$1 RETURNING *",
    )
    .bind(id)
    .fetch_optional(pool)
    .await
}

pub async fn purge_completed_jobs(pool: &PgPool, older_than_days: i64) -> Result<u64> {
    let result = sqlx::query(
        "DELETE FROM jobs WHERE status='completed' AND completed_at < now() - ($1::text || ' days')::interval",
    )
    .bind(older_than_days)
    .execute(pool)
    .await?;
    Ok(result.rows_affected())
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

pub async fn list_active_jobs_for_repo(pool: &PgPool, repository_id: i64) -> Result<Vec<Job>> {
    sqlx::query_as::<_, Job>(
        "SELECT j.*
         FROM jobs j
         WHERE j.status IN ('queued', 'running')
           AND (
             (j.payload->>'repository_id')::bigint = $1
             OR (
               j.kind IN ('backfill_scan', 'backfill_subject')
               AND EXISTS (
                 SELECT 1
                 FROM backfill_runs br
                 WHERE br.id = (j.payload->>'backfill_run_id')::bigint
                   AND br.repository_id = $1
               )
             )
           )
         ORDER BY j.priority, j.run_at, j.id
         LIMIT 200",
    )
    .bind(repository_id)
    .fetch_all(pool)
    .await
}
