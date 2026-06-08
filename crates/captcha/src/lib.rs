use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct CaptchaVerification {
    pub success: bool,
    pub provider: String,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub error_codes: Vec<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub challenge_ts: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub hostname: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub action: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub cdata: Option<String>,
}

impl CaptchaVerification {
    pub fn success(provider: impl Into<String>) -> Self {
        Self {
            success: true,
            provider: provider.into(),
            error_codes: Vec::new(),
            challenge_ts: None,
            hostname: None,
            action: None,
            cdata: None,
        }
    }

    pub fn failure(provider: impl Into<String>, error_codes: Vec<String>) -> Self {
        Self {
            success: false,
            provider: provider.into(),
            error_codes,
            challenge_ts: None,
            hostname: None,
            action: None,
            cdata: None,
        }
    }
}

#[derive(Debug, thiserror::Error)]
pub enum CaptchaError {
    #[error("captcha provider request failed: {0}")]
    Request(#[from] reqwest::Error),
}

#[async_trait::async_trait]
pub trait CaptchaProvider: Send + Sync {
    async fn verify(
        &self,
        token: &str,
        remote_ip: Option<&str>,
    ) -> Result<CaptchaVerification, CaptchaError>;
}

#[derive(Debug, Clone)]
pub struct CloudflareTurnstile {
    client: reqwest::Client,
    secret: String,
    endpoint: String,
}

impl CloudflareTurnstile {
    pub const DEFAULT_ENDPOINT: &'static str =
        "https://challenges.cloudflare.com/turnstile/v0/siteverify";

    pub fn new(secret: impl Into<String>) -> Self {
        Self::with_endpoint(secret, Self::DEFAULT_ENDPOINT)
    }

    pub fn with_endpoint(secret: impl Into<String>, endpoint: impl Into<String>) -> Self {
        Self {
            client: reqwest::Client::new(),
            secret: secret.into(),
            endpoint: endpoint.into(),
        }
    }

    pub fn with_client_and_endpoint(
        client: reqwest::Client,
        secret: impl Into<String>,
        endpoint: impl Into<String>,
    ) -> Self {
        Self {
            client,
            secret: secret.into(),
            endpoint: endpoint.into(),
        }
    }
}

#[derive(Debug, Deserialize)]
struct TurnstileSiteverifyResponse {
    success: bool,
    #[serde(default, rename = "error-codes")]
    error_codes: Vec<String>,
    #[serde(default)]
    challenge_ts: Option<String>,
    #[serde(default)]
    hostname: Option<String>,
    #[serde(default)]
    action: Option<String>,
    #[serde(default)]
    cdata: Option<String>,
}

#[async_trait::async_trait]
impl CaptchaProvider for CloudflareTurnstile {
    async fn verify(
        &self,
        token: &str,
        remote_ip: Option<&str>,
    ) -> Result<CaptchaVerification, CaptchaError> {
        let mut form = vec![
            ("secret".to_string(), self.secret.clone()),
            ("response".to_string(), token.to_string()),
        ];
        if let Some(remote_ip) = remote_ip {
            form.push(("remoteip".to_string(), remote_ip.to_string()));
        }

        let response = self
            .client
            .post(&self.endpoint)
            .form(&form)
            .send()
            .await?
            .error_for_status()?
            .json::<TurnstileSiteverifyResponse>()
            .await?;

        Ok(CaptchaVerification {
            success: response.success,
            provider: "cloudflare-turnstile".to_string(),
            error_codes: response.error_codes,
            challenge_ts: response.challenge_ts,
            hostname: response.hostname,
            action: response.action,
            cdata: response.cdata,
        })
    }
}

#[derive(Debug, Clone, Copy)]
pub struct DevBypass {
    enabled: bool,
}

impl DevBypass {
    pub const TOKEN: &'static str = "dev-pass";

    pub fn new(enabled: bool) -> Self {
        Self { enabled }
    }
}

#[async_trait::async_trait]
impl CaptchaProvider for DevBypass {
    async fn verify(
        &self,
        token: &str,
        _remote_ip: Option<&str>,
    ) -> Result<CaptchaVerification, CaptchaError> {
        if self.enabled && token == Self::TOKEN {
            Ok(CaptchaVerification::success("dev-bypass"))
        } else {
            Ok(CaptchaVerification::failure(
                "dev-bypass",
                vec!["invalid-input-response".to_string()],
            ))
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tokio::io::{AsyncReadExt, AsyncWriteExt};
    use tokio::net::TcpListener;

    #[tokio::test]
    async fn dev_bypass_only_accepts_expected_token_when_enabled() {
        let provider = DevBypass::new(true);
        assert!(provider.verify("dev-pass", None).await.unwrap().success);
        assert!(!provider.verify("wrong", None).await.unwrap().success);
        assert!(
            !DevBypass::new(false)
                .verify("dev-pass", None)
                .await
                .unwrap()
                .success
        );
    }

    #[tokio::test]
    async fn turnstile_posts_to_injected_endpoint_and_parses_response() {
        let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
        let addr = listener.local_addr().unwrap();

        let server = tokio::spawn(async move {
            let (mut socket, _) = listener.accept().await.unwrap();
            let mut buffer = vec![0_u8; 4096];
            let n = socket.read(&mut buffer).await.unwrap();
            let request = String::from_utf8_lossy(&buffer[..n]);
            assert!(request.starts_with("POST /siteverify HTTP/1.1"));
            assert!(request.contains("secret=test-secret"));
            assert!(request.contains("response=test-token"));
            assert!(request.contains("remoteip=203.0.113.10"));

            let body =
                r#"{"success":true,"hostname":"example.com","action":"login","cdata":"abc"}"#;
            let response = format!(
                "HTTP/1.1 200 OK\r\ncontent-type: application/json\r\ncontent-length: {}\r\nconnection: close\r\n\r\n{}",
                body.len(),
                body
            );
            socket.write_all(response.as_bytes()).await.unwrap();
        });

        let provider =
            CloudflareTurnstile::with_endpoint("test-secret", format!("http://{addr}/siteverify"));
        let result = provider
            .verify("test-token", Some("203.0.113.10"))
            .await
            .unwrap();

        assert!(result.success);
        assert_eq!(result.provider, "cloudflare-turnstile");
        assert_eq!(result.hostname.as_deref(), Some("example.com"));
        assert_eq!(result.action.as_deref(), Some("login"));
        assert_eq!(result.cdata.as_deref(), Some("abc"));
        server.await.unwrap();
    }
}
