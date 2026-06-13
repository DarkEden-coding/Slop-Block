use axum::{
    extract::{Path, Query, State},
    http::{header, HeaderMap, StatusCode},
    response::{IntoResponse, Redirect, Response},
    routing::{get, post},
    Json, Router,
};
use base64::{engine::general_purpose::URL_SAFE_NO_PAD, Engine as _};
use captcha::{CaptchaProvider, PROVIDER_DEV_BYPASS};
use github::{CheckRunRequest, GitHubApi};
use hmac::{Hmac, Mac};
use policy::VerificationPolicy;
use rand::{rngs::OsRng, RngCore};
use serde::{Deserialize, Serialize};
use serde_json::json;
use sha2::{Digest, Sha256};
use time::OffsetDateTime;
use uuid::Uuid;

use crate::captcha_config::{self, SessionCaptchaConfig};
use crate::AppState;

type HmacSha256 = Hmac<Sha256>;
const OAUTH_COOKIE: &str = "gho_oauth_state";

pub fn routes() -> Router<AppState> {
    Router::new()
        .route("/api/github/oauth/start", get(oauth_start))
        .route("/api/github/oauth/callback", get(oauth_callback))
        .route("/api/verify/:session_id", get(get_verify))
        .route("/api/verify/:session_id/captcha", post(post_captcha))
}

#[derive(Deserialize)]
struct StartQuery {
    session_id: Uuid,
    token: String,
}

#[derive(Deserialize)]
struct CallbackQuery {
    code: Option<String>,
    state: Option<String>,
    error: Option<String>,
}

#[derive(Deserialize)]
struct CaptchaBody {
    token: String,
    session_token: String,
    #[serde(default)]
    provider: Option<String>,
}

