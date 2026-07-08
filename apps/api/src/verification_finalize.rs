use std::collections::HashSet;

use github::{CheckRunRequest, GitHubApi};
use policy::VerificationPolicy;
use serde_json::json;

use crate::github_helpers::{ensure_policy_labels, github_content_delay_seconds};
use crate::github_tokens::installation_token;
use crate::AppState;

pub async fn load_repo_policy(pool: &db::PgPool, repository_id: i64) -> VerificationPolicy {
    match db::get_policy(pool, repository_id).await {
        Ok(Some(stored)) if stored.enabled => {
            serde_json::from_value(stored.policy).unwrap_or_default()
        }
        _ => VerificationPolicy::default(),
    }
}

pub async fn finalize(state: &AppState, s: &db::VerificationSession) {
    let Some(pool) = state.db.as_ref() else {
        return;
    };
    let login = s
        .metadata
        .get("login")
        .and_then(|v| v.as_str())
        .unwrap_or(&s.subject_id);
    let subject_id = s
        .github_user_id
        .map(|id| id.to_string())
        .unwrap_or_else(|| login.to_string());
    let _ = db::trust_subject(
        pool,
        s.repository_id,
        "github_user",
        &subject_id,
        s.github_user_id,
        Some("oauth_captcha_verified"),
        None,
        json!({"session": s.public_id, "login": login, "source": "oauth_captcha"}),
    )
    .await;
    let Ok(Some(repo)) = db::get_repository(pool, s.repository_id).await else {
        return;
    };
    let p: VerificationPolicy = match db::get_policy(pool, repo.repository_id).await {
        Ok(Some(pol)) if pol.enabled => serde_json::from_value(pol.policy).unwrap_or_default(),
        Ok(Some(_)) => return,
        _ => VerificationPolicy::default(),
    };
    let token = installation_token(state, repo.installation_id as u64)
        .await
        .ok();
    if let Some(token) = token {
        let client = github::ReqwestGitHubClient::with_base_url(&state.config.github_api_base);
        let mut sessions = vec![s.clone()];
        if let Some(user_id) = s.github_user_id {
            if let Ok(done) =
                db::complete_pending_sessions_for_user(pool, s.repository_id, user_id).await
            {
                sessions.extend(done.into_iter().filter(|session| session.id != s.id));
            }
        }
        let mut handled = HashSet::new();
        for session in sessions {
            let issue_number = session.subject_id.parse().unwrap_or(0);
            let subject_type = session.subject_type.as_str();
            apply_verified_state(
                pool,
                &client,
                &token,
                &repo,
                &p,
                subject_type,
                issue_number,
                None,
                None,
            )
            .await;
            handled.insert((subject_type.to_string(), issue_number));
            tokio::time::sleep(std::time::Duration::from_secs(
                github_content_delay_seconds(),
            ))
            .await;
        }

        if let Some(user_id) = s.github_user_id {
            if let Ok(open_items) = client
                .list_open_issues(&token, &repo.owner, &repo.name)
                .await
            {
                for issue in open_items {
                    if issue.user.id as i64 != user_id {
                        continue;
                    }
                    let subject_type = if issue.pull_request.is_some() {
                        "pull_request"
                    } else {
                        "issue"
                    };
                    if handled.contains(&(subject_type.to_string(), issue.number)) {
                        continue;
                    }
                    apply_verified_state(
                        pool,
                        &client,
                        &token,
                        &repo,
                        &p,
                        subject_type,
                        issue.number,
                        None,
                        Some(
                            issue
                                .labels
                                .iter()
                                .map(|label| label.name.clone())
                                .collect(),
                        ),
                    )
                    .await;
                    handled.insert((subject_type.to_string(), issue.number));
                    tokio::time::sleep(std::time::Duration::from_secs(
                        github_content_delay_seconds(),
                    ))
                    .await;
                }
            }
        }
    }
}

#[allow(clippy::too_many_arguments)]
async fn apply_verified_state(
    pool: &db::PgPool,
    client: &github::ReqwestGitHubClient,
    token: &str,
    repo: &db::GithubRepository,
    policy: &policy::VerificationPolicy,
    subject_type: &str,
    issue_number: u64,
    head_sha: Option<&str>,
    current_labels: Option<Vec<String>>,
) {
    let labels = match current_labels {
        Some(labels) => labels,
        None => client
            .issue_labels(token, &repo.owner, &repo.name, issue_number)
            .await
            .map(|labels| labels.into_iter().map(|label| label.name).collect())
            .unwrap_or_default(),
    };
    let remove = [policy.apply_label.as_ref(), policy.pending_label.as_ref()]
        .into_iter()
        .flatten()
        .collect::<std::collections::HashSet<_>>();
    let mut next_labels = labels
        .into_iter()
        .filter(|label| !remove.contains(label))
        .collect::<Vec<_>>();
    if let Some(label) = policy.verified_label.as_ref() {
        if !next_labels.iter().any(|existing| existing == label) {
            next_labels.push(label.clone());
        }
    }
    if let Err(err) = client
        .set_labels(token, &repo.owner, &repo.name, issue_number, &next_labels)
        .await
    {
        if matches!(err, github::GitHubError::ApiStatus(status) if status.as_u16() == 404) {
            ensure_policy_labels(client, token, repo, policy).await;
            let _ = client
                .set_labels(token, &repo.owner, &repo.name, issue_number, &next_labels)
                .await;
        }
    }
    if let Ok(Some(artifact)) = db::get_bot_artifact(
        pool,
        repo.repository_id,
        subject_type,
        &issue_number.to_string(),
        "comment",
    )
    .await
    {
        if let Some(id) = artifact.external_id.and_then(|x| x.parse().ok()) {
            let _ = client
                .delete_issue_comment(token, &repo.owner, &repo.name, id)
                .await;
        }
    }
    if subject_type == "pull_request" {
        if let Ok(Some(artifact)) = db::get_bot_artifact(
            pool,
            repo.repository_id,
            subject_type,
            &issue_number.to_string(),
            "check_run",
        )
        .await
        {
            if let Some(id) = artifact.external_id.and_then(|x| x.parse().ok()) {
                let sha = head_sha
                    .or_else(|| artifact.data.get("sha").and_then(|v| v.as_str()))
                    .unwrap_or("");
                if !sha.is_empty() {
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
                    let _ = client
                        .update_check_run(token, &repo.owner, &repo.name, id, &req)
                        .await;
                }
            }
        }
    }
}
