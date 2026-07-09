use std::{
    collections::HashMap,
    sync::Arc,
    time::{Duration, Instant},
};
use tokio::sync::Mutex;

/// Spaces GitHub content mutations for each installation.
pub struct GitHubContentGate {
    inner: Mutex<HashMap<u64, Instant>>,
    interval: Duration,
}

impl GitHubContentGate {
    pub fn new(per_minute: u32, per_hour: u32) -> Self {
        let minute = Duration::from_secs_f64(60.0 / per_minute.max(1) as f64);
        let hour = Duration::from_secs_f64(3600.0 / per_hour.max(1) as f64);
        Self {
            inner: Mutex::new(HashMap::new()),
            interval: minute.max(hour),
        }
    }

    pub async fn acquire(&self, installation_id: u64) {
        let wait = {
            let mut slots = self.inner.lock().await;
            let now = Instant::now();
            let slot = slots.entry(installation_id).or_insert(now);
            let reserved = (*slot).max(now);
            *slot = reserved + self.interval;
            reserved.saturating_duration_since(now)
        };
        if !wait.is_zero() {
            tokio::time::sleep(wait).await;
        }
    }
}

pub type SharedGitHubContentGate = Arc<GitHubContentGate>;
