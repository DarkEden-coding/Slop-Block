use github::GitHubApi;
use serde_json::json;

pub fn github_content_delay_seconds() -> u64 {
    std::env::var("BACKFILL_SUBJECT_DELAY_SECONDS")
        .ok()
        .and_then(|v| v.parse::<u64>().ok())
        .unwrap_or(22)
        .max(1)
}

pub async fn sync_user_installations(
    pool: &db::PgPool,
    installations: &[github::Installation],
    github_user_id: i64,
    login: &str,
    source: &str,
) -> Result<(), sqlx::Error> {
    for installation in installations {
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
        let _ = client
            .create_label(token, &repo.owner, &repo.name, label, color, description)
            .await;
    }
}
