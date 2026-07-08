use std::time::Duration;
use time::OffsetDateTime;

use crate::{
    github_subjects::{process_subject, SubjectWork},
    AppState,
};
use db::PgPool;
use github::GitHubApi;
use jobs::{runner_loop, JobKind, RetryPolicy};
use serde::Deserialize;
use serde_json::json;

pub fn spawn_job_runner(state: AppState, pool: PgPool) -> Vec<tokio::task::JoinHandle<()>> {
    let workers = std::env::var("JOB_WORKERS")
        .ok()
        .and_then(|v| v.parse::<usize>().ok())
        .unwrap_or(4)
        .max(1);
    (0..workers)
        .map(|_| {
            let pool = pool.clone();
            let state = state.clone();
            tokio::spawn(async move {
                let worker = format!("api-{}", uuid::Uuid::new_v4());
                runner_loop(
                    pool,
                    worker,
                    Duration::from_millis(
                        std::env::var("JOB_POLL_INTERVAL_MS")
                            .ok()
                            .and_then(|v| v.parse().ok())
                            .unwrap_or(1000),
                    ),
                    RetryPolicy::default(),
                    move |job| {
                        let state = state.clone();
                        async move { handle_job(state, job).await }
                    },
                    std::future::pending::<()>(),
                )
                .await;
            })
        })
        .collect()
}

async fn handle_job(state: AppState, job: db::Job) -> anyhow::Result<()> {
    let pool = state
        .db
        .as_ref()
        .ok_or_else(|| anyhow::anyhow!("db unavailable"))?;
    match job.kind.as_str() {
        "github_subject_event" => {
            let work: SubjectWork = serde_json::from_value(job.payload)?;
            process_subject(&state, pool, work).await?;
            Ok(())
        }
        "backfill_scan" => handle_backfill_scan(&state, pool, job.payload).await,
        "backfill_subject" => handle_backfill_subject(&state, pool, job.payload).await,
        "github_artifact_update" | "auto_close" => Ok(()),
        other => Err(anyhow::anyhow!("unsupported job kind: {other}")),
    }
}

#[derive(Deserialize)]
struct BackfillScanPayload {
    backfill_run_id: i64,
    repository_id: i64,
}
#[derive(Deserialize)]
struct BackfillSubjectPayload {
    backfill_run_id: i64,
    backfill_item_id: i64,
}

async fn installation_token(state: &AppState, installation_id: u64) -> anyhow::Result<String> {
    let app_id = state
        .config
        .github_app_id
        .as_ref()
        .ok_or_else(|| anyhow::anyhow!("github app id missing"))?;
    let pk = state
        .config
        .github_private_key
        .as_ref()
        .ok_or_else(|| anyhow::anyhow!("github private key missing"))?;
    let jwt = github::create_app_jwt(app_id, pk)?;
    let gh = github::ReqwestGitHubClient::with_base_url(&state.config.github_api_base);
    Ok(gh
        .exchange_installation_token(&jwt, installation_id)
        .await?
        .token)
}

async fn handle_backfill_scan(
    state: &AppState,
    pool: &PgPool,
    payload: serde_json::Value,
) -> anyhow::Result<()> {
    let p: BackfillScanPayload = serde_json::from_value(payload)?;
    let run = db::get_backfill_run(pool, p.backfill_run_id)
        .await?
        .ok_or_else(|| anyhow::anyhow!("backfill run not found"))?;
    if run.status == "cancelled" {
        return Ok(());
    }
    let repo = db::get_repository(pool, p.repository_id)
        .await?
        .ok_or_else(|| anyhow::anyhow!("repository not found"))?;
    let token = installation_token(state, repo.installation_id as u64).await?;
    let gh = github::ReqwestGitHubClient::with_base_url(&state.config.github_api_base);
    db::mark_backfill_phase(pool, run.id, "scanning", Some("scanning_issues")).await?;
    let mut enqueued = 0;
    let mut discovered = 0;
    if run.include_issues {
        for issue in gh.list_open_issues(&token, &repo.owner, &repo.name).await? {
            if issue.pull_request.is_some() {
                continue;
            }
            discovered += 1;
            let url = format!(
                "https://github.com/{}/issues/{}",
                repo.full_name, issue.number
            );
            if let Some(item) = db::insert_backfill_item(
                pool,
                run.id,
                repo.repository_id,
                "issue",
                &issue.number.to_string(),
                Some(issue.user.id as i64),
                Some(&issue.user.login),
                Some(&url),
                None,
            )
            .await?
            {
                enqueue_item(pool, run.id, item.id, enqueued).await?;
                enqueued += 1;
            }
        }
    }
    db::mark_backfill_phase(pool, run.id, "scanning", Some("scanning_pull_requests")).await?;
    if run.include_pull_requests {
        for pr in gh
            .list_open_pull_requests(&token, &repo.owner, &repo.name)
            .await?
        {
            discovered += 1;
            let url = format!("https://github.com/{}/pull/{}", repo.full_name, pr.number);
            if let Some(item) = db::insert_backfill_item(
                pool,
                run.id,
                repo.repository_id,
                "pull_request",
                &pr.number.to_string(),
                Some(pr.user.id as i64),
                Some(&pr.user.login),
                Some(&url),
                Some(&pr.head.sha),
            )
            .await?
            {
                enqueue_item(pool, run.id, item.id, enqueued).await?;
                enqueued += 1;
            }
        }
    }
    db::increment_backfill_discovered(pool, run.id, discovered, enqueued).await?;
    let status = if enqueued == 0 {
        "completed"
    } else {
        "running"
    };
    db::mark_backfill_phase(
        pool,
        run.id,
        status,
        Some(if enqueued == 0 {
            "finalizing"
        } else {
            "processing"
        }),
    )
    .await?;
    Ok(())
}

