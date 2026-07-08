use serde::{Deserialize, Serialize};
use std::collections::HashMap;

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
pub(crate) struct JwtClaims {
    pub(crate) iat: i64,
    pub(crate) exp: i64,
    pub(crate) iss: String,
}
