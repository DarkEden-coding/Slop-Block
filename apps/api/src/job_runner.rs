use std::time::Duration;

use crate::{
    backfill_jobs::{handle_backfill_scan, handle_backfill_subject},
    github_subjects::{process_subject, SubjectWork},
    AppState,
};
use db::PgPool;
use jobs::{runner_loop, RetryPolicy};

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
        other => Err(anyhow::anyhow!("unsupported job kind: {other}")),
    }
}
