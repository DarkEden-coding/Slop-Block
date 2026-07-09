use axum::{
    body::Bytes,
    extract::State,
    http::{HeaderMap, StatusCode},
    response::{IntoResponse, Response},
    routing::post,
    Json, Router,
};
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
use thiserror::Error;
use tracing::{error, info};

use crate::{AppState, ErrorBody, ErrorDetail};

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

    let job_payload = json!({
        "delivery_id": delivery_id,
        "event_type": event_type,
        "installation_id": installation_id,
        "repository_id": repository_id,
        "payload": compact_processing_payload(&event_type, &payload),
    });
    let dedupe = format!("webhook:{delivery_id}");
    jobs::enqueue_deduped(
        pool,
        jobs::JobKind::GitHubWebhookDispatch,
        job_payload,
        None,
        8,
        Some(&dedupe),
        5,
    )
    .await
    .map_err(WebhookError::Db)?;

    Ok(Json(WebhookResponse { status: "accepted" }))
}

pub async fn handle_webhook_dispatch(
    state: &AppState,
    pool: &db::PgPool,
    payload: Value,
) -> anyhow::Result<()> {
    #[derive(Deserialize)]
    struct DispatchPayload {
        delivery_id: String,
        event_type: String,
        installation_id: Option<i64>,
        repository_id: Option<i64>,
        payload: Value,
    }
    let dispatch: DispatchPayload = serde_json::from_value(payload)?;
    match process_event(state, pool, &dispatch.event_type, &dispatch.payload).await {
        Ok(()) => {
            db::mark_webhook_processed(pool, &dispatch.delivery_id, None).await?;
            db::insert_audit(
                pool,
                Some("github_webhook"),
                "github.webhook.processed",
                dispatch.repository_id,
                None,
                None,
                json!({
                    "delivery_id": dispatch.delivery_id,
                    "event_type": dispatch.event_type,
                    "installation_id": dispatch.installation_id
                }),
            )
            .await?;
            Ok(())
        }
        Err(err) => {
            let message = err.to_string();
            error!(
                delivery_id = %dispatch.delivery_id,
                event_type = %dispatch.event_type,
                error = %message,
                "github webhook processing failed"
            );
            // Leave processed_at NULL so retries can recover.
            Err(anyhow::anyhow!(message))
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
            if let Some(repos) = payload
                .get("repositories_removed")
                .and_then(Value::as_array)
            {
                for repo in repos {
                    if let Some(repository_id) = repo.get("id").and_then(Value::as_i64) {
                        db::mark_repository_inactive(pool, repository_id)
                            .await
                            .map_err(WebhookError::Db)?;
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
    _state: &AppState,
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
    if ev
        .installation_id
        .is_some_and(|id| id as i64 != repo.installation_id)
    {
        return Err(WebhookError::InstallationMismatch);
    }
    let payload = serde_json::to_value(crate::github_subjects::SubjectWork {
        repository_id: repo.repository_id,
        installation_id: ev.installation_id.or(Some(repo.installation_id as u64)),
        subject_type: ev.subject_type.to_string(),
        number: ev.number,
        html_url: ev.html_url,
        head_sha: ev.head_sha,
        github_user_id: ev.user.id,
        login: ev.user.login,
        user_type: ev.user.kind,
        source: "github_webhook".into(),
        notify_author: false,
        force_new_comment: false,
    })
    .map_err(|_| WebhookError::InvalidJson)?;
    let dedupe = if ev.subject_type == "pull_request" {
        // Coalesce synchronize bursts onto one active job per PR.
        format!(
            "subject:webhook:repo:{}:pr:{}",
            repo.repository_id, ev.number
        )
    } else {
        format!(
            "subject:webhook:repo:{}:issue:{}",
            repo.repository_id, ev.number
        )
    };
    jobs::enqueue_deduped(
        pool,
        jobs::JobKind::GitHubSubjectEvent,
        payload,
        None,
        8,
        Some(&dedupe),
        10,
    )
    .await
    .map_err(WebhookError::Db)?;
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

/// Compact payload for async webhook jobs: keep routing fields, drop issue/PR bodies.
fn compact_processing_payload(event_type: &str, payload: &Value) -> Value {
    let mut out = payload.clone();
    if matches!(event_type, "issues" | "pull_request") {
        if let Some(obj) = out.get_mut("issue").and_then(Value::as_object_mut) {
            obj.remove("body");
            obj.remove("title");
        }
        if let Some(obj) = out.get_mut("pull_request").and_then(Value::as_object_mut) {
            obj.remove("body");
            obj.remove("title");
        }
    }
    out
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

fn verify_signature(secret: &str, headers: &HeaderMap, body: &[u8]) -> Result<(), WebhookError> {
    let signature = header_str(headers, "x-hub-signature-256")?;
    github::verify_webhook_signature(secret.as_bytes(), body, signature)
        .map_err(|_| WebhookError::InvalidSignature)
}

fn header_str<'a>(headers: &'a HeaderMap, name: &'static str) -> Result<&'a str, WebhookError> {
    headers
        .get(name)
        .and_then(|value| value.to_str().ok())
        .ok_or(WebhookError::MissingHeader(name))
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
    #[error("webhook installation does not match stored repository installation")]
    InstallationMismatch,
    #[error("database error")]
    Db(#[from] sqlx::Error),
}

impl IntoResponse for WebhookError {
    fn into_response(self) -> Response {
        let (status, code) = match self {
            WebhookError::MissingSecret | WebhookError::DbUnavailable => {
                (StatusCode::SERVICE_UNAVAILABLE, "service_unavailable")
            }
            WebhookError::MissingHeader(_) | WebhookError::InvalidSignature => {
                (StatusCode::UNAUTHORIZED, "invalid_signature")
            }
            WebhookError::InvalidJson
            | WebhookError::MissingField(_)
            | WebhookError::InstallationMismatch => (StatusCode::BAD_REQUEST, "bad_request"),
            WebhookError::Db(_) => (StatusCode::INTERNAL_SERVER_ERROR, "internal_error"),
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
        let mut config = crate::Config::test_fixture();
        config.cookie_secure = true;
        config.github_webhook_secret = secret.map(str::to_owned);
        AppState::without_db(config)
    }

    fn signature(secret: &str, body: &[u8]) -> String {
        use hmac::{Hmac, Mac};
        use sha2::Sha256;
        let mut mac = Hmac::<Sha256>::new_from_slice(secret.as_bytes()).unwrap();
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
