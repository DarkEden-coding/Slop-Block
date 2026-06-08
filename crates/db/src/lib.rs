use serde::{Deserialize, Serialize};
use serde_json::Value;
use sqlx::{postgres::PgPoolOptions, FromRow};
use time::OffsetDateTime;
use uuid::Uuid;

pub type PgPool = sqlx::PgPool;
pub type Result<T> = std::result::Result<T, sqlx::Error>;

pub async fn connect(database_url: &str) -> Result<PgPool> {
    PgPoolOptions::new()
        .max_connections(10)
        .connect(database_url)
        .await
}

pub async fn migrate(pool: &PgPool) -> Result<()> {
    sqlx::migrate!("../../migrations").run(pool).await?;
    Ok(())
}

#[derive(Debug, Clone, Serialize, Deserialize, FromRow)]
pub struct GithubInstallation {
    pub id: i64,
    pub installation_id: i64,
    pub account_login: String,
    pub account_id: Option<i64>,
    pub account_type: Option<String>,
    pub access_token: Option<String>,
    pub access_token_expires_at: Option<OffsetDateTime>,
    pub raw: Value,
    pub created_at: OffsetDateTime,
    pub updated_at: OffsetDateTime,
}

#[derive(Debug, Clone, Serialize, Deserialize, FromRow)]
pub struct GithubRepository {
    pub id: i64,
    pub repository_id: i64,
    pub installation_id: i64,
    pub owner: String,
    pub name: String,
    pub full_name: String,
    pub private: bool,
    pub default_branch: Option<String>,
    pub raw: Value,
    pub created_at: OffsetDateTime,
    pub updated_at: OffsetDateTime,
}

#[derive(Debug, Clone, Serialize, Deserialize, FromRow)]
pub struct RepositoryPolicy {
    pub id: i64,
    pub repository_id: i64,
    pub policy: Value,
    pub enabled: bool,
    pub created_at: OffsetDateTime,
    pub updated_at: OffsetDateTime,
}

#[derive(Debug, Clone, Serialize, Deserialize, FromRow)]
pub struct GithubUser {
    pub id: i64,
    pub github_user_id: i64,
    pub login: String,
    pub avatar_url: Option<String>,
    pub raw: Value,
    pub created_at: OffsetDateTime,
    pub updated_at: OffsetDateTime,
}

#[derive(Debug, Clone, Serialize, Deserialize, FromRow)]
pub struct TrustedSubject {
    pub id: i64,
    pub repository_id: i64,
    pub subject_type: String,
    pub subject_id: String,
    pub github_user_id: Option<i64>,
    pub trusted: bool,
    pub trusted_at: OffsetDateTime,
    pub revoked_at: Option<OffsetDateTime>,
    pub expires_at: Option<OffsetDateTime>,
    pub reason: Option<String>,
    pub metadata: Value,
    pub created_at: OffsetDateTime,
    pub updated_at: OffsetDateTime,
}

#[derive(Debug, Clone, Serialize, Deserialize, FromRow)]
pub struct VerificationSession {
    pub id: i64,
    pub public_id: Uuid,
    pub repository_id: i64,
    pub subject_type: String,
    pub subject_id: String,
    pub github_user_id: Option<i64>,
    pub token_hash: String,
    pub status: String,
    pub challenge_provider: Option<String>,
    pub completed_at: Option<OffsetDateTime>,
    pub expires_at: OffsetDateTime,
    pub metadata: Value,
    pub created_at: OffsetDateTime,
    pub updated_at: OffsetDateTime,
}

#[derive(Debug, Clone, Serialize, Deserialize, FromRow)]
pub struct BotArtifact {
    pub id: i64,
    pub repository_id: i64,
    pub subject_type: String,
    pub subject_id: String,
    pub artifact_type: String,
    pub external_id: Option<String>,
    pub data: Value,
    pub created_at: OffsetDateTime,
    pub updated_at: OffsetDateTime,
}

#[derive(Debug, Clone, Serialize, Deserialize, FromRow)]
pub struct WebhookEvent {
    pub id: i64,
    pub delivery_id: String,
    pub event_type: String,
    pub installation_id: Option<i64>,
    pub repository_id: Option<i64>,
    pub payload: Value,
    pub processed_at: Option<OffsetDateTime>,
    pub processing_error: Option<String>,
    pub created_at: OffsetDateTime,
}

#[derive(Debug, Clone, Serialize, Deserialize, FromRow)]
pub struct Job {
    pub id: i64,
    pub kind: String,
    pub payload: Value,
    pub status: String,
    pub attempts: i32,
    pub max_attempts: i32,
    pub run_at: OffsetDateTime,
    pub locked_by: Option<String>,
    pub locked_at: Option<OffsetDateTime>,
    pub last_error: Option<String>,
    pub completed_at: Option<OffsetDateTime>,
    pub created_at: OffsetDateTime,
    pub updated_at: OffsetDateTime,
}

