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
    pub raw: Value,
    pub deleted_at: Option<OffsetDateTime>,
    pub suspended_at: Option<OffsetDateTime>,
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
    pub active: bool,
    pub created_at: OffsetDateTime,
    pub updated_at: OffsetDateTime,
}

#[derive(Debug, Clone, Serialize, Deserialize, FromRow)]
pub struct InstallationAdmin {
    pub id: i64,
    pub installation_id: i64,
    pub github_user_id: i64,
    pub login: String,
    pub role: String,
    pub created_at: OffsetDateTime,
    pub updated_at: OffsetDateTime,
}

#[derive(Debug, Clone, Serialize, Deserialize, FromRow)]
pub struct DashboardOauthToken {
    pub github_user_id: i64,
    pub login: String,
    pub access_token_encrypted: String,
    pub updated_at: OffsetDateTime,
    pub created_at: OffsetDateTime,
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
    pub dedupe_key: Option<String>,
    pub priority: i32,
    pub available_after_rate_limit: bool,
    pub rate_limit_reset_at: Option<OffsetDateTime>,
}

#[derive(Debug, Clone, Serialize, Deserialize, FromRow)]
pub struct BackfillRun {
    pub id: i64,
    pub repository_id: i64,
    pub requested_by_github_user_id: Option<i64>,
    pub requested_by_login: Option<String>,
    pub include_issues: bool,
    pub include_pull_requests: bool,
    pub notify_authors: bool,
    pub force_new_comments: bool,
    pub status: String,
    pub total_discovered: i32,
    pub total_enqueued: i32,
    pub total_processed: i32,
    pub total_succeeded: i32,
    pub total_failed: i32,
    pub total_skipped: i32,
    pub current_phase: Option<String>,
    pub last_error: Option<String>,
    pub started_at: Option<OffsetDateTime>,
    pub completed_at: Option<OffsetDateTime>,
    pub cancelled_at: Option<OffsetDateTime>,
    pub created_at: OffsetDateTime,
    pub updated_at: OffsetDateTime,
}

#[derive(Debug, Clone, Serialize, Deserialize, FromRow)]
pub struct BackfillItem {
    pub id: i64,
    pub backfill_run_id: i64,
    pub repository_id: i64,
    pub subject_type: String,
    pub subject_id: String,
    pub github_user_id: Option<i64>,
    pub login: Option<String>,
    pub html_url: Option<String>,
    pub head_sha: Option<String>,
    pub status: String,
    pub decision_required: Option<bool>,
    pub decision_reason: Option<String>,
    pub attempts: i32,
    pub error: Option<String>,
    pub started_at: Option<OffsetDateTime>,
    pub processed_at: Option<OffsetDateTime>,
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
    sqlx::query_as::<_, GithubInstallation>("INSERT INTO github_installations (installation_id, account_login, account_id, account_type, raw) VALUES ($1,$2,$3,$4,$5) ON CONFLICT (installation_id) DO UPDATE SET account_login=EXCLUDED.account_login, account_id=EXCLUDED.account_id, account_type=EXCLUDED.account_type, raw=EXCLUDED.raw, deleted_at=NULL, suspended_at=NULL, updated_at=now() RETURNING *")
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
    sqlx::query_as::<_, GithubRepository>("INSERT INTO github_repositories (repository_id, installation_id, owner, name, full_name, private, default_branch, raw, active) VALUES ($1,$2,$3,$4,$5,$6,$7,$8,true) ON CONFLICT (repository_id) DO UPDATE SET installation_id=EXCLUDED.installation_id, owner=EXCLUDED.owner, name=EXCLUDED.name, full_name=EXCLUDED.full_name, private=EXCLUDED.private, default_branch=EXCLUDED.default_branch, raw=EXCLUDED.raw, active=true, updated_at=now() RETURNING *")
        .bind(repository_id).bind(installation_id).bind(owner).bind(name).bind(full_name).bind(private).bind(default_branch).bind(raw).fetch_one(pool).await
}

pub async fn get_repository(pool: &PgPool, repository_id: i64) -> Result<Option<GithubRepository>> {
    sqlx::query_as::<_, GithubRepository>(
        "SELECT * FROM github_repositories WHERE repository_id=$1 AND active=true",
    )
    .bind(repository_id)
    .fetch_optional(pool)
    .await
}

pub async fn mark_installation_deleted(pool: &PgPool, installation_id: i64) -> Result<()> {
    sqlx::query("UPDATE github_installations SET deleted_at=now(), updated_at=now() WHERE installation_id=$1")
        .bind(installation_id)
        .execute(pool)
        .await?;
    sqlx::query(
        "UPDATE github_repositories SET active=false, updated_at=now() WHERE installation_id=$1",
    )
    .bind(installation_id)
    .execute(pool)
    .await?;
    Ok(())
}

