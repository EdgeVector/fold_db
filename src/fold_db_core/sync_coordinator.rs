//! SyncCoordinator - manages the optional cloud sync engine lifecycle.
//!
//! Sync is opt-in. In local mode, the coordinator holds no engine and all
//! operations are no-ops (or return None). This type encapsulates the
//! interior mutability (RwLock + Mutex) needed so FoldDB can expose sync
//! operations via `&self`.

use std::sync::{Mutex, RwLock};

use log::{debug, warn};
use tokio::task::JoinHandle;

use std::sync::Arc;

use crate::db_operations::DbOperations;
use crate::storage::SledPool;
use crate::sync::{SyncEngine, SyncError, SyncState, SyncStatus};

/// Coordinates the optional cloud sync engine lifecycle.
pub struct SyncCoordinator {
    engine: RwLock<Option<Arc<SyncEngine>>>,
    task: Mutex<Option<JoinHandle<()>>>,
}

impl SyncCoordinator {
    pub fn new() -> Self {
        Self {
            engine: RwLock::new(None),
            task: Mutex::new(None),
        }
    }

    /// Store the sync engine. Caller is responsible for registering any
    /// reloader callbacks on the engine before calling this.
    pub fn set_engine(&self, engine: Arc<SyncEngine>) {
        *self.engine.write().unwrap() = Some(engine);
    }

    /// Returns a clone of the sync engine Arc, if configured.
    pub fn engine(&self) -> Option<Arc<SyncEngine>> {
        self.engine.read().unwrap().clone()
    }

    /// Returns true if a sync engine is configured.
    pub fn is_enabled(&self) -> bool {
        self.engine.read().unwrap().is_some()
    }

    /// Spawn the background sync timer task. No-op if no engine is configured.
    ///
    /// `db_ops` and `sled_pool` are needed to purge org data when the engine
    /// reports that the local node has been removed from an organization.
    pub fn start_background_sync(
        &self,
        interval_ms: u64,
        db_ops: Arc<DbOperations>,
        sled_pool: Option<Arc<SledPool>>,
    ) {
        let engine = match &*self.engine.read().unwrap() {
            Some(e) => Arc::clone(e),
            None => return,
        };

        let handle = tokio::spawn(async move {
            let interval = tokio::time::Duration::from_millis(interval_ms);
            loop {
                tokio::time::sleep(interval).await;
                // Always run sync — even without pending writes, we need to
                // download org data from other members.
                let has_pending = engine.state().await == SyncState::Dirty;
                let has_orgs = engine.has_org_sync().await;
                if has_pending || has_orgs {
                    if let Err(e) = engine.sync().await {
                        if let SyncError::OrgMembershipRevoked(ref org_hash) = e {
                            warn!("🚨 SYSTEM ALERT: You have been removed from organization (hash: {}) by an administrator. Proceeding to securely purge all locally cached copies of its data and schema to prevent orphans.", org_hash);

                            // 1. Delete membership structure locally (if running on Sled backend)
                            if let Some(pool) = &sled_pool {
                                let _ = crate::org::operations::delete_org(pool, org_hash).map_err(
                                    |err| log::error!("Failed to delete org structure: {}", err),
                                );
                            }

                            // 2. Erase the orphaned physical footprints in local DB
                            let _ = db_ops
                                .purge_org_data(org_hash)
                                .await
                                .map_err(|err| log::error!("Failed to purge org data: {}", err));
                        } else {
                            warn!("sync cycle failed: {e}");
                        }
                    }
                }
            }
        });

        *self.task.lock().unwrap() = Some(handle);
    }

    /// Force an immediate sync. No-op if no engine is configured.
    pub async fn force_sync(&self) -> Result<(), SyncError> {
        let engine = self.engine.read().unwrap().clone();
        if let Some(engine) = engine {
            engine.sync().await?;
        }
        Ok(())
    }

    /// Stop the background sync task and run a final sync.
    pub async fn stop(&self) -> Result<(), SyncError> {
        if let Some(handle) = self.task.lock().unwrap().take() {
            handle.abort();
        }
        self.force_sync().await
    }

    /// Get the sync engine state, if configured.
    pub async fn state(&self) -> Option<SyncState> {
        let engine = self.engine.read().unwrap().clone();
        match engine {
            Some(engine) => Some(engine.state().await),
            None => None,
        }
    }

    /// Get a full sync status snapshot, if configured.
    pub async fn status(&self) -> Option<SyncStatus> {
        let engine = self.engine.read().unwrap().clone();
        match engine {
            Some(engine) => Some(engine.status().await),
            None => None,
        }
    }

    /// Get the number of pending (unsynced) log entries, if configured.
    pub async fn pending_count(&self) -> Option<usize> {
        let engine = self.engine.read().unwrap().clone();
        match engine {
            Some(engine) => Some(engine.pending_count().await),
            None => None,
        }
    }

    /// Abort the background task without running a final sync.
    /// Called from Drop to avoid tokio panics.
    pub(crate) fn abort_task(&self) {
        if let Some(handle) = self.task.lock().unwrap().take() {
            debug!("SyncCoordinator: aborting background sync task on drop");
            handle.abort();
        }
    }
}

impl Default for SyncCoordinator {
    fn default() -> Self {
        Self::new()
    }
}
