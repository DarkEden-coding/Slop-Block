use async_trait::async_trait;
use reqwest::StatusCode;
use serde::{Deserialize, Serialize};
use time::OffsetDateTime;

use crate::error::GitHubError;
use crate::types::{
    CheckRun, CheckRunRequest, CollaboratorPermission, Installation, InstallationToken, Issue,
    IssueComment, Label, OAuthToken, PullRequest, Repository, User,
};

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
    async fn list_open_issues_page(
        &self,
        token: &str,
        owner: &str,
        repo: &str,
        page: u32,
        per_page: u32,
    ) -> Result<Vec<Issue>, GitHubError>;
    async fn list_open_pull_requests_page(
        &self,
        token: &str,
        owner: &str,
        repo: &str,
        page: u32,
        per_page: u32,
    ) -> Result<Vec<PullRequest>, GitHubError>;
    async fn list_open_issues_by_creator(
        &self,
        token: &str,
        owner: &str,
        repo: &str,
        creator: &str,
    ) -> Result<Vec<Issue>, GitHubError>;
    async fn issue_labels(
        &self,
        token: &str,
        owner: &str,
        repo: &str,
        issue_number: u64,
    ) -> Result<Vec<Label>, GitHubError>;
    async fn add_labels(
        &self,
        token: &str,
        owner: &str,
        repo: &str,
        issue_number: u64,
        labels: &[String],
    ) -> Result<Vec<Label>, GitHubError>;
    async fn set_labels(
        &self,
        token: &str,
        owner: &str,
        repo: &str,
        issue_number: u64,
        labels: &[String],
    ) -> Result<Vec<Label>, GitHubError>;
    async fn create_label(
        &self,
        token: &str,
        owner: &str,
        repo: &str,
        name: &str,
        color: &str,
        description: Option<&str>,
    ) -> Result<Label, GitHubError>;
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

impl ReqwestGitHubClient {
    pub fn with_base_url(api_base: impl Into<String>) -> Self {
        Self::with_timeouts(api_base, 30, 10)
    }

    pub fn with_timeouts(
        api_base: impl Into<String>,
        request_timeout_secs: u64,
        connect_timeout_secs: u64,
    ) -> Self {
        let client = reqwest::Client::builder()
            .timeout(std::time::Duration::from_secs(request_timeout_secs.max(1)))
            .connect_timeout(std::time::Duration::from_secs(connect_timeout_secs.max(1)))
            .pool_max_idle_per_host(4)
            .build()
            .unwrap_or_else(|_| reqwest::Client::new());
        Self {
            client,
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
        for page in 1..=MAX_LIST_PAGES {
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
        for page in 1..=MAX_LIST_PAGES {
            let resp = self
                .list_open_issues_page(token, owner, repo, page, 100)
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
        for page in 1..=MAX_LIST_PAGES {
            let resp = self
                .list_open_pull_requests_page(token, owner, repo, page, 100)
                .await?;
            let done = resp.len() < 100;
            out.extend(resp);
            if done {
                break;
            }
        }
        Ok(out)
    }
    async fn list_open_issues_page(
        &self,
        token: &str,
        owner: &str,
        repo: &str,
        page: u32,
        per_page: u32,
    ) -> Result<Vec<Issue>, GitHubError> {
        let per_page = per_page.clamp(1, 100);
        let page = page.max(1);
        self.send_json(self.authed(
            reqwest::Method::GET,
            &Self::repo_path(
                owner,
                repo,
                &format!("/issues?state=open&per_page={per_page}&page={page}"),
            ),
            token,
        ))
        .await
    }
    async fn list_open_pull_requests_page(
        &self,
        token: &str,
        owner: &str,
        repo: &str,
        page: u32,
        per_page: u32,
    ) -> Result<Vec<PullRequest>, GitHubError> {
        let per_page = per_page.clamp(1, 100);
        let page = page.max(1);
        self.send_json(self.authed(
            reqwest::Method::GET,
            &Self::repo_path(
                owner,
                repo,
                &format!("/pulls?state=open&per_page={per_page}&page={page}"),
            ),
            token,
        ))
        .await
    }
    async fn list_open_issues_by_creator(
        &self,
        token: &str,
        owner: &str,
        repo: &str,
        creator: &str,
    ) -> Result<Vec<Issue>, GitHubError> {
        let mut out = Vec::new();
        let creator = path_segment(creator);
        for page in 1..=MAX_LIST_PAGES {
            let resp: Vec<Issue> = self
                .send_json(self.authed(
                    reqwest::Method::GET,
                    &Self::repo_path(
                        owner,
                        repo,
                        &format!("/issues?state=open&creator={creator}&per_page=100&page={page}"),
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
    async fn issue_labels(
        &self,
        token: &str,
        owner: &str,
        repo: &str,
        issue_number: u64,
    ) -> Result<Vec<Label>, GitHubError> {
        self.send_json(self.authed(
            reqwest::Method::GET,
            &Self::repo_path(owner, repo, &format!("/issues/{issue_number}/labels")),
            token,
        ))
        .await
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
    async fn set_labels(
        &self,
        token: &str,
        owner: &str,
        repo: &str,
        issue_number: u64,
        labels: &[String],
    ) -> Result<Vec<Label>, GitHubError> {
        self.send_json(
            self.authed(
                reqwest::Method::PUT,
                &Self::repo_path(owner, repo, &format!("/issues/{issue_number}/labels")),
                token,
            )
            .json(&serde_json::json!({"labels": labels})),
        )
        .await
    }
    async fn create_label(
        &self,
        token: &str,
        owner: &str,
        repo: &str,
        name: &str,
        color: &str,
        description: Option<&str>,
    ) -> Result<Label, GitHubError> {
        self.send_json(
            self.authed(
                reqwest::Method::POST,
                &Self::repo_path(owner, repo, "/labels"),
                token,
            )
            .json(&serde_json::json!({"name": name, "color": color, "description": description})),
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
            Err(classify_api_error(r).await)
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
            Err(classify_api_error(r).await)
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

const MAX_LIST_PAGES: u32 = 100;

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

#[cfg(test)]
mod tests {
    use super::path_segment;

    #[test]
    fn encodes_path_segments() {
        assert_eq!(
            path_segment("owner/name with space"),
            "owner%2Fname%20with%20space"
        );
    }
}
