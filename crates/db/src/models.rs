use serde::{Deserialize, Serialize};
use serde_json::Value;
use sqlx::FromRow;
use time::OffsetDateTime;
use uuid::Uuid;

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
