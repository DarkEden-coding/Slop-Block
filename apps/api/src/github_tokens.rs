use std::{
    collections::HashMap,
    sync::Arc,
    time::{Duration, Instant},
};

use github::GitHubApi;
use time::OffsetDateTime;
use tokio::sync::Mutex;

use crate::AppState;

#[derive(Clone)]
struct CachedToken {
    token: String,
    expires_at: OffsetDateTime,
}

#[derive(Clone)]
struct CachedJwt {
    token: String,
    expires_at: Instant,
}

#[derive(Default)]
pub struct TokenCache {
    installation_tokens: Mutex<HashMap<u64, CachedToken>>,
    app_jwt: Mutex<Option<CachedJwt>>,
}

impl TokenCache {
    pub fn new() -> Self {
        Self::default()
    }

    pub async fn invalidate_installation(&self, installation_id: u64) {
        self.installation_tokens
            .lock()
            .await
            .remove(&installation_id);
    }
}

pub fn app_jwt(state: &AppState) -> anyhow::Result<String> {
    // Sync path kept for callers that cannot await; prefer cached_app_jwt.
    let app_id = state
        .config
        .github_app_id
        .as_deref()
        .ok_or_else(|| anyhow::anyhow!("github app id missing"))?;
    let private_key = state
        .config
        .github_private_key
        .as_deref()
        .ok_or_else(|| anyhow::anyhow!("github private key missing"))?;
    github::create_app_jwt(app_id, private_key).map_err(Into::into)
}

async fn cached_app_jwt(state: &AppState) -> anyhow::Result<String> {
    {
        let guard = state.token_cache.app_jwt.lock().await;
        if let Some(cached) = guard.as_ref() {
            if cached.expires_at > Instant::now() {
                return Ok(cached.token.clone());
            }
        }
    }
    let token = app_jwt(state)?;
    let mut guard = state.token_cache.app_jwt.lock().await;
    *guard = Some(CachedJwt {
        token: token.clone(),
        // App JWTs are valid for ~9 minutes; refresh a minute early.
        expires_at: Instant::now() + Duration::from_secs(8 * 60),
    });
    Ok(token)
}

pub async fn installation_token(state: &AppState, installation_id: u64) -> anyhow::Result<String> {
    let now = OffsetDateTime::now_utc();
    {
        let guard = state.token_cache.installation_tokens.lock().await;
        if let Some(cached) = guard.get(&installation_id) {
            if cached.expires_at > now + time::Duration::seconds(60) {
                return Ok(cached.token.clone());
            }
        }
    }

    let jwt = cached_app_jwt(state).await?;
    let client = github::ReqwestGitHubClient::with_timeouts(
        &state.config.github_api_base,
        state.config.github_http_timeout_secs,
        state.config.github_http_connect_timeout_secs,
    );
    let exchanged = client
        .exchange_installation_token(&jwt, installation_id)
        .await?;
    let expires_at = OffsetDateTime::parse(
        &exchanged.expires_at,
        &time::format_description::well_known::Rfc3339,
    )
    .unwrap_or_else(|_| now + time::Duration::minutes(50));

    let mut guard = state.token_cache.installation_tokens.lock().await;
    guard.insert(
        installation_id,
        CachedToken {
            token: exchanged.token.clone(),
            expires_at,
        },
    );
    Ok(exchanged.token)
}

pub fn github_client(state: &AppState) -> std::sync::Arc<github::ReqwestGitHubClient> {
    state.github_client.clone()
}

pub type SharedTokenCache = Arc<TokenCache>;
