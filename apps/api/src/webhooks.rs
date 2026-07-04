use axum::{
    body::Bytes,
    extract::State,
    http::{HeaderMap, StatusCode},
    response::{IntoResponse, Response},
    routing::post,
    Json, Router,
};
use base64::{engine::general_purpose::URL_SAFE_NO_PAD, Engine as _};
use github::{CheckRunRequest, GitHubApi, ReqwestGitHubClient};
use hmac::{Hmac, Mac};
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
use sha2::Sha256;
use thiserror::Error;
use time::OffsetDateTime;
use tracing::{error, info};

use crate::{AppState, ErrorBody, ErrorDetail};

type HmacSha256 = Hmac<Sha256>;

pub fn routes() -> Router<AppState> {
    Router::new().route("/api/github/webhook", post(handle_github_webhook))
}

#[derive(Debug, Serialize)]
pub struct WebhookResponse {
    status: &'static str,
}

pub async fn handle_github_webhook(
    State(state): State<AppState>,
    headers: HeaderMap,
    body: Bytes,
) -> Result<Json<WebhookResponse>, WebhookError> {
    let secret = state
        .config
        .github_webhook_secret
        .as_deref()
        .ok_or(WebhookError::MissingSecret)?;
    verify_signature(secret, &headers, &body)?;

    let delivery_id = header_str(&headers, "x-github-delivery")?.to_owned();
    let event_type = header_str(&headers, "x-github-event")?.to_owned();
    let payload: Value = serde_json::from_slice(&body).map_err(|_| WebhookError::InvalidJson)?;
    let installation_id = payload.pointer("/installation/id").and_then(Value::as_i64);
    let repository_id = payload.pointer("/repository/id").and_then(Value::as_i64);

    let pool = state.db.as_ref().ok_or(WebhookError::DbUnavailable)?;
    let inserted = db::insert_webhook_event(
        pool,
        &delivery_id,
        &event_type,
        installation_id,
        repository_id,
        payload_summary(&payload),
    )
    .await
    .map_err(WebhookError::Db)?;

    if inserted.is_none() {
        info!(%delivery_id, %event_type, "duplicate github webhook delivery ignored");
        return Ok(Json(WebhookResponse {
            status: "duplicate",
        }));
    }

    let processing = process_event(&state, pool, &event_type, &payload).await;
    match processing {
        Ok(()) => {
            db::mark_webhook_processed(pool, &delivery_id, None)
                .await
                .map_err(WebhookError::Db)?;
            db::insert_audit(
                pool,
                Some("github_webhook"),
                "github.webhook.processed",
                repository_id,
                None,
                None,
                json!({"delivery_id": delivery_id, "event_type": event_type, "installation_id": installation_id}),
            )
            .await
            .map_err(WebhookError::Db)?;
            Ok(Json(WebhookResponse {
                status: "processed",
            }))
        }
        Err(err) => {
            let message = err.to_string();
            error!(%delivery_id, %event_type, error = %message, "github webhook processing failed");
            db::mark_webhook_processed(pool, &delivery_id, Some(&message))
                .await
                .map_err(WebhookError::Db)?;
            Err(err)
        }
    }
}

