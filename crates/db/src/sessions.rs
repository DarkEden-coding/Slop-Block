use serde_json::Value;
use time::OffsetDateTime;
use uuid::Uuid;

use crate::connection::{PgPool, Result};
use crate::models::VerificationSession;

#[allow(clippy::too_many_arguments)]
pub async fn create_verification_session(
    pool: &PgPool,
    repository_id: i64,
    subject_type: &str,
    subject_id: &str,
    github_user_id: Option<i64>,
    token_hash: &str,
    expires_at: OffsetDateTime,
    metadata: Value,
) -> Result<VerificationSession> {
    sqlx::query_as::<_, VerificationSession>("INSERT INTO verification_sessions (repository_id,subject_type,subject_id,github_user_id,token_hash,expires_at,metadata) VALUES ($1,$2,$3,$4,$5,$6,$7) RETURNING *").bind(repository_id).bind(subject_type).bind(subject_id).bind(github_user_id).bind(token_hash).bind(expires_at).bind(metadata).fetch_one(pool).await
}

pub async fn get_verification_session(
    pool: &PgPool,
    public_id: Uuid,
    token_hash: &str,
) -> Result<Option<VerificationSession>> {
    sqlx::query_as::<_, VerificationSession>(
        "SELECT * FROM verification_sessions WHERE public_id=$1 AND token_hash=$2",
    )
    .bind(public_id)
    .bind(token_hash)
    .fetch_optional(pool)
    .await
}

pub async fn complete_verification_session(
    pool: &PgPool,
    public_id: Uuid,
    token_hash: &str,
) -> Result<Option<VerificationSession>> {
    sqlx::query_as::<_, VerificationSession>("UPDATE verification_sessions SET status='completed', completed_at=now(), updated_at=now() WHERE public_id=$1 AND token_hash=$2 AND status='pending' RETURNING *").bind(public_id).bind(token_hash).fetch_optional(pool).await
}

pub async fn rotate_verification_session_token(
    pool: &PgPool,
    public_id: Uuid,
    old_token_hash: &str,
    new_token_hash: &str,
) -> Result<Option<VerificationSession>> {
    sqlx::query_as::<_, VerificationSession>("UPDATE verification_sessions SET token_hash=$3, updated_at=now() WHERE public_id=$1 AND token_hash=$2 AND status='pending' RETURNING *")
        .bind(public_id)
        .bind(old_token_hash)
        .bind(new_token_hash)
        .fetch_optional(pool)
        .await
}

pub async fn mark_verification_session_oauth_verified(
    pool: &PgPool,
    public_id: Uuid,
    token_hash: &str,
    oauth_login: &str,
    oauth_user_id: i64,
) -> Result<Option<VerificationSession>> {
    let patch = serde_json::json!({
        "oauth_verified": true,
        "oauth_login": oauth_login,
        "oauth_user_id": oauth_user_id,
        "oauth_verified_at": OffsetDateTime::now_utc().unix_timestamp(),
        "trust_source": "oauth_captcha",
    });
    sqlx::query_as::<_, VerificationSession>(
        "UPDATE verification_sessions SET metadata = metadata || $3::jsonb, updated_at=now() WHERE public_id=$1 AND token_hash=$2 AND status='pending' RETURNING *",
    )
    .bind(public_id)
    .bind(token_hash)
    .bind(patch)
    .fetch_optional(pool)
    .await
}

pub async fn complete_pending_sessions_for_user(
    pool: &PgPool,
    repository_id: i64,
    github_user_id: i64,
) -> Result<Vec<VerificationSession>> {
    sqlx::query_as::<_, VerificationSession>("UPDATE verification_sessions SET status='completed', completed_at=COALESCE(completed_at, now()), updated_at=now() WHERE repository_id=$1 AND github_user_id=$2 AND status='pending' RETURNING *")
        .bind(repository_id)
        .bind(github_user_id)
        .fetch_all(pool)
        .await
}
