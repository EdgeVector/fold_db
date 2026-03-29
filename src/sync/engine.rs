use super::auth::AuthClient;
use super::error::{SyncError, SyncResult};
use super::log::{LogEntry, LogOp};
use super::s3::S3Client;
use super::snapshot::Snapshot;
use crate::crypto::CryptoProvider;
use crate::storage::traits::NamespacedStore;
use serde::Serialize;
use std::sync::Arc;
use tokio::sync::Mutex;

/// Sync engine state.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum SyncState {
    /// No unsynced changes.
    Idle,
    /// Local changes not yet uploaded.
    Dirty,
    /// Upload in progress.
    Syncing,
    /// Network unavailable, will retry.
    Offline,
}

/// Snapshot of sync engine status for external consumers.
#[derive(Debug, Clone, Serialize)]
pub struct SyncStatus {
    /// Current state of the sync engine.
    pub state: SyncState,
    /// Number of pending (unsynced) log entries.
    pub pending_count: usize,
    /// Unix timestamp (seconds) of last successful sync, if any.
    pub last_sync_at: Option<u64>,
    /// Last sync error message, if the most recent sync failed.
    pub last_error: Option<String>,
}

/// Configuration for the sync engine.
#[derive(Debug, Clone)]
pub struct SyncConfig {
    /// How often to sync when dirty (milliseconds).
    pub sync_interval_ms: u64,
    /// Number of log entries before triggering compaction (snapshot + delete old logs).
    pub compaction_threshold: u64,
    /// Device lock TTL in seconds.
    pub lock_ttl_secs: u64,
    /// Maximum retries for network operations.
    pub max_retries: u32,
}

impl Default for SyncConfig {
    fn default() -> Self {
        Self {
            sync_interval_ms: 30_000,
            compaction_threshold: 100,
            lock_ttl_secs: 300,
            max_retries: 2,
        }
    }
}

/// Callback for sync status changes.
pub type StatusCallback = Box<dyn Fn(SyncState, Option<&str>) + Send + Sync>;

/// The sync engine manages replication of a local Sled database to S3.
///
/// Architecture:
/// ```text
/// fold_db (local) ──▶ SyncEngine ──▶ Auth Lambda ──▶ S3 (encrypted blobs)
///
/// State machine:
///   IDLE ──mutation──▶ DIRTY ──timer──▶ SYNCING ──success──▶ IDLE
///                       ▲                  │
///                       └──── failure ─────┘
/// ```
///
/// The engine:
/// 1. Records KvStore operations as encrypted log entries
/// 2. Uploads log entries to S3 via presigned URLs
/// 3. Periodically compacts logs into snapshots
/// 4. Manages single-device write lock
/// 5. Supports bootstrap (download snapshot + replay logs) for new devices
pub struct SyncEngine {
    state: Arc<Mutex<SyncState>>,
    /// Pending log entries not yet uploaded.
    pending: Arc<Mutex<Vec<LogEntry>>>,
    /// Current sequence number.
    seq: Arc<Mutex<u64>>,
    /// Device identifier (unique per device).
    device_id: String,
    /// Encryption provider for sealing log entries and snapshots.
    crypto: Arc<dyn CryptoProvider>,
    /// S3 client for uploads/downloads.
    s3: S3Client,
    /// Auth client for presigned URLs and lock management.
    auth: AuthClient,
    /// The local namespaced store (for snapshot creation).
    store: Arc<dyn NamespacedStore>,
    /// Configuration.
    config: SyncConfig,
    /// Optional callback for status changes.
    status_callback: Option<StatusCallback>,
    /// Unix timestamp (seconds) of last successful sync.
    last_sync_at: Arc<Mutex<Option<u64>>>,
    /// Last sync error message (cleared on success).
    last_error: Arc<Mutex<Option<String>>>,
}

impl SyncEngine {
    pub fn new(
        device_id: String,
        crypto: Arc<dyn CryptoProvider>,
        s3: S3Client,
        auth: AuthClient,
        store: Arc<dyn NamespacedStore>,
        config: SyncConfig,
    ) -> Self {
        Self {
            state: Arc::new(Mutex::new(SyncState::Idle)),
            pending: Arc::new(Mutex::new(Vec::new())),
            seq: Arc::new(Mutex::new(0)),
            device_id,
            crypto,
            s3,
            auth,
            store,
            config,
            status_callback: None,
            last_sync_at: Arc::new(Mutex::new(None)),
            last_error: Arc::new(Mutex::new(None)),
        }
    }