async fn process_event(
    state: &AppState,
    pool: &db::PgPool,
    event_type: &str,
    payload: &Value,
) -> Result<(), WebhookError> {
    match event_type {
        "installation" => {
            upsert_installation_from_payload(pool, payload).await?;
            let installation_id = payload.pointer("/installation/id").and_then(Value::as_i64);
            match payload.get("action").and_then(Value::as_str) {
                Some("deleted") => {
                    if let Some(id) = installation_id {
                        db::mark_installation_deleted(pool, id)
                            .await
                            .map_err(WebhookError::Db)?;
                    }
                }
                Some("suspend") => {
                    if let Some(id) = installation_id {
                        db::mark_installation_suspended(pool, id, true)
                            .await
                            .map_err(WebhookError::Db)?;
                    }
                }
                Some("unsuspend") => {
                    if let Some(id) = installation_id {
                        db::mark_installation_suspended(pool, id, false)
                            .await
                            .map_err(WebhookError::Db)?;
                    }
                }
                _ => {}
            }
            if let Some(repos) = payload.get("repositories").and_then(Value::as_array) {
                for repo in repos {
                    upsert_repository_from_value(pool, repo, installation_id).await?;
                }
            }
        }
        "repositories" | "installation_repositories" => {
            upsert_installation_from_payload(pool, payload).await?;
            let installation_id = payload.pointer("/installation/id").and_then(Value::as_i64);
            for key in ["repositories", "repositories_added"] {
                if let Some(repos) = payload.get(key).and_then(Value::as_array) {
                    for repo in repos {
                        upsert_repository_from_value(pool, repo, installation_id).await?;
                    }
                }
            }
        }
        "issues" => {
            upsert_installation_from_payload(pool, payload).await?;
            if let Some(repo) = payload.get("repository") {
                let installation_id = payload.pointer("/installation/id").and_then(Value::as_i64);
                upsert_repository_from_value(pool, repo, installation_id).await?;
            }
            let event: IssueLikeEvent =
                serde_json::from_value(payload.clone()).map_err(|_| WebhookError::InvalidJson)?;
            if matches!(event.action.as_str(), "opened" | "reopened") {
                process_subject_event(state, pool, event.into_subject()).await?;
            }
        }
        "pull_request" => {
            upsert_installation_from_payload(pool, payload).await?;
            if let Some(repo) = payload.get("repository") {
                let installation_id = payload.pointer("/installation/id").and_then(Value::as_i64);
                upsert_repository_from_value(pool, repo, installation_id).await?;
            }
            let event: PrLikeEvent =
                serde_json::from_value(payload.clone()).map_err(|_| WebhookError::InvalidJson)?;
            if matches!(event.action.as_str(), "opened" | "reopened" | "synchronize") {
                process_subject_event(state, pool, event.into_subject()).await?;
            }
        }
        _ => {}
    }
    Ok(())
}

#[derive(Debug, Deserialize)]
struct HookUser {
    id: i64,
    login: String,
    #[serde(rename = "type")]
    kind: Option<String>,
}
#[derive(Debug, Deserialize)]
struct HookRepo {
    id: i64,
}
#[derive(Debug, Deserialize)]
struct HookInstallation {
    id: u64,
}
#[derive(Debug, Deserialize)]
struct HookIssue {
    number: u64,
    html_url: String,
    user: HookUser,
}
#[derive(Debug, Deserialize)]
struct HookPrHead {
    sha: String,
}
#[derive(Debug, Deserialize)]
struct HookPr {
    number: u64,
    html_url: String,
    user: HookUser,
    head: HookPrHead,
}
#[derive(Debug, Deserialize)]
struct IssueLikeEvent {
    action: String,
    issue: HookIssue,
    repository: HookRepo,
    installation: Option<HookInstallation>,
}
#[derive(Debug, Deserialize)]
struct PrLikeEvent {
    action: String,
    pull_request: HookPr,
    repository: HookRepo,
    installation: Option<HookInstallation>,
}

struct SubjectEvent {
    target: policy::TargetKind,
    subject_type: &'static str,
    number: u64,
    html_url: String,
    head_sha: Option<String>,
    user: HookUser,
    repo: HookRepo,
    installation_id: Option<u64>,
}
impl IssueLikeEvent {
    fn into_subject(self) -> SubjectEvent {
        SubjectEvent {
            target: policy::TargetKind::Issue,
            subject_type: "issue",
            number: self.issue.number,
            html_url: self.issue.html_url,
            head_sha: None,
            user: self.issue.user,
            repo: self.repository,
            installation_id: self.installation.map(|i| i.id),
        }
    }
}
impl PrLikeEvent {
    fn into_subject(self) -> SubjectEvent {
        SubjectEvent {
            target: policy::TargetKind::PullRequest,
            subject_type: "pull_request",
            number: self.pull_request.number,
            html_url: self.pull_request.html_url,
            head_sha: Some(self.pull_request.head.sha),
            user: self.pull_request.user,
            repo: self.repository,
            installation_id: self.installation.map(|i| i.id),
        }
    }
}

