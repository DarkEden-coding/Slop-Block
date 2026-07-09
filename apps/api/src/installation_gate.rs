use std::{collections::HashMap, sync::Arc};

use tokio::sync::{Mutex, OwnedSemaphorePermit, Semaphore};

/// Limits concurrent GitHub-mutating work per installation across job workers.
#[derive(Default)]
pub struct InstallationGate {
    inner: Mutex<HashMap<u64, Arc<Semaphore>>>,
    max_per_installation: usize,
}

impl InstallationGate {
    pub fn new(max_per_installation: usize) -> Self {
        Self {
            inner: Mutex::new(HashMap::new()),
            max_per_installation: max_per_installation.max(1),
        }
    }

    pub async fn acquire(&self, installation_id: u64) -> OwnedSemaphorePermit {
        let semaphore = {
            let mut map = self.inner.lock().await;
            map.entry(installation_id)
                .or_insert_with(|| Arc::new(Semaphore::new(self.max_per_installation)))
                .clone()
        };
        semaphore
            .acquire_owned()
            .await
            .expect("installation semaphore closed")
    }
}

pub type SharedInstallationGate = Arc<InstallationGate>;
