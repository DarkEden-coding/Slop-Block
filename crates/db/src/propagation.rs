use time::OffsetDateTime;
use uuid::Uuid;

use crate::connection::{PgPool, Result};
use crate::models::PropagationRun;

pub async fn create_propagation_run(
    pool: &PgPool,
    repository_id: i64,
    github_user_id: Option<i64>,
    login: Option<&str>,
    session_public_id: Uuid,
) -> Result<PropagationRun> {
    sqlx::query_as::<_, PropagationRun>(
        "INSERT INTO propagation_runs (repository_id, github_user_id, login, session_public_id, status)
         VALUES ($1, $2, $3, $4, 'running')
         RETURNING *",
    )
    .bind(repository_id)
    .bind(github_user_id)
    .bind(login)
    .bind(session_public_id)
    .fetch_one(pool)
    .await
}

pub async fn set_propagation_total(pool: &PgPool, id: i64, total_subjects: i32) -> Result<()> {
    sqlx::query(
        "UPDATE propagation_runs SET total_subjects=$2, updated_at=now() WHERE id=$1 AND status='running'",
    )
    .bind(id)
    .bind(total_subjects)
    .execute(pool)
    .await?;
    Ok(())
}

pub async fn advance_propagation_progress(
    pool: &PgPool,
    id: i64,
    subject_type: &str,
    subject_id: &str,
) -> Result<()> {
    sqlx::query(
        "UPDATE propagation_runs
         SET processed_subjects=processed_subjects+1,
             current_subject_type=$2,
             current_subject_id=$3,
             updated_at=now()
         WHERE id=$1 AND status='running'",
    )
    .bind(id)
    .bind(subject_type)
    .bind(subject_id)
    .execute(pool)
    .await?;
    Ok(())
}

pub async fn complete_propagation_run(pool: &PgPool, id: i64) -> Result<Option<PropagationRun>> {
    sqlx::query_as::<_, PropagationRun>(
        "UPDATE propagation_runs
         SET status='completed',
             completed_at=now(),
             current_subject_type=NULL,
             current_subject_id=NULL,
             updated_at=now()
         WHERE id=$1 AND status='running'
         RETURNING *",
    )
    .bind(id)
    .fetch_optional(pool)
    .await
}

pub async fn fail_propagation_run(
    pool: &PgPool,
    id: i64,
    error: &str,
) -> Result<Option<PropagationRun>> {
    sqlx::query_as::<_, PropagationRun>(
        "UPDATE propagation_runs
         SET status='failed',
             last_error=$2,
             completed_at=now(),
             current_subject_type=NULL,
             current_subject_id=NULL,
             updated_at=now()
         WHERE id=$1 AND status='running'
         RETURNING *",
    )
    .bind(id)
    .bind(error)
    .fetch_optional(pool)
    .await
}

pub async fn list_active_propagation_runs(
    pool: &PgPool,
    repository_id: i64,
) -> Result<Vec<PropagationRun>> {
    sqlx::query_as::<_, PropagationRun>(
        "SELECT * FROM propagation_runs
         WHERE repository_id=$1 AND status='running'
         ORDER BY started_at DESC",
    )
    .bind(repository_id)
    .fetch_all(pool)
    .await
}

pub async fn list_recent_propagation_runs(
    pool: &PgPool,
    repository_id: i64,
    limit: i64,
) -> Result<Vec<PropagationRun>> {
    sqlx::query_as::<_, PropagationRun>(
        "SELECT * FROM propagation_runs
         WHERE repository_id=$1
         ORDER BY started_at DESC
         LIMIT $2",
    )
    .bind(repository_id)
    .bind(limit)
    .fetch_all(pool)
    .await
}

pub async fn stale_propagation_runs(
    pool: &PgPool,
    older_than: OffsetDateTime,
) -> Result<Vec<PropagationRun>> {
    sqlx::query_as::<_, PropagationRun>(
        "SELECT * FROM propagation_runs
         WHERE status='running' AND started_at < $1",
    )
    .bind(older_than)
    .fetch_all(pool)
    .await
}