async fn process_subject_event(
    state: &AppState,
    pool: &db::PgPool,
    ev: SubjectEvent,
) -> Result<(), WebhookError> {
    let repo = match db::get_repository(pool, ev.repo.id)
        .await
        .map_err(WebhookError::Db)?
    {
        Some(r) => r,
        None => return Ok(()),
    };
    let policy: policy::VerificationPolicy = match db::get_policy(pool, repo.repository_id)
        .await
        .map_err(WebhookError::Db)?
    {
        Some(p) if p.enabled => serde_json::from_value(p.policy).unwrap_or_default(),
        Some(_) => return Ok(()),
        None => return Ok(()),
    };

    let app_id = state
        .config
        .github_app_id
        .as_deref()
        .ok_or(WebhookError::GitHubNotConfigured)?;
    let private_key = state
        .config
        .github_private_key
        .as_deref()
        .ok_or(WebhookError::GitHubNotConfigured)?;
    let jwt = github::create_app_jwt(app_id, private_key)
        .map_err(|_| WebhookError::GitHubNotConfigured)?;
    let client = ReqwestGitHubClient::with_base_url(&state.config.github_api_base);
    if ev
        .installation_id
        .is_some_and(|id| id as i64 != repo.installation_id)
    {
        return Err(WebhookError::InstallationMismatch);
    }
    let installation_id = ev.installation_id.unwrap_or(repo.installation_id as u64);
    let token = client
        .exchange_installation_token(&jwt, installation_id)
        .await
        .map_err(WebhookError::GitHub)?
        .token;

    let is_bot = matches!(ev.user.kind.as_deref(), Some("Bot"));
    let is_app = matches!(ev.user.kind.as_deref(), Some("App"));
    let perm = client
        .collaborator_permission(&token, &repo.owner, &repo.name, &ev.user.login)
        .await
        .ok();
    let is_collaborator = perm
        .as_ref()
        .is_some_and(|p| matches!(p.permission.as_str(), "admin" | "maintain" | "write"));
    let trust = db::get_trusted_subject(
        pool,
        repo.repository_id,
        "github_user",
        &ev.user.id.to_string(),
    )
    .await
    .map_err(WebhookError::Db)?;
    let now = OffsetDateTime::now_utc().unix_timestamp();
    let input = policy::DecisionInput {
        target: ev.target,
        subject: policy::Subject {
            login: ev.user.login.clone(),
            github_user_id: Some(ev.user.id),
            is_collaborator,
            is_bot,
            is_app,
        },
        trust: trust
            .as_ref()
            .map(|t| policy::TrustState {
                trusted: t.trusted,
                manually_exempt: t
                    .metadata
                    .get("source")
                    .and_then(|v| v.as_str())
                    .is_some_and(|source| source == "manual_allowlist"),
                trusted_at: Some(t.trusted_at.unix_timestamp()),
                expires_at: t.expires_at.map(|x| x.unix_timestamp()),
            })
            .unwrap_or_default(),
        now,
    };
    let decision = policy::decide(&policy, &input);
    db::insert_audit(pool, Some("github_webhook"), "github.webhook.decision", Some(repo.repository_id), Some(ev.subject_type), Some(&ev.number.to_string()), json!({"reason": decision.reason, "required": decision.required, "allowed": decision.allowed})).await.map_err(WebhookError::Db)?;
    if !decision.required {
        // If the author is already trusted/exempt when an issue or PR is opened/reopened,
        // still reflect that state on GitHub so maintainers can filter/sort by label.
        // Previously the verified label was only added to subjects that first entered
        // the pending flow and were later verified through OAuth/CAPTCHA.
        for action in &decision.actions {
            match action {
                policy::PolicyAction::AddLabel(label) => {
                    let _ = client
                        .add_labels(&token, &repo.owner, &repo.name, ev.number, &[label.clone()])
                        .await;
                }
                policy::PolicyAction::RemoveLabel(label) => {
                    let _ = client
                        .remove_label(&token, &repo.owner, &repo.name, ev.number, label)
                        .await;
                }
                _ => {}
            }
        }
        return Ok(());
    }

    db::upsert_github_user(
        pool,
        ev.user.id,
        &ev.user.login,
        None,
        json!({"login": ev.user.login, "type": ev.user.kind}),
    )
    .await
    .map_err(WebhookError::Db)?;

    let verify_url = source_verify_url(
        state,
        repo.repository_id,
        ev.subject_type,
        ev.number,
        ev.user.id,
        &ev.user.login,
        &ev.html_url,
    )?;

    for label in [policy.apply_label.as_ref(), policy.pending_label.as_ref()]
        .into_iter()
        .flatten()
    {
        let _ = client
            .add_labels(&token, &repo.owner, &repo.name, ev.number, &[label.clone()])
            .await;
    }
    if policy.comment_on_required {
        let body = format!("Human verification is required. Verify here: {verify_url}");
        let artifact = db::get_bot_artifact(
            pool,
            repo.repository_id,
            ev.subject_type,
            &ev.number.to_string(),
            "comment",
        )
        .await
        .map_err(WebhookError::Db)?;
        let comment = if let Some(a) =
            artifact.and_then(|a| a.external_id.and_then(|id| id.parse::<u64>().ok()))
        {
            client
                .update_issue_comment(&token, &repo.owner, &repo.name, a, &body)
                .await
        } else {
            client
                .create_issue_comment(&token, &repo.owner, &repo.name, ev.number, &body)
                .await
        };
        if let Ok(c) = comment {
            db::upsert_bot_artifact(
                pool,
                repo.repository_id,
                ev.subject_type,
                &ev.number.to_string(),
                "comment",
                Some(&c.id.to_string()),
                json!({"url": c.html_url, "source_url": verify_url}),
            )
            .await
            .map_err(WebhookError::Db)?;
        }
    }
    if ev.target == policy::TargetKind::PullRequest && policy.check_mode != policy::CheckMode::Off {
        if let Some(sha) = ev.head_sha {
            let req = CheckRunRequest {
                name: "Human Auth".into(),
                head_sha: sha,
                status: Some("completed".into()),
                conclusion: Some(
                    if policy.check_mode == policy::CheckMode::Audit {
                        "neutral"
                    } else {
                        "action_required"
                    }
                    .into(),
                ),
                details_url: Some(verify_url.clone()),
                output: Some(
                    json!({"title":"Human verification required","summary":"Complete verification to proceed."}),
                ),
            };
            let artifact = db::get_bot_artifact(
                pool,
                repo.repository_id,
                ev.subject_type,
                &ev.number.to_string(),
                "check_run",
            )
            .await
            .map_err(WebhookError::Db)?;
            let check = if let Some(a) =
                artifact.and_then(|a| a.external_id.and_then(|id| id.parse::<u64>().ok()))
            {
                client
                    .update_check_run(&token, &repo.owner, &repo.name, a, &req)
                    .await
            } else {
                client
                    .create_check_run(&token, &repo.owner, &repo.name, &req)
                    .await
            };
            if let Ok(c) = check {
                db::upsert_bot_artifact(
                    pool,
                    repo.repository_id,
                    ev.subject_type,
                    &ev.number.to_string(),
                    "check_run",
                    Some(&c.id.to_string()),
                    json!({"sha": c.head_sha, "source_url": verify_url}),
                )
                .await
                .map_err(WebhookError::Db)?;
            }
        }
    }
    Ok(())
}

