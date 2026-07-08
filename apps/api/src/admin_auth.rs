use axum::{
    extract::{Query, State},
    http::{header, HeaderMap, StatusCode},
    response::{IntoResponse, Redirect, Response},
    routing::{get, post},
    Json, Router,
};
use base64::{engine::general_purpose::URL_SAFE_NO_PAD, Engine as _};
use github::GitHubApi;
use serde::{Deserialize, Serialize};
use time::OffsetDateTime;

use crate::error::api_json_error;
use crate::github_helpers::sync_user_installations;
use crate::web_util::{constant_time_eq, find_cookie, random_state, sign_hmac_url_safe};
use crate::AppState;

const ADMIN_OAUTH_STATE_COOKIE: &str = "gho_admin_oauth_state";
// Sessions are stateless HMAC cookies with no server-side revocation, so keep the
// window short; re-authentication is a single GitHub OAuth redirect.
const ADMIN_SESSION_TTL_SECONDS: i64 = 86_400;

pub fn routes() -> Router<AppState> {
    Router::new()
        .route("/api/auth/me", get(me))
        .route("/api/auth/github/start", get(github_start))
        .route("/api/auth/logout", post(logout))
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct AdminUser {
    pub id: u64,
    pub login: String,
    pub avatar_url: Option<String>,
    pub html_url: Option<String>,
}

#[derive(Debug, Serialize)]
struct MeResponse {
    authenticated: bool,
    user: Option<AdminUser>,
    login_url: String,
}

#[derive(Deserialize)]
struct StartQuery {
    #[serde(default)]
    return_to: Option<String>,
}

async fn me(State(state): State<AppState>, headers: HeaderMap) -> Json<MeResponse> {
    let user = current_admin_user(&state, &headers);
    Json(MeResponse {
        authenticated: user.is_some(),
        user,
        login_url: format!(
            "{}/api/auth/github/start",
            state.config.api_base_url.trim_end_matches('/')
        ),
    })
}

async fn github_start(
    State(state): State<AppState>,
    Query(q): Query<StartQuery>,
) -> Result<Response, AdminAuthError> {
    let client_id = state
        .config
        .github_oauth_client_id
        .as_ref()
        .ok_or(AdminAuthError::NotConfigured)?;
    let st = random_state();
    let return_to = q
        .return_to
        .as_deref()
        .filter(|p| p.starts_with('/') && !p.starts_with("//"))
        .map(|p| URL_SAFE_NO_PAD.encode(p))
        .unwrap_or_default();
    let cookie = format!(
        "{}={}.{}; Path=/api; HttpOnly; SameSite=Lax; Max-Age=600{}",
        ADMIN_OAUTH_STATE_COOKIE,
        st,
        return_to,
        secure_suffix(&state)
    );
    let redirect_uri = shared_oauth_callback_url(&state);
    let url = format!(
        "https://github.com/login/oauth/authorize?client_id={client_id}&redirect_uri={redirect_uri}&state={st}"
    );
    let mut res = Redirect::temporary(&url).into_response();
    res.headers_mut()
        .insert(header::SET_COOKIE, cookie.parse().unwrap());
    Ok(res)
}

pub async fn handle_shared_oauth_callback(
    state: &AppState,
    headers: &HeaderMap,
    code: &str,
    st: &str,
) -> Option<Response> {
    // Only claim the shared callback when the state actually belongs to the admin flow;
    // a stale admin state cookie must not intercept a contributor verification callback.
    let cookie_value = find_cookie(headers, ADMIN_OAUTH_STATE_COOKIE)?;
    let (cookie_state, _) = cookie_value
        .split_once('.')
        .unwrap_or((cookie_value.as_str(), ""));
    if cookie_state != st {
        return None;
    }
    Some(
        process_admin_oauth_callback(state, headers, code, st, &shared_oauth_callback_url(state))
            .await
            .unwrap_or_else(IntoResponse::into_response),
    )
}

async fn process_admin_oauth_callback(
    state: &AppState,
    headers: &HeaderMap,
    code: &str,
    st: &str,
    redirect_uri: &str,
) -> Result<Response, AdminAuthError> {
    let cookie_value =
        find_cookie(headers, ADMIN_OAUTH_STATE_COOKIE).ok_or(AdminAuthError::InvalidState)?;
    let (cookie_state, return_to_b64) = cookie_value
        .split_once('.')
        .unwrap_or((cookie_value.as_str(), ""));
    if cookie_state != st {
        return Err(AdminAuthError::InvalidState);
    }
    let return_to = URL_SAFE_NO_PAD
        .decode(return_to_b64)
        .ok()
        .and_then(|bytes| String::from_utf8(bytes).ok())
        .filter(|p| p.starts_with('/') && !p.starts_with("//"));
    let client_id = state
        .config
        .github_oauth_client_id
        .as_ref()
        .ok_or(AdminAuthError::NotConfigured)?;
    let client_secret = state
        .config
        .github_oauth_client_secret
        .as_ref()
        .ok_or(AdminAuthError::NotConfigured)?;
    let gh = github::ReqwestGitHubClient::with_base_url(&state.config.github_api_base);
    let tok = gh
        .exchange_oauth_code(client_id, client_secret, code, Some(redirect_uri))
        .await
        .map_err(|_| AdminAuthError::Upstream)?;
    let user = gh
        .current_user(&tok.access_token)
        .await
        .map_err(|_| AdminAuthError::Upstream)?;
    if !state.config.hosted_mode && !is_allowed_login(state, &user.login) {
        return Ok(redirect_home(state, "auth=unauthorized"));
    }
    if state.config.hosted_mode {
        persist_hosted_oauth_access(state, &gh, &tok.access_token, user.id as i64, &user.login)
            .await?;
    }
    let admin = AdminUser {
        id: user.id,
        login: user.login,
        avatar_url: user.avatar_url,
        html_url: user.html_url,
    };
    let session = encode_session(state, &admin).ok_or(AdminAuthError::NotConfigured)?;
    let cookie = format!(
        "{}={}; Path=/; HttpOnly; SameSite=Lax; Max-Age={ADMIN_SESSION_TTL_SECONDS}{}",
        state.config.admin_session_cookie_name,
        session,
        secure_suffix(state)
    );
    let clear_state = format!(
        "{}=; Path=/api; HttpOnly; SameSite=Lax; Max-Age=0{}",
        ADMIN_OAUTH_STATE_COOKIE,
        secure_suffix(state)
    );
    let mut res = match return_to {
        Some(path) => Redirect::temporary(&format!(
            "{}{}",
            state.config.web_base_url.trim_end_matches('/'),
            path
        ))
        .into_response(),
        None => redirect_home(state, "auth=ok"),
    };
    res.headers_mut()
        .append(header::SET_COOKIE, cookie.parse().unwrap());
    res.headers_mut()
        .append(header::SET_COOKIE, clear_state.parse().unwrap());
    Ok(res)
}

async fn persist_hosted_oauth_access(
    state: &AppState,
    gh: &github::ReqwestGitHubClient,
    access_token: &str,
    github_user_id: i64,
    login: &str,
) -> Result<(), AdminAuthError> {
    let pool = state.db.as_ref().ok_or(AdminAuthError::NotConfigured)?;
    let encrypted = crate::secret_box::encrypt_field(&state.config, access_token)
        .map_err(|_| AdminAuthError::NotConfigured)?;
    db::upsert_dashboard_oauth_token(pool, github_user_id, login, &encrypted)
        .await
        .map_err(|_| AdminAuthError::Upstream)?;
    let installations = gh
        .user_installations(access_token)
        .await
        .map_err(|_| AdminAuthError::Upstream)?;
    sync_user_installations(
        pool,
        &installations,
        github_user_id,
        login,
        "user_installations",
    )
    .await
    .map_err(|_| AdminAuthError::Upstream)?;
    Ok(())
}

async fn logout(State(state): State<AppState>) -> Response {
    let cookie = format!(
        "{}=; Path=/; HttpOnly; SameSite=Lax; Max-Age=0{}",
        state.config.admin_session_cookie_name,
        secure_suffix(&state)
    );
    let mut res = StatusCode::NO_CONTENT.into_response();
    res.headers_mut()
        .insert(header::SET_COOKIE, cookie.parse().unwrap());
    res
}

pub fn authorize_admin(state: &AppState, headers: &HeaderMap) -> bool {
    bearer_authorized(state, headers) || current_admin_user(state, headers).is_some()
}

pub fn authorize_admin_mutation(state: &AppState, headers: &HeaderMap) -> bool {
    bearer_authorized(state, headers)
        || (current_admin_user(state, headers).is_some()
            && headers
                .get("x-requested-with")
                .and_then(|value| value.to_str().ok())
                .is_some_and(|value| value == "github-human-auth"))
}

pub fn bearer_authorized(state: &AppState, headers: &HeaderMap) -> bool {
    if let Some(expected) = state.config.admin_api_token.as_deref() {
        if let Some(provided) = bearer_token(headers) {
            return constant_time_eq(provided.as_bytes(), expected.as_bytes());
        }
    }
    false
}

pub fn current_admin_user(state: &AppState, headers: &HeaderMap) -> Option<AdminUser> {
    let raw = find_cookie(headers, &state.config.admin_session_cookie_name)?;
    let user = decode_session(state, &raw)?;
    (state.config.hosted_mode || is_allowed_login(state, &user.login)).then_some(user)
}

pub fn mutation_header_present(headers: &HeaderMap) -> bool {
    headers
        .get("x-requested-with")
        .and_then(|value| value.to_str().ok())
        .is_some_and(|value| value == "github-human-auth")
}

fn encode_session(state: &AppState, user: &AdminUser) -> Option<String> {
    #[derive(Serialize)]
    struct SessionPayload<'a> {
        v: u8,
        exp: i64,
        user: &'a AdminUser,
    }

    let secret = session_secret(state)?;
    let payload = serde_json::to_vec(&SessionPayload {
        v: 1,
        exp: OffsetDateTime::now_utc().unix_timestamp() + ADMIN_SESSION_TTL_SECONDS,
        user,
    })
    .ok()?;
    let sig = sign_hmac_url_safe(secret, &payload)?;
    Some(format!("{}.{}", URL_SAFE_NO_PAD.encode(payload), sig))
}