#[derive(Serialize)]
struct VerifyResponse {
    session_id: Uuid,
    status: String,
    subject_type: String,
    subject_id: String,
    expires_at: i64,
    #[serde(skip_serializing_if = "Option::is_none")]
    repo: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    github_login: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    oauth_url: Option<String>,
    captcha_required: bool,
    oauth_required: bool,
    oauth_verified: bool,
    #[serde(skip_serializing_if = "Option::is_none")]
    oauth_login: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    issue_or_pr_url: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    redirect_url: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    captcha: Option<SessionCaptchaConfig>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct StateCookie {
    pub session_id: Uuid,
    pub token_hash: String,
    pub state: String,
    pub session_token: String,
}

pub fn token_hash(token: &str) -> String {
    hex::encode(Sha256::digest(token.as_bytes()))
}

pub fn encode_state_cookie(
    secret: &str,
    session_id: Uuid,
    token_hash: &str,
    state: &str,
    session_token: &str,
) -> Option<String> {
    let payload = format!("{session_id}:{token_hash}:{state}:{session_token}");
    let sig = sign_cookie(secret, payload.as_bytes())?;
    Some(format!("{payload}:{sig}"))
}

pub fn parse_state_cookie(value: &str, expected_state: &str, secret: &str) -> Option<StateCookie> {
    let (payload, sig) = value.rsplit_once(':')?;
    if !constant_time_eq(
        sign_cookie(secret, payload.as_bytes())?.as_bytes(),
        sig.as_bytes(),
    ) {
        return None;
    }
    let mut parts = payload.splitn(4, ':');
    let session_id = parts.next()?.parse().ok()?;
    let token_hash = parts.next()?.to_string();
    let state = parts.next()?.to_string();
    let session_token = parts.next()?.to_string();
    if state != expected_state || token_hash.len() != 64 || session_token.is_empty() {
        return None;
    }
    Some(StateCookie {
        session_id,
        token_hash,
        state,
        session_token,
    })
}

fn sign_cookie(secret: &str, msg: &[u8]) -> Option<String> {
    let mut mac = HmacSha256::new_from_slice(secret.as_bytes()).ok()?;
    mac.update(msg);
    Some(URL_SAFE_NO_PAD.encode(mac.finalize().into_bytes()))
}

fn constant_time_eq(a: &[u8], b: &[u8]) -> bool {
    if a.len() != b.len() {
        return false;
    }
    a.iter().zip(b).fold(0u8, |acc, (x, y)| acc | (x ^ y)) == 0
}

fn session_oauth_verified(metadata: &serde_json::Value) -> bool {
    metadata
        .get("oauth_verified")
        .and_then(|value| value.as_bool())
        .unwrap_or(false)
}

fn subject_url(repo_full_name: &str, subject_type: &str, subject_id: &str) -> Option<String> {
    let (owner, name) = repo_full_name.split_once('/')?;
    let path = match subject_type {
        "pull_request" => format!("pull/{subject_id}"),
        "issue" => format!("issues/{subject_id}"),
        _ => return None,
    };
    Some(format!("https://github.com/{owner}/{name}/{path}"))
}

fn verify_response(
    session_id: Uuid,
    session: &db::VerificationSession,
    repo: Option<String>,
    oauth_url: Option<String>,
    captcha: Option<SessionCaptchaConfig>,
) -> VerifyResponse {
    let oauth_verified = session_oauth_verified(&session.metadata);
    let expected_login = session
        .metadata
        .get("login")
        .and_then(|value| value.as_str())
        .map(str::to_string);
    let oauth_login = if oauth_verified {
        session
            .metadata
            .get("oauth_login")
            .and_then(|value| value.as_str())
            .map(str::to_string)
    } else {
        None
    };
    let issue_or_pr_url = session
        .metadata
        .get("subject_url")
        .and_then(|value| value.as_str())
        .map(str::to_string)
        .or_else(|| {
            repo.as_deref().and_then(|full_name| {
                subject_url(full_name, &session.subject_type, &session.subject_id)
            })
        });
    VerifyResponse {
        session_id,
        status: session.status.clone(),
        subject_type: session.subject_type.clone(),
        subject_id: session.subject_id.clone(),
        expires_at: session.expires_at.unix_timestamp(),
        repo,
        github_login: expected_login,
        oauth_url,
        captcha_required: captcha.is_some() && oauth_verified,
        oauth_required: !oauth_verified,
        oauth_verified,
        oauth_login,
        issue_or_pr_url: issue_or_pr_url.clone(),
        redirect_url: issue_or_pr_url,
        captcha: if oauth_verified { captcha } else { None },
    }
}

fn random_state() -> String {
    let mut bytes = [0_u8; 32];
    OsRng.fill_bytes(&mut bytes);
    URL_SAFE_NO_PAD.encode(bytes)
}

async fn oauth_start(
    State(state): State<AppState>,
    Query(q): Query<StartQuery>,
) -> Result<Response, OAuthError> {
    let client_id = state
        .config
        .github_oauth_client_id
        .as_ref()
        .ok_or(OAuthError::NotConfigured)?;
    let pool = state.db.as_ref().ok_or(OAuthError::NotConfigured)?;
    let hash = token_hash(&q.token);
    let session = db::get_verification_session(pool, q.session_id, &hash)
        .await?
        .ok_or(OAuthError::NotFound)?;
    if session.status != "pending" || session.expires_at <= OffsetDateTime::now_utc() {
        return Err(OAuthError::InvalidSession);
    }
    let st = random_state();
    let signing_secret = state
        .config
        .admin_session_secret
        .as_deref()
        .ok_or(OAuthError::NotConfigured)?;
    let state_cookie = encode_state_cookie(signing_secret, q.session_id, &hash, &st, &q.token)
        .ok_or(OAuthError::NotConfigured)?;
    let cookie = format!(
        "{}={}; Path=/api/github/oauth; HttpOnly; SameSite=Lax; Max-Age=600{}",
        OAUTH_COOKIE,
        state_cookie,
        if state.config.cookie_secure {
            "; Secure"
        } else {
            ""
        }
    );
    let redirect_uri = format!(
        "{}/api/github/oauth/callback",
        state.config.api_base_url.trim_end_matches('/')
    );
    let url = format!(
        "https://github.com/login/oauth/authorize?client_id={client_id}&redirect_uri={redirect_uri}&state={st}"
    );
    let mut res = Redirect::temporary(&url).into_response();
    res.headers_mut()
        .insert(header::SET_COOKIE, cookie.parse().unwrap());
    Ok(res)
}

async fn oauth_callback(
    State(state): State<AppState>,
    headers: HeaderMap,
    Query(q): Query<CallbackQuery>,
) -> Result<Response, OAuthError> {
    if q.error.is_some() {
        if let Some(raw_cookie) = find_cookie(&headers, OAUTH_COOKIE) {
            if let Some(secret) = state.config.admin_session_secret.as_deref() {
                if let Some(sc) =
                    parse_state_cookie(&raw_cookie, q.state.as_deref().unwrap_or(""), secret)
                {
                    return Ok(redirect_verify(
                        &state,
                        sc.session_id,
                        &sc.session_token,
                        "error",
                        "oauth_denied",
                    ));
                }
            }
        }
        return Ok(redirect_verify_error(&state, "oauth_denied"));
    }
    let code = q.code.ok_or(OAuthError::BadRequest)?;
    let st = q.state.ok_or(OAuthError::BadRequest)?;
    if let Some(response) =
        crate::admin_auth::handle_shared_oauth_callback(&state, &headers, &code, &st).await
    {
        return Ok(response);
    }
    let raw_cookie = find_cookie(&headers, OAUTH_COOKIE).ok_or(OAuthError::InvalidState)?;
    let signing_secret = state
        .config
        .admin_session_secret
        .as_deref()
        .ok_or(OAuthError::NotConfigured)?;
    let sc =
        parse_state_cookie(&raw_cookie, &st, signing_secret).ok_or(OAuthError::InvalidState)?;
    let pool = state.db.as_ref().ok_or(OAuthError::NotConfigured)?;
    let session = db::get_verification_session(pool, sc.session_id, &sc.token_hash)
        .await?
        .ok_or(OAuthError::NotFound)?;
    if session.status != "pending" || session.expires_at <= OffsetDateTime::now_utc() {
        return Err(OAuthError::InvalidSession);
    }
    let client_id = state
        .config
        .github_oauth_client_id
        .as_ref()
        .ok_or(OAuthError::NotConfigured)?;
    let client_secret = state
        .config
        .github_oauth_client_secret
        .as_ref()
        .ok_or(OAuthError::NotConfigured)?;
    let gh = github::ReqwestGitHubClient::with_base_url(&state.config.github_api_base);
    let redirect_uri = format!(
        "{}/api/github/oauth/callback",
        state.config.api_base_url.trim_end_matches('/')
    );
    let tok = gh
        .exchange_oauth_code(client_id, client_secret, &code, Some(&redirect_uri))
        .await
        .map_err(|_| OAuthError::Upstream)?;
    let user = gh
        .current_user(&tok.access_token)
        .await
        .map_err(|_| OAuthError::Upstream)?;
    if session
        .github_user_id
        .is_some_and(|id| id != user.id as i64)
    {
        return Ok(redirect_verify(
            &state,
            sc.session_id,
            &sc.session_token,
            "error",
            "github_user_mismatch",
        ));
    }
    if let Some(expected_login) = session
        .metadata
        .get("login")
        .and_then(|value| value.as_str())
    {
        if !expected_login.eq_ignore_ascii_case(&user.login) {
            return Ok(redirect_verify(
                &state,
                sc.session_id,
                &sc.session_token,
                "error",
                "github_user_mismatch",
            ));
        }
    }
    db::mark_verification_session_oauth_verified(
        pool,
        sc.session_id,
        &sc.token_hash,
        &user.login,
        user.id as i64,
    )
    .await?;
    let new_token = random_state();
    let new_hash = token_hash(&new_token);
    db::rotate_verification_session_token(pool, sc.session_id, &sc.token_hash, &new_hash)
        .await?
        .ok_or(OAuthError::InvalidSession)?;
    Ok(redirect_verify(
        &state,
        sc.session_id,
        &new_token,
        "continue",
        "oauth_verified",
    ))
}

async fn get_verify(
    State(state): State<AppState>,
    Path(session_id): Path<Uuid>,
    Query(q): Query<std::collections::HashMap<String, String>>,
) -> Result<Json<VerifyResponse>, OAuthError> {
    let token = q.get("token").ok_or(OAuthError::BadRequest)?;
    let pool = state.db.as_ref().ok_or(OAuthError::NotConfigured)?;
    let s = db::get_verification_session(pool, session_id, &token_hash(token))
        .await?
        .ok_or(OAuthError::NotFound)?;
    let policy = load_repo_policy(pool, s.repository_id).await;
    let settings = captcha_config::load_settings(&state).await;
    let provider_id =
        captcha_config::resolve_provider_id(&settings, &policy).ok_or(OAuthError::NotConfigured)?;
    let captcha = captcha_config::session_captcha_config(&settings, &provider_id);
    let repo = db::get_repository(pool, s.repository_id)
        .await?
        .map(|repo| repo.full_name);
    let oauth_url = format!(
        "{}/api/github/oauth/start?session_id={session_id}&token={token}",
        state.config.api_base_url.trim_end_matches('/')
    );
    Ok(Json(verify_response(
        session_id,
        &s,
        repo,
        Some(oauth_url),
        captcha,
    )))
}

async fn post_captcha(
    State(state): State<AppState>,
    Path(session_id): Path<Uuid>,
    Json(body): Json<CaptchaBody>,
) -> Result<Json<VerifyResponse>, OAuthError> {
    let pool = state.db.as_ref().ok_or(OAuthError::NotConfigured)?;
    let hash = token_hash(&body.session_token);
    let s = db::get_verification_session(pool, session_id, &hash)
        .await?
        .ok_or(OAuthError::NotFound)?;
    if s.status != "pending" || s.expires_at <= OffsetDateTime::now_utc() {
        return Err(OAuthError::InvalidSession);
    }
    if !session_oauth_verified(&s.metadata) {
        return Err(OAuthError::OAuthRequired);
    }
    let policy = load_repo_policy(pool, s.repository_id).await;
    let settings = captcha_config::load_settings(&state).await;
    let provider_id = body
        .provider
        .clone()
        .or_else(|| captcha_config::resolve_provider_id(&settings, &policy))
        .ok_or(OAuthError::NotConfigured)?;
    if !settings
        .enabled_providers
        .contains(&provider_id.to_string())
    {
        return Err(OAuthError::BadRequest);
    }
    let cap = if state.config.turnstile_dev_bypass && provider_id == PROVIDER_DEV_BYPASS {
        captcha::DevBypass::new(true)
            .verify(&body.token, None)
            .await?
    } else {
        let stored = db::get_app_setting(pool, captcha_config::SETTINGS_KEY).await?;
        captcha_config::verify_token(&state.config, stored.as_ref(), &provider_id, &body.token)
            .await?
    };
    if !cap.success {
        return Err(OAuthError::CaptchaFailed);
    }
    if let Some(hostname) = cap.hostname.as_deref() {
        if !captcha_config::hostname_allowed(&state.config, hostname) {
            return Err(OAuthError::CaptchaFailed);
        }
    }
    let done = db::complete_verification_session(pool, session_id, &hash)
        .await?
        .ok_or(OAuthError::InvalidSession)?;
    finalize(&state, &done).await;
    let repo = db::get_repository(pool, done.repository_id)
        .await?
        .map(|repo| repo.full_name);
    Ok(Json(verify_response(session_id, &done, repo, None, None)))
}

async fn load_repo_policy(pool: &db::PgPool, repository_id: i64) -> VerificationPolicy {
    match db::get_policy(pool, repository_id).await {
        Ok(Some(stored)) if stored.enabled => {
            serde_json::from_value(stored.policy).unwrap_or_default()
        }
        _ => VerificationPolicy::default(),
    }
}

async fn finalize(state: &AppState, s: &db::VerificationSession) {
    let Some(pool) = state.db.as_ref() else {
        return;
    };
    let login = s
        .metadata
        .get("login")
        .and_then(|v| v.as_str())
        .unwrap_or(&s.subject_id);
    let subject_id = s
        .github_user_id
        .map(|id| id.to_string())
        .unwrap_or_else(|| login.to_string());
    let _ = db::trust_subject(
        pool,
        s.repository_id,
        "github_user",
        &subject_id,
        s.github_user_id,
        Some("oauth_captcha_verified"),
        None,
        json!({"session": s.public_id, "login": login, "source": "oauth_captcha"}),
    )
    .await;
    let Ok(Some(repo)) = db::get_repository(pool, s.repository_id).await else {
        return;
    };
    let p: policy::VerificationPolicy = match db::get_policy(pool, repo.repository_id).await {
        Ok(Some(pol)) if pol.enabled => serde_json::from_value(pol.policy).unwrap_or_default(),
        Ok(Some(_)) => return,
        _ => policy::VerificationPolicy::default(),
    };
    let token = installation_token(state, repo.installation_id as u64)
        .await
        .ok();
    if let Some(token) = token {
        let client = github::ReqwestGitHubClient::with_base_url(&state.config.github_api_base);
        let mut sessions = vec![s.clone()];
        if let Some(user_id) = s.github_user_id {
            if let Ok(done) =
                db::complete_pending_sessions_for_user(pool, s.repository_id, user_id).await
            {
                sessions.extend(done.into_iter().filter(|session| session.id != s.id));
            }
            if let Ok(comments) =
                db::list_bot_artifacts_for_user(pool, s.repository_id, user_id, "comment").await
            {
                for a in comments {
                    if let Some(id) = a.external_id.and_then(|x| x.parse().ok()) {
                        let _ = client
                            .delete_issue_comment(&token, &repo.owner, &repo.name, id)
                            .await;
                    }
                }
            }
            if let Ok(checks) =
                db::list_bot_artifacts_for_user(pool, s.repository_id, user_id, "check_run").await
            {
                for a in checks {
                    if let Some(id) = a.external_id.and_then(|x| x.parse().ok()) {
                        let sha = a.data.get("sha").and_then(|v| v.as_str()).unwrap_or("");
                        let req = CheckRunRequest {
                            name: "Human Auth".into(),
                            head_sha: sha.into(),
                            status: Some("completed".into()),
                            conclusion: Some("success".into()),
                            details_url: None,
                            output: Some(
                                json!({"title":"Human verification complete","summary":"The author completed OAuth and CAPTCHA verification."}),
                            ),
                        };
                        let _ = client
                            .update_check_run(&token, &repo.owner, &repo.name, id, &req)
                            .await;
                    }
                }
            }
        }
        for session in sessions {
            let issue_number = session.subject_id.parse().unwrap_or(0);
            for label in [p.apply_label.as_ref(), p.pending_label.as_ref()]
                .into_iter()
                .flatten()
            {
                let _ = client
                    .remove_label(&token, &repo.owner, &repo.name, issue_number, label)
                    .await;
            }
            if let Some(label) = p.verified_label.clone() {
                let _ = client
                    .add_labels(&token, &repo.owner, &repo.name, issue_number, &[label])
                    .await;
            }
        }
    }
}

async fn installation_token(state: &AppState, installation_id: u64) -> Result<String, OAuthError> {
    let app_id = state
        .config
        .github_app_id
        .as_ref()
        .ok_or(OAuthError::NotConfigured)?;
    let pk = state
        .config
        .github_private_key
        .as_ref()
        .ok_or(OAuthError::NotConfigured)?;
    let jwt = github::create_app_jwt(app_id, pk).map_err(|_| OAuthError::NotConfigured)?;
    let gh = github::ReqwestGitHubClient::with_base_url(&state.config.github_api_base);
    Ok(gh
        .exchange_installation_token(&jwt, installation_id)
        .await
        .map_err(|_| OAuthError::Upstream)?
        .token)
}

fn find_cookie(headers: &HeaderMap, name: &str) -> Option<String> {
    headers
        .get(header::COOKIE)?
        .to_str()
        .ok()?
        .split(';')
        .map(str::trim)
        .find_map(|c| c.strip_prefix(&format!("{name}=")).map(ToOwned::to_owned))
}

fn redirect_verify(
    state: &AppState,
    session_id: Uuid,
    token: &str,
    page: &str,
    code: &str,
) -> Response {
    let web = state.config.web_base_url.trim_end_matches('/');
    let url = if page == "continue" {
        format!("{web}/verify/{session_id}?token={token}&oauth={code}")
    } else {
        format!("{web}/verify/{session_id}/{page}?token={token}&code={code}")
    };
    Redirect::temporary(&url).into_response()
}

fn redirect_verify_error(state: &AppState, code: &str) -> Response {
    Redirect::temporary(&format!(
        "{}/verify/error?code={code}",
        state.config.web_base_url.trim_end_matches('/')
    ))
    .into_response()
}

#[derive(Debug, thiserror::Error)]
enum OAuthError {
    #[error("not configured")]
    NotConfigured,
    #[error("bad request")]
    BadRequest,
    #[error("invalid state")]
    InvalidState,
    #[error("session not found")]
    NotFound,
    #[error("invalid session")]
    InvalidSession,
    #[error("captcha failed")]
    CaptchaFailed,
    #[error("github oauth required")]
    OAuthRequired,
    #[error("upstream error")]
    Upstream,
    #[error("database error")]
    Db(#[from] sqlx::Error),
    #[error("captcha error")]
    Captcha(#[from] captcha::CaptchaError),
}

impl IntoResponse for OAuthError {
    fn into_response(self) -> Response {
        let status = match self {
            OAuthError::NotFound => StatusCode::NOT_FOUND,
            OAuthError::BadRequest
            | OAuthError::InvalidState
            | OAuthError::InvalidSession
            | OAuthError::CaptchaFailed
            | OAuthError::OAuthRequired => StatusCode::BAD_REQUEST,
            OAuthError::NotConfigured
            | OAuthError::Captcha(captcha::CaptchaError::NotConfigured) => {
                StatusCode::SERVICE_UNAVAILABLE
            }
            _ => StatusCode::BAD_GATEWAY,
        };
        (status, Json(json!({"error":{"code": format!("{self:?}").to_lowercase(), "message": self.to_string()}}))).into_response()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn state_cookie_round_trip_and_rejects_bad_state() {
        let id = Uuid::new_v4();
        let hash = token_hash("secret-token");
        let value =
            encode_state_cookie("signing-secret", id, &hash, "state-1", "session-token").unwrap();
        let parsed = parse_state_cookie(&value, "state-1", "signing-secret").unwrap();
        assert_eq!(parsed.session_id, id);
        assert_eq!(parsed.session_token, "session-token");
        assert!(parse_state_cookie(&value, "other", "signing-secret").is_none());
        assert!(parse_state_cookie(&value, "state-1", "wrong-secret").is_none());
    }
    #[test]
    fn token_hash_is_stable() {
        assert_eq!(token_hash("abc"), token_hash("abc"));
        assert_ne!(token_hash("abc"), token_hash("abcd"));
    }
}