/// GitHub payloads can include private repository content (issue/PR titles and bodies).
/// Persist only the minimal routing fields so sensitive data never lands in the database.
fn payload_summary(payload: &Value) -> Value {
    json!({
        "action": payload.get("action"),
        "installation_id": payload.pointer("/installation/id"),
        "repository_id": payload.pointer("/repository/id"),
        "repository": payload.pointer("/repository/full_name"),
        "sender": payload.pointer("/sender/login"),
        "issue_number": payload.pointer("/issue/number"),
        "pull_request_number": payload.pointer("/pull_request/number"),
    })
}

async fn upsert_installation_from_payload(
    pool: &db::PgPool,
    payload: &Value,
) -> Result<(), WebhookError> {
    let installation = payload
        .get("installation")
        .ok_or(WebhookError::MissingField("installation"))?;
    let installation_id = installation
        .get("id")
        .and_then(Value::as_i64)
        .ok_or(WebhookError::MissingField("installation.id"))?;
    let account = installation.get("account").unwrap_or(&Value::Null);
    let account_login = account
        .get("login")
        .and_then(Value::as_str)
        .unwrap_or("unknown");
    let account_id = account.get("id").and_then(Value::as_i64);
    let account_type = account.get("type").and_then(Value::as_str);
    db::upsert_installation(
        pool,
        installation_id,
        account_login,
        account_id,
        account_type,
        json!({
            "id": installation_id,
            "account": {"login": account_login, "id": account_id, "type": account_type},
            "app_id": installation.get("app_id"),
        }),
    )
    .await
    .map_err(WebhookError::Db)?;
    Ok(())
}

