use crate::connection::{PgPool, Result};
use crate::models::{BackfillItem, BackfillRun};

#[allow(clippy::too_many_arguments)]
pub async fn create_backfill_run(
    pool: &PgPool,
    repository_id: i64,
    requested_by_github_user_id: Option<i64>,
    requested_by_login: Option<&str>,
    include_issues: bool,
    include_pull_requests: bool,
    notify_authors: bool,
    force_new_comments: bool,
) -> Result<BackfillRun> {
    sqlx::query_as::<_, BackfillRun>("INSERT INTO backfill_runs (repository_id,requested_by_github_user_id,requested_by_login,include_issues,include_pull_requests,notify_authors,force_new_comments,current_phase) VALUES ($1,$2,$3,$4,$5,$6,$7,'queued') ON CONFLICT (repository_id) WHERE status IN ('queued','scanning','running') DO UPDATE SET updated_at=backfill_runs.updated_at RETURNING *")
        .bind(repository_id).bind(requested_by_github_user_id).bind(requested_by_login).bind(include_issues).bind(include_pull_requests).bind(notify_authors).bind(force_new_comments).fetch_one(pool).await
}

pub async fn get_backfill_run(pool: &PgPool, id: i64) -> Result<Option<BackfillRun>> {
    sqlx::query_as::<_, BackfillRun>("SELECT * FROM backfill_runs WHERE id=$1")
        .bind(id)
        .fetch_optional(pool)
        .await
}

pub async fn latest_backfill_run(pool: &PgPool, repository_id: i64) -> Result<Option<BackfillRun>> {
    sqlx::query_as::<_, BackfillRun>(
        "SELECT * FROM backfill_runs WHERE repository_id=$1 ORDER BY created_at DESC LIMIT 1",
    )
    .bind(repository_id)
    .fetch_optional(pool)
    .await
}

pub async fn mark_backfill_phase(
    pool: &PgPool,
    id: i64,
    status: &str,
    phase: Option<&str>,
) -> Result<Option<BackfillRun>> {
    sqlx::query_as::<_, BackfillRun>("UPDATE backfill_runs SET status=$2, current_phase=$3, started_at=COALESCE(started_at, now()), updated_at=now() WHERE id=$1 RETURNING *")
        .bind(id).bind(status).bind(phase).fetch_optional(pool).await
}

pub async fn cancel_backfill_run(pool: &PgPool, id: i64) -> Result<Option<BackfillRun>> {
    sqlx::query_as::<_, BackfillRun>("UPDATE backfill_runs SET status='cancelled', cancelled_at=now(), completed_at=now(), updated_at=now() WHERE id=$1 AND status IN ('queued','scanning','running') RETURNING *")
        .bind(id).fetch_optional(pool).await
}

#[allow(clippy::too_many_arguments)]
pub async fn insert_backfill_item(
    pool: &PgPool,
    run_id: i64,
    repository_id: i64,
    subject_type: &str,
    subject_id: &str,
    github_user_id: Option<i64>,
    login: Option<&str>,
    html_url: Option<&str>,
    head_sha: Option<&str>,
) -> Result<Option<BackfillItem>> {
    sqlx::query_as::<_, BackfillItem>("INSERT INTO backfill_items (backfill_run_id,repository_id,subject_type,subject_id,github_user_id,login,html_url,head_sha) VALUES ($1,$2,$3,$4,$5,$6,$7,$8) ON CONFLICT (backfill_run_id,repository_id,subject_type,subject_id) DO NOTHING RETURNING *")
        .bind(run_id).bind(repository_id).bind(subject_type).bind(subject_id).bind(github_user_id).bind(login).bind(html_url).bind(head_sha).fetch_optional(pool).await
}

pub async fn get_backfill_item(pool: &PgPool, id: i64) -> Result<Option<BackfillItem>> {
    sqlx::query_as::<_, BackfillItem>("SELECT * FROM backfill_items WHERE id=$1")
        .bind(id)
        .fetch_optional(pool)
        .await
}

pub async fn mark_backfill_item_running(pool: &PgPool, id: i64) -> Result<Option<BackfillItem>> {
    sqlx::query_as::<_, BackfillItem>("UPDATE backfill_items SET status='running', attempts=attempts+1, started_at=COALESCE(started_at, now()), updated_at=now() WHERE id=$1 AND status='queued' RETURNING *").bind(id).fetch_optional(pool).await
}

pub async fn finish_backfill_item(
    pool: &PgPool,
    run_id: i64,
    item_id: i64,
    status: &str,
    decision_required: Option<bool>,
    decision_reason: Option<&str>,
    error: Option<&str>,
) -> Result<()> {
    let mut tx = pool.begin().await?;
    sqlx::query("UPDATE backfill_items SET status=$3, decision_required=$4, decision_reason=$5, error=$6, processed_at=now(), updated_at=now() WHERE id=$1 AND backfill_run_id=$2 AND status IN ('queued','running')")
        .bind(item_id).bind(run_id).bind(status).bind(decision_required).bind(decision_reason).bind(error).execute(&mut *tx).await?;
    sqlx::query("UPDATE backfill_runs SET total_processed=total_processed+1, total_succeeded=total_succeeded+CASE WHEN $2='succeeded' THEN 1 ELSE 0 END, total_failed=total_failed+CASE WHEN $2='failed' THEN 1 ELSE 0 END, total_skipped=total_skipped+CASE WHEN $2='skipped' THEN 1 ELSE 0 END, last_error=COALESCE($3,last_error), updated_at=now() WHERE id=$1")
        .bind(run_id).bind(status).bind(error).execute(&mut *tx).await?;
    sqlx::query("UPDATE backfill_runs SET status=CASE WHEN total_failed > 0 THEN 'failed' ELSE 'completed' END, current_phase='finalizing', completed_at=now(), updated_at=now() WHERE id=$1 AND status='running' AND total_enqueued > 0 AND total_processed >= total_enqueued")
        .bind(run_id).execute(&mut *tx).await?;
    tx.commit().await?;
    Ok(())
}

pub async fn increment_backfill_discovered(
    pool: &PgPool,
    run_id: i64,
    discovered_delta: i32,
    enqueued_delta: i32,
) -> Result<()> {
    sqlx::query("UPDATE backfill_runs SET total_discovered=total_discovered+$2, total_enqueued=total_enqueued+$3, updated_at=now() WHERE id=$1")
        .bind(run_id).bind(discovered_delta).bind(enqueued_delta).execute(pool).await?;
    Ok(())
}
