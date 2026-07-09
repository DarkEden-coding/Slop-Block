use github::{CheckRunRequest, GitHubApi};
use serde::{Deserialize, Serialize};
use serde_json::json;
use time::OffsetDateTime;

use crate::github_helpers::ensure_policy_labels;
use crate::github_tokens::installation_token;
use crate::web_util::sign_source_payload;
use crate::AppState;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SubjectWork {
    pub repository_id: i64,
    pub installation_id: Option<u64>,
    pub subject_type: String,
    pub number: u64,
    pub html_url: String,
    pub head_sha: Option<String>,
    pub github_user_id: i64,
    pub login: String,
    pub user_type: Option<String>,
    pub source: String,
    #[serde(default)]
    pub notify_author: bool,
    #[serde(default)]
    pub force_new_comment: bool,
}

#[derive(Debug, Clone)]
pub struct SubjectOutcome {
    pub required: bool,
    pub reason: String,
    pub skipped: bool,
}

pub async fn process_subject(
    state: &AppState,
    pool: &db::PgPool,
    work: SubjectWork,
) -> anyhow::Result<SubjectOutcome> {
    let repo = db::get_repository(pool, work.repository_id)
        .await?
        .ok_or_else(|| anyhow::anyhow!("repository not found"))?;
    let policy: policy::VerificationPolicy = match db::get_policy(pool, repo.repository_id).await? {
        Some(p) if p.enabled => serde_json::from_value(p.policy).unwrap_or_default(),
        _ => {
            return Ok(SubjectOutcome {
                required: false,
                reason: "policy_disabled".into(),
                skipped: true,
            })
        }
    };

    if work
        .installation_id
        .is_some_and(|id| id as i64 != repo.installation_id)
    {
        anyhow::bail!("installation mismatch");
    }
    let installation_id = work.installation_id.unwrap_or(repo.installation_id as u64);
    let _permit = state.installation_gate.acquire(installation_id).await;
    let client = crate::github_tokens::github_client(state);
    let bucket = format!("installation:{installation_id}:core");
    if let Some(paused) = db::github_pause_until(pool, &bucket).await? {
        anyhow::bail!("github_rate_limited_until:{}", paused.unix_timestamp());
    }
    let token = match installation_token(state, installation_id).await {
        Ok(token) => token,
        Err(err) => match err.downcast::<github::GitHubError>() {
            Ok(gh_err) => return Err(record_github_error(pool, &bucket, gh_err).await),
            Err(err) => return Err(err),
        },
    };

    let is_bot = matches!(work.user_type.as_deref(), Some("Bot"));
    let is_app = matches!(work.user_type.as_deref(), Some("App"));
    let trust = db::get_trusted_subject(
        pool,
        repo.repository_id,
        "github_user",
        &work.github_user_id.to_string(),
    )
    .await?;
    let is_collaborator = if !policy.exempt_collaborators || is_bot || is_app {
        false
    } else {
        match client
            .collaborator_permission(&token, &repo.owner, &repo.name, &work.login)
            .await
        {
            Ok(perm) => matches!(perm.permission.as_str(), "admin" | "maintain" | "write"),
            Err(_) => false,
        }
    };
    let target = if work.subject_type == "pull_request" {
        policy::TargetKind::PullRequest
    } else {
        policy::TargetKind::Issue
    };
    let input = policy::DecisionInput {
        target,
        subject: policy::Subject {
            login: work.login.clone(),
            github_user_id: Some(work.github_user_id),
            is_collaborator,
            is_bot,
            is_app,
        },
        trust: trust
            .as_ref()
            .map(|t| policy::TrustState {
                trusted: t.trusted,
                manually_exempt: t
                    .metadata
                    .get("source")
                    .and_then(|v| v.as_str())
                    .is_some_and(|source| source == "manual_allowlist"),
                trusted_at: Some(t.trusted_at.unix_timestamp()),
                expires_at: t.expires_at.map(|x| x.unix_timestamp()),
            })
            .unwrap_or_default(),
        now: OffsetDateTime::now_utc().unix_timestamp(),
    };
    let decision = policy::decide(&policy, &input);
    db::insert_audit(pool, Some(&work.source), "github.subject.decision", Some(repo.repository_id), Some(&work.subject_type), Some(&work.number.to_string()), json!({"reason": decision.reason, "required": decision.required, "allowed": decision.allowed})).await?;

    if !decision.required {
        if work.source == "backfill" {
            return Ok(SubjectOutcome {
                required: false,
                reason: format!("{:?}", decision.reason),
                skipped: true,
            });
        }
        for action in &decision.actions {
            match action {
                policy::PolicyAction::AddLabel(label) => {
                    let _ = client
                        .add_labels(
                            &token,
                            &repo.owner,
                            &repo.name,
                            work.number,
                            std::slice::from_ref(label),
                        )
                        .await;
                }
                policy::PolicyAction::RemoveLabel(label) => {
                    let _ = client
                        .remove_label(&token, &repo.owner, &repo.name, work.number, label)
                        .await;
                }
                _ => {}
            }
        }
        return Ok(SubjectOutcome {
            required: false,
            reason: format!("{:?}", decision.reason),
            skipped: false,
        });
    }

    if work.source == "backfill" && !work.force_new_comment {
        let has_comment = !policy.comment_on_required
            || db::get_bot_artifact(
                pool,
                repo.repository_id,
                &work.subject_type,
                &work.number.to_string(),
                "comment",
            )
            .await?
            .is_some();
        let has_check = target != policy::TargetKind::PullRequest
            || policy.check_mode == policy::CheckMode::Off
            || db::get_bot_artifact(
                pool,
                repo.repository_id,
                &work.subject_type,
                &work.number.to_string(),
                "check_run",
            )
            .await?
            .is_some();
        if has_comment && has_check {
            return Ok(SubjectOutcome {
                required: true,
                reason: format!("{:?}", decision.reason),
                skipped: true,
            });
        }
    }

    db::upsert_github_user(
        pool,
        work.github_user_id,
        &work.login,
        None,
        json!({"login": work.login, "type": work.user_type}),
    )
    .await?;
    let verify_url = source_verify_url(
        state,
        repo.repository_id,
        &work.subject_type,
        work.number,
        work.github_user_id,
        &work.login,
        &work.html_url,
    )?;
    let required_labels = [policy.apply_label.clone(), policy.pending_label.clone()]
        .into_iter()
        .flatten()
        .collect::<Vec<_>>();
    if !required_labels.is_empty() {
        if let Err(err) = client
            .add_labels(
                &token,
                &repo.owner,
                &repo.name,
                work.number,
                &required_labels,
            )
            .await
        {
            if matches!(err, github::GitHubError::ApiStatus(status) if status.as_u16() == 404) {
                ensure_policy_labels(&client, &token, &repo, &policy).await;
                if let Err(err) = client
                    .add_labels(
                        &token,
                        &repo.owner,
                        &repo.name,
                        work.number,
                        &required_labels,
                    )
                    .await
                {
                    return Err(record_github_error(pool, &bucket, err).await);
                }
            } else {
                return Err(record_github_error(pool, &bucket, err).await);
            }
        }
    }
    if policy.comment_on_required {
        let body = if work.notify_author {
            format!(
                "@{} Human verification is required. Verify here: {verify_url}",
                work.login
            )
        } else {
            format!("Human verification is required. Verify here: {verify_url}")
        };
        let artifact = db::get_bot_artifact(
            pool,
            repo.repository_id,
            &work.subject_type,
            &work.number.to_string(),
            "comment",
        )
        .await?;
        let comment = if !work.force_new_comment {
            if let Some(a) =
                artifact.and_then(|a| a.external_id.and_then(|id| id.parse::<u64>().ok()))
            {
                match client
                    .update_issue_comment(&token, &repo.owner, &repo.name, a, &body)
                    .await
                {
                    Ok(comment) => Ok(comment),
                    // Verification propagation can delete the old verification comment
                    // while a backfill item is running. Treat that race as stale state
                    // and create the comment again if this item still requires it.
                    Err(github::GitHubError::ApiStatus(status)) if status.as_u16() == 404 => {
                        client
                            .create_issue_comment(
                                &token,
                                &repo.owner,
                                &repo.name,
                                work.number,
                                &body,
                            )
                            .await
                    }
                    Err(err) => Err(err),
                }
            } else {
                client
                    .create_issue_comment(&token, &repo.owner, &repo.name, work.number, &body)
                    .await
            }
        } else {
            client
                .create_issue_comment(&token, &repo.owner, &repo.name, work.number, &body)
                .await
        };
        let c = match comment {
            Ok(c) => c,
            Err(err) => return Err(record_github_error(pool, &bucket, err).await),
        };
        db::upsert_bot_artifact(
            pool,
            repo.repository_id,
            &work.subject_type,
            &work.number.to_string(),
            "comment",
            Some(&c.id.to_string()),
            json!({"url": c.html_url, "source_url": verify_url}),
        )
        .await?;
    }
    if target == policy::TargetKind::PullRequest && policy.check_mode != policy::CheckMode::Off {
        if let Some(sha) = work.head_sha.as_ref() {
            let req = CheckRunRequest {
                name: "Human Auth".into(),
                head_sha: sha.clone(),
                status: Some("completed".into()),
                conclusion: Some(
                    if policy.check_mode == policy::CheckMode::Audit {
                        "neutral"
                    } else {
                        "action_required"
                    }
                    .into(),
                ),
                details_url: Some(verify_url.clone()),
                output: Some(
                    json!({"title":"Human verification required","summary":"Complete verification to proceed."}),
                ),
            };
            let artifact = db::get_bot_artifact(
                pool,
                repo.repository_id,
                &work.subject_type,
                &work.number.to_string(),
                "check_run",
            )
            .await?;
            let check = if let Some(a) =
                artifact.and_then(|a| a.external_id.and_then(|id| id.parse::<u64>().ok()))
            {
                match client
                    .update_check_run(&token, &repo.owner, &repo.name, a, &req)
                    .await
                {
                    Ok(check) => Ok(check),
                    // A concurrent verification can make tracked check-run state stale.
                    // If the old check run is gone, recreate the required check run.
                    Err(github::GitHubError::ApiStatus(status)) if status.as_u16() == 404 => {
                        client
                            .create_check_run(&token, &repo.owner, &repo.name, &req)
                            .await
                    }
                    Err(err) => Err(err),
                }
            } else {
                client
                    .create_check_run(&token, &repo.owner, &repo.name, &req)
                    .await
            };
            let c = match check {
                Ok(c) => c,
                Err(err) => return Err(record_github_error(pool, &bucket, err).await),
            };
            db::upsert_bot_artifact(
                pool,
                repo.repository_id,
                &work.subject_type,
                &work.number.to_string(),
                "check_run",
                Some(&c.id.to_string()),
                json!({"sha": c.head_sha, "source_url": verify_url}),
            )
            .await?;
        }
    }
    Ok(SubjectOutcome {
        required: true,
        reason: format!("{:?}", decision.reason),
        skipped: false,
    })
}

