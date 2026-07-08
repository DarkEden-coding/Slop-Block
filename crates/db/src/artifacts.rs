use serde_json::Value;

use crate::connection::{PgPool, Result};
use crate::models::{BotArtifact, WebhookEvent};

pub async fn insert_webhook_event(
    pool: &PgPool,
    delivery_id: &str,
    event_type: &str,
    installation_id: Option<i64>,
    repository_id: Option<i64>,
    payload: Value,
) -> Result<Option<WebhookEvent>> {
    sqlx::query_as::<_, WebhookEvent>("INSERT INTO webhook_events (delivery_id,event_type,installation_id,repository_id,payload) VALUES ($1,$2,$3,$4,$5) ON CONFLICT (delivery_id) DO NOTHING RETURNING *")
        .bind(delivery_id).bind(event_type).bind(installation_id).bind(repository_id).bind(payload).fetch_optional(pool).await
}

pub async fn mark_webhook_processed(
    pool: &PgPool,
    delivery_id: &str,
    error: Option<&str>,
) -> Result<()> {
    sqlx::query(
        "UPDATE webhook_events SET processed_at=now(), processing_error=$2 WHERE delivery_id=$1",
    )
    .bind(delivery_id)
    .bind(error)
    .execute(pool)
    .await?;
    Ok(())
}

pub async fn upsert_bot_artifact(
    pool: &PgPool,
    repository_id: i64,
    subject_type: &str,
    subject_id: &str,
    artifact_type: &str,
    external_id: Option<&str>,
    data: Value,
) -> Result<BotArtifact> {
    sqlx::query_as::<_, BotArtifact>("INSERT INTO bot_artifacts (repository_id,subject_type,subject_id,artifact_type,external_id,data) VALUES ($1,$2,$3,$4,$5,$6) ON CONFLICT (repository_id,subject_type,subject_id,artifact_type) DO UPDATE SET external_id=EXCLUDED.external_id, data=EXCLUDED.data, updated_at=now() RETURNING *").bind(repository_id).bind(subject_type).bind(subject_id).bind(artifact_type).bind(external_id).bind(data).fetch_one(pool).await
}

pub async fn get_bot_artifact(
    pool: &PgPool,
    repository_id: i64,
    subject_type: &str,
    subject_id: &str,
    artifact_type: &str,
) -> Result<Option<BotArtifact>> {
    sqlx::query_as::<_, BotArtifact>("SELECT * FROM bot_artifacts WHERE repository_id=$1 AND subject_type=$2 AND subject_id=$3 AND artifact_type=$4").bind(repository_id).bind(subject_type).bind(subject_id).bind(artifact_type).fetch_optional(pool).await
}

pub async fn insert_audit(
    pool: &PgPool,
    actor: Option<&str>,
    action: &str,
    repository_id: Option<i64>,
    subject_type: Option<&str>,
    subject_id: Option<&str>,
    metadata: Value,
) -> Result<i64> {
    let (id,): (i64,) = sqlx::query_as("INSERT INTO audit_log (actor,action,repository_id,subject_type,subject_id,metadata) VALUES ($1,$2,$3,$4,$5,$6) RETURNING id").bind(actor).bind(action).bind(repository_id).bind(subject_type).bind(subject_id).bind(metadata).fetch_one(pool).await?;
    Ok(id)
}