    /// Set a callback that fires on state changes.
    pub fn set_status_callback(&mut self, cb: StatusCallback) {
        self.status_callback = Some(cb);
    }

    /// Get the device identifier.
    pub fn device_id(&self) -> &str {
        &self.device_id
    }

    /// Get the current sync state.
    pub async fn state(&self) -> SyncState {
        *self.state.lock().await
    }

    /// Get the number of pending (unsynced) log entries.
    pub async fn pending_count(&self) -> usize {
        self.pending.lock().await.len()
    }

    /// Get a full status snapshot of the sync engine.
    pub async fn status(&self) -> SyncStatus {
        SyncStatus {
            state: *self.state.lock().await,
            pending_count: self.pending.lock().await.len(),
            last_sync_at: *self.last_sync_at.lock().await,
            last_error: self.last_error.lock().await.clone(),
        }
    }

    async fn set_state(&self, new_state: SyncState, message: Option<&str>) {
        let mut state = self.state.lock().await;
        *state = new_state;
        if let Some(cb) = &self.status_callback {
            cb(new_state, message);
        }
    }

    // =========================================================================
    // Recording operations
    // =========================================================================

    /// Record a put operation for sync.
    pub async fn record_put(&self, namespace: &str, key: &[u8], value: &[u8]) {
        let entry = self
            .make_entry(LogOp::Put {
                namespace: namespace.to_string(),
                key: LogOp::encode_bytes(key),
                value: LogOp::encode_bytes(value),
            })
            .await;

        self.pending.lock().await.push(entry);
        self.set_state(SyncState::Dirty, None).await;
    }

    /// Record a delete operation for sync.
    pub async fn record_delete(&self, namespace: &str, key: &[u8]) {
        let entry = self
            .make_entry(LogOp::Delete {
                namespace: namespace.to_string(),
                key: LogOp::encode_bytes(key),
            })
            .await;

        self.pending.lock().await.push(entry);
        self.set_state(SyncState::Dirty, None).await;
    }

    /// Record a batch put operation for sync.
    pub async fn record_batch_put(&self, namespace: &str, items: &[(Vec<u8>, Vec<u8>)]) {
        let encoded_items: Vec<(String, String)> = items
            .iter()
            .map(|(k, v)| (LogOp::encode_bytes(k), LogOp::encode_bytes(v)))
            .collect();

        let entry = self
            .make_entry(LogOp::BatchPut {
                namespace: namespace.to_string(),
                items: encoded_items,
            })
            .await;

        self.pending.lock().await.push(entry);
        self.set_state(SyncState::Dirty, None).await;
    }

    /// Record a batch delete operation for sync.
    pub async fn record_batch_delete(&self, namespace: &str, keys: &[Vec<u8>]) {
        let encoded_keys: Vec<String> = keys.iter().map(|k| LogOp::encode_bytes(k)).collect();

        let entry = self
            .make_entry(LogOp::BatchDelete {
                namespace: namespace.to_string(),
                keys: encoded_keys,
            })
            .await;

        self.pending.lock().await.push(entry);
        self.set_state(SyncState::Dirty, None).await;
    }

    async fn make_entry(&self, op: LogOp) -> LogEntry {
        let mut seq = self.seq.lock().await;
        *seq += 1;
        let now = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap_or_default()
            .as_millis() as u64;

        LogEntry {
            seq: *seq,
            timestamp_ms: now,
            device_id: self.device_id.clone(),
            op,
        }
    }

    // =========================================================================
    // Sync cycle
    // =========================================================================

    /// Run one sync cycle: upload pending log entries, compact if needed.
    ///
    /// Returns Ok(true) if all pending entries were uploaded,
    /// Ok(false) if there was nothing to sync.
    pub async fn sync(&self) -> SyncResult<bool> {
        // Atomic check-and-set: hold the lock for both the state check and transition
        {
            let mut state = self.state.lock().await;
            if *state == SyncState::Syncing || *state == SyncState::Idle {
                return Ok(false);
            }
            *state = SyncState::Syncing;
        }
        if let Some(cb) = &self.status_callback {
            cb(SyncState::Syncing, Some("uploading"));
        }

        match self.do_sync().await {
            Ok(synced) => {
                if synced {
                    let now = std::time::SystemTime::now()
                        .duration_since(std::time::UNIX_EPOCH)
                        .unwrap_or_default()
                        .as_secs();
                    *self.last_sync_at.lock().await = Some(now);
                    *self.last_error.lock().await = None;
                }
                self.set_state(SyncState::Idle, None).await;
                Ok(synced)
            }
            Err(e) => {
                let msg = e.to_string();
                *self.last_error.lock().await = Some(msg.clone());
                match &e {
                    SyncError::Network(_) => {
                        self.set_state(SyncState::Offline, Some(&msg)).await;
                    }
                    SyncError::Auth(_) => {
                        self.set_state(SyncState::Dirty, Some(&msg)).await;
                    }
                    _ => {
                        self.set_state(SyncState::Dirty, Some(&msg)).await;
                    }
                }
                Err(e)
            }
        }
    }

