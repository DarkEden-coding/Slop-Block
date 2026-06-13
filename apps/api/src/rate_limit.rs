use axum::{
    extract::{ConnectInfo, Request, State},
    http::{header::HeaderMap, StatusCode},
    middleware::Next,
    response::{IntoResponse, Response},
    Json,
};
use serde_json::json;
use std::{
    collections::HashMap,
    net::SocketAddr,
    sync::Mutex,
    time::{Duration, Instant},
};

use crate::AppState;

const MAX_TRACKED_KEYS: usize = 10_000;

/// Fixed-window per-client rate limiter for the unauthenticated verification and
/// OAuth endpoints. Token guessing is already infeasible (256-bit tokens); this guards
/// against CAPTCHA siteverify relay abuse and GitHub API amplification.
#[derive(Debug)]
pub struct RateLimiter {
    max_requests: u32,
    window: Duration,
    buckets: Mutex<HashMap<String, (u32, Instant)>>,
}

impl RateLimiter {
    pub fn new(max_requests: u32, window: Duration) -> Self {
        Self {
            max_requests,
            window,
            buckets: Mutex::new(HashMap::new()),
        }
    }

    pub fn allow(&self, key: &str) -> bool {
        let now = Instant::now();
        let mut buckets = self.buckets.lock().unwrap_or_else(|e| e.into_inner());
        if buckets.len() >= MAX_TRACKED_KEYS && !buckets.contains_key(key) {
            let window = self.window;
            buckets.retain(|_, (_, start)| now.duration_since(*start) < window);
        }
        let entry = buckets.entry(key.to_owned()).or_insert((0, now));
        if now.duration_since(entry.1) >= self.window {
            *entry = (0, now);
        }
        entry.0 = entry.0.saturating_add(1);
        entry.0 <= self.max_requests
    }
}

/// Best-effort client identity: Cloudflare/proxy headers first (the production
/// deployment terminates TLS at a Cloudflare tunnel), then the socket peer address.
fn client_key(headers: &HeaderMap, request: &Request, trust_proxy_headers: bool) -> String {
    if trust_proxy_headers {
        if let Some(ip) = headers
            .get("cf-connecting-ip")
            .and_then(|v| v.to_str().ok())
            .map(str::trim)
            .filter(|v| !v.is_empty())
        {
            return ip.to_owned();
        }
        if let Some(ip) = headers
            .get("x-forwarded-for")
            .and_then(|v| v.to_str().ok())
            .and_then(|v| v.split(',').next())
            .map(str::trim)
            .filter(|v| !v.is_empty())
        {
            return ip.to_owned();
        }
    }
    request
        .extensions()
        .get::<ConnectInfo<SocketAddr>>()
        .map(|ConnectInfo(addr)| addr.ip().to_string())
        .unwrap_or_else(|| "unknown".to_owned())
}

pub async fn rate_limit_middleware(
    State(state): State<AppState>,
    request: Request,
    next: Next,
) -> Response {
    let key = client_key(
        request.headers(),
        &request,
        state.config.trust_proxy_headers,
    );
    if state.rate_limiter.allow(&key) {
        next.run(request).await
    } else {
        (
            StatusCode::TOO_MANY_REQUESTS,
            Json(json!({"error": {"code": "rate_limited", "message": "too many requests"}})),
        )
            .into_response()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn enforces_fixed_window_per_key() {
        let limiter = RateLimiter::new(3, Duration::from_secs(60));
        assert!(limiter.allow("a"));
        assert!(limiter.allow("a"));
        assert!(limiter.allow("a"));
        assert!(!limiter.allow("a"));
        assert!(limiter.allow("b"));
    }

    #[test]
    fn resets_after_window() {
        let limiter = RateLimiter::new(1, Duration::from_millis(10));
        assert!(limiter.allow("a"));
        assert!(!limiter.allow("a"));
        std::thread::sleep(Duration::from_millis(15));
        assert!(limiter.allow("a"));
    }
}