async fn record_github_error(
    pool: &db::PgPool,
    bucket: &str,
    err: github::GitHubError,
) -> anyhow::Error {
    let now = OffsetDateTime::now_utc();
    let paused_until = match &err {
        github::GitHubError::RateLimited {
            reset_at,
            retry_after_seconds,
            ..
        } => reset_at
            .or_else(|| retry_after_seconds.map(|s| now + time::Duration::seconds(s as i64))),
        github::GitHubError::SecondaryRateLimited {
            retry_after_seconds,
            ..
        } => Some(now + time::Duration::seconds(retry_after_seconds.unwrap_or(60) as i64)),
        _ => None,
    };
    if paused_until.is_some() {
        let _ = db::upsert_github_rate_limit(
            pool,
            bucket,
            None,
            paused_until,
            paused_until,
            None,
            Some(&err.to_string()),
        )
        .await;
    }
    if let Some(paused) = paused_until {
        anyhow::anyhow!("github_rate_limited_until:{}", paused.unix_timestamp())
    } else {
        anyhow::anyhow!(err)
    }
}

fn source_verify_url(
    state: &AppState,
    repository_id: i64,
    subject_type: &str,
    number: u64,
    github_user_id: i64,
    login: &str,
    subject_url: &str,
) -> anyhow::Result<String> {
    let secret = state
        .config
        .admin_session_secret
        .as_ref()
        .or(state.config.github_webhook_secret.as_ref())
        .ok_or_else(|| anyhow::anyhow!("missing signing secret"))?;
    let sig = sign_source_payload(
        secret,
        repository_id,
        subject_type,
        number,
        github_user_id,
        login,
        subject_url,
    )
    .ok_or_else(|| anyhow::anyhow!("missing signing secret"))?;
    Ok(format!(
        "{}/verify/source?repo={repository_id}&type={}&number={number}&user_id={github_user_id}&login={}&url={}&sig={sig}",
        state.config.web_base_url.trim_end_matches('/'),
        urlencoding::encode(subject_type),
        urlencoding::encode(login),
        urlencoding::encode(subject_url)
    ))
}