    async fn do_sync(&self) -> SyncResult<bool> {
        let entries = {
            let pending = self.pending.lock().await;
            if pending.is_empty() {
                return Ok(false);
            }
            pending.clone()
        };

        // Seal each entry individually
        let mut sealed_entries = Vec::with_capacity(entries.len());
        for entry in &entries {
            let sealed = entry.seal(&self.crypto).await?;
            sealed_entries.push((entry.seq, sealed));
        }

        // Get presigned URLs for all entries
        let seq_numbers: Vec<u64> = sealed_entries.iter().map(|(seq, _)| *seq).collect();
        let urls = self.auth.presign_log_upload(&seq_numbers).await?;

        if urls.len() != sealed_entries.len() {
            return Err(SyncError::Auth(format!(
                "expected {} presigned URLs, got {}",
                sealed_entries.len(),
                urls.len()
            )));
        }

        // Upload each sealed entry
        for ((_seq, sealed), url) in sealed_entries.into_iter().zip(urls.iter()) {
            self.s3.upload(url, sealed.bytes).await?;
        }

        // Clear uploaded entries from pending
        {
            let mut pending = self.pending.lock().await;
            let uploaded_count = entries.len();
            let count = uploaded_count.min(pending.len());
            pending.drain(..count);
        }

        // Check if compaction is needed
        let current_seq = *self.seq.lock().await;
        if current_seq > 0 && current_seq % self.config.compaction_threshold == 0 {
            if let Err(e) = self.compact(current_seq).await {
                log::warn!("compaction failed (non-fatal): {e}");
            }
        }

        Ok(true)
    }

    // =========================================================================
    // Compaction (snapshot + delete old logs)
    // =========================================================================

    async fn compact(&self, last_seq: u64) -> SyncResult<()> {
        log::info!("compacting: creating snapshot at seq {last_seq}");

        let snapshot = Snapshot::create(self.store.as_ref(), &self.device_id, last_seq).await?;

        let sealed = snapshot.seal(&self.crypto).await?;

        // Upload as {seq}.enc
        let snapshot_name = format!("{last_seq}.enc");
        let url = self.auth.presign_snapshot_upload(&snapshot_name).await?;
        self.s3.upload(&url, sealed.clone()).await?;

        // Upload same sealed bytes as latest.enc
        let latest_url = self.auth.presign_snapshot_upload("latest.enc").await?;
        self.s3.upload(&latest_url, sealed).await?;

        // Delete old log entries that were compacted into this snapshot
        let seq_numbers: Vec<u64> = (1..=last_seq).collect();
        if !seq_numbers.is_empty() {
            match self.auth.presign_log_delete(&seq_numbers).await {
                Ok(delete_urls) => {
                    for url in &delete_urls {
                        if let Err(e) = self.s3.delete(url).await {
                            log::warn!("failed to delete compacted log entry (non-fatal): {e}");
                        }
                    }
                    log::info!("deleted {} compacted log entries", delete_urls.len());
                }
                Err(e) => {
                    log::warn!("failed to get delete URLs for compacted logs (non-fatal): {e}");
                }
            }
        }

        log::info!("compaction complete: snapshot at seq {last_seq}");
        Ok(())
    }

    // =========================================================================
    // Bootstrap (download snapshot + replay logs)
    // =========================================================================

