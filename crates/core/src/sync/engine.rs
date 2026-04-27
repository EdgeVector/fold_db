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
use crate::security::Ed25519KeyPair;
use crate::storage::traits::NamespacedStore;
use chrono::Utc;
use serde::Serialize;
use std::sync::Arc;
use tokio::sync::Mutex;

/// Maximum number of seq_numbers per presign request.
///
/// `storage_service` (the personal/org presign Lambda) hard-caps `seq_numbers`
/// (and the org `count`) at 1000 per call — see `lambdas/storage_service/
/// src/handlers.rs` (`presign_log_upload` / `presign_log_download` /
/// `presign_log_delete` arms). Any pending bucket larger than this must be
/// chunked here; otherwise the Lambda returns
/// `seq_numbers exceeds maximum length of 1000` (surfaced as `SyncError::Auth`)
/// and the entire bucket fails to upload.
///
/// Personal sync queues can blow past this easily on a fresh node — pulling
/// schemas, native_index entries, and a handful of mutations in the first
/// cycle is enough — so chunking is mandatory, not optional.
const MAX_PRESIGN_BATCH: usize = 1000;

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

/// Outcome of `SyncEngine::purge_personal_log`. Counts of cloud-side
/// objects deleted, returned to the caller for logging and progress
/// reporting.
#[derive(Debug, Default, Clone, Serialize)]
pub struct PurgeOutcome {
    /// Number of `{user_hash}/log/{seq}.enc` objects deleted.
    pub deleted_log_objects: usize,
    /// Number of `{user_hash}/snapshots/*.enc` objects deleted.
    pub deleted_snapshots: usize,
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
    /// Maximum pending entries before oldest are dropped. Prevents unbounded
    /// memory growth during long offline periods with active writes.
    /// 0 means unlimited (not recommended for production).
    pub max_pending: usize,
}

impl Default for SyncConfig {
    fn default() -> Self {
        Self {
            sync_interval_ms: 30_000,
            compaction_threshold: 100,
            lock_ttl_secs: 300,
            max_retries: 2,
            max_pending: 10_000,
        }
    }
}

/// Callback for sync status changes.
pub type StatusCallback = Box<dyn Fn(SyncState, Option<&str>) + Send + Sync>;

/// Unified merge interface for all molecule types.
///
/// Each molecule type has a `merge` method but with slightly different return types
/// (`Vec<MergeConflict>` vs `Option<MergeConflict>`). This trait normalizes them
/// into a single `Vec<MergeConflict>` so `try_merge` can be generic.
///
/// The `keypair` is the node signer plumbed through `SyncEngine`. For `Molecule`
/// (single-atom) merges it signs the resulting canonical bytes so a remote peer
/// can attribute the merged write to this node. Collection-typed merges
/// (`MoleculeHash`, `MoleculeRange`, `MoleculeHashRange`) preserve per-entry
/// signatures from the original writers, so the keypair is unused there.
trait MergeMolecule {
    fn merge_into_conflicts(
        &mut self,
        other: &Self,
        keypair: &Ed25519KeyPair,
    ) -> Vec<MergeConflict>;
}

impl MergeMolecule for MoleculeHash {
    fn merge_into_conflicts(
        &mut self,
        other: &Self,
        keypair: &Ed25519KeyPair,
    ) -> Vec<MergeConflict> {
        self.merge(other, keypair)
    }
}

impl MergeMolecule for MoleculeRange {
    fn merge_into_conflicts(
        &mut self,
        other: &Self,
        keypair: &Ed25519KeyPair,
    ) -> Vec<MergeConflict> {
        self.merge(other, keypair)
    }
}

impl MergeMolecule for MoleculeHashRange {
    fn merge_into_conflicts(
        &mut self,
        other: &Self,
        keypair: &Ed25519KeyPair,
    ) -> Vec<MergeConflict> {
        self.merge(other, keypair)
    }
}

impl MergeMolecule for Molecule {
    fn merge_into_conflicts(
        &mut self,
        other: &Self,
        keypair: &Ed25519KeyPair,
    ) -> Vec<MergeConflict> {
        self.merge(other, keypair).into_iter().collect()
    }
}

/// Async callback that reloads an in-memory cache from persistent storage.
/// Returns the number of newly added items, or an error string.
/// Used for both schema and embedding reloaders — same signature.
pub type ReloadCallback = Arc<
    dyn Fn() -> std::pin::Pin<Box<dyn std::future::Future<Output = Result<usize, String>> + Send>>
        + Send
        + Sync,
>;

/// Callback that reloads schemas from the persistent store into the in-memory cache.
pub type SchemaReloadCallback = ReloadCallback;

/// Callback that reloads embeddings from the persistent store into the in-memory index.
pub type EmbeddingReloadCallback = ReloadCallback;

/// Summary of a single `bootstrap_target` invocation.
///
/// Consumed by `bootstrap_all` to decide whether schema/embedding reloaders
/// need to fire after a multi-target restore, and by fold_db_node's
/// `bootstrap_from_cloud` flow to report what was restored.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct BootstrapOutcome {
    /// Highest sequence number restored (from snapshot + log replay).
    /// Zero if the target had no prior data.
    pub last_seq: u64,
    /// Count of log entries replayed after the snapshot.
    pub entries_replayed: usize,
    /// True if at least one replayed entry wrote to the `schemas` namespace.
    /// Used to decide whether the schema reloader should fire.
    pub schemas_replayed: bool,
    /// True if at least one replayed entry wrote to the `native_index`
    /// namespace. Used to decide whether the embedding reloader should fire.
    pub embeddings_replayed: bool,
}

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
    /// Optional callback invoked after sync replay writes native_index entries to Sled.
    /// This lets the EmbeddingIndex refresh without a hard dependency.
    embedding_reloader: Arc<Mutex<Option<EmbeddingReloadCallback>>>,
    /// Optional callback to refresh authentication credentials on 401.
    /// When set, the sync engine will call this on `SyncError::Auth`, update
    /// the `AuthClient`, and retry the sync cycle once before giving up.
    auth_refresh: Option<AuthRefreshCallback>,
    /// Wake handle notified by `record_op` whenever a new local write appends
    /// to the pending queue. The background sync coordinator races its next
    /// sleep against this notification so a write can trigger a near-immediate
    /// flush instead of waiting the full `sync_interval_ms`. A `Notify` holds
    /// at most one pending notification across multiple writes — concurrent
    /// writes coalesce into the next sync cycle naturally.
    wake: Arc<tokio::sync::Notify>,
    /// Node signing keypair, shared with `MutationManager`. Used to sign
    /// `Molecule` merge results during sync replay so a peer can attribute the
    /// merged write to this node's identity instead of an ephemeral keypair.
    node_signer: Arc<Ed25519KeyPair>,
}