async fn upsert_repository_from_value(
    pool: &db::PgPool,
    repo: &Value,
    installation_id: Option<i64>,
) -> Result<(), WebhookError> {
    let repository_id = repo
        .get("id")
        .and_then(Value::as_i64)
        .ok_or(WebhookError::MissingField("repository.id"))?;
    let installation_id = installation_id.ok_or(WebhookError::MissingField("installation.id"))?;
    let full_name = repo
        .get("full_name")
        .and_then(Value::as_str)
        .ok_or(WebhookError::MissingField("repository.full_name"))?;
    let name = repo
        .get("name")
        .and_then(Value::as_str)
        .unwrap_or_else(|| full_name.rsplit('/').next().unwrap_or(full_name));
    let owner = repo
        .pointer("/owner/login")
        .and_then(Value::as_str)
        .unwrap_or_else(|| full_name.split('/').next().unwrap_or("unknown"));
    let private = repo
        .get("private")
        .and_then(Value::as_bool)
        .unwrap_or(false);
    let default_branch = repo.get("default_branch").and_then(Value::as_str);
    db::upsert_repository(
        pool,
        repository_id,
        installation_id,
        owner,
        name,
        full_name,
        private,
        default_branch,
        json!({
            "id": repository_id,
            "full_name": full_name,
            "private": private,
            "default_branch": default_branch,
            "owner": {"login": owner},
        }),
    )
    .await
    .map_err(WebhookError::Db)?;
    Ok(())
}

fn source_verify_url(
    state: &AppState,
    repository_id: i64,
    subject_type: &str,
    number: u64,
    github_user_id: i64,
    login: &str,
    subject_url: &str,
) -> Result<String, WebhookError> {
    let secret = state
        .config
        .admin_session_secret
        .as_ref()
        .or(state.config.github_webhook_secret.as_ref())
        .ok_or(WebhookError::GitHubNotConfigured)?;
    let payload =
        format!("{repository_id}|{subject_type}|{number}|{github_user_id}|{login}|{subject_url}");
    let mut mac = HmacSha256::new_from_slice(secret.as_bytes())
        .map_err(|_| WebhookError::GitHubNotConfigured)?;
    mac.update(payload.as_bytes());
    let sig = URL_SAFE_NO_PAD.encode(mac.finalize().into_bytes());
    Ok(format!(
        "{}/verify/source?repo={repository_id}&type={}&number={number}&user_id={github_user_id}&login={}&url={}&sig={sig}",
        state.config.web_base_url.trim_end_matches('/'),
        urlencoding::encode(subject_type),
        urlencoding::encode(login),
        urlencoding::encode(subject_url),
    ))
}

fn verify_signature(secret: &str, headers: &HeaderMap, body: &[u8]) -> Result<(), WebhookError> {
    let signature = header_str(headers, "x-hub-signature-256")?;
    let expected = signature
        .strip_prefix("sha256=")
        .ok_or(WebhookError::InvalidSignature)?;
    let mut mac = HmacSha256::new_from_slice(secret.as_bytes())
        .map_err(|_| WebhookError::InvalidSignature)?;
    mac.update(body);
    let actual = hex::encode(mac.finalize().into_bytes());
    if constant_time_eq(actual.as_bytes(), expected.as_bytes()) {
        Ok(())
    } else {
        Err(WebhookError::InvalidSignature)
    }
}

fn header_str<'a>(headers: &'a HeaderMap, name: &'static str) -> Result<&'a str, WebhookError> {
    headers
        .get(name)
        .and_then(|value| value.to_str().ok())
        .ok_or(WebhookError::MissingHeader(name))
}

fn constant_time_eq(a: &[u8], b: &[u8]) -> bool {
    if a.len() != b.len() {
        return false;
    }
    a.iter().zip(b).fold(0u8, |acc, (x, y)| acc | (x ^ y)) == 0
}

