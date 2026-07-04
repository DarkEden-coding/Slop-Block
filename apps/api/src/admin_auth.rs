use axum::{
    extract::{Query, State},
    http::{header, HeaderMap, StatusCode},
    response::{IntoResponse, Redirect, Response},
    routing::{get, post},
    Json, Router,
};
use base64::{engine::general_purpose::URL_SAFE_NO_PAD, Engine as _};
use github::GitHubApi;
use hmac::{Hmac, Mac};
use rand::{rngs::OsRng, RngCore};
use serde::{Deserialize, Serialize};
use serde_json::json;
use sha2::Sha256;
use time::OffsetDateTime;

use crate::AppState;

type HmacSha256 = Hmac<Sha256>;
const ADMIN_OAUTH_STATE_COOKIE: &str = "gho_admin_oauth_state";
// Sessions are stateless HMAC cookies with no server-side revocation, so keep the
// window short; re-authentication is a single GitHub OAuth redirect.
const ADMIN_SESSION_TTL_SECONDS: i64 = 86_400;

pub fn routes() -> Router<AppState> {
    Router::new()
        .route("/api/auth/me", get(me))
        .route("/api/auth/github/start", get(github_start))
        .route("/api/auth/github/callback", get(github_callback))
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

#[derive(Deserialize)]
struct CallbackQuery {
    code: Option<String>,
    state: Option<String>,
    error: Option<String>,
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

async fn github_callback(
    State(state): State<AppState>,
    headers: HeaderMap,
    Query(q): Query<CallbackQuery>,
) -> Result<Response, AdminAuthError> {
    if q.error.is_some() {
        return Ok(redirect_home(&state, "auth=denied"));
    }
    let code = q.code.ok_or(AdminAuthError::BadRequest)?;
    let st = q.state.ok_or(AdminAuthError::BadRequest)?;
    process_admin_oauth_callback(&state, &headers, &code, &st, &admin_callback_url(&state)).await
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
    for installation in installations {
        let account = installation.account;
        let account_login = account
            .as_ref()
            .map(|a| a.login.as_str())
            .unwrap_or("unknown");
        let account_id = account.as_ref().map(|a| a.id as i64);
        db::upsert_installation(
            pool,
            installation.id as i64,
            account_login,
            account_id,
            None,
            serde_json::json!({"source":"user_installations"}),
        )
        .await
        .map_err(|_| AdminAuthError::Upstream)?;
        db::upsert_installation_admin(pool, installation.id as i64, github_user_id, login)
            .await
            .map_err(|_| AdminAuthError::Upstream)?;
    }
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

fn bearer_authorized(state: &AppState, headers: &HeaderMap) -> bool {
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

pub fn bearer_is_authorized(state: &AppState, headers: &HeaderMap) -> bool {
    bearer_authorized(state, headers)
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
    let sig = sign(secret, &payload)?;
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
    if !constant_time_eq(sign(secret, &payload)?.as_bytes(), sig.as_bytes()) {
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

fn sign(secret: &str, msg: &[u8]) -> Option<String> {
    let mut mac = HmacSha256::new_from_slice(secret.as_bytes()).ok()?;
    mac.update(msg);
    Some(URL_SAFE_NO_PAD.encode(mac.finalize().into_bytes()))
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

fn find_cookie(headers: &HeaderMap, name: &str) -> Option<String> {
    headers
        .get(header::COOKIE)?
        .to_str()
        .ok()?
        .split(';')
        .map(str::trim)
        .find_map(|c| c.strip_prefix(&format!("{name}=")).map(ToOwned::to_owned))
}

fn random_state() -> String {
    let mut bytes = [0_u8; 32];
    OsRng.fill_bytes(&mut bytes);
    URL_SAFE_NO_PAD.encode(bytes)
}

fn admin_callback_url(state: &AppState) -> String {
    format!(
        "{}/api/auth/github/callback",
        state.config.api_base_url.trim_end_matches('/')
    )
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

fn constant_time_eq(a: &[u8], b: &[u8]) -> bool {
    if a.len() != b.len() {
        return false;
    }
    a.iter().zip(b).fold(0u8, |acc, (x, y)| acc | (x ^ y)) == 0
}

#[derive(Debug, thiserror::Error)]
enum AdminAuthError {
    #[error("not configured")]
    NotConfigured,
    #[error("bad request")]
    BadRequest,
    #[error("invalid state")]
    InvalidState,
    #[error("upstream error")]
    Upstream,
}

impl IntoResponse for AdminAuthError {
    fn into_response(self) -> Response {
        let status = match self {
            AdminAuthError::NotConfigured => StatusCode::SERVICE_UNAVAILABLE,
            AdminAuthError::BadRequest | AdminAuthError::InvalidState => StatusCode::BAD_REQUEST,
            AdminAuthError::Upstream => StatusCode::BAD_GATEWAY,
        };
        (
            status,
            Json(json!({"error":{"code": format!("{self:?}").to_lowercase(), "message": self.to_string()}})),
        )
            .into_response()
    }
}