fn decode_session(state: &AppState, raw: &str) -> Option<AdminUser> {
    #[derive(Deserialize)]
    struct SessionPayload {
        v: u8,
        exp: i64,
        user: AdminUser,
    }

    let (payload_b64, sig) = raw.split_once('.')?;
    let payload = URL_SAFE_NO_PAD.decode(payload_b64).ok()?;
    let secret = session_secret(state)?;
    if !constant_time_eq(
        sign_hmac_url_safe(secret, &payload)?.as_bytes(),
        sig.as_bytes(),
    ) {
        return None;
    }
    let payload: SessionPayload = serde_json::from_slice(&payload).ok()?;
    if payload.v != 1 || payload.exp <= OffsetDateTime::now_utc().unix_timestamp() {
        return None;
    }
    Some(payload.user)
}

fn session_secret(state: &AppState) -> Option<&str> {
    state.config.admin_session_secret.as_deref()
}

fn is_allowed_login(state: &AppState, login: &str) -> bool {
    // Fail closed: an empty allowlist authorizes nobody. Config validation already requires a
    // non-empty allowlist whenever OAuth admin login is enabled.
    state
        .config
        .admin_github_logins
        .iter()
        .any(|allowed| allowed.eq_ignore_ascii_case(login))
}

fn bearer_token(headers: &HeaderMap) -> Option<&str> {
    headers
        .get(header::AUTHORIZATION)
        .and_then(|value| value.to_str().ok())
        .and_then(|value| value.strip_prefix("Bearer "))
}

