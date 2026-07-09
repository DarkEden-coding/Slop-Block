use std::collections::HashSet;
use std::time::Duration;

use github::{CheckRunRequest, GitHubApi};
use policy::VerificationPolicy;
use serde::Deserialize;
use serde_json::json;
use time::OffsetDateTime;
use uuid::Uuid;

use crate::github_helpers::{ensure_policy_labels, github_content_delay_seconds};
use crate::github_tokens::{github_client, installation_token};
use crate::AppState;

pub async fn load_repo_policy(pool: &db::PgPool, repository_id: i64) -> VerificationPolicy {
    match db::get_policy(pool, repository_id).await {
        Ok(Some(stored)) if stored.enabled => {
            serde_json::from_value(stored.policy).unwrap_or_default()
        }
        _ => VerificationPolicy::default(),
    }
}

pub async fn record_verification_trust(state: &AppState, s: &db::VerificationSession) {
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
}

pub async fn enqueue_propagation(state: &AppState, s: &db::VerificationSession) {
    let Some(pool) = state.db.as_ref() else {
        return;
    };
    let payload = json!({
        "session_public_id": s.public_id,
        "repository_id": s.repository_id,
    });
    let key = format!("propagation:session:{}", s.public_id);
    let _ = jobs::enqueue_deduped(
        pool,
        jobs::JobKind::PropagationPlan,
        payload,
        None,
        8,
        Some(&key),
        30,
    )
    .await;
}

#[derive(Deserialize)]
struct PropagationPlanPayload {
    session_public_id: Uuid,
    repository_id: i64,
}

#[derive(Deserialize)]
struct PropagationSubjectPayload {
    propagation_run_id: i64,
    repository_id: i64,
    subject_type: String,
    number: u64,
    head_sha: Option<String>,
    label_names: Option<Vec<String>>,
}

pub async fn handle_propagation_plan(
    state: &AppState,
    pool: &db::PgPool,
    payload: serde_json::Value,
) -> anyhow::Result<()> {
    let p: PropagationPlanPayload = serde_json::from_value(payload)?;
    let session = db::get_verification_session_by_public_id(pool, p.session_public_id)
        .await?
        .ok_or_else(|| anyhow::anyhow!("verification session not found"))?;

    let repo = db::get_repository(pool, p.repository_id)
        .await?
        .ok_or_else(|| anyhow::anyhow!("repository not found"))?;
    let policy: VerificationPolicy = match db::get_policy(pool, repo.repository_id).await? {
        Some(pol) if pol.enabled => serde_json::from_value(pol.policy).unwrap_or_default(),
        Some(_) => return Ok(()),
        None => VerificationPolicy::default(),
    };
    let _ = policy;
    let login = session
        .metadata
        .get("login")
        .and_then(|v| v.as_str())
        .unwrap_or(&session.subject_id);
    let run = db::create_propagation_run(
        pool,
        session.repository_id,
        session.github_user_id,
        Some(login),
        session.public_id,
    )
    .await?;

    let mut sessions = vec![session.clone()];
    if let Some(user_id) = session.github_user_id {
        if let Ok(done) =
            db::complete_pending_sessions_for_user(pool, session.repository_id, user_id).await
        {
            sessions.extend(done.into_iter().filter(|s| s.id != session.id));
        }
    }

    let mut handled = HashSet::new();
    let mut sequence = 0i32;
    for s in &sessions {
        let number = s.subject_id.parse().unwrap_or(0);
        let subject_type = s.subject_type.clone();
        handled.insert((subject_type.clone(), number));
        enqueue_propagation_subject(
            state,
            pool,
            run.id,
            repo.repository_id,
            &subject_type,
            number,
            None,
            None,
            sequence,
        )
        .await?;
        sequence += 1;
    }

    if let Some(user_id) = session.github_user_id {
        let _permit = state
            .installation_gate
            .acquire(repo.installation_id as u64)
            .await;
        let token = installation_token(state, repo.installation_id as u64).await?;
        let client = github_client(state);
        let open_items = client
            .list_open_issues_by_creator(&token, &repo.owner, &repo.name, login)
            .await?;
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
            handled.insert((subject_type.to_string(), issue.number));
            let labels = issue.labels.iter().map(|l| l.name.clone()).collect();
            enqueue_propagation_subject(
                state,
                pool,
                run.id,
                repo.repository_id,
                subject_type,
                issue.number,
                None,
                Some(labels),
                sequence,
            )
            .await?;
            sequence += 1;
        }
    }

    db::set_propagation_total(pool, run.id, sequence).await?;
    if sequence == 0 {
        db::complete_propagation_run(pool, run.id).await?;
    }
    Ok(())
}

#[allow(clippy::too_many_arguments)]
async fn enqueue_propagation_subject(
    state: &AppState,
    pool: &db::PgPool,
    run_id: i64,
    repository_id: i64,
    subject_type: &str,
    number: u64,
    head_sha: Option<&str>,
    label_names: Option<Vec<String>>,
    sequence: i32,
) -> anyhow::Result<()> {
    let payload = json!({
        "propagation_run_id": run_id,
        "repository_id": repository_id,
        "subject_type": subject_type,
        "number": number,
        "head_sha": head_sha,
        "label_names": label_names,
    });
    let key = format!("propagation:{run_id}:{subject_type}:{number}");
    let delay = github_content_delay_seconds(state) as i64;
    let run_at =
        OffsetDateTime::now_utc() + Duration::from_secs((i64::from(sequence) * delay) as u64);
    jobs::enqueue_deduped(
        pool,
        jobs::JobKind::PropagationSubject,
        payload,
        Some(run_at),
        8,
        Some(&key),
        35,
    )
    .await?;
    Ok(())
}

pub async fn handle_propagation_subject(
    state: &AppState,
    pool: &db::PgPool,
    payload: serde_json::Value,
) -> anyhow::Result<()> {
    let p: PropagationSubjectPayload = serde_json::from_value(payload)?;
    let repo = db::get_repository(pool, p.repository_id)
        .await?
        .ok_or_else(|| anyhow::anyhow!("repository not found"))?;
    let policy = load_repo_policy(pool, repo.repository_id).await;
    let _permit = state
        .installation_gate
        .acquire(repo.installation_id as u64)
        .await;
    let token = installation_token(state, repo.installation_id as u64).await?;
    let client = github_client(state);
    apply_verified_state(
        pool,
        &client,
        &token,
        &repo,
        &policy,
        &p.subject_type,
        p.number,
        p.head_sha.as_deref(),
        p.label_names,
    )
    .await;
    db::advance_propagation_progress(
        pool,
        p.propagation_run_id,
        &p.subject_type,
        &p.number.to_string(),
    )
    .await?;

    let remaining: (i64,) = sqlx::query_as(
        "SELECT COUNT(*) FROM jobs WHERE status IN ('queued','running') AND kind='propagation_subject' AND (payload->>'propagation_run_id')::bigint=$1",
    )
    .bind(p.propagation_run_id)
    .fetch_one(pool)
    .await?;
    // Current job is still running until the runner marks it complete; count of 1 means we are last.
    if remaining.0 <= 1 {
        db::complete_propagation_run(pool, p.propagation_run_id).await?;
    }
    Ok(())
}

pub async fn finalize(state: &AppState, s: &db::VerificationSession) {
    record_verification_trust(state, s).await;
    enqueue_propagation(state, s).await;
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