#[derive(Debug, Error)]
pub enum WebhookError {
    #[error("github webhook secret is not configured")]
    MissingSecret,
    #[error("database pool is not configured")]
    DbUnavailable,
    #[error("missing required header {0}")]
    MissingHeader(&'static str),
    #[error("invalid webhook signature")]
    InvalidSignature,
    #[error("invalid json payload")]
    InvalidJson,
    #[error("missing required field {0}")]
    MissingField(&'static str),
    #[error("GitHub app credentials are not configured")]
    GitHubNotConfigured,
    #[error("webhook installation does not match stored repository installation")]
    InstallationMismatch,
    #[error("GitHub API error: {0}")]
    GitHub(#[from] github::GitHubError),
    #[error("database error")]
    Db(#[from] sqlx::Error),
}

impl IntoResponse for WebhookError {
    fn into_response(self) -> Response {
        let (status, code) = match self {
            WebhookError::MissingSecret
            | WebhookError::DbUnavailable
            | WebhookError::GitHubNotConfigured => {
                (StatusCode::SERVICE_UNAVAILABLE, "service_unavailable")
            }
            WebhookError::MissingHeader(_) | WebhookError::InvalidSignature => {
                (StatusCode::UNAUTHORIZED, "invalid_signature")
            }
            WebhookError::InvalidJson
            | WebhookError::MissingField(_)
            | WebhookError::InstallationMismatch => (StatusCode::BAD_REQUEST, "bad_request"),
            WebhookError::GitHub(_) | WebhookError::Db(_) => {
                (StatusCode::INTERNAL_SERVER_ERROR, "internal_error")
            }
        };
        let body = Json(ErrorBody {
            error: ErrorDetail {
                code: code.to_owned(),
                message: self.to_string(),
            },
        });
        (status, body).into_response()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use axum::{body::Body, http::Request};
    use tower::ServiceExt;

    fn test_state(secret: Option<&str>) -> AppState {
        AppState::without_db(crate::Config {
            host: "127.0.0.1".into(),
            port: 8080,
            database_url: "postgres://user:pass@localhost/db".into(),
            cors_allowed_origins: vec!["http://localhost:3000".into()],
            cookie_secure: true,
            session_cookie_name: "gho_session".into(),
            github_webhook_secret: secret.map(str::to_owned),
            github_app_id: None,
            github_private_key: None,
            github_web_url: "http://localhost:3000".into(),
            github_api_base: "https://api.github.com".into(),
            github_oauth_client_id: None,
            github_oauth_client_secret: None,
            api_base_url: "http://127.0.0.1:8080".into(),
            web_base_url: "http://localhost:3000".into(),
            turnstile_secret: None,
            turnstile_site_key: None,
            hcaptcha_secret: None,
            hcaptcha_site_key: None,
            recaptcha_secret: None,
            recaptcha_site_key: None,
            turnstile_dev_bypass: false,
            admin_api_token: None,
            admin_github_logins: vec![],
            admin_session_cookie_name: "gho_admin_session".into(),
            admin_session_secret: None,
            secrets_encryption_key: None,
            trust_proxy_headers: false,
            hosted_mode: false,
            github_app_slug: None,
        })
    }

    fn signature(secret: &str, body: &[u8]) -> String {
        let mut mac = HmacSha256::new_from_slice(secret.as_bytes()).unwrap();
        mac.update(body);
        format!("sha256={}", hex::encode(mac.finalize().into_bytes()))
    }

    #[tokio::test]
    async fn rejects_bad_signature_before_db() {
        let app = routes().with_state(test_state(Some("secret")));
        let res = app
            .oneshot(
                Request::builder()
                    .method("POST")
                    .uri("/api/github/webhook")
                    .header("x-github-delivery", "delivery-1")
                    .header("x-github-event", "installation")
                    .header("x-hub-signature-256", "sha256=bad")
                    .body(Body::from("{}"))
                    .unwrap(),
            )
            .await
            .unwrap();
        assert_eq!(res.status(), StatusCode::UNAUTHORIZED);
    }

    #[tokio::test]
    async fn accepts_signature_then_reports_missing_db() {
        let body = br#"{"installation":{"id":1}}"#;
        let app = routes().with_state(test_state(Some("secret")));
        let res = app
            .oneshot(
                Request::builder()
                    .method("POST")
                    .uri("/api/github/webhook")
                    .header("x-github-delivery", "delivery-1")
                    .header("x-github-event", "installation")
                    .header("x-hub-signature-256", signature("secret", body))
                    .body(Body::from(&body[..]))
                    .unwrap(),
            )
            .await
            .unwrap();
        assert_eq!(res.status(), StatusCode::SERVICE_UNAVAILABLE);
    }
}