pub async fn mark_installation_suspended(
    pool: &PgPool,
    installation_id: i64,
    suspended: bool,
) -> Result<()> {
    let sql = if suspended {
        "UPDATE github_installations SET suspended_at=now(), updated_at=now() WHERE installation_id=$1"
    } else {
        "UPDATE github_installations SET suspended_at=NULL, updated_at=now() WHERE installation_id=$1"
    };
    sqlx::query(sql).bind(installation_id).execute(pool).await?;
    Ok(())
}

pub async fn upsert_installation_admin(
    pool: &PgPool,
    installation_id: i64,
    github_user_id: i64,
    login: &str,
) -> Result<InstallationAdmin> {
    sqlx::query_as::<_, InstallationAdmin>("INSERT INTO installation_admins (installation_id, github_user_id, login) VALUES ($1,$2,$3) ON CONFLICT (installation_id, github_user_id) DO UPDATE SET login=EXCLUDED.login, updated_at=now() RETURNING *")
        .bind(installation_id)
        .bind(github_user_id)
        .bind(login)
        .fetch_one(pool)
        .await
}

pub async fn user_can_manage_installation(
    pool: &PgPool,
    installation_id: i64,
    github_user_id: i64,
) -> Result<bool> {
    let (exists,): (bool,) = sqlx::query_as("SELECT EXISTS(SELECT 1 FROM installation_admins WHERE installation_id=$1 AND github_user_id=$2)")
        .bind(installation_id)
        .bind(github_user_id)
        .fetch_one(pool)
        .await?;
    Ok(exists)
}

pub async fn user_can_manage_repo(
    pool: &PgPool,
    repository_id: i64,
    github_user_id: i64,
) -> Result<bool> {
    let (exists,): (bool,) = sqlx::query_as("SELECT EXISTS(SELECT 1 FROM github_repositories r JOIN installation_admins a ON a.installation_id=r.installation_id WHERE r.repository_id=$1 AND r.active=true AND a.github_user_id=$2)")
        .bind(repository_id)
        .bind(github_user_id)
        .fetch_one(pool)
        .await?;
    Ok(exists)
}

pub async fn upsert_dashboard_oauth_token(
    pool: &PgPool,
    github_user_id: i64,
    login: &str,
    access_token_encrypted: &str,
) -> Result<DashboardOauthToken> {
    sqlx::query_as::<_, DashboardOauthToken>("INSERT INTO dashboard_oauth_tokens (github_user_id, login, access_token_encrypted) VALUES ($1,$2,$3) ON CONFLICT (github_user_id) DO UPDATE SET login=EXCLUDED.login, access_token_encrypted=EXCLUDED.access_token_encrypted, updated_at=now() RETURNING *")
        .bind(github_user_id)
        .bind(login)
        .bind(access_token_encrypted)
        .fetch_one(pool)
        .await
}

pub async fn get_dashboard_oauth_token(
    pool: &PgPool,
    github_user_id: i64,
) -> Result<Option<DashboardOauthToken>> {
    sqlx::query_as::<_, DashboardOauthToken>(
        "SELECT * FROM dashboard_oauth_tokens WHERE github_user_id=$1",
    )
    .bind(github_user_id)
    .fetch_optional(pool)
    .await
}

pub async fn list_repositories(
    pool: &PgPool,
    installation_id: i64,
) -> Result<Vec<GithubRepository>> {
    sqlx::query_as::<_, GithubRepository>(
        "SELECT * FROM github_repositories WHERE installation_id=$1 AND active=true ORDER BY full_name",
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

pub async fn upsert_github_rate_limit(
    pool: &PgPool,
    bucket: &str,
    remaining: Option<i32>,
    reset_at: Option<OffsetDateTime>,
    paused_until: Option<OffsetDateTime>,
    status: Option<i32>,
    error: Option<&str>,
) -> Result<()> {
    sqlx::query("INSERT INTO github_rate_limits (bucket,remaining,reset_at,paused_until,last_status,last_error) VALUES ($1,$2,$3,$4,$5,$6) ON CONFLICT (bucket) DO UPDATE SET remaining=EXCLUDED.remaining, reset_at=EXCLUDED.reset_at, paused_until=EXCLUDED.paused_until, last_status=EXCLUDED.last_status, last_error=EXCLUDED.last_error, updated_at=now()")
        .bind(bucket).bind(remaining).bind(reset_at).bind(paused_until).bind(status).bind(error).execute(pool).await?;
    Ok(())
}

pub async fn github_pause_until(pool: &PgPool, bucket: &str) -> Result<Option<OffsetDateTime>> {
    let row: Option<(Option<OffsetDateTime>,)> = sqlx::query_as(
        "SELECT paused_until FROM github_rate_limits WHERE bucket=$1 AND paused_until > now()",
    )
    .bind(bucket)
    .fetch_optional(pool)
    .await?;
    Ok(row.and_then(|(x,)| x))
}
