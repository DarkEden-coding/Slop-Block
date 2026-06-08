use axum::{
    extract::{Path, Query, State},
    http::{header, HeaderMap, StatusCode},
    response::{IntoResponse, Redirect, Response},
    routing::{get, post},
    Json, Router,
};
use base64::{engine::general_purpose::URL_SAFE_NO_PAD, Engine as _};
use captcha::CaptchaProvider;
use github::{CheckRunRequest, GitHubApi};
use rand::{rngs::OsRng, RngCore};
use serde::{Deserialize, Serialize};
use serde_json::json;
use sha2::{Digest, Sha256};
use time::OffsetDateTime;
use uuid::Uuid;

use crate::AppState;

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
}

#[derive(Serialize)]
struct VerifyResponse {
    session_id: Uuid,
    status: String,
    subject_type: String,
    subject_id: String,
    expires_at: i64,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct StateCookie {
    pub session_id: Uuid,
    pub token_hash: String,
    pub state: String,
}

pub fn token_hash(token: &str) -> String {
    hex::encode(Sha256::digest(token.as_bytes()))
}

pub fn encode_state_cookie(session_id: Uuid, token_hash: &str, state: &str) -> String {
    format!("{session_id}:{token_hash}:{state}")
}

pub fn parse_state_cookie(value: &str, expected_state: &str) -> Option<StateCookie> {
    let mut parts = value.split(':');
    let session_id = parts.next()?.parse().ok()?;
    let token_hash = parts.next()?.to_string();
    let state = parts.next()?.to_string();
    if parts.next().is_some() || state != expected_state || token_hash.len() != 64 {
        return None;
    }
    Some(StateCookie {
        session_id,
        token_hash,
        state,
    })
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
    let cookie = format!(
        "{}={}; Path=/api/github/oauth; HttpOnly; SameSite=Lax; Max-Age=600{}",
        OAUTH_COOKIE,
        encode_state_cookie(q.session_id, &hash, &st),
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
        return Ok(redirect_web(&state, "error", "oauth_denied"));
    }
    let code = q.code.ok_or(OAuthError::BadRequest)?;
    let st = q.state.ok_or(OAuthError::BadRequest)?;
    if let Some(response) =
        crate::admin_auth::handle_shared_oauth_callback(&state, &headers, &code, &st).await
    {
        return Ok(response);
    }
    let raw_cookie = find_cookie(&headers, OAUTH_COOKIE).ok_or(OAuthError::InvalidState)?;
    let sc = parse_state_cookie(&raw_cookie, &st).ok_or(OAuthError::InvalidState)?;
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
        return Ok(redirect_web(&state, "error", "github_user_mismatch"));
    }
    Ok(redirect_web(&state, "success", "oauth_verified"))
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
    Ok(Json(VerifyResponse {
        session_id,
        status: s.status,
        subject_type: s.subject_type,
        subject_id: s.subject_id,
        expires_at: s.expires_at.unix_timestamp(),
    }))
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
    let cap = if state.config.turnstile_dev_bypass {
        captcha::DevBypass::new(true)
            .verify(&body.token, None)
            .await?
    } else {
        captcha::CloudflareTurnstile::new(
            state
                .config
                .turnstile_secret
                .as_ref()
                .ok_or(OAuthError::NotConfigured)?,
        )
        .verify(&body.token, None)
        .await?
    };
    if !cap.success {
        return Err(OAuthError::CaptchaFailed);
    }
    let done = db::complete_verification_session(pool, session_id, &hash)
        .await?
        .ok_or(OAuthError::InvalidSession)?;
    finalize(&state, &done).await;
    Ok(Json(VerifyResponse {
        session_id,
        status: done.status,
        subject_type: done.subject_type,
        subject_id: done.subject_id,
        expires_at: done.expires_at.unix_timestamp(),
    }))
}

async fn finalize(state: &AppState, s: &db::VerificationSession) {
    let Some(pool) = state.db.as_ref() else {
        return;
    };
    let _ = db::trust_subject(
        pool,
        s.repository_id,
        "github_user",
        s.metadata
            .get("login")
            .and_then(|v| v.as_str())
            .unwrap_or(&s.subject_id),
        s.github_user_id,
        Some("oauth_captcha_verified"),
        None,
        json!({"session": s.public_id}),
    )
    .await;
    let Ok(Some(repo)) = db::get_repository(pool, s.repository_id).await else {
        return;
    };
    let Ok(Some(pol)) = db::get_policy(pool, repo.repository_id).await else {
        return;
    };
    let p: policy::VerificationPolicy = serde_json::from_value(pol.policy).unwrap_or_default();
    let token = installation_token(state, repo.installation_id as u64)
        .await
        .ok();
    if let Some(token) = token {
        for label in [p.apply_label.as_ref(), p.pending_label.as_ref()]
            .into_iter()
            .flatten()
        {
            let _ = github::ReqwestGitHubClient::with_base_url(&state.config.github_api_base)
                .remove_label(
                    &token,
                    &repo.owner,
                    &repo.name,
                    s.subject_id.parse().unwrap_or(0),
                    label,
                )
                .await;
        }
        if let Some(label) = p.verified_label {
            let _ = github::ReqwestGitHubClient::with_base_url(&state.config.github_api_base)
                .add_labels(
                    &token,
                    &repo.owner,
                    &repo.name,
                    s.subject_id.parse().unwrap_or(0),
                    &[label],
                )
                .await;
        }
        if let Ok(Some(a)) = db::get_bot_artifact(
            pool,
            repo.repository_id,
            &s.subject_type,
            &s.subject_id,
            "comment",
        )
        .await
        {
            if let Some(id) = a.external_id.and_then(|x| x.parse().ok()) {
                let _ = github::ReqwestGitHubClient::with_base_url(&state.config.github_api_base)
                    .update_issue_comment(
                        &token,
                        &repo.owner,
                        &repo.name,
                        id,
                        "Human verification completed. Thank you.",
                    )
                    .await;
            }
        }
        if let Ok(Some(a)) = db::get_bot_artifact(
            pool,
            repo.repository_id,
            &s.subject_type,
            &s.subject_id,
            "check_run",
        )
        .await
        {
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
                let _ = github::ReqwestGitHubClient::with_base_url(&state.config.github_api_base)
                    .update_check_run(&token, &repo.owner, &repo.name, id, &req)
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

fn redirect_web(state: &AppState, status: &str, code: &str) -> Response {
    Redirect::temporary(&format!(
        "{}/verify/{}?code={}",
        state.config.web_base_url.trim_end_matches('/'),
        status,
        code
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
            | OAuthError::CaptchaFailed => StatusCode::BAD_REQUEST,
            OAuthError::NotConfigured => StatusCode::SERVICE_UNAVAILABLE,
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
        let value = encode_state_cookie(id, &hash, "state-1");
        assert_eq!(
            parse_state_cookie(&value, "state-1").unwrap().session_id,
            id
        );
        assert!(parse_state_cookie(&value, "other").is_none());
    }
    #[test]
    fn token_hash_is_stable() {
        assert_eq!(token_hash("abc"), token_hash("abc"));
        assert_ne!(token_hash("abc"), token_hash("abcd"));
    }
}