impl SyncEngine {
    pub fn new(
        device_id: String,
        crypto: Arc<dyn CryptoProvider>,
        s3: S3Client,
        auth: AuthClient,
        store: Arc<dyn NamespacedStore>,
        config: SyncConfig,
        node_signer: Arc<Ed25519KeyPair>,
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
            embedding_reloader: Arc::new(Mutex::new(None)),
            auth_refresh: None,
            wake: Arc::new(tokio::sync::Notify::new()),
            node_signer,
        }
    }

    /// Handle to the wake notification. The background sync coordinator holds
    /// a clone and races its next sleep against `wake.notified()`, so local
    /// writes can trigger an immediate sync cycle instead of waiting out the
    /// full `sync_interval_ms`.
    pub fn wake_handle(&self) -> Arc<tokio::sync::Notify> {
        self.wake.clone()
    }

    /// Load persisted download cursors from storage.
    /// Called on startup to resume incremental downloads.
    pub async fn load_download_cursors(&self) {
        let kv = match self.store.open_namespace("sync_cursors").await {
            Ok(kv) => kv,
            Err(e) => {
                tracing::warn!("Failed to open sync_cursors namespace: {}", e);
                return;
            }
        };
        let entries = match kv.scan_prefix(b"cursor:").await {
            Ok(entries) => entries,
            Err(e) => {
                tracing::warn!("Failed to scan cursor keys: {}", e);
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
            tracing::info!("Loaded {} download cursors from storage", cursors.len());
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

    /// Register a callback that reloads the EmbeddingIndex after sync
    /// replays native_index entries into Sled. The callback returns the number
    /// of newly added embeddings, or an error string.
    pub async fn set_embedding_reloader(&self, reloader: EmbeddingReloadCallback) {
        *self.embedding_reloader.lock().await = Some(reloader);
    }

    /// Invoke a reload callback, logging the result. `kind` is a human label
    /// (e.g. "schema", "embedding") and `target_label` identifies the sync target.
    async fn invoke_reloader(
        &self,
        reloader_slot: &Mutex<Option<ReloadCallback>>,
        kind: &str,
        target_label: &str,
    ) {
        if let Some(reloader) = reloader_slot.lock().await.as_ref() {
            match reloader().await {
                Ok(count) if count > 0 => {
                    tracing::info!(
                        "{kind} reloader added {count} item(s) after sync from '{target_label}'"
                    );
                }
                Ok(_) => {}
                Err(e) => {
                    tracing::warn!("failed to reload {kind}s after sync: {e}");
                }
            }
        }
    }

    /// Get the device identifier.
    pub fn device_id(&self) -> &str {
        &self.device_id
    }

    /// Returns the node signing keypair used to sign merge results during replay.
    /// Shared with `MutationManager` so local writes and merged writes trace to
    /// the same node identity.
    pub fn node_signer(&self) -> &Arc<Ed25519KeyPair> {
        &self.node_signer
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

    /// Record an operation: create a log entry, push to pending, mark dirty.
    /// If the pending queue exceeds max_pending, drops the oldest entries to
    /// prevent unbounded memory growth during long offline periods.
    async fn record_op(&self, op: LogOp) {
        let entry = self.make_entry(op).await;
        let mut pending = self.pending.lock().await;
        pending.push(entry);
        let max = self.config.max_pending;
        if max > 0 && pending.len() > max {
            let overflow = pending.len() - max;
            // Dropping pending entries silently would cause permanent data
            // loss on peers — upgrade from warn to error so this is
            // unmissable in logs (alpha BLOCKER 4439b class of bug).
            tracing::error!(
                "SYNC DATA LOSS: pending queue exceeded max_pending ({}), dropping {} oldest entries — these writes will NOT reach sync peers",
                max,
                overflow
            );
            pending.drain(..overflow);
        }
        drop(pending);
        self.set_state(SyncState::Dirty, None).await;
        // Wake the background sync coordinator so a flush fires near-immediately
        // instead of waiting out the full `sync_interval_ms`. Safe under burst
        // writes: `Notify` holds at most one pending notification, so a rapid
        // sequence of writes coalesces into a single early wake-up, and the
        // subsequent sync cycle drains every pending entry in one batch.
        self.wake.notify_one();
    }

    /// Record a put operation for sync.
    pub async fn record_put(&self, namespace: &str, key: &[u8], value: &[u8]) {
        self.record_op(LogOp::Put {
            namespace: namespace.to_string(),
            key: LogOp::encode_bytes(key),
            value: LogOp::encode_bytes(value),
        })
        .await;
    }

    /// Record a delete operation for sync.
    pub async fn record_delete(&self, namespace: &str, key: &[u8]) {
        self.record_op(LogOp::Delete {
            namespace: namespace.to_string(),
            key: LogOp::encode_bytes(key),
        })
        .await;
    }

    /// Record a batch put operation for sync.
    pub async fn record_batch_put(&self, namespace: &str, items: &[(Vec<u8>, Vec<u8>)]) {
        let encoded_items: Vec<(String, String)> = items
            .iter()
            .map(|(k, v)| (LogOp::encode_bytes(k), LogOp::encode_bytes(v)))
            .collect();

        self.record_op(LogOp::BatchPut {
            namespace: namespace.to_string(),
            items: encoded_items,
        })
        .await;
    }

    /// Record a batch delete operation for sync.
    pub async fn record_batch_delete(&self, namespace: &str, keys: &[Vec<u8>]) {
        let encoded_keys: Vec<String> = keys.iter().map(|k| LogOp::encode_bytes(k)).collect();

        self.record_op(LogOp::BatchDelete {
            namespace: namespace.to_string(),
            keys: encoded_keys,
        })
        .await;
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

    /// Record a successful sync: update last_sync_at timestamp and clear last_error.
    async fn record_sync_success(&self) {
        let now = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap_or_default()
            .as_secs();
        *self.last_sync_at.lock().await = Some(now);
        *self.last_error.lock().await = None;
    }

    /// Record a sync failure: store the error message and transition state.
    async fn record_sync_failure(&self, err: &SyncError) {
        let msg = err.to_string();
        *self.last_error.lock().await = Some(msg.clone());
        let new_state = match err {
            SyncError::Network(_) => SyncState::Offline,
            _ => SyncState::Dirty,
        };
        self.set_state(new_state, Some(&msg)).await;
    }

    /// Run one sync cycle: upload pending log entries, compact if needed.
    ///
    /// Returns Ok(true) if all pending entries were uploaded,
    /// Ok(false) if there was nothing to sync.
    ///
    /// A cycle always runs when the engine is configured, even if the local
    /// state is `Idle`. Multi-device users have peers uploading to the shared
    /// `{user_hash}/log/` prefix, and a passive reader needs to poll
    /// downloads to pick those up. Previously the engine returned early when
    /// `Idle && !has_orgs` — that blocked personal multi-device sync
    /// completely. The only early exits now are "already syncing" (to avoid
    /// overlapping cycles) and an empty targets list (no sync configured).
    pub async fn sync(&self) -> SyncResult<bool> {
        // Atomic check-and-set: hold the lock for both the state check and transition
        {
            let mut state = self.state.lock().await;
            if *state == SyncState::Syncing {
                tracing::info!("sync skipped: already syncing");
                return Ok(false);
            }
            *state = SyncState::Syncing;
        }
        if let Some(cb) = &self.status_callback {
            cb(SyncState::Syncing, Some("uploading"));
        }

        // Acquire the write lock before uploading. This coordinates with other
        // devices so only one uploads at a time (prevents wasted presign URLs
        // and duplicate sequence assignment). Lock failures are non-fatal —
        // data integrity is guaranteed by the append-only log with nanosecond
        // keys, not by the lock.
        let lock_held = match self.acquire_lock().await {
            Ok(()) => true,
            Err(SyncError::Auth(_)) => false, // auth error — will be caught by do_sync
            Err(e) => {
                tracing::warn!("failed to acquire sync lock (proceeding anyway): {e}");
                false
            }
        };

        // Try do_sync; on auth error, attempt token refresh and retry once.
        let result = match self.do_sync().await {
            Ok(synced) => Ok(synced),
            Err(ref e) if matches!(e, SyncError::Auth(_)) => self.try_refresh_and_retry().await,
            Err(e) => Err(e),
        };

        // Always release the lock, even on error
        if lock_held {
            if let Err(e) = self.release_lock().await {
                tracing::warn!("failed to release sync lock: {e}");
            }
        }

        match result {
            Ok(synced) => {
                if synced {
                    self.record_sync_success().await;
                }
                self.set_state(SyncState::Idle, None).await;
                Ok(synced)
            }
            Err(e) => {
                self.record_sync_failure(&e).await;
                Err(e)
            }
        }
    }

    /// Attempt to refresh auth credentials and retry do_sync once.
    /// Falls through to the original auth error if refresh isn't available or fails.
    async fn try_refresh_and_retry(&self) -> SyncResult<bool> {
        self.refresh_auth_once("sync").await?;
        self.do_sync().await
    }

    /// Invoke the auth-refresh callback (if any) and update the shared
    /// `AuthClient` with the new credential. Errors if no callback is wired
    /// or the callback itself fails.
    ///
    /// `context` is a short label used only in log messages so on-demand
    /// paths (snapshot backup, restore) can be distinguished from the
    /// periodic sync cycle in logs.
    async fn refresh_auth_once(&self, context: &str) -> SyncResult<()> {
        let refresh_cb = match self.auth_refresh {
            Some(ref cb) => cb.clone(),
            None => return Err(SyncError::Auth("authentication failed".to_string())),
        };

        tracing::info!("{context} auth failed, attempting token refresh");
        let new_auth = refresh_cb().await.map_err(|e| {
            tracing::warn!("{context} token refresh failed: {e}");
            SyncError::Auth("authentication failed after token refresh failure".to_string())
        })?;

        self.auth.update_auth(new_auth).await;
        tracing::info!("{context} token refreshed");
        Ok(())
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

            // Partition entries across targets by key prefix.
            // Batches with mixed-prefix keys are split into one sub-entry per target
            // so each chunk is sealed under the correct crypto provider.
            let mut buckets: std::collections::HashMap<usize, Vec<LogEntry>> =
                std::collections::HashMap::new();
            for entry in &entries {
                for (idx, sub_entry) in Self::partition_entry(&partitioner, entry, &targets) {
                    buckets.entry(idx).or_default().push(sub_entry);
                }
            }
            drop(partitioner);

            // Upload each bucket with its target's crypto
            let mut upload_auth_error = false;
            for (target_idx, bucket) in &buckets {
                tracing::info!(
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
                    Err(ref e) if matches!(e, SyncError::Auth(_)) => {
                        tracing::warn!("upload to '{}' failed (auth): {e}", target.label);
                        upload_auth_error = true;
                    }
                    Err(e) => tracing::warn!("upload to '{}' failed: {e}", target.label),
                }
            }

            // Propagate auth errors so the top-level sync() can refresh and retry
            if upload_auth_error {
                return Err(SyncError::Auth(
                    "upload failed due to auth error".to_string(),
                ));
            }

            // Clear uploaded entries from pending.
            // Only drain if ALL targets succeeded (uploaded == total entries).
            // If some targets failed, keep all entries so they retry next cycle.
            if uploaded >= entries.len() {
                let mut pending = self.pending.lock().await;
                let count = entries.len().min(pending.len());
                pending.drain(..count);
            } else if uploaded > 0 {
                tracing::warn!(
                    "partial upload: {}/{} entries succeeded, keeping all in pending for retry",
                    uploaded,
                    entries.len()
                );
            }
        }

        // Download from every configured target — personal included. Personal
        // multi-device users have peers (other devices restored from the same
        // mnemonic) writing to the shared `{user_hash}/log/` prefix, so we
        // need to poll that prefix on every cycle, not just the org ones.
        // Skipping personal broke cross-device convergence for passive
        // readers: Node A would never see Node B's writes until Node A wrote
        // something locally.
        for target in &targets {
            match self.download_with_auth_retry(target).await {
                Ok(n) => downloaded += n,
                Err(e) => tracing::warn!("download from '{}' failed: {e}", target.label),
            }
        }

        // Compaction: snapshot personal data periodically
        if uploaded >= self.config.compaction_threshold as usize {
            let current_seq = *self.seq.lock().await;
            if current_seq > 0 {
                if let Err(e) = self.compact(current_seq).await {
                    tracing::warn!("compaction failed (non-fatal): {e}");
                }
            }
        }

        Ok(uploaded > 0 || downloaded > 0)
    }

    /// Resolve a `SyncDestination` to a target index in `targets`.
    ///
    /// Returns 0 (personal) if the destination is `Personal` or no org target
    /// with a matching prefix is configured.
    fn destination_to_target_idx(dest: &SyncDestination, targets: &[SyncTarget]) -> usize {
        if let SyncDestination::Org { org_hash, .. } = dest {
            for (i, t) in targets.iter().enumerate() {
                if !t.prefix.is_empty() && t.prefix == *org_hash {
                    return i;
                }
            }
        }
        0
    }

    /// Partition a single pending entry into one or more (target_idx, sub_entry)
    /// pairs.
    ///
    /// For `Put` / `Delete`, this returns exactly one pair, with the original
    /// entry unchanged. For `BatchPut` / `BatchDelete`, items are grouped by
    /// target index — homogeneous batches still produce a single pair with the
    /// original batch intact (no allocation of new items). Mixed-prefix batches
    /// are split into one sub-batch per target, each sealed under the correct
    /// crypto provider. All sub-entries share the original `seq`, `timestamp_ms`,
    /// and `device_id`; seq ordering / dedupe tracking is preserved because the
    /// original log position is unchanged.
    fn partition_entry(
        partitioner: &Option<SyncPartitioner>,
        entry: &LogEntry,
        targets: &[SyncTarget],
    ) -> Vec<(usize, LogEntry)> {
        let Some(p) = partitioner else {
            return vec![(0, entry.clone())];
        };

        match &entry.op {
            LogOp::Put { namespace, key, .. } | LogOp::Delete { namespace, key } => {
                let dest = p.partition_log_key(namespace, key);
                let idx = Self::destination_to_target_idx(&dest, targets);
                vec![(idx, entry.clone())]
            }
            LogOp::BatchPut { namespace, items } => {
                let mut by_target: std::collections::BTreeMap<usize, Vec<(String, String)>> =
                    std::collections::BTreeMap::new();
                for (k, v) in items {
                    let dest = p.partition_log_key(namespace, k);
                    let idx = Self::destination_to_target_idx(&dest, targets);
                    by_target
                        .entry(idx)
                        .or_default()
                        .push((k.clone(), v.clone()));
                }
                if by_target.len() == 1 {
                    // Homogeneous: return the original batch intact, no clone of items.
                    let idx = *by_target.keys().next().expect("len == 1");
                    return vec![(idx, entry.clone())];
                }
                by_target
                    .into_iter()
                    .map(|(idx, sub_items)| {
                        let sub_entry = LogEntry {
                            seq: entry.seq,
                            timestamp_ms: entry.timestamp_ms,
                            device_id: entry.device_id.clone(),
                            op: LogOp::BatchPut {
                                namespace: namespace.clone(),
                                items: sub_items,
                            },
                        };
                        (idx, sub_entry)
                    })
                    .collect()
            }
            LogOp::BatchDelete { namespace, keys } => {
                let mut by_target: std::collections::BTreeMap<usize, Vec<String>> =
                    std::collections::BTreeMap::new();
                for k in keys {
                    let dest = p.partition_log_key(namespace, k);
                    let idx = Self::destination_to_target_idx(&dest, targets);
                    by_target.entry(idx).or_default().push(k.clone());
                }
                if by_target.len() == 1 {
                    let idx = *by_target.keys().next().expect("len == 1");
                    return vec![(idx, entry.clone())];
                }
                by_target
                    .into_iter()
                    .map(|(idx, sub_keys)| {
                        let sub_entry = LogEntry {
                            seq: entry.seq,
                            timestamp_ms: entry.timestamp_ms,
                            device_id: entry.device_id.clone(),
                            op: LogOp::BatchDelete {
                                namespace: namespace.clone(),
                                keys: sub_keys,
                            },
                        };
                        (idx, sub_entry)
                    })
                    .collect()
            }
        }
    }

    /// Retry an S3 operation with exponential backoff.
    /// Auth errors are NOT retried (they need token refresh at a higher level).
    async fn retry_s3<F, Fut, T>(&self, label: &str, mut op: F) -> SyncResult<T>
    where
        F: FnMut() -> Fut,
        Fut: std::future::Future<Output = SyncResult<T>>,
    {
        let max_retries = self.config.max_retries;
        for attempt in 0..max_retries {
            match op().await {
                Ok(v) => return Ok(v),
                Err(e) if matches!(&e, SyncError::Auth(_)) => return Err(e),
                Err(e) => {
                    let delay_ms = 500 * 2u64.pow(attempt);
                    tracing::warn!(
                        "{}: attempt {}/{} failed ({}), retrying in {}ms",
                        label,
                        attempt + 1,
                        max_retries + 1,
                        e,
                        delay_ms
                    );
                    tokio::time::sleep(std::time::Duration::from_millis(delay_ms)).await;
                }
            }
        }
        // Final attempt — no retry, just propagate
        op().await
    }

    /// Upload entries to a single sync target.
    ///
    /// Personal targets (empty prefix) upload under each entry's own
    /// client-assigned nanosecond `entry.seq`. Org targets let the server
    /// atomically allocate a contiguous block of seqs via
    /// `presign_upload_alloc`; each entry's `seq` is rewritten to its
    /// server-assigned value before sealing so the S3 key, the sealed
    /// payload, and the downloader's parsed seq all agree.
    ///
    /// Chunked at `MAX_PRESIGN_BATCH` to respect storage_service's per-request
    /// cap on `seq_numbers`. Earlier successful chunks are counted toward the
    /// returned `usize` even when a later chunk fails — `do_sync` uses that
    /// count to decide whether to drain pending or keep everything for retry,
    /// and reporting fewer than expected is the correct signal there. Each
    /// chunk's presign + seal + S3 PUTs happen as one logical step;
    /// retries across chunks are safe because S3 PUTs at the same seq key
    /// overwrite idempotently.
    async fn upload_entries(&self, target: &SyncTarget, entries: &[LogEntry]) -> SyncResult<usize> {
        if entries.is_empty() {
            return Ok(0);
        }
        let mut total = 0;
        for chunk in entries.chunks(MAX_PRESIGN_BATCH) {
            match self.upload_entries_chunk(target, chunk).await {
                Ok(n) => total += n,
                Err(e) => {
                    if total > 0 {
                        tracing::warn!(
                            "upload to '{}' partial: {}/{} entries uploaded across chunks before {}",
                            target.label,
                            total,
                            entries.len(),
                            e
                        );
                    }
                    return Err(e);
                }
            }
        }
        Ok(total)
    }

    /// Upload a single chunk of entries (≤ `MAX_PRESIGN_BATCH`) to a target.
    /// Caller is responsible for chunking; this is the original single-batch
    /// presign + seal + S3 PUT path, factored out so `upload_entries` can loop.
    async fn upload_entries_chunk(
        &self,
        target: &SyncTarget,
        entries: &[LogEntry],
    ) -> SyncResult<usize> {
        if entries.is_empty() {
            return Ok(0);
        }
        debug_assert!(
            entries.len() <= MAX_PRESIGN_BATCH,
            "upload_entries_chunk called with {} entries (cap {})",
            entries.len(),
            MAX_PRESIGN_BATCH
        );

        let is_org = !target.prefix.is_empty();

        let (sealed, urls): (
            Vec<(u64, super::log::SealedLogEntry)>,
            Vec<super::s3::PresignedUrl>,
        ) = if is_org {
            let pairs = self
                .auth
                .presign_upload_alloc(target, entries.len() as u32)
                .await?;
            if pairs.len() != entries.len() {
                return Err(SyncError::Auth(format!(
                    "expected {} server-assigned seqs for '{}', got {}",
                    entries.len(),
                    target.label,
                    pairs.len(),
                )));
            }
            let mut sealed = Vec::with_capacity(entries.len());
            let mut urls = Vec::with_capacity(entries.len());
            for (entry, (server_seq, url)) in entries.iter().zip(pairs) {
                let mut rewritten = entry.clone();
                rewritten.seq = server_seq;
                let s = rewritten.seal(&target.crypto).await?;
                sealed.push((server_seq, s));
                urls.push(url);
            }
            (sealed, urls)
        } else {
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
            (sealed, urls)
        };

        let mut uploaded_count = 0;
        for ((seq, s), url) in sealed.into_iter().zip(urls.iter()) {
            let url = url.clone();
            let s3 = &self.s3;
            let bytes = s.bytes;
            self.retry_s3(&format!("upload seq {}", seq), || {
                let url = url.clone();
                let bytes = bytes.clone();
                async move { s3.upload(&url, bytes).await }
            })
            .await?;
            uploaded_count += 1;
        }

        Ok(uploaded_count)
    }

    /// Download entries from a target, refreshing auth once on 401.
    async fn download_with_auth_retry(&self, target: &SyncTarget) -> SyncResult<u64> {
        match self.download_entries(target).await {
            Ok(n) => Ok(n),
            Err(ref e) if matches!(e, SyncError::Auth(_)) => {
                if let Some(ref refresh_cb) = self.auth_refresh {
                    tracing::info!(
                        "org download from '{}' auth failed, refreshing",
                        target.label
                    );
                    if let Ok(new_auth) = refresh_cb().await {
                        self.auth.update_auth(new_auth).await;
                        return self.download_entries(target).await;
                    }
                }
                Err(SyncError::Auth(format!(
                    "download from '{}' failed after auth refresh",
                    target.label
                )))
            }
            Err(e) => Err(e),
        }
    }

    /// Download new entries from a sync target.
    ///
    /// Lists `/{prefix}/log/{seq}.enc`, downloads, unseals with the target's
    /// crypto, and replays.
    ///
    /// ### Why we list the full log prefix (no S3 `start_after`)
    ///
    /// S3 `start_after` orders keys **lexicographically**, not numerically.
    /// Our keys use unpadded decimal seqs (`log/52.enc`, `log/100.enc`), so
    /// `log/100.enc` lex-sorts *before* `log/52.enc` (because `'1' < '5'`).
    /// Using the local numeric cursor as a lex `start_after` would silently
    /// hide every key whose seq crosses a digit-length boundary under the
    /// cursor's lex prefix — e.g., cursor=52 hides seqs 100..104 permanently.
    /// That is alpha BLOCKER 30a7b: Alice uploads 104 entries, Bob's second
    /// poll sees `start_after=log/52.enc` and misses seqs 100..104.
    ///
    /// The server-side `list_objects_v2` handler already paginates via
    /// `continuation_token`, so a full prefix list is bounded by the log's
    /// current size (which compaction keeps small). We filter `seq > cursor`
    /// client-side and sort numerically to preserve replay order.
    async fn download_entries(&self, target: &SyncTarget) -> SyncResult<u64> {
        let cursor = {
            let cursors = self.download_cursors.lock().await;
            cursors.get(&target.prefix).copied().unwrap_or(0)
        };

        let objects = self.auth.list_log_objects(target).await?;

        // Parse flat log keys: log/{seq}.enc. Filter numerically — see the
        // method docstring for why we cannot use S3 `start_after` against
        // unpadded decimal seq keys.
        let mut new_seqs: Vec<u64> = objects
            .iter()
            .filter_map(|obj| parse_flat_log_key(&obj.key))
            .filter(|s| *s > cursor)
            .collect();
        new_seqs.sort();

        // Loud-failure invariant: for ORG prefixes, the set of new seqs must
        // be contiguous (no holes between the smallest and largest) because
        // org seqs are server-allocated in atomic blocks. A hole there means
        // S3 returned a partial view of the log — the exact silent-drop
        // pattern 30a7b produced — and would cause `max_contiguous_seq` to
        // advance past a seq that later appears.
        //
        // PERSONAL prefixes (empty prefix) use client-assigned nanosecond
        // timestamps, so the seq space is inherently sparse: a device writes
        // at T1, then again at T1+100000, and the listing has "gaps" of
        // hundreds of thousands of unused seq values between them. The
        // contiguity check can't distinguish natural sparseness from S3
        // dropping a seq, so it's skipped for personal and we trust the
        // listing's completeness (typical path) plus cursor-replay semantics
        // (missed seqs re-appear in future listings because cursor only
        // advances to seqs we saw). See `designs/unified_sync.md` for the
        // personal-vs-org split rationale.
        let is_org_prefix = !target.prefix.is_empty();
        if is_org_prefix {
            if let (Some(&first), Some(&last)) = (new_seqs.first(), new_seqs.last()) {
                let expected = (last - first + 1) as usize;
                if new_seqs.len() != expected {
                    let set: std::collections::BTreeSet<u64> = new_seqs.iter().copied().collect();
                    let missing: Vec<u64> = (first..=last).filter(|s| !set.contains(s)).collect();
                    tracing::error!(
                        "sync list '{}' returned non-contiguous seqs after cursor={}: first={} last={} count={} expected={} missing_sample={:?}",
                        target.label,
                        cursor,
                        first,
                        last,
                        new_seqs.len(),
                        expected,
                        missing.iter().take(16).collect::<Vec<_>>()
                    );
                    return Err(SyncError::S3(format!(
                        "non-contiguous log listing for '{}': cursor={} got {} seqs in range {}..={} (expected {})",
                        target.label,
                        cursor,
                        new_seqs.len(),
                        first,
                        last,
                        expected
                    )));
                }
            }
        }

        if new_seqs.is_empty() {
            tracing::info!(
                "download '{}': 0 new entries (cursor={})",
                target.label,
                cursor
            );
            return Ok(0);
        }

        tracing::info!(
            "download '{}': {} new entries after cursor={}",
            target.label,
            new_seqs.len(),
            cursor
        );

        let mut total_replayed = 0u64;
        // Advance the cursor contiguously: if any seq in this batch fails, we
        // stop before it so the next cycle re-downloads it. Silent drops (where
        // the cursor skips past a failed entry) caused alpha BLOCKER 4439b.
        let mut max_contiguous_seq = cursor;
        let mut schemas_replayed = false;
        let mut embeddings_replayed = false;

        // Chunk the presign call at MAX_PRESIGN_BATCH — storage_service caps
        // `seq_numbers` per request and a fresh node can easily list >1000
        // new objects on first sync. Chunks are processed in seq order so the
        // contiguous-cursor invariant still holds: if chunk N fails partway,
        // chunks 1..N-1 fully replayed and `max_contiguous_seq` reflects that.
        for chunk in new_seqs.chunks(MAX_PRESIGN_BATCH) {
            let urls = self.auth.presign_download(target, chunk).await?;
            if urls.len() != chunk.len() {
                return Err(SyncError::Auth(format!(
                    "presign_download '{}': expected {} urls, got {}",
                    target.label,
                    chunk.len(),
                    urls.len()
                )));
            }
            for (seq, url) in chunk.iter().zip(urls.iter()) {
                let downloaded = self
                    .retry_s3(&format!("download '{}' seq {}", target.label, seq), || {
                        let url = url.clone();
                        async move { self.s3.download(&url).await }
                    })
                    .await?;
                let bytes = downloaded.ok_or_else(|| {
                    tracing::error!(
                        "sync replay aborted: entry not found in '{}' seq={} (server listed this seq but S3 returned 404); cursor will NOT advance past seq={}",
                        target.label,
                        seq,
                        max_contiguous_seq
                    );
                    SyncError::CorruptEntry {
                        seq: *seq,
                        reason: format!(
                            "object missing from '{}' after server listed it",
                            target.label
                        ),
                    }
                })?;
                let entry = LogEntry::unseal(&bytes, &target.crypto).await.map_err(|e| {
                    tracing::error!(
                        "sync replay aborted: failed to unseal entry in '{}' seq={}: {}; cursor will NOT advance past seq={}",
                        target.label,
                        seq,
                        e,
                        max_contiguous_seq
                    );
                    SyncError::CorruptEntry {
                        seq: *seq,
                        reason: format!("unseal failed: {e}"),
                    }
                })?;
                // `schema_states` shares the schema reloader: replaying a pure
                // state flip on an org schema (approve/block) must still refresh
                // the in-memory state cache, otherwise `/api/schemas` lags the
                // on-disk truth until the next schemas-namespace write lands.
                match entry.op.namespace() {
                    "schemas" | "schema_states" => schemas_replayed = true,
                    "native_index" => embeddings_replayed = true,
                    _ => {}
                }
                tracing::info!(
                    "replay '{}' seq={}: {}",
                    target.label,
                    seq,
                    entry.op.describe()
                );
                self.replay_entry(&entry, Some(target)).await.map_err(|e| {
                    tracing::error!(
                        "sync replay aborted: apply failed in '{}' seq={}: {}; cursor will NOT advance past seq={}",
                        target.label,
                        seq,
                        e,
                        max_contiguous_seq
                    );
                    e
                })?;
                total_replayed += 1;
                max_contiguous_seq = *seq;
            }
        }

        // Invoke reloaders for any namespaces that received new entries
        if schemas_replayed {
            self.invoke_reloader(&self.schema_reloader, "schema", &target.label)
                .await;
        }
        if embeddings_replayed {
            self.invoke_reloader(&self.embedding_reloader, "embedding", &target.label)
                .await;
        }

        // Update cursor to the highest contiguously-replayed seq. We never
        // advance past a seq that failed to replay — the next sync cycle
        // will retry from there.
        if max_contiguous_seq > cursor {
            let mut cursors = self.download_cursors.lock().await;
            cursors.insert(target.prefix.clone(), max_contiguous_seq);
            drop(cursors);
            self.save_download_cursor(&target.prefix, max_contiguous_seq)
                .await;
        }

        Ok(total_replayed)
    }

    /// Persist a download cursor to Sled.
    async fn save_download_cursor(&self, prefix: &str, seq: u64) {
        let cursor_key = format!("cursor:{}", prefix);
        match self.store.open_namespace("sync_cursors").await {
            Ok(kv) => {
                if let Err(e) = kv
                    .put(cursor_key.as_bytes(), seq.to_be_bytes().to_vec())
                    .await
                {
                    tracing::error!(
                        "failed to save download cursor for '{}' at seq {}: {} — next restart will re-download from last saved cursor",
                        prefix, seq, e
                    );
                }
            }
            Err(e) => {
                tracing::error!(
                    "failed to open sync_cursors namespace for '{}': {} — cursor not persisted",
                    prefix,
                    e
                );
            }
        }
    }

    // =========================================================================
    // Compaction (snapshot + delete old logs)
    // =========================================================================

    async fn compact(&self, last_seq: u64) -> SyncResult<()> {
        tracing::info!("compacting: creating snapshot at seq {last_seq}");

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
                    let mut deleted = 0usize;
                    let mut presign_err: Option<SyncError> = None;
                    for chunk in old_seqs.chunks(MAX_PRESIGN_BATCH) {
                        match self.auth.presign_log_delete(chunk).await {
                            Ok(delete_urls) => {
                                for url in &delete_urls {
                                    if let Err(e) = self.s3.delete(url).await {
                                        tracing::warn!(
                                            "failed to delete compacted log (non-fatal): {e}"
                                        );
                                    }
                                }
                                deleted += delete_urls.len();
                            }
                            Err(e) => {
                                presign_err = Some(e);
                                break;
                            }
                        }
                    }
                    if let Some(e) = presign_err {
                        tracing::warn!(
                            "failed to get delete URLs for compacted logs (non-fatal): {e}"
                        );
                    }
                    if deleted > 0 {
                        tracing::info!("deleted {} compacted log entries", deleted);
                    }
                }
            }
            Err(e) => {
                tracing::warn!("failed to list logs for compaction cleanup (non-fatal): {e}");
            }
        }

        tracing::info!("compaction complete: snapshot at seq {last_seq}");
        Ok(())
    }

    // =========================================================================
    // Bootstrap (download snapshot + replay logs)
    // =========================================================================

    /// Bootstrap this device from S3 (personal target only).
    ///
    /// Downloads the latest snapshot, restores it to the local store,
    /// then replays any log entries after the snapshot's sequence number.
    ///
    /// This is a thin shim over `bootstrap_target(0)` preserved for
    /// backward compatibility with existing callers. New callers that
    /// need multi-target restore (personal + orgs) should use
    /// `bootstrap_all` instead.
    pub async fn bootstrap(&self) -> SyncResult<u64> {
        let outcome = self.bootstrap_target(0).await?;
        // Preserve original behavior: personal bootstrap also fires the
        // schema reloader once if any schemas were replayed. `bootstrap_all`
        // handles this centrally; the single-target shim does it here so
        // the classic `bootstrap()` path still refreshes the SchemaCore.
        if outcome.schemas_replayed {
            self.invoke_reloader(&self.schema_reloader, "schema", "personal")
                .await;
        }
        if outcome.embeddings_replayed {
            self.invoke_reloader(&self.embedding_reloader, "embedding", "personal")
                .await;
        }
        Ok(outcome.last_seq)
    }

    /// Bootstrap a single sync target by index into `self.targets`.
    ///
    /// For the personal target (idx == 0), downloads `latest.enc`, restores
    /// the snapshot, and replays any log entries after the snapshot's
    /// sequence number. For org targets (idx > 0), snapshots are not yet
    /// supported by the storage service, so only log replay is performed
    /// starting from seq 0 using the target's own crypto provider.
    ///
    /// Returns a `BootstrapOutcome` describing what was replayed. If
    /// `latest.enc` does not exist (new prefix), returns an outcome with
    /// `last_seq = 0` and no entries replayed — not an error.
    ///
    /// This method does NOT invoke schema/embedding reloaders. Callers that
    /// need cache refresh should either use `bootstrap` (single target) or
    /// `bootstrap_all` (multi target), which handle reloader dispatch.
    pub async fn bootstrap_target(&self, idx: usize) -> SyncResult<BootstrapOutcome> {
        let target = {
            let targets = self.targets.lock().await;
            if idx >= targets.len() {
                return Err(SyncError::Storage(format!(
                    "bootstrap_target: index {} out of range (have {} targets)",
                    idx,
                    targets.len()
                )));
            }
            targets[idx].clone()
        };

        tracing::info!(
            "bootstrapping target '{}' (idx={}, prefix='{}')",
            target.label,
            idx,
            target.prefix
        );

        // Snapshot restore is only supported for the personal target today.
        // The storage service's presign_snapshot_download endpoint does not
        // accept an org prefix. Org targets start from seq 0 and replay all
        // log entries.
        let snapshot_last_seq = if idx == 0 {
            let snapshot_url = self.auth.presign_snapshot_download("latest.enc").await?;
            let snapshot_data = self.s3.download(&snapshot_url).await?;
            match snapshot_data {
                Some(data) => {
                    let snapshot = Snapshot::unseal(&data, &target.crypto).await?;
                    let last_seq = snapshot.last_seq;
                    tracing::info!(
                        "restoring snapshot for '{}': {} namespaces, last_seq={}",
                        target.label,
                        snapshot.namespaces.len(),
                        last_seq
                    );
                    snapshot.restore(self.store.as_ref()).await?;
                    last_seq
                }
                None => {
                    tracing::info!("no snapshot found for '{}' — starting fresh", target.label);
                    0
                }
            }
        } else {
            0
        };

        // List and replay log entries after the snapshot for this target.
        let log_objects = self.auth.list_log_objects(&target).await?;
        let mut log_seqs: Vec<u64> = log_objects
            .iter()
            .filter_map(|obj| parse_flat_log_key(&obj.key))
            .filter(|seq| *seq > snapshot_last_seq)
            .collect();
        log_seqs.sort();

        let mut schemas_replayed = false;
        let mut embeddings_replayed = false;
        let mut entries_replayed: usize = 0;

        if !log_seqs.is_empty() {
            tracing::info!(
                "replaying {} log entries for '{}' (seq {}..={})",
                log_seqs.len(),
                target.label,
                log_seqs[0],
                log_seqs[log_seqs.len() - 1]
            );

            let urls = self.auth.presign_download(&target, &log_seqs).await?;

            for (seq, url) in log_seqs.iter().zip(urls.iter()) {
                let data = self.s3.download(url).await?;
                let bytes = data.ok_or_else(|| {
                    tracing::error!(
                        "bootstrap aborted: log entry for '{}' seq={seq} not found in S3 after listing",
                        target.label
                    );
                    SyncError::CorruptEntry {
                        seq: *seq,
                        reason: format!(
                            "object missing from '{}' after bootstrap listed it",
                            target.label
                        ),
                    }
                })?;
                let entry = LogEntry::unseal(&bytes, &target.crypto)
                    .await
                    .map_err(|e| {
                        tracing::error!(
                            "bootstrap aborted: failed to unseal log entry for '{}' seq={seq}: {e}",
                            target.label
                        );
                        SyncError::CorruptEntry {
                            seq: *seq,
                            reason: format!("unseal failed: {e}"),
                        }
                    })?;
                // `schema_states` shares the schema reloader: see
                // `download_entries` for rationale.
                match entry.op.namespace() {
                    "schemas" | "schema_states" => schemas_replayed = true,
                    "native_index" => embeddings_replayed = true,
                    _ => {}
                }
                tracing::info!(
                    "bootstrap replay '{}' seq={}: {}",
                    target.label,
                    seq,
                    entry.op.describe()
                );
                self.replay_entry(&entry, Some(&target)).await?;
                entries_replayed += 1;
            }
        }

        // Bootstrap replayed every listed seq contiguously (we return Err on
        // any failure), so `last_seq` is safe to advance to the max listed.
        let last_seq = log_seqs.last().copied().unwrap_or(snapshot_last_seq);

        // Advance the local sequence counter only from the personal target.
        // Org targets write to their own R2 prefix and must not rewind the
        // personal counter used for upload sequencing.
        if idx == 0 {
            *self.seq.lock().await = last_seq;
        }

        // Also update this target's download cursor so subsequent sync cycles
        // don't re-download log entries we already replayed.
        if last_seq > snapshot_last_seq {
            {
                let mut cursors = self.download_cursors.lock().await;
                cursors.insert(target.prefix.clone(), last_seq);
            }
            self.save_download_cursor(&target.prefix, last_seq).await;
        }

        tracing::info!(
            "bootstrap of '{}' complete at seq {} ({} entries replayed)",
            target.label,
            last_seq,
            entries_replayed
        );

        Ok(BootstrapOutcome {
            last_seq,
            entries_replayed,
            schemas_replayed,
            embeddings_replayed,
        })
    }

    /// Bootstrap all configured sync targets (personal + orgs).
    ///
    /// Iterates `self.targets` in order and calls `bootstrap_target(idx)` for
    /// each. Fails fast: if any target errors, aborts immediately and returns
    /// `Err` with context identifying which target failed — subsequent
    /// targets are NOT invoked. Partial success is not useful in the restore
    /// case.
    ///
    /// After all targets succeed, invokes the schema reloader ONCE if any
    /// outcome reported schema replays, and the embedding reloader ONCE if
    /// any outcome reported embedding replays. This avoids redundant
    /// SchemaCore/EmbeddingIndex refreshes when many targets restore in
    /// sequence.
    ///
    /// Callers are responsible for configuring org targets (via
    /// `configure_org_sync`) before invoking this method.
    pub async fn bootstrap_all(&self) -> SyncResult<Vec<BootstrapOutcome>> {
        // Snapshot target count and release the lock before iterating so
        // per-target calls can reacquire it.
        let target_count = self.targets.lock().await.len();
        tracing::info!("bootstrap_all: starting restore of {target_count} target(s)");

        let mut outcomes: Vec<BootstrapOutcome> = Vec::with_capacity(target_count);
        for idx in 0..target_count {
            match self.bootstrap_target(idx).await {
                Ok(outcome) => outcomes.push(outcome),
                Err(e) => {
                    // Recover the failed target's label for context. We
                    // re-lock here because the per-target call has already
                    // returned.
                    let label = self
                        .targets
                        .lock()
                        .await
                        .get(idx)
                        .map(|t| t.label.clone())
                        .unwrap_or_else(|| format!("idx={idx}"));
                    return Err(SyncError::Storage(format!(
                        "bootstrap_all: target '{label}' (idx={idx}) failed: {e}"
                    )));
                }
            }
        }

        // Fire reloaders once at the end, only if any target reported the
        // corresponding namespace. Fixes the G2 gap where restored schemas
        // would otherwise remain stale in the SchemaCore cache.
        if outcomes.iter().any(|o| o.schemas_replayed) {
            self.invoke_reloader(&self.schema_reloader, "schema", "bootstrap_all")
                .await;
        }
        if outcomes.iter().any(|o| o.embeddings_replayed) {
            self.invoke_reloader(&self.embedding_reloader, "embedding", "bootstrap_all")
                .await;
        }

        tracing::info!(
            "bootstrap_all: completed {} target(s) successfully",
            outcomes.len()
        );
        Ok(outcomes)
    }

    /// Replay a single log entry with convergent ref handling.
    ///
    /// Non-ref keys (atoms, history) are written unconditionally.
    /// Ref keys (`ref:` or `{org_hash}:ref:`) use molecule merge so all
    /// nodes converge to the same state regardless of replay order.
    pub async fn replay_entry(
        &self,
        entry: &LogEntry,
        target: Option<&SyncTarget>,
    ) -> SyncResult<()> {
        match &entry.op {
            LogOp::Put {
                namespace,
                key,
                value,
            } => {
                let final_key = Self::rewrite_key_if_needed(namespace, key, target)?;
                self.replay_put(
                    namespace,
                    &LogOp::encode_bytes(&final_key),
                    value,
                    entry.timestamp_ms,
                    &entry.device_id,
                )
                .await?;
            }
            LogOp::Delete { namespace, key } => {
                let kv = self.store.open_namespace(namespace).await?;
                let final_key = Self::rewrite_key_if_needed(namespace, key, target)?;
                kv.delete(&final_key).await?;
            }
            LogOp::BatchPut { namespace, items } => {
                // Per-item apply — any failure aborts the whole entry so the
                // cursor doesn't advance past a partial replay (alpha BLOCKER
                // 4439b).
                let total = items.len();
                for (idx, (key, value)) in items.iter().enumerate() {
                    let final_key =
                        Self::rewrite_key_if_needed(namespace, key, target).map_err(|e| {
                            tracing::error!(
                                "replay BatchPut abort: key rewrite failed at item {}/{} ns={}: {}",
                                idx + 1,
                                total,
                                namespace,
                                e
                            );
                            e
                        })?;
                    self.replay_put(
                        namespace,
                        &LogOp::encode_bytes(&final_key),
                        value,
                        entry.timestamp_ms,
                        &entry.device_id,
                    )
                    .await
                    .map_err(|e| {
                        tracing::error!(
                            "replay BatchPut abort: item {}/{} ns={} failed to apply: {}",
                            idx + 1,
                            total,
                            namespace,
                            e
                        );
                        e
                    })?;
                }
            }
            LogOp::BatchDelete { namespace, keys } => {
                let kv = self.store.open_namespace(namespace).await?;
                let decoded: Vec<Vec<u8>> = keys
                    .iter()
                    .map(|k| Self::rewrite_key_if_needed(namespace, k, target))
                    .collect::<SyncResult<Vec<_>>>()?;
                kv.batch_delete(decoded).await?;
            }
        }
        Ok(())
    }

    /// Rewrites log entry keys based on namespace isolation rules.
    ///
    /// Two rewrites apply:
    ///
    /// 1. **Share subscriptions** — `{share_prefix}:…` keys replayed from an
    ///    inbound share become `from:{sender_hash}:…` locally, so the
    ///    receiver reads shared data through a distinct namespace.
    /// 2. **Org schemas** — `{org_hash}:{schema_name}` entries replayed into
    ///    the `schemas` or `schema_states` namespaces from an org target are
    ///    stripped back to the bare `{schema_name}`. Schemas are addressed
    ///    by name locally; the org prefix exists only to drive sync routing
    ///    on the writer side. Without this rewrite, peers would store the
    ///    schema under a name like `{org_hash}:sync_notes` and name-based
    ///    lookups (`/api/schemas`, `get_schema`) would miss it — orphaning
    ///    every org-prefixed molecule (alpha BLOCKER af4ba).
    fn rewrite_key_if_needed(
        namespace: &str,
        key_b64: &str,
        target: Option<&SyncTarget>,
    ) -> SyncResult<Vec<u8>> {
        let key_bytes = LogOp::decode_bytes(key_b64)?;

        if let Some(t) = target {
            if t.prefix.starts_with("share:") {
                let mut parts = t.prefix.split(':');
                parts.next(); // skip "share"
                if let Some(sender_hash) = parts.next() {
                    let prefix_str = format!("{}:", t.prefix);
                    let prefix_bytes = prefix_str.as_bytes();

                    if key_bytes.starts_with(prefix_bytes) {
                        let new_prefix_str = format!("from:{}:", sender_hash);
                        let new_prefix_bytes = new_prefix_str.as_bytes();

                        let mut final_key = Vec::with_capacity(
                            new_prefix_bytes.len() + key_bytes.len() - prefix_bytes.len(),
                        );
                        final_key.extend_from_slice(new_prefix_bytes);
                        final_key.extend_from_slice(&key_bytes[prefix_bytes.len()..]);

                        return Ok(final_key);
                    }
                }
            } else if !t.prefix.is_empty()
                && (namespace == "schemas" || namespace == "schema_states")
            {
                let prefix_str = format!("{}:", t.prefix);
                let prefix_bytes = prefix_str.as_bytes();
                if key_bytes.starts_with(prefix_bytes) {
                    return Ok(key_bytes[prefix_bytes.len()..].to_vec());
                }
            }
        }

        Ok(key_bytes)
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
                    let (merged_bytes, conflicts) = self.merge_molecules(&local, &value_bytes)?;
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
        }

        Ok(())
    }

    /// Try to parse both byte slices as type `T`, merge, and serialize back.
    /// Returns `None` if either side fails to deserialize (caller should try the next type).
    fn try_merge<T>(
        local_bytes: &[u8],
        incoming_bytes: &[u8],
        keypair: &Ed25519KeyPair,
    ) -> Option<SyncResult<(Vec<u8>, Vec<MergeConflict>)>>
    where
        T: serde::de::DeserializeOwned + serde::Serialize + MergeMolecule,
    {
        let (mut local, incoming) = match (
            serde_json::from_slice::<T>(local_bytes),
            serde_json::from_slice::<T>(incoming_bytes),
        ) {
            (Ok(l), Ok(i)) => (l, i),
            _ => return None,
        };
        let conflicts = local.merge_into_conflicts(&incoming, keypair);
        Some(
            serde_json::to_vec(&local)
                .map(|merged| (merged, conflicts))
                .map_err(Into::into),
        )
    }

    /// Attempt molecule merge by trying each molecule type in order.
    /// Returns the serialized merged result and any conflicts. The node signer
    /// is used to re-sign single-`Molecule` merge results so a peer can verify
    /// the merge was committed by this node.
    fn merge_molecules(
        &self,
        local_bytes: &[u8],
        incoming_bytes: &[u8],
    ) -> SyncResult<(Vec<u8>, Vec<MergeConflict>)> {
        let kp = self.node_signer.as_ref();
        if let Some(result) = Self::try_merge::<MoleculeHash>(local_bytes, incoming_bytes, kp) {
            return result;
        }
        if let Some(result) = Self::try_merge::<MoleculeRange>(local_bytes, incoming_bytes, kp) {
            return result;
        }
        if let Some(result) = Self::try_merge::<MoleculeHashRange>(local_bytes, incoming_bytes, kp)
        {
            return result;
        }
        if let Some(result) = Self::try_merge::<Molecule>(local_bytes, incoming_bytes, kp) {
            return result;
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
                writer_pubkey: String::new(),
                signature: String::new(),
                // Conflict-originated events are synthesized during merge
                // resolution and have no originating user mutation in scope;
                // leave provenance `None`.
                provenance: None,
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
    // User-triggered snapshot backup
    // =========================================================================

    /// Create a snapshot of the current local store and upload it to the cloud.
    ///
    /// The snapshot is sealed with the personal crypto provider, uploaded under
    /// both `snapshots/{seq}.enc` (point-in-time) and `snapshots/latest.enc`
    /// (the key read by `bootstrap_target(0)` on new-device restore). Unlike
    /// `compact`, this does NOT delete any log entries — it is an explicit
    /// backup checkpoint users or the CLI can trigger on demand.
    ///
    /// Returns the sequence number of the uploaded snapshot.
    pub async fn backup_snapshot(&self) -> SyncResult<u64> {
        match self.backup_snapshot_once().await {
            Ok(seq) => Ok(seq),
            Err(SyncError::Auth(_)) if self.auth_refresh.is_some() => {
                self.refresh_auth_once("backup_snapshot").await?;
                self.backup_snapshot_once().await
            }
            Err(e) => Err(e),
        }
    }

    async fn backup_snapshot_once(&self) -> SyncResult<u64> {
        let current_seq = *self.seq.lock().await;
        tracing::info!(
            "backup_snapshot: creating snapshot at seq {} (device='{}')",
            current_seq,
            self.device_id,
        );

        let snapshot = Snapshot::create(self.store.as_ref(), &self.device_id, current_seq).await?;
        let namespace_count = snapshot.namespaces.len();
        let sealed = snapshot.seal(&self.crypto).await?;

        let seq_name = format!("{current_seq}.enc");
        let seq_url = self.auth.presign_snapshot_upload(&seq_name).await?;
        self.s3.upload(&seq_url, sealed.clone()).await?;

        let latest_url = self.auth.presign_snapshot_upload("latest.enc").await?;
        self.s3.upload(&latest_url, sealed).await?;

        tracing::info!(
            "backup_snapshot: uploaded {} namespaces at seq {} as 'latest.enc' and '{}' (device='{}')",
            namespace_count,
            current_seq,
            seq_name,
            self.device_id,
        );
        Ok(current_seq)
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
    // Cloud-aware reset
    // =========================================================================

    /// Delete every cloud-side log entry and snapshot for the personal
    /// target, leaving the prefix empty.
    ///
    /// Used by `fold_db_node`'s `reset-database` flow to make local wipe
    /// actually stick: without this, the next sync cycle would re-bootstrap
    /// from `latest.enc` and replay the full personal log, undoing the
    /// reset.
    ///
    /// Org targets (`targets[1..]`) are intentionally untouched. Org logs
    /// are shared state across members; leaving an org is a separate flow
    /// the user must perform explicitly.
    ///
    /// The device write lock is acquired for the duration of the purge so
    /// no other device can race uploads against us. The lock is released
    /// best-effort on the way out — a stuck lock self-heals via TTL.
    pub async fn purge_personal_log(&self) -> SyncResult<PurgeOutcome> {
        self.acquire_lock().await?;
        let result = self.purge_personal_log_inner().await;
        if let Err(e) = self.release_lock().await {
            tracing::warn!("purge_personal_log: failed to release device lock (non-fatal): {e}");
        }
        result
    }

    async fn purge_personal_log_inner(&self) -> SyncResult<PurgeOutcome> {
        let personal = self.targets.lock().await[0].clone();
        if !personal.prefix.is_empty() {
            // Defense-in-depth: targets[0] is always personal (empty
            // prefix) by construction. If this ever changes upstream we
            // want to fail loud, not nuke the wrong prefix.
            return Err(SyncError::Auth(
                "purge_personal_log: targets[0] is not the personal target".to_string(),
            ));
        }

        let mut outcome = PurgeOutcome::default();

        // 1) Delete every log object: list → presign DELETE in chunks of
        //    1000 (Lambda cap) → DELETE each presigned URL.
        let log_objects = self.auth.list_log_objects(&personal).await?;
        let log_seqs: Vec<u64> = log_objects
            .iter()
            .filter_map(|obj| parse_flat_log_key(&obj.key))
            .collect();

        for chunk in log_seqs.chunks(MAX_PRESIGN_BATCH) {
            let urls = self.auth.presign_log_delete(chunk).await?;
            for url in &urls {
                self.s3.delete(url).await?;
            }
            outcome.deleted_log_objects += urls.len();
        }

        // 2) Delete every snapshot under `snapshots/`. Includes
        //    `latest.enc` plus any compacted `{seq}.enc` left behind by
        //    prior compactions. Listing first (rather than only attempting
        //    `latest.enc`) ensures the prefix is fully empty after reset.
        let snapshot_objects = self.auth.list_snapshot_objects(&personal).await?;
        for obj in snapshot_objects {
            // The Lambda strips the scope prefix on list responses, so
            // keys come back as `snapshots/{name}` — we want the bare
            // `{name}` for the delete request.
            let Some(name) = obj.key.strip_prefix("snapshots/") else {
                tracing::warn!(
                    "purge_personal_log: skipping snapshot key '{}' that doesn't start with 'snapshots/'",
                    obj.key
                );
                continue;
            };
            let url = self.auth.presign_snapshot_delete(name).await?;
            self.s3.delete(&url).await?;
            outcome.deleted_snapshots += 1;
        }

        tracing::info!(
            "purge_personal_log: deleted {} log objects, {} snapshots",
            outcome.deleted_log_objects,
            outcome.deleted_snapshots,
        );
        Ok(outcome)
    }

    // =========================================================================
    // Non-personal sync configuration (orgs + cross-user shares)
    // =========================================================================

    /// Configure non-personal sync targets (orgs + cross-user shares).
    ///
    /// Replaces all non-personal targets atomically. The personal target at
    /// index 0 is always preserved. The partitioner classifies pending log
    /// entries to the correct target by key prefix.
    ///
    /// This is the runtime reconfiguration entry point: callers MUST invoke
    /// this every time org membership, share rules, or share subscriptions
    /// change in Sled so the sync engine picks up new upload/download prefixes
    /// without restarting the node.
    ///
    /// `extra_targets` should contain one `SyncTarget` per:
    /// - active org membership (uploads + downloads under `{org_hash}/log/`)
    /// - active outbound share rule (uploads under `{share_prefix}/log/`)
    /// - active inbound share subscription (downloads under
    ///   `{share_prefix}/log/`)
    ///
    /// The `partitioner` must be built from the same memberships + share
    /// rules so write routing stays consistent with the target list.
    pub async fn configure_org_sync(
        &self,
        partitioner: SyncPartitioner,
        extra_targets: Vec<SyncTarget>,
    ) {
        *self.partitioner.lock().await = Some(partitioner);
        let mut targets = self.targets.lock().await;
        targets.truncate(1); // Keep personal target
        targets.extend(extra_targets);
    }

    /// Alias for [`configure_org_sync`] used by sharing-related call sites.
    ///
    /// Semantically identical — provided so callers that are wiring up a
    /// share rule or subscription can express intent more clearly than
    /// "configure_org_sync". Both names accept the full set of non-personal
    /// targets (orgs, outbound shares, inbound shares).
    pub async fn reconfigure_sharing(
        &self,
        partitioner: SyncPartitioner,
        extra_targets: Vec<SyncTarget>,
    ) {
        self.configure_org_sync(partitioner, extra_targets).await;
    }

    /// Check if any non-personal sync targets (orgs or shares) are configured.
    pub async fn has_org_sync(&self) -> bool {
        self.targets.lock().await.len() > 1
    }

    /// Return the R2 prefix of every configured sync target, in order.
    ///
    /// Index 0 is the personal prefix. Remaining entries are org or share
    /// prefixes in the order they were registered via `configure_org_sync`
    /// / `reconfigure_sharing`. Primarily intended for tests and status
    /// endpoints that need to verify runtime reconfiguration took effect.
    pub async fn target_prefixes(&self) -> Vec<String> {
        self.targets
            .lock()
            .await
            .iter()
            .map(|t| t.prefix.clone())
            .collect()
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

    /// `MAX_PRESIGN_BATCH` must stay aligned with `storage_service`'s
    /// hard cap of 1000 (`lambdas/storage_service/src/handlers.rs`).
    /// Bumping this without bumping the Lambda — or vice versa — silently
    /// re-breaks `seq_numbers exceeds maximum length of 1000` on personal
    /// uploads. Pin the value here so a thoughtless change trips this test.
    #[test]
    fn presign_batch_cap_matches_storage_service() {
        assert_eq!(MAX_PRESIGN_BATCH, 1000);
    }

    /// Verify the chunking arithmetic that `upload_entries` /
    /// `download_entries` rely on: ceil(N / MAX_PRESIGN_BATCH) chunks,
    /// with no chunk exceeding the cap. Catches off-by-one regressions if
    /// someone replaces `chunks(MAX_PRESIGN_BATCH)` with `chunks(N - 1)`
    /// or similar.
    #[test]
    fn presign_chunking_produces_no_oversized_chunks() {
        for n in [0usize, 1, 999, 1000, 1001, 1999, 2000, 2001, 5_000] {
            let v: Vec<u64> = (0..n as u64).collect();
            let chunks: Vec<&[u64]> = v.chunks(MAX_PRESIGN_BATCH).collect();
            let expected_chunk_count = n.div_ceil(MAX_PRESIGN_BATCH);
            assert_eq!(
                chunks.len(),
                expected_chunk_count,
                "n={n}: expected {expected_chunk_count} chunks, got {}",
                chunks.len()
            );
            let total: usize = chunks.iter().map(|c| c.len()).sum();
            assert_eq!(total, n, "n={n}: chunks must cover every element");
            for (i, chunk) in chunks.iter().enumerate() {
                assert!(
                    chunk.len() <= MAX_PRESIGN_BATCH,
                    "n={n} chunk {i}: {} entries exceeds cap",
                    chunk.len()
                );
            }
        }
    }

    /// Regression for alpha BLOCKER 30a7b: the old `download_entries` built a
    /// `start_after` bound as `log/{cursor}.enc` using unpadded decimal seqs.
    /// S3 `start_after` is **lexicographic**, so a numeric cursor of 52
    /// excludes `log/100.enc..log/104.enc` (they lex-sort *before*
    /// `log/52.enc` because `'1' < '5'`). On dogfood run-8 this dropped
    /// exactly 5 entries from Bob's view (seqs 100..=104).
    ///
    /// This test models both paths:
    /// - "old buggy" path: simulate S3 lex filter on `start_after=log/52.enc`,
    ///   then apply the numeric `seq > cursor` client filter. Shows the 5-entry
    ///   drop.
    /// - "new fixed" path: no `start_after`, numeric filter only. Shows the
    ///   full 52 post-cursor seqs.
    #[test]
    fn lex_start_after_drops_keys_across_digit_boundaries_30a7b() {
        // Alice uploads seqs 1..=104 with keys `log/{seq}.enc`.
        let keys: Vec<String> = (1u64..=104).map(|s| format!("log/{s}.enc")).collect();

        let cursor: u64 = 52;

        // ---- Old buggy path: S3 lex start_after + client numeric filter ----
        let lex_bound = format!("log/{cursor}.enc");
        let mut lex_filtered: Vec<String> = keys
            .iter()
            .filter(|k| k.as_str() > lex_bound.as_str())
            .cloned()
            .collect();
        lex_filtered.sort();
        let buggy_new_seqs: Vec<u64> = lex_filtered
            .iter()
            .filter_map(|k| parse_flat_log_key(k))
            .filter(|s| *s > cursor)
            .collect();
        assert_eq!(
            buggy_new_seqs.len(),
            47,
            "sanity: the old lex-ordered path must drop seqs 100..=104 (5 entries) exactly as seen on dogfood run-8"
        );
        for missed in 100u64..=104 {
            assert!(
                !buggy_new_seqs.contains(&missed),
                "old lex path should miss seq {missed}, regression check"
            );
        }

        // ---- New fixed path: full prefix list + numeric filter ------------
        let mut fixed_new_seqs: Vec<u64> = keys
            .iter()
            .filter_map(|k| parse_flat_log_key(k))
            .filter(|s| *s > cursor)
            .collect();
        fixed_new_seqs.sort();
        assert_eq!(
            fixed_new_seqs.len(),
            52,
            "fixed path must return every seq 53..=104 (52 entries); digit-boundary keys MUST NOT be hidden"
        );
        assert_eq!(fixed_new_seqs.first().copied(), Some(53));
        assert_eq!(fixed_new_seqs.last().copied(), Some(104));
        for expected in 53u64..=104 {
            assert!(
                fixed_new_seqs.contains(&expected),
                "fixed path must include seq {expected}"
            );
        }
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

    // ---- partition_entry tests (mixed-prefix batch splitting) ----

    fn test_targets() -> Vec<SyncTarget> {
        use crate::crypto::provider::LocalCryptoProvider;
        vec![
            SyncTarget {
                label: "personal".to_string(),
                prefix: "personal_user".to_string(),
                crypto: Arc::new(LocalCryptoProvider::from_key([0x01u8; 32])),
            },
            SyncTarget {
                label: "org_a".to_string(),
                prefix: "org_a_hash".to_string(),
                crypto: Arc::new(LocalCryptoProvider::from_key([0x02u8; 32])),
            },
            SyncTarget {
                label: "org_b".to_string(),
                prefix: "org_b_hash".to_string(),
                crypto: Arc::new(LocalCryptoProvider::from_key([0x03u8; 32])),
            },
        ]
    }

    fn test_partitioner() -> SyncPartitioner {
        use crate::org::OrgMembership;
        let memberships = vec![
            OrgMembership {
                org_name: "Org A".to_string(),
                org_hash: "org_a_hash".to_string(),
                org_public_key: "pk_a".to_string(),
                org_secret_key: None,
                org_e2e_secret: "secret_a".to_string(),
                role: crate::org::OrgRole::Member,
                members: vec![],
                created_at: 0,
                joined_at: 0,
            },
            OrgMembership {
                org_name: "Org B".to_string(),
                org_hash: "org_b_hash".to_string(),
                org_public_key: "pk_b".to_string(),
                org_secret_key: None,
                org_e2e_secret: "secret_b".to_string(),
                role: crate::org::OrgRole::Member,
                members: vec![],
                created_at: 0,
                joined_at: 0,
            },
        ];
        SyncPartitioner::new(&memberships, &[])
    }

    fn batch_put(items: &[&[u8]]) -> LogEntry {
        LogEntry {
            seq: 42,
            timestamp_ms: 1000,
            device_id: "dev".to_string(),
            op: LogOp::BatchPut {
                namespace: "main".to_string(),
                items: items
                    .iter()
                    .map(|k| (LogOp::encode_bytes(k), LogOp::encode_bytes(b"v")))
                    .collect(),
            },
        }
    }

    fn batch_delete(keys: &[&[u8]]) -> LogEntry {
        LogEntry {
            seq: 42,
            timestamp_ms: 1000,
            device_id: "dev".to_string(),
            op: LogOp::BatchDelete {
                namespace: "main".to_string(),
                keys: keys.iter().map(|k| LogOp::encode_bytes(k)).collect(),
            },
        }
    }

    fn batch_len(entry: &LogEntry) -> usize {
        match &entry.op {
            LogOp::BatchPut { items, .. } => items.len(),
            LogOp::BatchDelete { keys, .. } => keys.len(),
            _ => panic!("expected batch"),
        }
    }

    #[test]
    fn partition_entry_homogeneous_org_batch_unchanged() {
        let partitioner = Some(test_partitioner());
        let targets = test_targets();
        let entry = batch_put(&[b"org_a_hash:atom:foo", b"org_a_hash:atom:bar"]);

        let result = SyncEngine::partition_entry(&partitioner, &entry, &targets);
        assert_eq!(result.len(), 1, "homogeneous batch must produce one bucket");
        let (idx, sub) = &result[0];
        assert_eq!(*idx, 1, "org_a is target index 1");
        assert_eq!(sub.seq, 42);
        assert_eq!(batch_len(sub), 2, "original batch intact");
    }

    #[test]
    fn partition_entry_mixed_personal_and_org_splits() {
        let partitioner = Some(test_partitioner());
        let targets = test_targets();
        let entry = batch_put(&[b"atom:personal-1", b"org_a_hash:atom:shared-1"]);

        let result = SyncEngine::partition_entry(&partitioner, &entry, &targets);
        assert_eq!(result.len(), 2, "mixed batch must split");

        let by_idx: std::collections::HashMap<usize, &LogEntry> =
            result.iter().map(|(i, e)| (*i, e)).collect();
        let personal = by_idx.get(&0).expect("personal bucket");
        let org_a = by_idx.get(&1).expect("org_a bucket");
        assert_eq!(batch_len(personal), 1);
        assert_eq!(batch_len(org_a), 1);
        assert_eq!(personal.seq, 42);
        assert_eq!(org_a.seq, 42, "seq preserved across split");
    }

    #[test]
    fn partition_entry_mixed_three_orgs_splits() {
        let partitioner = Some(test_partitioner());
        let targets = test_targets();
        let entry = batch_put(&[
            b"atom:personal-1",
            b"org_a_hash:atom:shared-1",
            b"org_a_hash:atom:shared-2",
            b"org_b_hash:atom:shared-3",
        ]);

        let result = SyncEngine::partition_entry(&partitioner, &entry, &targets);
        assert_eq!(result.len(), 3);
        let by_idx: std::collections::HashMap<usize, &LogEntry> =
            result.iter().map(|(i, e)| (*i, e)).collect();
        assert_eq!(batch_len(by_idx.get(&0).expect("personal")), 1);
        assert_eq!(batch_len(by_idx.get(&1).expect("org_a")), 2);
        assert_eq!(batch_len(by_idx.get(&2).expect("org_b")), 1);
    }

    #[test]
    fn partition_entry_batch_delete_mixed_splits() {
        let partitioner = Some(test_partitioner());
        let targets = test_targets();
        let entry = batch_delete(&[b"atom:personal-1", b"org_b_hash:atom:shared-1"]);

        let result = SyncEngine::partition_entry(&partitioner, &entry, &targets);
        assert_eq!(result.len(), 2);
        let by_idx: std::collections::HashMap<usize, &LogEntry> =
            result.iter().map(|(i, e)| (*i, e)).collect();
        assert_eq!(batch_len(by_idx.get(&0).expect("personal")), 1);
        assert_eq!(batch_len(by_idx.get(&2).expect("org_b")), 1);
        // Confirm ops are BatchDelete
        for (_, sub) in &result {
            assert!(matches!(sub.op, LogOp::BatchDelete { .. }));
        }
    }

    #[test]
    fn partition_entry_single_put_unchanged() {
        let partitioner = Some(test_partitioner());
        let targets = test_targets();
        let entry = LogEntry {
            seq: 7,
            timestamp_ms: 1000,
            device_id: "dev".to_string(),
            op: LogOp::Put {
                namespace: "main".to_string(),
                key: LogOp::encode_bytes(b"org_b_hash:atom:x"),
                value: LogOp::encode_bytes(b"v"),
            },
        };
        let result = SyncEngine::partition_entry(&partitioner, &entry, &targets);
        assert_eq!(result.len(), 1);
        assert_eq!(result[0].0, 2);
    }

    // ---- BootstrapOutcome tests ----
    //
    // Full end-to-end bootstrap_target/bootstrap_all tests would require
    // mocking AuthClient + S3Client, and no such mocking infrastructure
    // exists in this crate (integration tests hit a real localhost URL and
    // never actually invoke bootstrap). Rather than introduce a heavy mock
    // harness in this PR, we cover the pure logic: the outcome shape, the
    // default / "nothing to do" case, and the aggregation predicates used
    // by `bootstrap_all` to decide whether reloaders fire. End-to-end
    // coverage is deferred to the fold_db_node follow-up that wires
    // `bootstrap_from_cloud` to the new API.

    #[test]
    fn bootstrap_outcome_default_is_empty() {
        let outcome = BootstrapOutcome::default();
        assert_eq!(outcome.last_seq, 0);
        assert_eq!(outcome.entries_replayed, 0);
        assert!(!outcome.schemas_replayed);
        assert!(!outcome.embeddings_replayed);
    }

    #[test]
    fn bootstrap_all_aggregation_fires_schema_reloader_when_any_target_replays_schemas() {
        // Mirror the predicate `bootstrap_all` uses to decide whether to
        // invoke the schema reloader exactly once at the end.
        let outcomes = [
            BootstrapOutcome {
                last_seq: 10,
                entries_replayed: 3,
                schemas_replayed: false,
                embeddings_replayed: false,
            },
            BootstrapOutcome {
                last_seq: 20,
                entries_replayed: 1,
                schemas_replayed: true,
                embeddings_replayed: false,
            },
            BootstrapOutcome::default(),
        ];
        assert!(outcomes.iter().any(|o| o.schemas_replayed));
        assert!(!outcomes.iter().any(|o| o.embeddings_replayed));
    }

    #[test]
    fn bootstrap_all_aggregation_skips_reloaders_when_no_replays() {
        let outcomes = [BootstrapOutcome::default(), BootstrapOutcome::default()];
        assert!(!outcomes.iter().any(|o| o.schemas_replayed));
        assert!(!outcomes.iter().any(|o| o.embeddings_replayed));
    }

    #[test]
    fn bootstrap_all_aggregation_independent_schema_and_embedding_flags() {
        let outcomes = [
            BootstrapOutcome {
                last_seq: 5,
                entries_replayed: 1,
                schemas_replayed: false,
                embeddings_replayed: true,
            },
            BootstrapOutcome {
                last_seq: 7,
                entries_replayed: 2,
                schemas_replayed: true,
                embeddings_replayed: false,
            },
        ];
        assert!(outcomes.iter().any(|o| o.schemas_replayed));
        assert!(outcomes.iter().any(|o| o.embeddings_replayed));
        // Counts aggregate independently.
        let total: usize = outcomes.iter().map(|o| o.entries_replayed).sum();
        assert_eq!(total, 3);
    }

    // ---- rewrite_key_if_needed tests (share prefix -> from: namespace) ----

    fn share_target(prefix: &str) -> SyncTarget {
        use crate::crypto::provider::LocalCryptoProvider;
        SyncTarget {
            label: "share".to_string(),
            prefix: prefix.to_string(),
            crypto: Arc::new(LocalCryptoProvider::from_key([0x09u8; 32])),
        }
    }

    #[test]
    fn test_rewrite_key_share_prefix_to_from_prefix() {
        let target = share_target("share:alice:me");
        let key_b64 = LogOp::encode_bytes(b"share:alice:me:atom:uuid-1");
        let result = SyncEngine::rewrite_key_if_needed("main", &key_b64, Some(&target)).unwrap();
        assert_eq!(result, b"from:alice:atom:uuid-1");
    }

    #[test]
    fn test_rewrite_key_no_target_passes_through() {
        let key_b64 = LogOp::encode_bytes(b"atom:uuid-1");
        let result = SyncEngine::rewrite_key_if_needed("main", &key_b64, None).unwrap();
        assert_eq!(result, b"atom:uuid-1");
    }

    #[test]
    fn test_rewrite_key_org_target_main_namespace_passes_through() {
        use crate::crypto::provider::LocalCryptoProvider;
        let target = SyncTarget {
            label: "org".to_string(),
            prefix: "org_hash_abc".to_string(),
            crypto: Arc::new(LocalCryptoProvider::from_key([0x10u8; 32])),
        };
        let key_b64 = LogOp::encode_bytes(b"org_hash_abc:atom:uuid-1");
        let result = SyncEngine::rewrite_key_if_needed("main", &key_b64, Some(&target)).unwrap();
        // Non-schema namespace: org prefix stays, because atom/ref keys are
        // stored org-prefixed locally on every peer.
        assert_eq!(result, b"org_hash_abc:atom:uuid-1");
    }

    #[test]
    fn test_rewrite_key_org_target_schemas_namespace_strips_prefix() {
        use crate::crypto::provider::LocalCryptoProvider;
        let target = SyncTarget {
            label: "org".to_string(),
            prefix: "org_hash_abc".to_string(),
            crypto: Arc::new(LocalCryptoProvider::from_key([0x10u8; 32])),
        };
        let key_b64 = LogOp::encode_bytes(b"org_hash_abc:sync_notes");
        let result = SyncEngine::rewrite_key_if_needed("schemas", &key_b64, Some(&target)).unwrap();
        assert_eq!(result, b"sync_notes");
    }

    #[test]
    fn test_rewrite_key_org_target_schema_states_namespace_strips_prefix() {
        use crate::crypto::provider::LocalCryptoProvider;
        let target = SyncTarget {
            label: "org".to_string(),
            prefix: "org_hash_abc".to_string(),
            crypto: Arc::new(LocalCryptoProvider::from_key([0x10u8; 32])),
        };
        let key_b64 = LogOp::encode_bytes(b"org_hash_abc:sync_notes");
        let result =
            SyncEngine::rewrite_key_if_needed("schema_states", &key_b64, Some(&target)).unwrap();
        assert_eq!(result, b"sync_notes");
    }

    #[test]
    fn test_rewrite_key_personal_target_schemas_namespace_passes_through() {
        use crate::crypto::provider::LocalCryptoProvider;
        let target = SyncTarget {
            label: "personal".to_string(),
            prefix: String::new(),
            crypto: Arc::new(LocalCryptoProvider::from_key([0x10u8; 32])),
        };
        let key_b64 = LogOp::encode_bytes(b"sync_notes");
        let result = SyncEngine::rewrite_key_if_needed("schemas", &key_b64, Some(&target)).unwrap();
        assert_eq!(result, b"sync_notes");
    }

    #[test]
    fn test_rewrite_key_share_target_but_key_not_matching_passes_through() {
        let target = share_target("share:alice:me");
        // Key has a different prefix than the target
        let key_b64 = LogOp::encode_bytes(b"atom:uuid-1");
        let result = SyncEngine::rewrite_key_if_needed("main", &key_b64, Some(&target)).unwrap();
        assert_eq!(result, b"atom:uuid-1");
    }

    // ---- auth refresh helper tests ----

    use crate::sync::auth::SyncAuth;

    fn make_auth_refresh_engine() -> SyncEngine {
        use crate::crypto::provider::LocalCryptoProvider;
        use crate::storage::inmemory_backend::InMemoryNamespacedStore;
        use crate::sync::auth::AuthClient;
        use crate::sync::s3::S3Client;

        // trace-egress: loopback (test scaffold targeting unreachable localhost)
        let http = Arc::new(reqwest::Client::new());
        let auth = AuthClient::new(
            http.clone(),
            "http://127.0.0.1:1".to_string(),
            SyncAuth::ApiKey("stale-key".to_string()),
        );
        let s3 = S3Client::new(http);
        let crypto: Arc<dyn CryptoProvider> = Arc::new(LocalCryptoProvider::from_key([0x77u8; 32]));
        let store: Arc<dyn NamespacedStore> = Arc::new(InMemoryNamespacedStore::new());
        let signer = Arc::new(Ed25519KeyPair::generate().unwrap());
        SyncEngine::new(
            "test-device".to_string(),
            crypto,
            s3,
            auth,
            store,
            SyncConfig::default(),
            signer,
        )
    }

    #[tokio::test]
    async fn refresh_auth_once_without_callback_returns_auth_error() {
        let engine = make_auth_refresh_engine();
        let err = engine
            .refresh_auth_once("test")
            .await
            .expect_err("expected auth error with no callback wired");
        match err {
            SyncError::Auth(msg) => assert_eq!(msg, "authentication failed"),
            other => panic!("expected SyncError::Auth, got {other:?}"),
        }
    }

    #[tokio::test]
    async fn refresh_auth_once_invokes_callback_and_updates_auth() {
        use std::sync::atomic::{AtomicUsize, Ordering};

        let mut engine = make_auth_refresh_engine();
        assert!(
            !engine.auth.is_bearer_token().await,
            "engine should start with ApiKey auth"
        );

        let call_count = Arc::new(AtomicUsize::new(0));
        let cb_calls = call_count.clone();
        let cb: AuthRefreshCallback = Arc::new(move || {
            let cb_calls = cb_calls.clone();
            Box::pin(async move {
                cb_calls.fetch_add(1, Ordering::SeqCst);
                Ok(SyncAuth::BearerToken("fresh-token".to_string()))
            })
        });
        engine.set_auth_refresh(cb);

        engine
            .refresh_auth_once("test")
            .await
            .expect("refresh should succeed when callback returns new auth");

        assert_eq!(
            call_count.load(Ordering::SeqCst),
            1,
            "callback must run exactly once per refresh call"
        );
        assert!(
            engine.auth.is_bearer_token().await,
            "AuthClient must be updated to the new credential"
        );
    }

    #[tokio::test]
    async fn refresh_auth_once_surfaces_callback_error() {
        let mut engine = make_auth_refresh_engine();
        let cb: AuthRefreshCallback =
            Arc::new(|| Box::pin(async { Err("exemem returned 403: banned".to_string()) }));
        engine.set_auth_refresh(cb);

        let err = engine
            .refresh_auth_once("test")
            .await
            .expect_err("refresh must fail when callback errors");
        match err {
            SyncError::Auth(msg) => assert!(
                msg.contains("after token refresh failure"),
                "error must mention refresh-after-failure, got: {msg}"
            ),
            other => panic!("expected SyncError::Auth, got {other:?}"),
        }
    }

    #[tokio::test]
    async fn record_put_notifies_wake_handle() {
        // record_put must fire the wake notification so the background sync
        // coordinator can flush immediately on local writes instead of waiting
        // out the full sync interval. Without this, two devices writing to the
        // same account see up to 2× sync_interval_ms of latency before the
        // second device sees the first device's change.
        let engine = make_auth_refresh_engine();
        let wake = engine.wake_handle();

        // Arm the notification listener BEFORE the write so we don't miss a
        // notify_one that already fired (Notify only holds one pending).
        let notified = wake.notified();
        tokio::pin!(notified);

        // Nothing should have fired yet.
        assert!(
            futures::future::poll_immediate(&mut notified)
                .await
                .is_none(),
            "wake must be idle before any write"
        );

        // A single record_put fires notify_one.
        engine.record_put("main", b"k", b"v").await;

        // The pinned notified future must now be ready.
        tokio::time::timeout(tokio::time::Duration::from_millis(50), notified)
            .await
            .expect("record_put must wake the coordinator within 50ms");
    }
}