pub async fn upsert_installation(
    pool: &PgPool,
    installation_id: i64,
    account_login: &str,
    account_id: Option<i64>,
    account_type: Option<&str>,
    raw: Value,
) -> Result<GithubInstallation> {
    sqlx::query_as::<_, GithubInstallation>("INSERT INTO github_installations (installation_id, account_login, account_id, account_type, raw) VALUES ($1,$2,$3,$4,$5) ON CONFLICT (installation_id) DO UPDATE SET account_login=EXCLUDED.account_login, account_id=EXCLUDED.account_id, account_type=EXCLUDED.account_type, raw=EXCLUDED.raw, updated_at=now() RETURNING *")
        .bind(installation_id).bind(account_login).bind(account_id).bind(account_type).bind(raw).fetch_one(pool).await
}

#[allow(clippy::too_many_arguments)]
pub async fn upsert_repository(
    pool: &PgPool,
    repository_id: i64,
    installation_id: i64,
    owner: &str,
    name: &str,
    full_name: &str,
    private: bool,
    default_branch: Option<&str>,
    raw: Value,
) -> Result<GithubRepository> {
    sqlx::query_as::<_, GithubRepository>("INSERT INTO github_repositories (repository_id, installation_id, owner, name, full_name, private, default_branch, raw) VALUES ($1,$2,$3,$4,$5,$6,$7,$8) ON CONFLICT (repository_id) DO UPDATE SET installation_id=EXCLUDED.installation_id, owner=EXCLUDED.owner, name=EXCLUDED.name, full_name=EXCLUDED.full_name, private=EXCLUDED.private, default_branch=EXCLUDED.default_branch, raw=EXCLUDED.raw, updated_at=now() RETURNING *")
        .bind(repository_id).bind(installation_id).bind(owner).bind(name).bind(full_name).bind(private).bind(default_branch).bind(raw).fetch_one(pool).await
}

pub async fn get_repository(pool: &PgPool, repository_id: i64) -> Result<Option<GithubRepository>> {
    sqlx::query_as::<_, GithubRepository>(
        "SELECT * FROM github_repositories WHERE repository_id=$1",
    )
    .bind(repository_id)
    .fetch_optional(pool)
    .await
}

pub async fn list_repositories(
    pool: &PgPool,
    installation_id: i64,
) -> Result<Vec<GithubRepository>> {
    sqlx::query_as::<_, GithubRepository>(
        "SELECT * FROM github_repositories WHERE installation_id=$1 ORDER BY full_name",
    )
    .bind(installation_id)
    .fetch_all(pool)
    .await
}

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

pub async fn list_bot_artifacts_for_user(
    pool: &PgPool,
    repository_id: i64,
    github_user_id: i64,
    artifact_type: &str,
) -> Result<Vec<BotArtifact>> {
    sqlx::query_as::<_, BotArtifact>("SELECT a.* FROM bot_artifacts a INNER JOIN verification_sessions s ON s.repository_id=a.repository_id AND s.subject_type=a.subject_type AND s.subject_id=a.subject_id WHERE a.repository_id=$1 AND s.github_user_id=$2 AND a.artifact_type=$3 ORDER BY a.created_at")
        .bind(repository_id)
        .bind(github_user_id)
        .bind(artifact_type)
        .fetch_all(pool)
        .await
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

pub async fn enqueue_job(
    pool: &PgPool,
    kind: &str,
    payload: Value,
    run_at: Option<OffsetDateTime>,
    max_attempts: i32,
) -> Result<Job> {
    sqlx::query_as::<_, Job>("INSERT INTO jobs (kind,payload,run_at,max_attempts) VALUES ($1,$2,COALESCE($3, now()),$4) RETURNING *").bind(kind).bind(payload).bind(run_at).bind(max_attempts).fetch_one(pool).await
}

pub async fn claim_job(pool: &PgPool, worker: &str) -> Result<Option<Job>> {
    sqlx::query_as::<_, Job>("UPDATE jobs SET status='running', attempts=attempts+1, locked_by=$1, locked_at=now(), updated_at=now() WHERE id = (SELECT id FROM jobs WHERE status='queued' AND run_at <= now() AND attempts < max_attempts ORDER BY run_at, id FOR UPDATE SKIP LOCKED LIMIT 1) RETURNING *").bind(worker).fetch_optional(pool).await
}

pub async fn complete_job(pool: &PgPool, id: i64) -> Result<Option<Job>> {
    sqlx::query_as::<_, Job>("UPDATE jobs SET status='completed', completed_at=now(), locked_by=NULL, locked_at=NULL, updated_at=now() WHERE id=$1 RETURNING *").bind(id).fetch_optional(pool).await
}

pub async fn fail_job(
    pool: &PgPool,
    id: i64,
    error: &str,
    retry_at: Option<OffsetDateTime>,
) -> Result<Option<Job>> {
    sqlx::query_as::<_, Job>("UPDATE jobs SET status=CASE WHEN attempts >= max_attempts THEN 'failed' ELSE 'queued' END, last_error=$2, run_at=COALESCE($3, now()), locked_by=NULL, locked_at=NULL, updated_at=now() WHERE id=$1 RETURNING *").bind(id).bind(error).bind(retry_at).fetch_optional(pool).await
}