async fn enqueue_item(
    pool: &PgPool,
    run_id: i64,
    item_id: i64,
    sequence: i32,
) -> anyhow::Result<()> {
    let payload = json!({"backfill_run_id": run_id, "backfill_item_id": item_id});
    let key = format!("backfill:{run_id}:item:{item_id}");
    // GitHub documents a general secondary content-creation limit of
    // 500 content-generating requests/hour. A required PR backfill can create
    // up to 4 content requests (2 label adds + 1 comment + 1 check run), so
    // schedule at floor(3600 / (500 / 4)) = 28.8s, rounded up to 29s.
    let delay_seconds = std::env::var("BACKFILL_SUBJECT_DELAY_SECONDS")
        .ok()
        .and_then(|v| v.parse::<i64>().ok())
        .unwrap_or(29)
        .max(1);
    let run_at = OffsetDateTime::now_utc()
        + Duration::from_secs((i64::from(sequence) * delay_seconds) as u64);
    jobs::enqueue_deduped(
        pool,
        JobKind::BackfillSubject,
        payload,
        Some(run_at),
        8,
        Some(&key),
        50,
    )
    .await?;
    Ok(())
}

async fn handle_backfill_subject(
    state: &AppState,
    pool: &PgPool,
    payload: serde_json::Value,
) -> anyhow::Result<()> {
    let p: BackfillSubjectPayload = serde_json::from_value(payload)?;
    let run = db::get_backfill_run(pool, p.backfill_run_id)
        .await?
        .ok_or_else(|| anyhow::anyhow!("backfill run not found"))?;
    if run.status == "cancelled" {
        db::finish_backfill_item(
            pool,
            run.id,
            p.backfill_item_id,
            "skipped",
            None,
            Some("cancelled"),
            None,
        )
        .await?;
        return Ok(());
    }
    let item = db::mark_backfill_item_running(pool, p.backfill_item_id)
        .await?
        .or(db::get_backfill_item(pool, p.backfill_item_id).await?)
        .ok_or_else(|| anyhow::anyhow!("backfill item not found"))?;
    let number = item.subject_id.parse::<u64>()?;
    let work = SubjectWork {
        repository_id: item.repository_id,
        installation_id: None,
        subject_type: item.subject_type.clone(),
        number,
        html_url: item.html_url.clone().unwrap_or_default(),
        head_sha: item.head_sha.clone(),
        github_user_id: item.github_user_id.unwrap_or_default(),
        login: item.login.clone().unwrap_or_else(|| "unknown".into()),
        user_type: Some("User".into()),
        source: "backfill".into(),
        notify_author: run.notify_authors,
        force_new_comment: run.force_new_comments,
    };
    match process_subject(state, pool, work).await {
        Ok(outcome) => {
            let status = if outcome.skipped || !outcome.required {
                "skipped"
            } else {
                "succeeded"
            };
            db::finish_backfill_item(
                pool,
                run.id,
                item.id,
                status,
                Some(outcome.required),
                Some(&outcome.reason),
                None,
            )
            .await?;
            Ok(())
        }
        Err(err) => {
            let error = err.to_string();
            if error.contains("secondary rate limited")
                || error.contains("temporarily blocked from content creation")
            {
                // Do not permanently fail the backfill item when GitHub asks us to slow down.
                // Let the jobs runner retry with exponential backoff instead.
                return Err(err);
            }
            db::finish_backfill_item(pool, run.id, item.id, "failed", None, None, Some(&error))
                .await?;
            Ok(())
        }
    }
}

#[allow(dead_code)]
fn _known_kind_names() -> [&'static str; 5] {
    [
        JobKind::GitHubSubjectEvent.as_str(),
        JobKind::BackfillScan.as_str(),
        JobKind::BackfillSubject.as_str(),
        JobKind::GitHubArtifactUpdate.as_str(),
        JobKind::AutoClose.as_str(),
    ]
}
