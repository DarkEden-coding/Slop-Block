use std::time::Duration;

use db::PgPool;
use jobs::{runner_loop, JobKind, RetryPolicy};

pub fn spawn_job_runner(pool: PgPool) -> tokio::task::JoinHandle<()> {
    tokio::spawn(async move {
        let worker = format!("api-{}", uuid::Uuid::new_v4());
        runner_loop(
            pool,
            worker,
            Duration::from_secs(5),
            RetryPolicy::default(),
            handle_job,
            std::future::pending::<()>(),
        )
        .await;
    })
}

async fn handle_job(job: db::Job) -> anyhow::Result<()> {
    match job.kind.as_str() {
        "github_artifact_update" => {
            tracing::info!(job_id = job.id, "github artifact job acknowledged");
            Ok(())
        }
        "auto_close" => {
            tracing::info!(job_id = job.id, "auto-close job acknowledged");
            Ok(())
        }
        other => Err(anyhow::anyhow!("unsupported job kind: {other}")),
    }
}

#[allow(dead_code)]
fn _known_kind_names() -> [&'static str; 2] {
    [
        JobKind::GitHubArtifactUpdate.as_str(),
        JobKind::AutoClose.as_str(),
    ]
}
