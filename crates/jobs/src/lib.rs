use db::{Job, PgPool};
use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::{future::Future, time::Duration};
use time::OffsetDateTime;

pub type Result<T> = std::result::Result<T, sqlx::Error>;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct JobId(pub i64);

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum JobStatus {
    Queued,
    Running,
    Completed,
    Failed,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum JobKind {
    GitHubArtifactUpdate,
    AutoClose,
}

impl JobKind {
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::GitHubArtifactUpdate => "github_artifact_update",
            Self::AutoClose => "auto_close",
        }
    }
}

impl std::fmt::Display for JobKind {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(self.as_str())
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct RetryPolicy {
    pub initial_delay: Duration,
    pub max_delay: Duration,
    pub multiplier: u32,
}

impl Default for RetryPolicy {
    fn default() -> Self {
        Self {
            initial_delay: Duration::from_secs(30),
            max_delay: Duration::from_secs(30 * 60),
            multiplier: 2,
        }
    }
}

impl RetryPolicy {
    pub fn backoff_delay(&self, attempts: i32) -> Duration {
        let retry_index = attempts.max(1) as u32 - 1;
        let factor = self.multiplier.saturating_pow(retry_index).max(1);
        self.initial_delay
            .saturating_mul(factor)
            .min(self.max_delay)
    }
}

pub async fn enqueue(
    pool: &PgPool,
    kind: JobKind,
    payload: Value,
    run_at: Option<OffsetDateTime>,
    max_attempts: i32,
) -> Result<Job> {
    db::enqueue_job(pool, kind.as_str(), payload, run_at, max_attempts).await
}

pub async fn claim(pool: &PgPool, worker: &str) -> Result<Option<Job>> {
    db::claim_job(pool, worker).await
}

pub async fn complete(pool: &PgPool, id: i64) -> Result<Option<Job>> {
    db::complete_job(pool, id).await
}

pub async fn fail(pool: &PgPool, id: i64, error: &str, policy: RetryPolicy) -> Result<Option<Job>> {
    let job = sqlx::query_as::<_, Job>("SELECT * FROM jobs WHERE id=$1")
        .bind(id)
        .fetch_one(pool)
        .await?;
    let retry_at = if job.attempts >= job.max_attempts {
        None
    } else {
        Some(OffsetDateTime::now_utc() + policy.backoff_delay(job.attempts))
    };
    db::fail_job(pool, id, error, retry_at).await
}

pub async fn runner_loop<F, Fut, C>(
    pool: PgPool,
    worker: String,
    poll_interval: Duration,
    retry_policy: RetryPolicy,
    mut handle: F,
    shutdown: C,
) where
    F: FnMut(Job) -> Fut,
    Fut: Future<Output = anyhow::Result<()>>,
    C: Future<Output = ()>,
{
    tokio::pin!(shutdown);

    loop {
        tokio::select! {
            _ = &mut shutdown => break,
            claimed = claim(&pool, &worker) => {
                match claimed {
                    Ok(Some(job)) => {
                        let id = job.id;
                        match handle(job).await {
                            Ok(()) => { let _ = complete(&pool, id).await; }
                            Err(err) => { let _ = fail(&pool, id, &err.to_string(), retry_policy).await; }
                        }
                    }
                    Ok(None) => {
                        tokio::select! {
                            _ = &mut shutdown => break,
                            _ = tokio::time::sleep(poll_interval) => {}
                        }
                    }
                    Err(_) => {
                        tokio::select! {
                            _ = &mut shutdown => break,
                            _ = tokio::time::sleep(poll_interval) => {}
                        }
                    }
                }
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn backoff_is_exponential_and_capped() {
        let policy = RetryPolicy {
            initial_delay: Duration::from_secs(10),
            max_delay: Duration::from_secs(60),
            multiplier: 2,
        };
        assert_eq!(policy.backoff_delay(1), Duration::from_secs(10));
        assert_eq!(policy.backoff_delay(2), Duration::from_secs(20));
        assert_eq!(policy.backoff_delay(3), Duration::from_secs(40));
        assert_eq!(policy.backoff_delay(4), Duration::from_secs(60));
        assert_eq!(policy.backoff_delay(99), Duration::from_secs(60));
    }
}
