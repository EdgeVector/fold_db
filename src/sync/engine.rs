use super::auth::{AuthClient, AuthRefreshCallback};
use super::error::{SyncError, SyncResult};
use super::log::{LogEntry, LogOp};
use super::org_sync::{SyncDestination, SyncPartitioner, SyncTarget};
use super::s3::S3Client;
use super::snapshot::Snapshot;
use crate::atom::{
    FieldKey, MergeConflict, Molecule, MoleculeHash, MoleculeHashRange, MoleculeRange,
    MutationEvent,
};
use crate::crypto::CryptoProvider;
use crate::storage::traits::NamespacedStore;
use chrono::Utc;
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

/// A merge conflict detected during sync replay.
/// Stored at key `conflict:{mol_uuid}:{ts}` for efficient scanning.
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct SyncConflict {
    /// Unique ID: "{mol_uuid}:{ts_nanos_padded}"
    pub id: String,
    /// The molecule where the conflict occurred.
    pub molecule_uuid: String,
    /// The key within the molecule (e.g. "single", hash key, "hash:range").
    pub conflict_key: String,
    /// The atom UUID that won (later written_at).
    pub winner_atom: String,
    /// The atom UUID that lost.
    pub loser_atom: String,
    /// Winner's write timestamp (nanos since epoch).
    pub winner_written_at: u64,
    /// Loser's write timestamp (nanos since epoch).
    pub loser_written_at: u64,
    /// When the conflict was detected.
    #[serde(with = "chrono::serde::ts_milliseconds")]
    pub detected_at: chrono::DateTime<chrono::Utc>,
    /// Whether this conflict has been acknowledged/resolved by the user.
    pub resolved: bool,
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

/// Callback that reloads schemas from the persistent store into the in-memory cache.
/// Returns the number of newly added schemas, or an error string.
pub type SchemaReloadCallback = Arc<
    dyn Fn() -> std::pin::Pin<Box<dyn std::future::Future<Output = Result<usize, String>> + Send>>
        + Send
        + Sync,
>;

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
    /// Partitioner for classifying pending entries by key prefix.
    partitioner: Arc<Mutex<Option<SyncPartitioner>>>,
    /// All sync targets. Index 0 is always the personal target.
    /// Org targets are appended via `configure_org_sync`.
    targets: Arc<Mutex<Vec<SyncTarget>>>,
    /// Per-prefix download cursor: maps prefix -> last_seq_downloaded.
    download_cursors: Arc<Mutex<std::collections::HashMap<String, u64>>>,
    /// Optional callback invoked after sync replay writes schemas to Sled.
    /// This lets the SchemaCore cache refresh without a hard dependency.
    schema_reloader: Arc<Mutex<Option<SchemaReloadCallback>>>,
    /// Optional callback to refresh authentication credentials on 401.
    /// When set, the sync engine will call this on `SyncError::Auth`, update
    /// the `AuthClient`, and retry the sync cycle once before giving up.
    auth_refresh: Option<AuthRefreshCallback>,
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
            crypto: crypto.clone(),
            s3,
            auth,
            store,
            config,
            status_callback: None,
            last_sync_at: Arc::new(Mutex::new(None)),
            last_error: Arc::new(Mutex::new(None)),
            partitioner: Arc::new(Mutex::new(None)),
            targets: Arc::new(Mutex::new(vec![SyncTarget {
                label: "personal".to_string(),
                prefix: String::new(),
                crypto,
            }])),
            download_cursors: Arc::new(Mutex::new(std::collections::HashMap::new())),
            schema_reloader: Arc::new(Mutex::new(None)),
            auth_refresh: None,
        }
    }

    /// Load persisted download cursors from storage.
    /// Called on startup to resume incremental downloads.
    pub async fn load_download_cursors(&self) {
        let kv = match self.store.open_namespace("sync_cursors").await {
            Ok(kv) => kv,
            Err(e) => {
                log::warn!("Failed to open sync_cursors namespace: {}", e);
                return;
            }
        };
        let entries = match kv.scan_prefix(b"cursor:").await {
            Ok(entries) => entries,
            Err(e) => {
                log::warn!("Failed to scan cursor keys: {}", e);
                return;
            }
        };
        let mut cursors = self.download_cursors.lock().await;
        for (key_bytes, val_bytes) in entries {
            if let Ok(key) = std::str::from_utf8(&key_bytes) {
                let prefix = key.strip_prefix("cursor:").unwrap_or(key);
                if val_bytes.len() == 8 {
                    let seq = u64::from_be_bytes(val_bytes.try_into().unwrap_or([0; 8]));
                    cursors.insert(prefix.to_string(), seq);
                }
            }
        }
        if !cursors.is_empty() {
            log::info!("Loaded {} download cursors from storage", cursors.len());
        }
    }

    /// Set a callback that fires on state changes.
    pub fn set_status_callback(&mut self, cb: StatusCallback) {
        self.status_callback = Some(cb);
    }

    /// Set a callback that refreshes authentication credentials on 401.
    ///
    /// When the sync engine encounters an auth error (expired token, etc.),
    /// it calls this callback to obtain fresh credentials, updates the
    /// `AuthClient`, and retries the sync cycle once.
    pub fn set_auth_refresh(&mut self, cb: AuthRefreshCallback) {
        self.auth_refresh = Some(cb);
    }

    /// Register a callback that reloads the SchemaCore cache after sync
    /// replays schema entries into Sled. The callback returns the number
    /// of newly added schemas, or an error string.
    pub async fn set_schema_reloader(&self, reloader: SchemaReloadCallback) {
        *self.schema_reloader.lock().await = Some(reloader);
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
        let now = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap_or_default();
        let nanos = now.as_nanos() as u64;

        // Ensure monotonically increasing: if clock gives same nanos as last
        // entry, bump by 1 to guarantee uniqueness within this process.
        let mut last = self.seq.lock().await;
        let seq = if nanos <= *last { *last + 1 } else { nanos };
        *last = seq;

        LogEntry {
            seq,
            timestamp_ms: now.as_millis() as u64,
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
        // Check org sync before acquiring state lock to avoid holding state
        // across an await on the partitioner lock.
        let has_orgs = self.has_org_sync().await;

        // Atomic check-and-set: hold the lock for both the state check and transition
        {
            let mut state = self.state.lock().await;
            if *state == SyncState::Syncing {
                log::info!("sync skipped: already syncing");
                return Ok(false);
            }
            // When Idle with no pending writes, we still need to proceed if org
            // sync is configured — other members may have uploaded data we need
            // to download. Skip only when Idle AND no org memberships.
            if *state == SyncState::Idle && !has_orgs {
                log::info!("sync skipped: idle with no org targets");
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
                // On auth errors, attempt to refresh credentials and retry once.
                if matches!(&e, SyncError::Auth(_)) {
                    if let Some(ref refresh_cb) = self.auth_refresh {
                        log::info!("sync auth failed, attempting token refresh");
                        match refresh_cb().await {
                            Ok(new_auth) => {
                                self.auth.update_auth(new_auth).await;
                                log::info!("token refreshed, retrying sync");
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
                                        return Ok(synced);
                                    }
                                    Err(retry_err) => {
                                        let msg = retry_err.to_string();
                                        log::warn!(
                                            "sync retry after token refresh failed: {msg}"
                                        );
                                        *self.last_error.lock().await = Some(msg.clone());
                                        self.set_state(SyncState::Dirty, Some(&msg)).await;
                                        return Err(retry_err);
                                    }
                                }
                            }
                            Err(refresh_err) => {
                                log::warn!("token refresh failed: {refresh_err}");
                            }
                        }
                    }
                }

                let msg = e.to_string();
                *self.last_error.lock().await = Some(msg.clone());
                match &e {
                    SyncError::Network(_) => {
                        self.set_state(SyncState::Offline, Some(&msg)).await;
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
        let targets = self.targets.lock().await.clone();
        let mut uploaded = 0usize;
        let mut downloaded = 0u64;

        // Upload pending entries, partitioned across targets
        let entries: Vec<LogEntry> = {
            let pending = self.pending.lock().await;
            pending.clone()
        };

        if !entries.is_empty() {
            let partitioner = self.partitioner.lock().await;

            // Partition entries across targets by key prefix
            let mut buckets: std::collections::HashMap<usize, Vec<LogEntry>> =
                std::collections::HashMap::new();
            for entry in &entries {
                let idx = Self::classify_to_target(&partitioner, entry, &targets);
                buckets.entry(idx).or_default().push(entry.clone());
            }
            drop(partitioner);

            // Upload each bucket with its target's crypto
            for (target_idx, bucket) in &buckets {
                log::info!(
                    "uploading {} entries to target {} ('{}')",
                    bucket.len(),
                    target_idx,
                    targets
                        .get(*target_idx)
                        .map(|t| t.label.as_str())
                        .unwrap_or("?")
                );
                let target = &targets[*target_idx];
                match self.upload_entries(target, bucket).await {
                    Ok(n) => uploaded += n,
                    Err(e) => log::warn!("upload to '{}' failed: {e}", target.label),
                }
            }

            // Clear uploaded entries from pending
            {
                let mut pending = self.pending.lock().await;
                let count = entries.len().min(pending.len());
                pending.drain(..count);
            }
        }

        // Download from all org targets
        for target in &targets {
            if !target.prefix.is_empty() {
                match self.download_entries(target).await {
                    Ok(n) => downloaded += n,
                    Err(e) => log::warn!("download from '{}' failed: {e}", target.label),
                }
            }
        }

        // Compaction: snapshot personal data periodically
        if uploaded >= self.config.compaction_threshold as usize {
            let current_seq = *self.seq.lock().await;
            if current_seq > 0 {
                if let Err(e) = self.compact(current_seq).await {
                    log::warn!("compaction failed (non-fatal): {e}");
                }
            }
        }

        Ok(uploaded > 0 || downloaded > 0)
    }

    /// Classify a pending entry to a target index.
    fn classify_to_target(
        partitioner: &Option<SyncPartitioner>,
        entry: &LogEntry,
        targets: &[SyncTarget],
    ) -> usize {
        if let Some(p) = partitioner {
            let dest = Self::classify_entry(p, entry);
            if let SyncDestination::Org { org_hash, .. } = dest {
                // Find the target with matching prefix
                for (i, t) in targets.iter().enumerate() {
                    if !t.prefix.is_empty() && t.prefix == org_hash {
                        return i;
                    }
                }
            }
        }
        0 // Default to personal target
    }

    /// Upload entries to a single sync target.
    async fn upload_entries(&self, target: &SyncTarget, entries: &[LogEntry]) -> SyncResult<usize> {
        if entries.is_empty() {
            return Ok(0);
        }

        let mut sealed = Vec::with_capacity(entries.len());
        for entry in entries {
            let s = entry.seal(&target.crypto).await?;
            sealed.push((entry.seq, s));
        }

        let seq_numbers: Vec<u64> = sealed.iter().map(|(seq, _)| *seq).collect();
        let urls = self.auth.presign_upload(target, &seq_numbers).await?;

        if urls.len() != sealed.len() {
            return Err(SyncError::Auth(format!(
                "expected {} presigned URLs for '{}', got {}",
                sealed.len(),
                target.label,
                urls.len()
            )));
        }

        for ((_seq, s), url) in sealed.into_iter().zip(urls.iter()) {
            self.s3.upload(url, s.bytes).await?;
        }

        Ok(entries.len())
    }

    /// Download new entries from a sync target.
    ///
    /// Lists `/{prefix}/log/{seq}.enc` starting after the local cursor,
    /// downloads, unseals with the target's crypto, and replays.
    async fn download_entries(&self, target: &SyncTarget) -> SyncResult<u64> {
        let cursor = {
            let cursors = self.download_cursors.lock().await;
            cursors.get(&target.prefix).copied().unwrap_or(0)
        };

        // Use start_after to filter server-side instead of listing everything
        let start_after = if cursor > 0 {
            Some(format!("log/{cursor}.enc"))
        } else {
            None
        };
        let objects = self
            .auth
            .list_log_objects_after(target, start_after.as_deref())
            .await?;

        // Parse flat log keys: log/{seq}.enc
        let mut new_seqs: Vec<u64> = objects
            .iter()
            .filter_map(|obj| parse_flat_log_key(&obj.key))
            .filter(|s| *s > cursor)
            .collect();
        new_seqs.sort();

        if new_seqs.is_empty() {
            log::info!(
                "download '{}': 0 new entries (cursor={})",
                target.label,
                cursor
            );
            return Ok(0);
        }

        log::info!(
            "download '{}': {} new entries after cursor={}",
            target.label,
            new_seqs.len(),
            cursor
        );

        let urls = self.auth.presign_download(target, &new_seqs).await?;

        let mut total_replayed = 0u64;
        let mut max_seq = cursor;
        let mut schemas_replayed = false;

        for (seq, url) in new_seqs.iter().zip(urls.iter()) {
            match self.s3.download(url).await? {
                Some(bytes) => match LogEntry::unseal(&bytes, &target.crypto).await {
                    Ok(entry) => {
                        if entry.op.namespace() == "schemas" {
                            schemas_replayed = true;
                        }
                        self.replay_entry(&entry).await?;
                        total_replayed += 1;
                        if *seq > max_seq {
                            max_seq = *seq;
                        }
                    }
                    Err(e) => {
                        log::warn!(
                            "skipping corrupt entry in '{}' seq={}: {}",
                            target.label,
                            seq,
                            e
                        );
                    }
                },
                None => {
                    log::warn!("entry not found in '{}' seq={}", target.label, seq);
                }
            }
        }

        // Reload SchemaCore cache if any schema entries were replayed
        if schemas_replayed {
            if let Some(reloader) = self.schema_reloader.lock().await.as_ref() {
                match reloader().await {
                    Ok(count) => {
                        if count > 0 {
                            log::info!(
                                "schema reloader added {} schema(s) after sync from '{}'",
                                count,
                                target.label
                            );
                        }
                    }
                    Err(e) => {
                        log::warn!("failed to reload schemas after sync: {}", e);
                    }
                }
            }
        }

        // Update cursor
        if max_seq > cursor {
            let mut cursors = self.download_cursors.lock().await;
            cursors.insert(target.prefix.clone(), max_seq);
            drop(cursors);
            self.save_download_cursor(&target.prefix, max_seq).await;
        }

        Ok(total_replayed)
    }

    /// Persist a download cursor to Sled.
    async fn save_download_cursor(&self, prefix: &str, seq: u64) {
        let cursor_key = format!("cursor:{}", prefix);
        if let Ok(kv) = self.store.open_namespace("sync_cursors").await {
            let _ = kv
                .put(cursor_key.as_bytes(), seq.to_be_bytes().to_vec())
                .await;
        }
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

        // Delete old log entries that were compacted into this snapshot.
        // List objects and delete those with seq <= last_seq.
        let personal = self.targets.lock().await[0].clone();
        match self.auth.list_log_objects(&personal).await {
            Ok(objects) => {
                let old_seqs: Vec<u64> = objects
                    .iter()
                    .filter_map(|obj| parse_flat_log_key(&obj.key))
                    .filter(|seq| *seq <= last_seq)
                    .collect();
                if !old_seqs.is_empty() {
                    match self.auth.presign_log_delete(&old_seqs).await {
                        Ok(delete_urls) => {
                            for url in &delete_urls {
                                if let Err(e) = self.s3.delete(url).await {
                                    log::warn!("failed to delete compacted log (non-fatal): {e}");
                                }
                            }
                            log::info!("deleted {} compacted log entries", delete_urls.len());
                        }
                        Err(e) => {
                            log::warn!(
                                "failed to get delete URLs for compacted logs (non-fatal): {e}"
                            );
                        }
                    }
                }
            }
            Err(e) => {
                log::warn!("failed to list logs for compaction cleanup (non-fatal): {e}");
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
        let personal = self.targets.lock().await[0].clone();
        let log_objects = self.auth.list_log_objects(&personal).await?;
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

            let urls = self.auth.presign_download(&personal, &log_seqs).await?;

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

    /// Replay a single log entry with convergent ref handling.
    ///
    /// Non-ref keys (atoms, history) are written unconditionally.
    /// Ref keys (`ref:` or `{org_hash}:ref:`) use molecule merge so all
    /// nodes converge to the same state regardless of replay order.
    pub async fn replay_entry(&self, entry: &LogEntry) -> SyncResult<()> {
        match &entry.op {
            LogOp::Put {
                namespace,
                key,
                value,
            } => {
                self.replay_put(namespace, key, value, entry.timestamp_ms, &entry.device_id)
                    .await?;
            }
            LogOp::Delete { namespace, key } => {
                let kv = self.store.open_namespace(namespace).await?;
                let key_bytes = LogOp::decode_bytes(key)?;
                kv.delete(&key_bytes).await?;
            }
            LogOp::BatchPut { namespace, items } => {
                for (key, value) in items {
                    self.replay_put(namespace, key, value, entry.timestamp_ms, &entry.device_id)
                        .await?;
                }
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

    /// Replay a single put. Ref keys use molecule merge; everything else is unconditional.
    async fn replay_put(
        &self,
        namespace: &str,
        key_b64: &str,
        value_b64: &str,
        _timestamp_ms: u64,
        _device_id: &str,
    ) -> SyncResult<()> {
        let key_bytes = LogOp::decode_bytes(key_b64)?;
        let value_bytes = LogOp::decode_bytes(value_b64)?;

        let is_ref_key = key_bytes.starts_with(b"ref:")
            || std::str::from_utf8(&key_bytes)
                .ok()
                .is_some_and(|s| s.contains(":ref:"));

        if is_ref_key {
            // Extract molecule UUID from the ref key (e.g. "ref:{uuid}" or "{org}:ref:{uuid}")
            let key_str = std::str::from_utf8(&key_bytes).unwrap_or("");
            let mol_uuid = key_str
                .rsplit_once("ref:")
                .map(|(_, uuid)| uuid)
                .unwrap_or(key_str)
                .to_string();

            let kv = self.store.open_namespace(namespace).await?;
            let local_bytes = kv.get(&key_bytes).await?;

            match local_bytes {
                Some(local) => {
                    // Both exist — try molecule merge
                    let (merged_bytes, conflicts) = Self::merge_molecules(&local, &value_bytes)?;
                    kv.put(&key_bytes, merged_bytes).await?;

                    // Store any merge conflicts
                    if !conflicts.is_empty() {
                        Self::store_merge_conflicts(&kv, &mol_uuid, &conflicts).await?;
                    }
                }
                None => {
                    // No local — just write incoming
                    kv.put(&key_bytes, value_bytes).await?;
                }
            }
        } else {
            let kv = self.store.open_namespace(namespace).await?;
            kv.put(&key_bytes, value_bytes.clone()).await?;

            // When a schema is replayed (from personal sync between devices
            // OR from org sync), write org-prefixed keys under the bare key
            // so get_schema finds them by name.
            if namespace == "schemas" {
                if let Ok(key_str) = std::str::from_utf8(&key_bytes) {
                    if let Some((_, base_key)) = crate::sync::org_sync::strip_org_prefix(key_str) {
                        kv.put(base_key.as_bytes(), value_bytes.clone()).await?;
                    }
                }
            }
        }

        Ok(())
    }

    /// Attempt molecule merge by trying each molecule type in order.
    /// Returns the serialized merged result and any conflicts.
    fn merge_molecules(
        local_bytes: &[u8],
        incoming_bytes: &[u8],
    ) -> SyncResult<(Vec<u8>, Vec<MergeConflict>)> {
        // Try MoleculeHash (HashMap-based atom_uuids)
        if let (Ok(mut local), Ok(incoming)) = (
            serde_json::from_slice::<MoleculeHash>(local_bytes),
            serde_json::from_slice::<MoleculeHash>(incoming_bytes),
        ) {
            let conflicts = local.merge(&incoming);
            let merged = serde_json::to_vec(&local)?;
            return Ok((merged, conflicts));
        }

        // Try MoleculeRange (BTreeMap-based atom_uuids)
        if let (Ok(mut local), Ok(incoming)) = (
            serde_json::from_slice::<MoleculeRange>(local_bytes),
            serde_json::from_slice::<MoleculeRange>(incoming_bytes),
        ) {
            let conflicts = local.merge(&incoming);
            let merged = serde_json::to_vec(&local)?;
            return Ok((merged, conflicts));
        }

        // Try MoleculeHashRange (nested HashMap<BTreeMap>)
        if let (Ok(mut local), Ok(incoming)) = (
            serde_json::from_slice::<MoleculeHashRange>(local_bytes),
            serde_json::from_slice::<MoleculeHashRange>(incoming_bytes),
        ) {
            let conflicts = local.merge(&incoming);
            let merged = serde_json::to_vec(&local)?;
            return Ok((merged, conflicts));
        }

        // Try Molecule (single atom ref)
        if let (Ok(mut local), Ok(incoming)) = (
            serde_json::from_slice::<Molecule>(local_bytes),
            serde_json::from_slice::<Molecule>(incoming_bytes),
        ) {
            let conflict = local.merge(&incoming);
            let conflicts = conflict.into_iter().collect::<Vec<_>>();
            let merged = serde_json::to_vec(&local)?;
            return Ok((merged, conflicts));
        }

        // None of the molecule types matched — just use incoming bytes as-is
        Ok((incoming_bytes.to_vec(), Vec::new()))
    }

    /// Store merge conflicts as MutationEvent entries in the atoms namespace
    /// and as dedicated conflict records for efficient scanning.
    async fn store_merge_conflicts(
        kv: &Arc<dyn crate::storage::traits::KvStore>,
        mol_uuid: &str,
        conflicts: &[MergeConflict],
    ) -> SyncResult<()> {
        let now = Utc::now();
        for (i, conflict) in conflicts.iter().enumerate() {
            let ts_nanos = now.timestamp_nanos_opt().unwrap_or(0) + i as i64;

            // Parse conflict.key into a proper FieldKey
            let field_key = if conflict.key == "single" {
                FieldKey::Single
            } else if let Some((hash, range)) = conflict.key.split_once(':') {
                FieldKey::HashRange {
                    hash: hash.to_string(),
                    range: range.to_string(),
                }
            } else {
                // Could be either Hash or Range — store as Hash since we can't tell
                FieldKey::Hash {
                    hash: conflict.key.clone(),
                }
            };

            // Store as mutation event in history
            let event = MutationEvent {
                molecule_uuid: mol_uuid.to_string(),
                timestamp: now,
                field_key,
                old_atom_uuid: Some(conflict.loser_atom.clone()),
                new_atom_uuid: conflict.winner_atom.clone(),
                version: 0,
                is_conflict: true,
                conflict_loser_atom: Some(conflict.loser_atom.clone()),
            };
            let event_key = format!("history:{}:{:020}", mol_uuid, ts_nanos);
            let event_bytes = serde_json::to_vec(&event)?;
            kv.put(event_key.as_bytes(), event_bytes).await?;

            // Also store in dedicated conflict index for efficient scanning
            let conflict_record = SyncConflict {
                id: format!("{}:{:020}", mol_uuid, ts_nanos),
                molecule_uuid: mol_uuid.to_string(),
                conflict_key: conflict.key.clone(),
                winner_atom: conflict.winner_atom.clone(),
                loser_atom: conflict.loser_atom.clone(),
                winner_written_at: conflict.winner_written_at,
                loser_written_at: conflict.loser_written_at,
                detected_at: now,
                resolved: false,
            };
            let conflict_key = format!("conflict:{}:{:020}", mol_uuid, ts_nanos);
            let conflict_bytes = serde_json::to_vec(&conflict_record)?;
            kv.put(conflict_key.as_bytes(), conflict_bytes).await?;
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

    // =========================================================================
    // Org sync configuration
    // =========================================================================

    /// Configure org sync partitioning.
    ///
    /// Configure org sync targets.
    ///
    /// Appends org targets after the personal target (index 0). Each target
    /// has its own R2 prefix and crypto provider. The partitioner classifies
    /// pending entries to the correct target.
    pub async fn configure_org_sync(
        &self,
        partitioner: SyncPartitioner,
        org_targets: Vec<SyncTarget>,
    ) {
        *self.partitioner.lock().await = Some(partitioner);
        let mut targets = self.targets.lock().await;
        targets.truncate(1); // Keep personal target
        targets.extend(org_targets);
    }

    /// Check if org sync is configured (any targets beyond personal).
    pub async fn has_org_sync(&self) -> bool {
        self.targets.lock().await.len() > 1
    }

    /// Classify a single log entry by examining its key.
    fn classify_entry(partitioner: &SyncPartitioner, entry: &LogEntry) -> SyncDestination {
        match &entry.op {
            LogOp::Put { namespace, key, .. } | LogOp::Delete { namespace, key } => {
                partitioner.partition_log_key(namespace, key)
            }
            LogOp::BatchPut {
                namespace, items, ..
            } => {
                if let Some((key, _)) = items.first() {
                    partitioner.partition_log_key(namespace, key)
                } else {
                    SyncDestination::Personal
                }
            }
            LogOp::BatchDelete {
                namespace, keys, ..
            } => {
                if let Some(key) = keys.first() {
                    partitioner.partition_log_key(namespace, key)
                } else {
                    SyncDestination::Personal
                }
            }
        }
    }
}

/// Parse a flat log key: `log/{seq}.enc`
fn parse_flat_log_key(key: &str) -> Option<u64> {
    let key = key.strip_prefix("log/")?;
    let seq_str = key.strip_suffix(".enc")?;
    seq_str.parse::<u64>().ok()
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

    #[test]
    fn test_parse_flat_log_key() {
        assert_eq!(parse_flat_log_key("log/42.enc"), Some(42));
        assert_eq!(parse_flat_log_key("log/1.enc"), Some(1));
        assert_eq!(parse_flat_log_key("log/999.enc"), Some(999));
        assert!(parse_flat_log_key("log/not_a_number.enc").is_none());
        assert!(parse_flat_log_key("single").is_none());
        assert!(parse_flat_log_key("").is_none());
    }

    #[test]
    fn test_classify_entry_personal() {
        use crate::org::OrgMembership;

        let memberships = vec![OrgMembership {
            org_name: "Test".to_string(),
            org_hash: "org_abc".to_string(),
            org_public_key: "pk".to_string(),
            org_secret_key: None,
            org_e2e_secret: "secret".to_string(),
            role: crate::org::OrgRole::Member,
            members: vec![],
            created_at: 0,
            joined_at: 0,
        }];
        let partitioner = SyncPartitioner::new(&memberships);

        let entry = LogEntry {
            seq: 1,
            timestamp_ms: 1000,
            device_id: "dev".to_string(),
            op: LogOp::Put {
                namespace: "main".to_string(),
                key: LogOp::encode_bytes(b"atom:uuid-1"),
                value: LogOp::encode_bytes(b"data"),
            },
        };

        assert_eq!(
            SyncEngine::classify_entry(&partitioner, &entry),
            SyncDestination::Personal
        );
    }

    #[test]
    fn test_classify_entry_org() {
        use crate::org::OrgMembership;

        let memberships = vec![OrgMembership {
            org_name: "Test".to_string(),
            org_hash: "org_abc".to_string(),
            org_public_key: "pk".to_string(),
            org_secret_key: None,
            org_e2e_secret: "secret".to_string(),
            role: crate::org::OrgRole::Member,
            members: vec![],
            created_at: 0,
            joined_at: 0,
        }];
        let partitioner = SyncPartitioner::new(&memberships);

        let entry = LogEntry {
            seq: 1,
            timestamp_ms: 1000,
            device_id: "dev".to_string(),
            op: LogOp::Put {
                namespace: "main".to_string(),
                key: LogOp::encode_bytes(b"org_abc:atom:uuid-1"),
                value: LogOp::encode_bytes(b"data"),
            },
        };

        assert_eq!(
            SyncEngine::classify_entry(&partitioner, &entry),
            SyncDestination::Org {
                org_hash: "org_abc".to_string(),
                org_e2e_secret: "secret".to_string(),
            }
        );
    }
}