    /// Bootstrap this device from S3.
    ///
    /// Downloads the latest snapshot, restores it to the local store,
    /// then replays any log entries after the snapshot's sequence number.
    pub async fn bootstrap(&self) -> SyncResult<u64> {
        log::info!("bootstrapping from S3");

        // Download latest snapshot
        let snapshot_url = self.auth.presign_snapshot_download("latest.enc").await?;
        let snapshot_data = self.s3.download(&snapshot_url).await?;

        let last_seq = match snapshot_data {
            Some(data) => {
                let snapshot = Snapshot::unseal(&data, &self.crypto).await?;
                let last_seq = snapshot.last_seq;

                log::info!(
                    "restoring snapshot: {} namespaces, last_seq={}",
                    snapshot.namespaces.len(),
                    last_seq
                );

                snapshot.restore(self.store.as_ref()).await?;
                last_seq
            }
            None => {
                log::info!("no snapshot found — starting fresh");
                0
            }
        };

        // List and replay log entries after the snapshot
        let log_objects = self.auth.list_objects("log/").await?;
        let mut log_seqs: Vec<u64> = log_objects
            .iter()
            .filter_map(|obj| {
                obj.key
                    .rsplit('/')
                    .next()
                    .and_then(|name| name.strip_suffix(".enc"))
                    .and_then(|s| s.parse::<u64>().ok())
            })
            .filter(|seq| *seq > last_seq)
            .collect();

        log_seqs.sort();

        if !log_seqs.is_empty() {
            log::info!(
                "replaying {} log entries (seq {}..={})",
                log_seqs.len(),
                log_seqs[0],
                log_seqs[log_seqs.len() - 1]
            );

            let urls = self.auth.presign_log_download(&log_seqs).await?;

            for (seq, url) in log_seqs.iter().zip(urls.iter()) {
                let data = self.s3.download(url).await?;
                match data {
                    Some(bytes) => match LogEntry::unseal(&bytes, &self.crypto).await {
                        Ok(entry) => {
                            self.replay_entry(&entry).await?;
                        }
                        Err(e) => {
                            log::warn!("skipping corrupt log entry seq={seq}: {e}");
                        }
                    },
                    None => {
                        log::warn!("log entry seq={seq} not found in S3, skipping");
                    }
                }
            }
        }

        // Update local sequence counter
        let final_seq = log_seqs.last().copied().unwrap_or(last_seq);
        *self.seq.lock().await = final_seq;

        log::info!("bootstrap complete at seq {final_seq}");
        Ok(final_seq)
    }

    /// Replay a single log entry by executing the operation against the local store.
    async fn replay_entry(&self, entry: &LogEntry) -> SyncResult<()> {
        match &entry.op {
            LogOp::Put {
                namespace,
                key,
                value,
            } => {
                let kv = self.store.open_namespace(namespace).await?;
                let key_bytes = LogOp::decode_bytes(key)?;
                let value_bytes = LogOp::decode_bytes(value)?;
                kv.put(&key_bytes, value_bytes).await?;
            }
            LogOp::Delete { namespace, key } => {
                let kv = self.store.open_namespace(namespace).await?;
                let key_bytes = LogOp::decode_bytes(key)?;
                kv.delete(&key_bytes).await?;
            }
            LogOp::BatchPut { namespace, items } => {
                let kv = self.store.open_namespace(namespace).await?;
                let decoded: Vec<(Vec<u8>, Vec<u8>)> = items
                    .iter()
                    .map(|(k, v)| Ok((LogOp::decode_bytes(k)?, LogOp::decode_bytes(v)?)))
                    .collect::<SyncResult<Vec<_>>>()?;
                kv.batch_put(decoded).await?;
            }
            LogOp::BatchDelete { namespace, keys } => {
                let kv = self.store.open_namespace(namespace).await?;
                let decoded: Vec<Vec<u8>> = keys
                    .iter()
                    .map(|k| LogOp::decode_bytes(k))
                    .collect::<SyncResult<Vec<_>>>()?;
                kv.batch_delete(decoded).await?;
            }
        }
        Ok(())
    }

    // =========================================================================
    // Lock management
    // =========================================================================

    /// Acquire the write lock for this device.
    pub async fn acquire_lock(&self) -> SyncResult<()> {
        self.auth
            .acquire_lock(&self.device_id, self.config.lock_ttl_secs)
            .await?;
        Ok(())
    }

    /// Release the write lock.
    pub async fn release_lock(&self) -> SyncResult<()> {
        self.auth.release_lock(&self.device_id).await
    }

    /// Renew the write lock (extend TTL).
    pub async fn renew_lock(&self) -> SyncResult<()> {
        self.auth
            .renew_lock(&self.device_id, self.config.lock_ttl_secs)
            .await
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn default_config() {
        let config = SyncConfig::default();
        assert_eq!(config.sync_interval_ms, 30_000);
        assert_eq!(config.compaction_threshold, 100);
        assert_eq!(config.lock_ttl_secs, 300);
        assert_eq!(config.max_retries, 2);
    }
}
