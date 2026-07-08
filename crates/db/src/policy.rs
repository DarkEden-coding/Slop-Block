use serde_json::Value;
use time::OffsetDateTime;

use crate::connection::{PgPool, Result};
use crate::models::{GithubUser, RepositoryPolicy, TrustedSubject};

pub async fn get_policy(pool: &PgPool, repository_id: i64) -> Result<Option<RepositoryPolicy>> {
    sqlx::query_as::<_, RepositoryPolicy>(
        "SELECT * FROM repository_policies WHERE repository_id=$1",
    )
    .bind(repository_id)
    .fetch_optional(pool)
    .await
}

pub async fn upsert_policy(
    pool: &PgPool,
    repository_id: i64,
    policy: Value,
    enabled: bool,
) -> Result<RepositoryPolicy> {
    sqlx::query_as::<_, RepositoryPolicy>("INSERT INTO repository_policies (repository_id,policy,enabled) VALUES ($1,$2,$3) ON CONFLICT (repository_id) DO UPDATE SET policy=EXCLUDED.policy, enabled=EXCLUDED.enabled, updated_at=now() RETURNING *").bind(repository_id).bind(policy).bind(enabled).fetch_one(pool).await
}

pub async fn upsert_github_user(
    pool: &PgPool,
    github_user_id: i64,
    login: &str,
    avatar_url: Option<&str>,
    raw: Value,
) -> Result<GithubUser> {
    sqlx::query_as::<_, GithubUser>("INSERT INTO github_users (github_user_id,login,avatar_url,raw) VALUES ($1,$2,$3,$4) ON CONFLICT (github_user_id) DO UPDATE SET login=EXCLUDED.login, avatar_url=EXCLUDED.avatar_url, raw=EXCLUDED.raw, updated_at=now() RETURNING *")
        .bind(github_user_id).bind(login).bind(avatar_url).bind(raw).fetch_one(pool).await
}

#[allow(clippy::too_many_arguments)]
pub async fn trust_subject(
    pool: &PgPool,
    repository_id: i64,
    subject_type: &str,
    subject_id: &str,
    github_user_id: Option<i64>,
    reason: Option<&str>,
    expires_at: Option<OffsetDateTime>,
    metadata: Value,
) -> Result<TrustedSubject> {
    sqlx::query_as::<_, TrustedSubject>("INSERT INTO trusted_subjects (repository_id,subject_type,subject_id,github_user_id,reason,expires_at,metadata) VALUES ($1,$2,$3,$4,$5,$6,$7) ON CONFLICT (repository_id,subject_type,subject_id) DO UPDATE SET github_user_id=EXCLUDED.github_user_id, trusted=true, trusted_at=now(), revoked_at=NULL, reason=EXCLUDED.reason, expires_at=EXCLUDED.expires_at, metadata=EXCLUDED.metadata, updated_at=now() RETURNING *")
        .bind(repository_id).bind(subject_type).bind(subject_id).bind(github_user_id).bind(reason).bind(expires_at).bind(metadata).fetch_one(pool).await
}

pub async fn revoke_subject(
    pool: &PgPool,
    repository_id: i64,
    subject_type: &str,
    subject_id: &str,
) -> Result<Option<TrustedSubject>> {
    sqlx::query_as::<_, TrustedSubject>("UPDATE trusted_subjects SET trusted=false, revoked_at=now(), updated_at=now() WHERE repository_id=$1 AND subject_type=$2 AND subject_id=$3 RETURNING *").bind(repository_id).bind(subject_type).bind(subject_id).fetch_optional(pool).await
}

pub async fn get_trusted_subject(
    pool: &PgPool,
    repository_id: i64,
    subject_type: &str,
    subject_id: &str,
) -> Result<Option<TrustedSubject>> {
    sqlx::query_as::<_, TrustedSubject>("SELECT * FROM trusted_subjects WHERE repository_id=$1 AND subject_type=$2 AND subject_id=$3 AND trusted=true AND (expires_at IS NULL OR expires_at > now())")
        .bind(repository_id)
        .bind(subject_type)
        .bind(subject_id)
        .fetch_optional(pool)
        .await
}

pub async fn list_trusted_subjects(
    pool: &PgPool,
    repository_id: i64,
) -> Result<Vec<TrustedSubject>> {
    sqlx::query_as::<_, TrustedSubject>("SELECT * FROM trusted_subjects WHERE repository_id=$1 AND trusted=true AND (expires_at IS NULL OR expires_at > now()) ORDER BY trusted_at DESC").bind(repository_id).fetch_all(pool).await
}