fn shared_oauth_callback_url(state: &AppState) -> String {
    format!(
        "{}/api/github/oauth/callback",
        state.config.api_base_url.trim_end_matches('/')
    )
}

fn redirect_home(state: &AppState, query: &str) -> Response {
    Redirect::temporary(&format!(
        "{}{}",
        state.config.web_base_url.trim_end_matches('/'),
        if query.is_empty() {
            String::new()
        } else {
            format!("/?{query}")
        }
    ))
    .into_response()
}

fn secure_suffix(state: &AppState) -> &'static str {
    if state.config.cookie_secure {
        "; Secure"
    } else {
        ""
    }
}

#[derive(Debug, thiserror::Error)]
enum AdminAuthError {
    #[error("not configured")]
    NotConfigured,
    #[error("invalid state")]
    InvalidState,
    #[error("upstream error")]
    Upstream,
}

impl IntoResponse for AdminAuthError {
    fn into_response(self) -> Response {
        let (status, code) = match self {
            AdminAuthError::NotConfigured => (StatusCode::SERVICE_UNAVAILABLE, "not_configured"),
            AdminAuthError::InvalidState => (StatusCode::BAD_REQUEST, "invalid_state"),
            AdminAuthError::Upstream => (StatusCode::BAD_GATEWAY, "upstream_error"),
        };
        api_json_error(status, code, self)
    }
}
