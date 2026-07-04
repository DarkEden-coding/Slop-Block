use async_trait::async_trait;
use hmac::{Hmac, Mac};
use jsonwebtoken::{Algorithm, EncodingKey, Header};
use reqwest::StatusCode;
use serde::{Deserialize, Serialize};
use sha2::Sha256;
use std::collections::HashMap;
use time::{Duration, OffsetDateTime};

type HmacSha256 = Hmac<Sha256>;

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct WebhookDelivery {
    pub id: String,
    pub event: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct WebhookHeaders {
    pub delivery: String,
    pub event: String,
    pub signature_sha256: String,
}

impl WebhookHeaders {
    pub fn new(
        delivery: impl Into<String>,
        event: impl Into<String>,
        signature_sha256: impl Into<String>,
    ) -> Self {
        Self {
            delivery: delivery.into(),
            event: event.into(),
            signature_sha256: signature_sha256.into(),
        }
    }
}

pub fn verify_webhook_signature(
    secret: &[u8],
    body: &[u8],
    signature_header: &str,
) -> Result<(), GitHubError> {
    let signature = signature_header
        .strip_prefix("sha256=")
        .ok_or(GitHubError::InvalidSignature)?;
    let provided = hex::decode(signature).map_err(|_| GitHubError::InvalidSignature)?;
    let mut mac = HmacSha256::new_from_slice(secret).map_err(|_| GitHubError::InvalidSignature)?;
    mac.update(body);
    mac.verify_slice(&provided)
        .map_err(|_| GitHubError::InvalidSignature)
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct User {
    pub id: u64,
    pub login: String,
    pub avatar_url: Option<String>,
    pub html_url: Option<String>,
}
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct Repository {
    pub id: u64,
    pub name: String,
    pub full_name: String,
    pub private: bool,
    pub owner: User,
    pub default_branch: Option<String>,
}
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct Installation {
    pub id: u64,
    pub account: Option<User>,
}
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct Label {
    pub id: u64,
    pub name: String,
    pub color: Option<String>,
    pub description: Option<String>,
}
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct Issue {
    pub id: u64,
    pub number: u64,
    pub title: String,
    pub user: User,
    pub state: String,
    pub labels: Vec<Label>,
    pub body: Option<String>,
    pub pull_request: Option<serde_json::Value>,
}
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct PullRequestRef {
    #[serde(rename = "ref")]
    pub ref_name: String,
    pub sha: String,
    pub repo: Repository,
}
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct PullRequest {
    pub id: u64,
    pub number: u64,
    pub title: String,
    pub user: User,
    pub state: String,
    pub body: Option<String>,
    pub head: PullRequestRef,
    pub base: PullRequestRef,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct InstallationEvent {
    pub action: String,
    pub installation: Installation,
    pub repositories: Option<Vec<Repository>>,
    pub sender: User,
}
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct RepositoriesEvent {
    pub action: String,
    pub installation: Installation,
    pub repository_selection: Option<String>,
    pub repositories_added: Vec<Repository>,
    pub repositories_removed: Vec<Repository>,
    pub sender: User,
}
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct IssuesEvent {
    pub action: String,
    pub issue: Issue,
    pub repository: Repository,
    pub installation: Option<Installation>,
    pub sender: User,
}
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct PullRequestEvent {
    pub action: String,
    pub number: u64,
    pub pull_request: PullRequest,
    pub repository: Repository,
    pub installation: Option<Installation>,
    pub sender: User,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct InstallationToken {
    pub token: String,
    pub expires_at: String,
    pub permissions: Option<HashMap<String, String>>,
    pub repository_selection: Option<String>,
}
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct OAuthToken {
    pub access_token: String,
    pub token_type: String,
    pub scope: String,
}
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct IssueComment {
    pub id: u64,
    pub body: String,
    pub html_url: Option<String>,
}
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct CheckRun {
    pub id: u64,
    pub name: String,
    pub head_sha: String,
    pub status: Option<String>,
    pub conclusion: Option<String>,
}
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct CollaboratorPermission {
    pub permission: String,
    pub user: User,
}

#[derive(Debug, Clone, Serialize)]
pub struct CheckRunRequest {
    pub name: String,
    pub head_sha: String,
    pub status: Option<String>,
    pub conclusion: Option<String>,
    pub details_url: Option<String>,
    pub output: Option<serde_json::Value>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct JwtClaims {
    iat: i64,
    exp: i64,
    iss: String,
}

pub fn create_app_jwt(app_id: impl ToString, private_key_pem: &str) -> Result<String, GitHubError> {
    create_app_jwt_at(app_id, private_key_pem, OffsetDateTime::now_utc())
}

pub fn create_app_jwt_at(
    app_id: impl ToString,
    private_key_pem: &str,
    now: OffsetDateTime,
) -> Result<String, GitHubError> {
    let claims = JwtClaims {
        iat: (now - Duration::seconds(60)).unix_timestamp(),
        exp: (now + Duration::minutes(9)).unix_timestamp(),
        iss: app_id.to_string(),
    };
    let key = EncodingKey::from_rsa_pem(private_key_pem.as_bytes())
        .map_err(|e| GitHubError::Jwt(e.to_string()))?;
    jsonwebtoken::encode(&Header::new(Algorithm::RS256), &claims, &key)
        .map_err(|e| GitHubError::Jwt(e.to_string()))
}

#[async_trait]
pub trait GitHubApi: Send + Sync {
    async fn exchange_installation_token(
        &self,
        app_jwt: &str,
        installation_id: u64,
    ) -> Result<InstallationToken, GitHubError>;
    async fn exchange_oauth_code(
        &self,
        client_id: &str,
        client_secret: &str,
        code: &str,
        redirect_uri: Option<&str>,
    ) -> Result<OAuthToken, GitHubError>;
    async fn current_user(&self, access_token: &str) -> Result<User, GitHubError>;
    async fn user_installations(
        &self,
        access_token: &str,
    ) -> Result<Vec<Installation>, GitHubError>;
    async fn installation_repositories(&self, token: &str) -> Result<Vec<Repository>, GitHubError>;
    async fn list_open_issues(
        &self,
        token: &str,
        owner: &str,
        repo: &str,
    ) -> Result<Vec<Issue>, GitHubError>;
    async fn list_open_pull_requests(
        &self,
        token: &str,
        owner: &str,
        repo: &str,
    ) -> Result<Vec<PullRequest>, GitHubError>;
    async fn add_labels(
        &self,
        token: &str,
        owner: &str,
        repo: &str,
        issue_number: u64,
        labels: &[String],
    ) -> Result<Vec<Label>, GitHubError>;
    async fn remove_label(
        &self,
        token: &str,
        owner: &str,
        repo: &str,
        issue_number: u64,
        label: &str,
    ) -> Result<(), GitHubError>;
    async fn create_issue_comment(
        &self,
        token: &str,
        owner: &str,
        repo: &str,
        issue_number: u64,
        body: &str,
    ) -> Result<IssueComment, GitHubError>;
    async fn update_issue_comment(
        &self,
        token: &str,
        owner: &str,
        repo: &str,
        comment_id: u64,
        body: &str,
    ) -> Result<IssueComment, GitHubError>;
    async fn delete_issue_comment(
        &self,
        token: &str,
        owner: &str,
        repo: &str,
        comment_id: u64,
    ) -> Result<(), GitHubError>;
    async fn create_check_run(
        &self,
        token: &str,
        owner: &str,
        repo: &str,
        req: &CheckRunRequest,
    ) -> Result<CheckRun, GitHubError>;
    async fn update_check_run(
        &self,
        token: &str,
        owner: &str,
        repo: &str,
        check_run_id: u64,
        req: &CheckRunRequest,
    ) -> Result<CheckRun, GitHubError>;
    async fn collaborator_permission(
        &self,
        token: &str,
        owner: &str,
        repo: &str,
        username: &str,
    ) -> Result<CollaboratorPermission, GitHubError>;
}

#[derive(Clone)]
pub struct ReqwestGitHubClient {
    client: reqwest::Client,
    api_base: String,
}

impl Default for ReqwestGitHubClient {
    fn default() -> Self {
        Self::new()
    }
}
impl ReqwestGitHubClient {
    pub fn new() -> Self {
        Self {
            client: reqwest::Client::new(),
            api_base: "https://api.github.com".into(),
        }
    }
    pub fn with_base_url(api_base: impl Into<String>) -> Self {
        Self {
            client: reqwest::Client::new(),
            api_base: api_base.into(),
        }
    }
    fn url(&self, path: &str) -> String {
        format!("{}{}", self.api_base.trim_end_matches('/'), path)
    }
    fn repo_path(owner: &str, repo: &str, suffix: &str) -> String {
        format!(
            "/repos/{}/{}{}",
            path_segment(owner),
            path_segment(repo),
            suffix
        )
    }
    fn authed(&self, method: reqwest::Method, path: &str, token: &str) -> reqwest::RequestBuilder {
        self.client
            .request(method, self.url(path))
            .bearer_auth(token)
            .header("Accept", "application/vnd.github+json")
            .header("X-GitHub-Api-Version", "2022-11-28")
            .header("User-Agent", "GHO-github-human-auth")
    }
    async fn send_json<T: for<'de> Deserialize<'de>>(
        &self,
        rb: reqwest::RequestBuilder,
    ) -> Result<T, GitHubError> {
        let r = rb.send().await.map_err(GitHubError::Http)?;
        let s = r.status();
        if !s.is_success() {
            return Err(classify_api_error(r).await);
        }
        r.json().await.map_err(GitHubError::Http)
    }
}

#[async_trait]
impl GitHubApi for ReqwestGitHubClient {
    async fn exchange_installation_token(
        &self,
        app_jwt: &str,
        installation_id: u64,
    ) -> Result<InstallationToken, GitHubError> {
        self.send_json(self.authed(
            reqwest::Method::POST,
            &format!("/app/installations/{installation_id}/access_tokens"),
            app_jwt,
        ))
        .await
    }
    async fn exchange_oauth_code(
        &self,
        client_id: &str,
        client_secret: &str,
        code: &str,
        redirect_uri: Option<&str>,
    ) -> Result<OAuthToken, GitHubError> {
        #[derive(Serialize)]
        struct Req<'a> {
            client_id: &'a str,
            client_secret: &'a str,
            code: &'a str,
            redirect_uri: Option<&'a str>,
        }
        self.send_json(
            self.client
                .post(oauth_token_url(&self.api_base))
                .header("Accept", "application/json")
                .header("User-Agent", "GHO-github-human-auth")
                .json(&Req {
                    client_id,
                    client_secret,
                    code,
                    redirect_uri,
                }),
        )
        .await
    }
    async fn current_user(&self, access_token: &str) -> Result<User, GitHubError> {
        self.send_json(self.authed(reqwest::Method::GET, "/user", access_token))
            .await
    }
    async fn user_installations(
        &self,
        access_token: &str,
    ) -> Result<Vec<Installation>, GitHubError> {
        #[derive(Deserialize)]
        struct Resp {
            installations: Vec<Installation>,
        }
        let mut out = Vec::new();
        for page in 1.. {
            let resp: Resp = self
                .send_json(self.authed(
                    reqwest::Method::GET,
                    &format!("/user/installations?per_page=100&page={page}"),
                    access_token,
                ))
                .await?;
            let done = resp.installations.len() < 100;
            out.extend(resp.installations);
            if done {
                break;
            }
        }
        Ok(out)
    }
    async fn installation_repositories(&self, token: &str) -> Result<Vec<Repository>, GitHubError> {
        #[derive(Deserialize)]
        struct Resp {
            repositories: Vec<Repository>,
        }
        let mut out = Vec::new();
        for page in 1.. {
            let resp: Resp = self
                .send_json(self.authed(
                    reqwest::Method::GET,
                    &format!("/installation/repositories?per_page=100&page={page}"),
                    token,
                ))
                .await?;
            let done = resp.repositories.len() < 100;
            out.extend(resp.repositories);
            if done {
                break;
            }
        }
        Ok(out)
    }
    async fn list_open_issues(
        &self,
        token: &str,
        owner: &str,
        repo: &str,
    ) -> Result<Vec<Issue>, GitHubError> {
        let mut out = Vec::new();
        for page in 1.. {
            let resp: Vec<Issue> = self
                .send_json(self.authed(
                    reqwest::Method::GET,
                    &Self::repo_path(
                        owner,
                        repo,
                        &format!("/issues?state=open&per_page=100&page={page}"),
                    ),
                    token,
                ))
                .await?;
            let done = resp.len() < 100;
            out.extend(resp);
            if done {
                break;
            }
        }
        Ok(out)
    }
    async fn list_open_pull_requests(
        &self,
        token: &str,
        owner: &str,
        repo: &str,
    ) -> Result<Vec<PullRequest>, GitHubError> {
        let mut out = Vec::new();
        for page in 1.. {
            let resp: Vec<PullRequest> = self
                .send_json(self.authed(
                    reqwest::Method::GET,
                    &Self::repo_path(
                        owner,
                        repo,
                        &format!("/pulls?state=open&per_page=100&page={page}"),
                    ),
                    token,
                ))
                .await?;
            let done = resp.len() < 100;
            out.extend(resp);
            if done {
                break;
            }
        }
        Ok(out)
    }
    async fn add_labels(
        &self,
        token: &str,
        owner: &str,
        repo: &str,
        issue_number: u64,
        labels: &[String],
    ) -> Result<Vec<Label>, GitHubError> {
        self.send_json(
            self.authed(
                reqwest::Method::POST,
                &Self::repo_path(owner, repo, &format!("/issues/{issue_number}/labels")),
                token,
            )
            .json(&serde_json::json!({"labels": labels})),
        )
        .await
    }
    async fn remove_label(
        &self,
        token: &str,
        owner: &str,
        repo: &str,
        issue_number: u64,
        label: &str,
    ) -> Result<(), GitHubError> {
        let r = self
            .authed(
                reqwest::Method::DELETE,
                &Self::repo_path(
                    owner,
                    repo,
                    &format!("/issues/{issue_number}/labels/{}", path_segment(label)),
                ),
                token,
            )
            .send()
            .await
            .map_err(GitHubError::Http)?;
        if r.status().is_success() || r.status() == StatusCode::NOT_FOUND {
            Ok(())
        } else {
            Err(GitHubError::ApiStatus(r.status()))
        }
    }
    async fn create_issue_comment(
        &self,
        token: &str,
        owner: &str,
        repo: &str,
        issue_number: u64,
        body: &str,
    ) -> Result<IssueComment, GitHubError> {
        self.send_json(
            self.authed(
                reqwest::Method::POST,
                &Self::repo_path(owner, repo, &format!("/issues/{issue_number}/comments")),
                token,
            )
            .json(&serde_json::json!({"body": body})),
        )
        .await
    }
    async fn update_issue_comment(
        &self,
        token: &str,
        owner: &str,
        repo: &str,
        comment_id: u64,
        body: &str,
    ) -> Result<IssueComment, GitHubError> {
        self.send_json(
            self.authed(
                reqwest::Method::PATCH,
                &Self::repo_path(owner, repo, &format!("/issues/comments/{comment_id}")),
                token,
            )
            .json(&serde_json::json!({"body": body})),
        )
        .await
    }
    async fn delete_issue_comment(
        &self,
        token: &str,
        owner: &str,
        repo: &str,
        comment_id: u64,
    ) -> Result<(), GitHubError> {
        let r = self
            .authed(
                reqwest::Method::DELETE,
                &Self::repo_path(owner, repo, &format!("/issues/comments/{comment_id}")),
                token,
            )
            .send()
            .await
            .map_err(GitHubError::Http)?;
        if r.status().is_success() || r.status() == StatusCode::NOT_FOUND {
            Ok(())
        } else {
            Err(GitHubError::ApiStatus(r.status()))
        }
    }
    async fn create_check_run(
        &self,
        token: &str,
        owner: &str,
        repo: &str,
        req: &CheckRunRequest,
    ) -> Result<CheckRun, GitHubError> {
        self.send_json(
            self.authed(
                reqwest::Method::POST,
                &Self::repo_path(owner, repo, "/check-runs"),
                token,
            )
            .json(req),
        )
        .await
    }
    async fn update_check_run(
        &self,
        token: &str,
        owner: &str,
        repo: &str,
        check_run_id: u64,
        req: &CheckRunRequest,
    ) -> Result<CheckRun, GitHubError> {
        self.send_json(
            self.authed(
                reqwest::Method::PATCH,
                &Self::repo_path(owner, repo, &format!("/check-runs/{check_run_id}")),
                token,
            )
            .json(req),
        )
        .await
    }
    async fn collaborator_permission(
        &self,
        token: &str,
        owner: &str,
        repo: &str,
        username: &str,
    ) -> Result<CollaboratorPermission, GitHubError> {
        self.send_json(self.authed(
            reqwest::Method::GET,
            &Self::repo_path(
                owner,
                repo,
                &format!("/collaborators/{}/permission", path_segment(username)),
            ),
            token,
        ))
        .await
    }
}

async fn classify_api_error(response: reqwest::Response) -> GitHubError {
    let status = response.status();
    let headers = response.headers().clone();
    let retry_after_seconds = headers
        .get("retry-after")
        .and_then(|v| v.to_str().ok())
        .and_then(|v| v.parse::<u64>().ok());
    let remaining = headers
        .get("x-ratelimit-remaining")
        .and_then(|v| v.to_str().ok())
        .and_then(|v| v.parse::<i64>().ok());
    let reset_at = headers
        .get("x-ratelimit-reset")
        .and_then(|v| v.to_str().ok())
        .and_then(|v| v.parse::<i64>().ok())
        .and_then(|ts| OffsetDateTime::from_unix_timestamp(ts).ok());
    let message = response.text().await.ok();
    let lower = message.as_deref().unwrap_or_default().to_ascii_lowercase();
    if lower.contains("secondary rate limit")
        || lower.contains("abuse detection")
        || status == StatusCode::TOO_MANY_REQUESTS
    {
        return GitHubError::SecondaryRateLimited {
            retry_after_seconds,
            message,
        };
    }
    if remaining == Some(0) || retry_after_seconds.is_some() {
        return GitHubError::RateLimited {
            status,
            reset_at,
            retry_after_seconds,
            remaining,
            message,
        };
    }
    GitHubError::ApiStatus(status)
}

fn oauth_token_url(api_base: &str) -> String {
    if api_base.trim_end_matches('/') == "https://api.github.com" {
        "https://github.com/login/oauth/access_token".into()
    } else {
        format!(
            "{}/login/oauth/access_token",
            api_base.trim_end_matches('/')
        )
    }
}

fn path_segment(input: &str) -> String {
    let mut encoded = String::with_capacity(input.len());
    for byte in input.bytes() {
        match byte {
            b'A'..=b'Z' | b'a'..=b'z' | b'0'..=b'9' | b'-' | b'.' | b'_' | b'~' => {
                encoded.push(byte as char)
            }
            _ => encoded.push_str(&format!("%{byte:02X}")),
        }
    }
    encoded
}

#[derive(Debug, thiserror::Error)]
pub enum GitHubError {
    #[error("GitHub integration is not configured")]
    NotConfigured,
    #[error("invalid GitHub webhook signature")]
    InvalidSignature,
    #[error("JWT error: {0}")]
    Jwt(String),
    #[error("GitHub HTTP error: {0}")]
    Http(#[from] reqwest::Error),
    #[error("GitHub API returned status {0}")]
    ApiStatus(StatusCode),
    #[error("GitHub rate limited until {reset_at:?}: {message:?}")]
    RateLimited {
        status: StatusCode,
        reset_at: Option<OffsetDateTime>,
        retry_after_seconds: Option<u64>,
        remaining: Option<i64>,
        message: Option<String>,
    },
    #[error("GitHub secondary rate limited: {message:?}")]
    SecondaryRateLimited {
        retry_after_seconds: Option<u64>,
        message: Option<String>,
    },
}

#[cfg(test)]
mod tests {
    use super::*;
    use base64::{engine::general_purpose::URL_SAFE_NO_PAD, Engine as _};

    #[test]
    fn encodes_path_segments() {
        assert_eq!(
            path_segment("owner/name with space"),
            "owner%2Fname%20with%20space"
        );
    }

    #[test]
    fn verifies_webhook_signature() {
        let secret = b"topsecret";
        let body = br#"{"action":"opened"}"#;
        let mut mac = HmacSha256::new_from_slice(secret).unwrap();
        mac.update(body);
        let sig = format!("sha256={}", hex::encode(mac.finalize().into_bytes()));
        assert!(verify_webhook_signature(secret, body, &sig).is_ok());
        assert!(verify_webhook_signature(secret, b"{}", &sig).is_err());
    }

    #[test]
    fn creates_decodable_app_jwt() {
        let pem = include_str!("../tests/fixtures/test_rsa.pem");
        let now = OffsetDateTime::from_unix_timestamp(1_700_000_000).unwrap();
        let jwt = create_app_jwt_at("12345", pem, now).unwrap();
        let parts: Vec<&str> = jwt.split('.').collect();
        assert_eq!(parts.len(), 3);
        let header: serde_json::Value =
            serde_json::from_slice(&URL_SAFE_NO_PAD.decode(parts[0]).unwrap()).unwrap();
        assert_eq!(header["alg"], "RS256");
        let claims: JwtClaims =
            serde_json::from_slice(&URL_SAFE_NO_PAD.decode(parts[1]).unwrap()).unwrap();
        assert_eq!(claims.iss, "12345");
        assert_eq!(claims.iat, 1_699_999_940);
        assert_eq!(claims.exp, 1_700_000_540);
        assert!(!parts[2].is_empty());
    }
}
