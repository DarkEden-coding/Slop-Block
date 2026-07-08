use axum::{
    extract::{Path, Query, State},
    http::{header, HeaderMap, StatusCode},
    response::{IntoResponse, Redirect, Response},
    routing::{get, post},
    Json, Router,
};
use github::GitHubApi;
use serde::{Deserialize, Serialize};
use serde_json::json;
use time::OffsetDateTime;
use uuid::Uuid;

use crate::captcha_config::{self, SessionCaptchaConfig};
use crate::error::api_json_error;
use crate::oauth_state::{encode_state_cookie, parse_state_cookie, token_hash};
use crate::verification_finalize::{
    load_repo_policy, propagate_verified_github_state, record_verification_trust,
};
use crate::web_util::{constant_time_eq, find_cookie, random_state, sign_source_payload};
use crate::AppState;
const OAUTH_COOKIE: &str = "gho_oauth_state";

pub fn routes() -> Router<AppState> {
    Router::new()
        .route("/api/github/oauth/start", get(oauth_start))
        .route("/api/github/oauth/callback", get(oauth_callback))
        .route("/api/verify/from-source", get(verify_from_source))
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

#[derive(Deserialize)]
struct SourceQuery {
    repo: i64,
    #[serde(rename = "type")]
    subject_type: String,
    number: u64,
    user_id: i64,
    login: String,
    url: String,
    sig: String,
}

#[derive(Serialize)]
struct SourceResponse {
    session_id: Option<Uuid>,
    token: Option<String>,
    already_verified: bool,
    redirect_url: String,
    message: String,
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
    if session.status != "pending" {
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
    if session.status != "pending" {
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

async fn verify_from_source(
    State(state): State<AppState>,
    Query(q): Query<SourceQuery>,
) -> Result<Json<SourceResponse>, OAuthError> {
    let secret = state
        .config
        .admin_session_secret
        .as_ref()
        .or(state.config.github_webhook_secret.as_ref())
        .ok_or(OAuthError::NotConfigured)?
        .clone();
    let expected = sign_source_payload(
        &secret,
        q.repo,
        &q.subject_type,
        q.number,
        q.user_id,
        &q.login,
        &q.url,
    )
    .ok_or(OAuthError::NotConfigured)?;
    if !constant_time_eq(expected.as_bytes(), q.sig.as_bytes()) {
        return Err(OAuthError::InvalidSession);
    }
    let pool = state.db.as_ref().ok_or(OAuthError::NotConfigured)?;
    if db::get_trusted_subject(pool, q.repo, "github_user", &q.user_id.to_string())
        .await
        .map_err(OAuthError::Db)?
        .is_some()
    {
        return Ok(Json(SourceResponse {
            session_id: None,
            token: None,
            already_verified: true,
            redirect_url: q.url,
            message: "You have already verified for this repository.".into(),
        }));
    }

    db::upsert_github_user(pool, q.user_id, &q.login, None, json!({"login": q.login}))
        .await
        .map_err(OAuthError::Db)?;
    let token_plain = random_state();
    let hash = token_hash(&token_plain);
    let session = db::create_verification_session(
        pool,
        q.repo,
        &q.subject_type,
        &q.number.to_string(),
        Some(q.user_id),
        &hash,
        OffsetDateTime::now_utc() + time::Duration::days(3650),
        json!({"login": q.login, "subject_url": q.url}),
    )
    .await
    .map_err(OAuthError::Db)?;
    Ok(Json(SourceResponse {
        session_id: Some(session.public_id),
        token: Some(token_plain),
        already_verified: false,
        redirect_url: q.url,
        message: "Verification session created.".into(),
    }))
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
    if s.status != "pending" {
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
    let stored = db::get_app_setting(pool, captcha_config::SETTINGS_KEY).await?;
    let cap =
        captcha_config::verify_token(&state.config, stored.as_ref(), &provider_id, &body.token)
            .await?;
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
    record_verification_trust(&state, &done).await;
    let bg_state = state.clone();
    let bg_session = done.clone();
    tokio::spawn(async move {
        propagate_verified_github_state(&bg_state, &bg_session).await;
    });
    let repo = db::get_repository(pool, done.repository_id)
        .await?
        .map(|repo| repo.full_name);
    Ok(Json(verify_response(session_id, &done, repo, None, None)))
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
        let (status, code) = match &self {
            OAuthError::NotFound => (StatusCode::NOT_FOUND, "not_found"),
            OAuthError::BadRequest => (StatusCode::BAD_REQUEST, "bad_request"),
            OAuthError::InvalidState => (StatusCode::BAD_REQUEST, "invalid_state"),
            OAuthError::InvalidSession => (StatusCode::BAD_REQUEST, "invalid_session"),
            OAuthError::CaptchaFailed => (StatusCode::BAD_REQUEST, "captcha_failed"),
            OAuthError::OAuthRequired => (StatusCode::BAD_REQUEST, "oauth_required"),
            OAuthError::NotConfigured => (StatusCode::SERVICE_UNAVAILABLE, "not_configured"),
            OAuthError::Captcha(captcha::CaptchaError::NotConfigured) => {
                (StatusCode::SERVICE_UNAVAILABLE, "not_configured")
            }
            OAuthError::Captcha(_) => (StatusCode::BAD_GATEWAY, "captcha_error"),
            OAuthError::Upstream => (StatusCode::BAD_GATEWAY, "upstream_error"),
            OAuthError::Db(_) => (StatusCode::INTERNAL_SERVER_ERROR, "internal_error"),
        };
        api_json_error(status, code, self)
    }
}
