use std::time::Duration;

use crate::{
    backfill_jobs::{handle_backfill_scan, handle_backfill_subject},
    github_subjects::{process_subject, SubjectWork},
    github_tokens::{github_client, installation_token},
    verification_finalize::{handle_propagation_plan, handle_propagation_subject},
    webhooks::handle_webhook_dispatch,
    AppState,
};
use db::PgPool;
use github::GitHubApi;
use jobs::{runner_loop, RetryPolicy};
use serde::Deserialize;
use serde_json::json;
use time::OffsetDateTime;

pub fn spawn_job_runner(state: AppState, pool: PgPool) -> Vec<tokio::task::JoinHandle<()>> {
    let workers = state.config.job_workers.max(1);
    let poll_ms = state.config.job_poll_interval_ms;
    enqueue_retention_bootstrap(&pool, state.config.retention_days);
    (0..workers)
        .map(|_| {
            let pool = pool.clone();
            let state = state.clone();
            tokio::spawn(async move {
                let worker = format!("api-{}", uuid::Uuid::new_v4());
                runner_loop(
                    pool,
                    worker,
                    Duration::from_millis(poll_ms),
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

fn enqueue_retention_bootstrap(pool: &PgPool, retention_days: i64) {
    let pool = pool.clone();
    tokio::spawn(async move {
        let payload = json!({ "retention_days": retention_days });
        let run_at = OffsetDateTime::now_utc() + Duration::from_secs(60);
        let _ = jobs::enqueue_deduped(
            &pool,
            jobs::JobKind::RetentionCleanup,
            payload,
            Some(run_at),
            3,
            Some("retention:cleanup"),
            200,
        )
        .await;
    });
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
        "github_webhook_dispatch" => handle_webhook_dispatch(&state, pool, job.payload).await,
        "sync_installation" => handle_sync_installation(&state, pool, job.payload).await,
        "propagation_plan" => handle_propagation_plan(&state, pool, job.payload).await,
        "propagation_subject" => handle_propagation_subject(&state, pool, job.payload).await,
        "retention_cleanup" => handle_retention_cleanup(&state, pool, job.payload).await,
        other => Err(anyhow::anyhow!("unsupported job kind: {other}")),
    }
}

#[derive(Deserialize)]
struct SyncInstallationPayload {
    installation_id: i64,
}

pub async fn handle_sync_installation(
    state: &AppState,
    pool: &PgPool,
    payload: serde_json::Value,
) -> anyhow::Result<()> {
    let p: SyncInstallationPayload = serde_json::from_value(payload)?;
    let _permit = state
        .installation_gate
        .acquire(p.installation_id as u64)
        .await;
    let token = installation_token(state, p.installation_id as u64).await?;
    let client = github_client(state);
    let repos = client.installation_repositories(&token).await?;
    let mut ids = Vec::with_capacity(repos.len());
    for repo in repos {
        ids.push(repo.id as i64);
        let owner = repo.owner.login;
        let name = repo.name;
        db::upsert_repository(
            pool,
            repo.id as i64,
            p.installation_id,
            &owner,
            &name,
            &repo.full_name,
            repo.private,
            repo.default_branch.as_deref(),
            json!({}),
        )
        .await?;
    }
    db::deactivate_repositories_not_in(pool, p.installation_id, &ids).await?;
    Ok(())
}

#[derive(Deserialize)]
struct RetentionPayload {
    #[serde(default)]
    retention_days: Option<i64>,
}

async fn handle_retention_cleanup(
    state: &AppState,
    pool: &PgPool,
    payload: serde_json::Value,
) -> anyhow::Result<()> {
    let p: RetentionPayload = serde_json::from_value(payload).unwrap_or(RetentionPayload {
        retention_days: None,
    });
    let days = p
        .retention_days
        .unwrap_or(state.config.retention_days)
        .max(1);
    let _ = db::purge_completed_jobs(pool, days).await?;
    let _ = db::purge_processed_webhook_events(pool, days).await?;
    let _ = db::purge_audit_log(pool, days).await?;

    let next = OffsetDateTime::now_utc() + Duration::from_secs(24 * 60 * 60);
    // Unique dedupe per scheduled run so we do not conflict with this still-running job.
    let key = format!("retention:cleanup:{}", next.unix_timestamp());
    jobs::enqueue_deduped(
        pool,
        jobs::JobKind::RetentionCleanup,
        json!({ "retention_days": days }),
        Some(next),
        3,
        Some(&key),
        200,
    )
    .await?;
    Ok(())
}
