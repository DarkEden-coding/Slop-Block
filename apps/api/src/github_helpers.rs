use github::GitHubApi;
use serde_json::json;

pub fn github_content_delay_seconds(state: &crate::AppState) -> u64 {
    state.config.backfill_subject_delay_seconds.max(1)
}

pub async fn sync_user_installations(
    pool: &db::PgPool,
    installations: &[github::Installation],
    github_user_id: i64,
    login: &str,
    source: &str,
) -> Result<(), sqlx::Error> {
    sync_user_installations_filtered(pool, installations, github_user_id, login, source, None).await
}

pub async fn sync_user_installations_filtered(
    pool: &db::PgPool,
    installations: &[github::Installation],
    github_user_id: i64,
    login: &str,
    source: &str,
    only_installation_id: Option<i64>,
) -> Result<(), sqlx::Error> {
    for installation in installations {
        if only_installation_id.is_some_and(|id| installation.id as i64 != id) {
            continue;
        }
        let account = &installation.account;
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
            json!({"source": source}),
        )
        .await?;
        db::upsert_installation_admin(pool, installation.id as i64, github_user_id, login).await?;
    }
    Ok(())
}

pub async fn ensure_policy_labels(
    content_gate: &crate::github_content_gate::GitHubContentGate,
    installation_id: u64,
    client: &github::ReqwestGitHubClient,
    token: &str,
    repo: &db::GithubRepository,
    policy: &policy::VerificationPolicy,
) {
    let labels = [
        policy.apply_label.as_ref(),
        policy.pending_label.as_ref(),
        policy.verified_label.as_ref(),
    ]
    .into_iter()
    .flatten();
    for label in labels {
        let (color, description) = match Some(label.as_str()) {
            _ if policy.apply_label.as_ref() == Some(label) => {
                ("d73a4a", Some("Human verification is required"))
            }
            _ if policy.pending_label.as_ref() == Some(label) => {
                ("fbca04", Some("Human verification is pending"))
            }
            _ => ("0e8a16", Some("Human verification is complete")),
        };
        content_gate.acquire(installation_id).await;
        let _ = client
            .create_label(token, &repo.owner, &repo.name, label, color, description)
            .await;
    }
}
