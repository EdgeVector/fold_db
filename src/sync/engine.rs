use super::auth::AuthClient;
use super::error::{SyncError, SyncResult};
use super::log::{LogEntry, LogOp};
use super::org_sync::{SyncDestination, SyncPartitioner};
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
    /// Optional partitioner for routing org-prefixed keys to org S3 prefixes.
    /// When set, pending entries are partitioned at upload time:
    /// - Personal keys -> personal sync path (as today)
    /// - Org-prefixed keys -> `/{org_hash}/log/{member_id}/{seq}.enc`
    partitioner: Arc<Mutex<Option<SyncPartitioner>>>,
    /// Short member ID for org sync (first 8 hex chars of SHA256 of node public key).
    /// Only needed when partitioner is set.
    member_id: Arc<Mutex<Option<String>>>,
    /// Org-specific crypto providers keyed by org_hash.
    /// Each org has its own E2E key for encrypting org data.
    org_crypto: Arc<Mutex<std::collections::HashMap<String, Arc<dyn CryptoProvider>>>>,
    /// Tracks the last downloaded sequence per org member for incremental download.
    /// Maps `{org_hash}:{member_id}` -> last_seq downloaded from that member.
    org_member_cursors: Arc<Mutex<std::collections::HashMap<String, u64>>>,
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
            partitioner: Arc::new(Mutex::new(None)),
            member_id: Arc::new(Mutex::new(None)),
            org_crypto: Arc::new(Mutex::new(std::collections::HashMap::new())),
            org_member_cursors: Arc::new(Mutex::new(std::collections::HashMap::new())),
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
        let has_pending = !self.pending.lock().await.is_empty();
        if !has_pending {
            // Even with no pending entries, we may need to download org entries
            let org_downloaded = self.sync_org_download().await.unwrap_or(0);
            return Ok(org_downloaded > 0);
        }

        // If org sync is configured, use partitioned upload
        let has_partitioner = self.partitioner.lock().await.is_some();
        if has_partitioner {
            let uploaded = self.sync_org_entries().await?;
            // Also download from org members
            let downloaded = self.sync_org_download().await.unwrap_or(0);

            // Check if compaction is needed (personal entries only)
            let current_seq = *self.seq.lock().await;
            if current_seq > 0 && current_seq % self.config.compaction_threshold == 0 {
                if let Err(e) = self.compact(current_seq).await {
                    log::warn!("compaction failed (non-fatal): {e}");
                }
            }

            return Ok(uploaded > 0 || downloaded > 0);
        }

        // Standard personal-only sync path (no partitioner)
        let entries = {
            let pending = self.pending.lock().await;
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

    // =========================================================================
    // Org sync configuration
    // =========================================================================

    /// Configure org sync partitioning.
    ///
    /// Once set, the sync engine will partition pending entries at upload time:
    /// personal-keyed entries sync as today, org-prefixed entries encrypt with the
    /// org's E2E key and upload to `/{org_hash}/log/{member_id}/{seq}.enc`.
    ///
    /// # Arguments
    /// - `partitioner`: routes keys to personal or org destinations
    /// - `member_id`: this node's short ID (first 8 hex chars of SHA256 of public key)
    /// - `org_crypto`: map of org_hash -> CryptoProvider initialized with that org's E2E key
    pub async fn configure_org_sync(
        &self,
        partitioner: SyncPartitioner,
        member_id: String,
        org_crypto: std::collections::HashMap<String, Arc<dyn CryptoProvider>>,
    ) {
        *self.partitioner.lock().await = Some(partitioner);
        *self.member_id.lock().await = Some(member_id);
        *self.org_crypto.lock().await = org_crypto;
    }

    /// Check if org sync is configured.
    pub async fn has_org_sync(&self) -> bool {
        self.partitioner.lock().await.is_some()
    }

    // =========================================================================
    // Org sync: upload org-partitioned entries
    // =========================================================================

    /// Upload org-partitioned pending entries.
    ///
    /// Called as part of `do_sync` when a partitioner is configured.
    /// Partitions pending entries into personal and org buckets, then:
    /// - Personal entries: uploaded as normal (existing path)
    /// - Org entries: sealed with org E2E key, uploaded to org S3 prefix
    ///
    /// Returns the number of entries uploaded (personal + org).
    async fn sync_org_entries(&self) -> SyncResult<usize> {
        let entries = {
            let pending = self.pending.lock().await;
            if pending.is_empty() {
                return Ok(0);
            }
            pending.clone()
        };

        let partitioner = self.partitioner.lock().await;
        let partitioner = match partitioner.as_ref() {
            Some(p) => p,
            None => return Ok(0), // No partitioner, nothing to do here
        };

        let member_id = self.member_id.lock().await.clone().unwrap_or_default();

        // Partition entries by destination
        let mut personal_entries: Vec<LogEntry> = Vec::new();
        let mut org_entries: std::collections::HashMap<String, Vec<LogEntry>> =
            std::collections::HashMap::new();

        for entry in &entries {
            let dest = Self::classify_entry(partitioner, entry);
            match dest {
                SyncDestination::Personal => {
                    personal_entries.push(entry.clone());
                }
                SyncDestination::Org { org_hash, .. } => {
                    org_entries
                        .entry(org_hash)
                        .or_default()
                        .push(entry.clone());
                }
            }
        }

        let mut uploaded = 0;

        // Upload personal entries via existing path
        if !personal_entries.is_empty() {
            let mut sealed = Vec::with_capacity(personal_entries.len());
            for entry in &personal_entries {
                let s = entry.seal(&self.crypto).await?;
                sealed.push((entry.seq, s));
            }

            let seq_numbers: Vec<u64> = sealed.iter().map(|(seq, _)| *seq).collect();
            let urls = self.auth.presign_log_upload(&seq_numbers).await?;

            if urls.len() != sealed.len() {
                return Err(SyncError::Auth(format!(
                    "expected {} presigned URLs, got {}",
                    sealed.len(),
                    urls.len()
                )));
            }

            for ((_seq, s), url) in sealed.into_iter().zip(urls.iter()) {
                self.s3.upload(url, s.bytes).await?;
            }
            uploaded += personal_entries.len();
        }

        // Upload org entries
        let org_crypto = self.org_crypto.lock().await;
        for (org_hash, entries) in &org_entries {
            let crypto = match org_crypto.get(org_hash) {
                Some(c) => c,
                None => {
                    log::warn!(
                        "no CryptoProvider for org_hash={}, skipping {} entries",
                        org_hash,
                        entries.len()
                    );
                    continue;
                }
            };

            let mut sealed = Vec::with_capacity(entries.len());
            for entry in entries {
                let s = entry.seal(crypto).await?;
                sealed.push((entry.seq, s));
            }

            let seq_numbers: Vec<u64> = sealed.iter().map(|(seq, _)| *seq).collect();
            let urls = self
                .auth
                .presign_org_log_upload(org_hash, &member_id, &seq_numbers)
                .await?;

            if urls.len() != sealed.len() {
                return Err(SyncError::Auth(format!(
                    "expected {} org presigned URLs, got {}",
                    sealed.len(),
                    urls.len()
                )));
            }

            for ((_seq, s), url) in sealed.into_iter().zip(urls.iter()) {
                self.s3.upload(url, s.bytes).await?;
            }
            uploaded += entries.len();
        }

        // Clear all uploaded entries from pending
        {
            let mut pending = self.pending.lock().await;
            let count = entries.len().min(pending.len());
            pending.drain(..count);
        }

        Ok(uploaded)
    }

    /// Classify a single log entry by examining its key.
    fn classify_entry(partitioner: &SyncPartitioner, entry: &LogEntry) -> SyncDestination {
        match &entry.op {
            LogOp::Put {
                namespace, key, ..
            }
            | LogOp::Delete { namespace, key } => partitioner.partition_log_key(namespace, key),
            LogOp::BatchPut {
                namespace, items, ..
            } => {
                // Use the first item's key to classify the batch
                // (batches within a single schema always share the same org prefix)
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

    // =========================================================================
    // Org sync: download from other members
    // =========================================================================

    /// Download and replay org log entries from all members of all orgs.
    ///
    /// For each org the node belongs to, lists `/{org_hash}/log/*/` to discover
    /// member sub-prefixes, then downloads new entries from each member.
    ///
    /// Returns the total number of entries replayed.
    pub async fn sync_org_download(&self) -> SyncResult<u64> {
        let org_hashes = {
            let partitioner = self.partitioner.lock().await;
            match partitioner.as_ref() {
                Some(p) => p.org_hashes(),
                None => return Ok(0),
            }
        };

        let member_id = self.member_id.lock().await.clone().unwrap_or_default();

        let mut total_replayed: u64 = 0;

        for org_hash in &org_hashes {
            let replayed = self
                .download_org_entries(org_hash, &member_id)
                .await?;
            total_replayed += replayed;
        }

        Ok(total_replayed)
    }

    /// Download and replay entries for a single org from all other members.
    async fn download_org_entries(
        &self,
        org_hash: &str,
        my_member_id: &str,
    ) -> SyncResult<u64> {
        // List all objects under the org's log prefix
        let all_objects = self.auth.list_org_objects(org_hash, "log/").await?;

        // Group objects by member_id.
        // Keys look like: log/{member_id}/{seq}.enc
        let mut member_entries: std::collections::HashMap<String, Vec<u64>> =
            std::collections::HashMap::new();

        for obj in &all_objects {
            if let Some(parsed) = parse_org_log_key(&obj.key) {
                // Skip our own entries
                if parsed.member_id == my_member_id {
                    continue;
                }
                member_entries
                    .entry(parsed.member_id)
                    .or_default()
                    .push(parsed.seq);
            }
        }

        let org_crypto = self.org_crypto.lock().await;
        let crypto = match org_crypto.get(org_hash) {
            Some(c) => c.clone(),
            None => {
                log::warn!("no CryptoProvider for org_hash={}, skipping download", org_hash);
                return Ok(0);
            }
        };
        drop(org_crypto);

        let mut total_replayed: u64 = 0;
        let mut cursors = self.org_member_cursors.lock().await;

        for (remote_member_id, mut seqs) in member_entries {
            seqs.sort();

            // Filter to only new entries
            let cursor_key = format!("{org_hash}:{remote_member_id}");
            let cursor = cursors.get(&cursor_key).copied().unwrap_or(0);
            let new_seqs: Vec<u64> = seqs.into_iter().filter(|s| *s > cursor).collect();

            if new_seqs.is_empty() {
                continue;
            }

            let urls = self
                .auth
                .presign_org_log_download(org_hash, &remote_member_id, &new_seqs)
                .await?;

            for (seq, url) in new_seqs.iter().zip(urls.iter()) {
                let data = self.s3.download(url).await?;
                match data {
                    Some(bytes) => match LogEntry::unseal(&bytes, &crypto).await {
                        Ok(entry) => {
                            self.replay_entry(&entry).await?;
                            total_replayed += 1;
                        }
                        Err(e) => {
                            log::warn!(
                                "skipping corrupt org log entry org={} member={} seq={}: {}",
                                org_hash,
                                remote_member_id,
                                seq,
                                e
                            );
                        }
                    },
                    None => {
                        log::warn!(
                            "org log entry not found: org={} member={} seq={}",
                            org_hash,
                            remote_member_id,
                            seq
                        );
                    }
                }
            }

            // Update cursor
            if let Some(max_seq) = new_seqs.last() {
                cursors.insert(cursor_key, *max_seq);
            }
        }

        Ok(total_replayed)
    }
}

/// Parsed components of an org log S3 key.
struct ParsedOrgLogKey {
    member_id: String,
    seq: u64,
}

/// Parse an S3 object key like `log/{member_id}/{seq}.enc` into its components.
fn parse_org_log_key(key: &str) -> Option<ParsedOrgLogKey> {
    // Key format: log/{member_id}/{seq}.enc
    // The key may or may not have the org_hash prefix stripped by the auth Lambda
    let parts: Vec<&str> = key.split('/').collect();

    // Try matching from the end: .../{member_id}/{seq}.enc
    if parts.len() < 2 {
        return None;
    }

    let filename = parts[parts.len() - 1];
    let member_id = parts[parts.len() - 2];

    let seq_str = filename.strip_suffix(".enc")?;
    let seq = seq_str.parse::<u64>().ok()?;

    Some(ParsedOrgLogKey {
        member_id: member_id.to_string(),
        seq,
    })
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
    fn test_parse_org_log_key() {
        let parsed = parse_org_log_key("log/a1b2c3d4/42.enc").unwrap();
        assert_eq!(parsed.member_id, "a1b2c3d4");
        assert_eq!(parsed.seq, 42);

        // With org_hash prefix (as it might appear in full S3 key)
        let parsed2 = parse_org_log_key("org_abc/log/e5f6a7b8/1.enc").unwrap();
        assert_eq!(parsed2.member_id, "e5f6a7b8");
        assert_eq!(parsed2.seq, 1);

        // Invalid
        assert!(parse_org_log_key("log/a1b2c3d4/not_a_number.enc").is_none());
        assert!(parse_org_log_key("single").is_none());
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
