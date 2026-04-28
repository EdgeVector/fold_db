//! SyncCoordinator - manages the optional cloud sync engine lifecycle.
//!
//! Sync is opt-in. In local mode, the coordinator holds no engine and all
//! operations are no-ops (or return None). This type encapsulates the
//! interior mutability (RwLock + Mutex) needed so FoldDB can expose sync
//! operations via `&self`.

use std::sync::{Mutex, RwLock};

use tokio::task::JoinHandle;
use tracing::{debug, warn};

use std::sync::Arc;

use crate::db_operations::DbOperations;
use crate::storage::SledPool;
use crate::sync::{SyncEngine, SyncError, SyncState, SyncStatus};

/// Cap on the exponential backoff between sync cycles while the engine is in
/// [`SyncState::Offline`]. Ten minutes is a balance between responsiveness
/// (user opens the lid, expects sync within a reasonable time) and not
/// hammering Exemem / the device battery during a sustained outage.
const MAX_OFFLINE_BACKOFF: tokio::time::Duration = tokio::time::Duration::from_secs(600);

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
    ///
    /// ### Offline backoff
    ///
    /// While the engine state is [`SyncState::Offline`] (i.e. the last sync
    /// failed with a network-class error), the inter-cycle delay doubles on
    /// each consecutive failure — `interval_ms`, 2×, 4×, … — capped at
    /// [`MAX_OFFLINE_BACKOFF`] (10 minutes). On the next successful cycle the
    /// delay resets to `interval_ms`. This matters for laptops resuming from
    /// sleep and phones with flaky connectivity: without the backoff, the
    /// coordinator would hammer the presign endpoint every `interval_ms` for
    /// the entire offline window (CPU and battery cost, plus noisy retries on
    /// cold Lambda).
    ///
    /// Backoff only engages for `SyncState::Offline`. Auth failures have their
    /// own refresh path (the `AuthRefreshCallback` on the engine) and permanent
    /// errors aren't retried here.
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

        let wake = engine.wake_handle();

        // lint:spawn-bare-ok boot-time sync poll loop — perpetual worker, no per-request parent span.
        let handle = tokio::spawn(async move {
            let base_interval = tokio::time::Duration::from_millis(interval_ms);
            let max_delay = MAX_OFFLINE_BACKOFF;
            let mut current_delay = base_interval;
            loop {
                // Sleep up to `current_delay`, or wake early if a local write
                // arrived. A write fires `engine.wake.notify_one()`, which
                // resolves the `notified()` future and aborts the timeout so
                // the flush happens near-immediately instead of waiting the
                // full polling interval. `timeout` returns `Err` on the timer
                // path and `Ok(())` on the wake path — either way, the same
                // check-and-sync logic below fires.
                let _ = tokio::time::timeout(current_delay, wake.notified()).await;
                // Always call sync() when a sync engine is configured. A
                // passive reader on a personal prefix (another device
                // restored from the same mnemonic) needs the poll to see
                // peer writes, even when locally clean and without org
                // memberships. Previous "skip if nothing to upload and no
                // orgs" check broke multi-device convergence — it matched
                // an equivalent bailout inside `sync()` that #607 removed,
                // but that was only half the fix. `sync()` is cheap on a
                // no-op cycle (one list request per target with an
                // already-advanced cursor → typically 0 matches).
                match engine.sync().await {
                    Ok(_) => {
                        // Success: reset backoff.
                        current_delay = base_interval;
                    }
                    Err(SyncError::OrgMembershipRevoked(ref org_hash)) => {
                        warn!("🚨 SYSTEM ALERT: You have been removed from organization (hash: {}) by an administrator. Proceeding to securely purge all locally cached copies of its data and schema to prevent orphans.", org_hash);

                        // 1. Delete membership structure locally (if running on Sled backend)
                        if let Some(pool) = &sled_pool {
                            let _ =
                                crate::org::operations::delete_org(pool, org_hash).map_err(|err| {
                                    tracing::error!("Failed to delete org structure: {}", err)
                                });
                        }

                        // 2. Erase the orphaned physical footprints in local DB
                        let _ = db_ops
                            .purge_org_data(org_hash)
                            .await
                            .map_err(|err| tracing::error!("Failed to purge org data: {}", err));

                        // Membership-revoked is not a transient retry target —
                        // stay responsive.
                        current_delay = base_interval;
                    }
                    Err(e) => {
                        warn!("sync cycle failed: {e}");
                        current_delay = next_backoff_on_failure(
                            current_delay,
                            base_interval,
                            max_delay,
                            engine.state().await,
                        );
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

/// Compute the next inter-cycle sleep duration after a sync failure.
///
/// - `Offline` state (network-class failure): double the current delay, cap
///   at `max`. This is the exponential backoff.
/// - Any other state (auth errors, permanent errors): reset to `base`.
///   Those failures have their own retry paths and we want to stay
///   responsive for the next cycle.
fn next_backoff_on_failure(
    current: tokio::time::Duration,
    base: tokio::time::Duration,
    max: tokio::time::Duration,
    state: SyncState,
) -> tokio::time::Duration {
    if state == SyncState::Offline {
        current.saturating_mul(2).min(max)
    } else {
        base
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tokio::time::Duration;

    #[test]
    fn offline_backoff_doubles_up_to_cap() {
        let base = Duration::from_secs(30);
        let max = MAX_OFFLINE_BACKOFF;

        // First failure: 30s → 60s
        let d1 = next_backoff_on_failure(base, base, max, SyncState::Offline);
        assert_eq!(d1, Duration::from_secs(60));

        // Second: 60s → 120s
        let d2 = next_backoff_on_failure(d1, base, max, SyncState::Offline);
        assert_eq!(d2, Duration::from_secs(120));

        // Third: 120s → 240s
        let d3 = next_backoff_on_failure(d2, base, max, SyncState::Offline);
        assert_eq!(d3, Duration::from_secs(240));

        // Fourth: 240s → 480s (still under 600s cap)
        let d4 = next_backoff_on_failure(d3, base, max, SyncState::Offline);
        assert_eq!(d4, Duration::from_secs(480));

        // Fifth: 480s → 600s (capped, not 960s)
        let d5 = next_backoff_on_failure(d4, base, max, SyncState::Offline);
        assert_eq!(d5, max);

        // Sixth: stays at cap.
        let d6 = next_backoff_on_failure(d5, base, max, SyncState::Offline);
        assert_eq!(d6, max);
    }

    #[test]
    fn non_offline_failure_resets_to_base() {
        let base = Duration::from_secs(30);
        let max = MAX_OFFLINE_BACKOFF;

        // Even after many offline cycles, a Dirty-state failure resets.
        let at_cap = max;
        let reset = next_backoff_on_failure(at_cap, base, max, SyncState::Dirty);
        assert_eq!(reset, base);

        // Same for other non-offline states.
        let reset_idle = next_backoff_on_failure(at_cap, base, max, SyncState::Idle);
        assert_eq!(reset_idle, base);
    }

    #[test]
    fn backoff_respects_custom_base() {
        let base = Duration::from_secs(5);
        let max = Duration::from_secs(60);

        let d1 = next_backoff_on_failure(base, base, max, SyncState::Offline);
        assert_eq!(d1, Duration::from_secs(10));

        let d2 = next_backoff_on_failure(d1, base, max, SyncState::Offline);
        assert_eq!(d2, Duration::from_secs(20));

        let d3 = next_backoff_on_failure(d2, base, max, SyncState::Offline);
        assert_eq!(d3, Duration::from_secs(40));

        // Next would be 80s but cap is 60s.
        let d4 = next_backoff_on_failure(d3, base, max, SyncState::Offline);
        assert_eq!(d4, max);
    }
}
