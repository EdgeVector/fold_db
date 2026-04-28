//! Ephemeral peer-to-peer sync for a single user's devices.
//!
//! `p2p` is a low-latency channel layered on top of the durable
//! log/snapshot sync. It pushes encrypted Sled write deltas to each peer's
//! `<user_hash>/p2p/<me>__<peer>/<seq>.enc` mailbox on R2 and polls each
//! peer's outgoing mailbox (`<user_hash>/p2p/<peer>__<me>/`) on a timer.
//!
//! ## Lifecycle and durability
//!
//! Objects under `p2p/` are auto-expired after 24h by an R2 lifecycle
//! rule. This is intentional — p2p is a best-effort shortcut, not the
//! source of truth. The durable log/snapshot sync (`SyncEngine`) is what
//! guarantees eventual convergence; p2p just lets a peer device pick up
//! a write within seconds instead of waiting for the next snapshot
//! bootstrap. There is no explicit ACK / DELETE flow: the 24h TTL
//! reclaims storage automatically.
//!
//! ## Deduplication
//!
//! Each peer's `download_cursors[peer_id]` records the highest seq this
//! device has applied from that peer. The poll loop lists the peer's
//! mailbox, filters out everything `<= cursor`, downloads and applies
//! the new entries in seq order, then advances the cursor.
//!
//! ## Auth and quota
//!
//! `presign_p2p_upload` skips the storage quota check (free for all
//! plans). Only the caller's own `<user_hash>/p2p/...` prefix is
//! reachable — the server enforces the scope.

use super::auth::AuthClient;
use super::error::{SyncError, SyncResult};
use super::log::{LogEntry, LogOp};
use super::s3::S3Client;
use crate::crypto::CryptoProvider;
use crate::storage::traits::NamespacedStore;
use std::collections::HashMap;
use std::sync::Arc;
use std::time::{SystemTime, UNIX_EPOCH};
use tokio::sync::Mutex;

/// Sled namespace where p2p sync state (last-seq-per-peer cursors and the
/// per-device monotonic seq counter) is persisted. Kept separate from the
/// `sync_cursors` namespace used by the durable `SyncEngine` so the two
/// systems never read each other's keys.
const P2P_STATE_NAMESPACE: &str = "p2p_sync_state";

/// Key prefix for per-peer download cursors inside `P2P_STATE_NAMESPACE`.
const CURSOR_KEY_PREFIX: &[u8] = b"cursor:";

/// Key for the local device's monotonic seq counter inside
/// `P2P_STATE_NAMESPACE`. The counter persists across restarts so two
/// processes sharing the same Sled path never reuse a seq.
const SEQ_COUNTER_KEY: &[u8] = b"seq_counter";

/// Configuration for the p2p sync engine.
#[derive(Clone, Debug)]
pub struct P2pConfig {
    /// Peer device IDs to push to and poll from.
    pub peer_device_ids: Vec<String>,
    /// Polling interval for each peer's mailbox.
    pub poll_interval_ms: u64,
}

impl Default for P2pConfig {
    fn default() -> Self {
        Self {
            peer_device_ids: Vec::new(),
            poll_interval_ms: 30_000,
        }
    }
}

/// Ephemeral peer-to-peer delta sync.
///
/// Reuses the existing `LogEntry` seal/unseal format so the same E2E
/// crypto key that protects the durable log also protects p2p deltas —
/// a peer device with the user's E2E key can open any p2p object;
/// nobody else can.
pub struct P2pSyncEngine {
    /// This device's id (the `<me>` in `<me>__<peer>` mailbox keys).
    device_id: String,
    /// E2E crypto provider — same key as the durable `SyncEngine`.
    crypto: Arc<dyn CryptoProvider>,
    /// HTTP layer for presigned URL exchange.
    auth: Arc<AuthClient>,
    /// S3 layer for the actual blob transfers.
    s3: S3Client,
    /// The NamespacedStore both for applying remote ops *and* for
    /// persisting p2p state (cursors, seq counter).
    store: Arc<dyn NamespacedStore>,
    /// Runtime config — peer set + poll interval.
    config: Mutex<P2pConfig>,
    /// In-memory copy of the per-device monotonic seq counter. Persisted
    /// to Sled on every advance so it survives restarts.
    seq_counter: Mutex<u64>,
    /// Pending entries waiting to be flushed to peers. Each entry is
    /// pushed to *every* peer's mailbox.
    pending: Mutex<Vec<LogEntry>>,
    /// Per-peer download cursors: `peer_id -> last_seq_applied`.
    download_cursors: Mutex<HashMap<String, u64>>,
}

impl P2pSyncEngine {
    /// Build a new p2p engine. Cursors and the seq counter are loaded
    /// lazily on first call to [`record_op`], [`flush_pending`], or
    /// [`poll_all_peers`] — call [`load_state`] explicitly if you want
    /// to fail fast at startup instead of on the first sync cycle.
    pub fn new(
        device_id: String,
        crypto: Arc<dyn CryptoProvider>,
        auth: Arc<AuthClient>,
        s3: S3Client,
        store: Arc<dyn NamespacedStore>,
        config: P2pConfig,
    ) -> Self {
        Self {
            device_id,
            crypto,
            auth,
            s3,
            store,
            config: Mutex::new(config),
            seq_counter: Mutex::new(0),
            pending: Mutex::new(Vec::new()),
            download_cursors: Mutex::new(HashMap::new()),
        }
    }

    /// This device's id.
    pub fn device_id(&self) -> &str {
        &self.device_id
    }

    /// Replace the runtime peer list. Cursors for dropped peers stay in
    /// Sled — re-adding the peer resumes from the persisted cursor.
    pub async fn set_peers(&self, peer_device_ids: Vec<String>) {
        self.config.lock().await.peer_device_ids = peer_device_ids;
    }

    /// Snapshot of the current peer list (used by tests and observability).
    pub async fn peers(&self) -> Vec<String> {
        self.config.lock().await.peer_device_ids.clone()
    }

    /// Number of pending entries waiting to be flushed.
    pub async fn pending_count(&self) -> usize {
        self.pending.lock().await.len()
    }

    /// Load persisted state (seq counter + per-peer cursors) from Sled.
    ///
    /// Safe to call multiple times — later calls overwrite the in-memory
    /// state. Errors propagate so the caller can fail fast at startup
    /// rather than swallowing a corrupt state file.
    pub async fn load_state(&self) -> SyncResult<()> {
        let kv = self.store.open_namespace(P2P_STATE_NAMESPACE).await?;

        // Seq counter
        if let Some(bytes) = kv.get(SEQ_COUNTER_KEY).await? {
            if bytes.len() != 8 {
                return Err(SyncError::Storage(format!(
                    "p2p seq counter has invalid length: {} (expected 8)",
                    bytes.len()
                )));
            }
            let seq = u64::from_be_bytes(bytes.try_into().unwrap_or([0; 8]));
            *self.seq_counter.lock().await = seq;
        }

        // Per-peer cursors
        let pairs = kv.scan_prefix(CURSOR_KEY_PREFIX).await?;
        let mut cursors = self.download_cursors.lock().await;
        for (key_bytes, val_bytes) in pairs {
            let suffix = key_bytes
                .strip_prefix(CURSOR_KEY_PREFIX)
                .ok_or_else(|| SyncError::Storage("p2p cursor key prefix mismatch".to_string()))?;
            let peer_id = std::str::from_utf8(suffix)
                .map_err(|e| SyncError::Storage(format!("p2p cursor key not utf8: {e}")))?
                .to_string();
            if val_bytes.len() != 8 {
                return Err(SyncError::Storage(format!(
                    "p2p cursor for {peer_id} has invalid length: {} (expected 8)",
                    val_bytes.len()
                )));
            }
            let seq = u64::from_be_bytes(val_bytes.try_into().unwrap_or([0; 8]));
            cursors.insert(peer_id, seq);
        }
        Ok(())
    }

    /// Build the next monotonic seq, persist the bumped counter to Sled.
    ///
    /// Persists synchronously so a crash between `make_entry` and a
    /// successful upload still advances the counter — preventing two
    /// devices (or a restarted same device) from ever minting the same
    /// p2p key.
    async fn next_seq(&self) -> SyncResult<u64> {
        let now_nanos = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap_or_default()
            .as_nanos() as u64;
        let mut last = self.seq_counter.lock().await;
        let seq = if now_nanos <= *last {
            *last + 1
        } else {
            now_nanos
        };
        *last = seq;

        let kv = self.store.open_namespace(P2P_STATE_NAMESPACE).await?;
        kv.put(SEQ_COUNTER_KEY, seq.to_be_bytes().to_vec()).await?;
        Ok(seq)
    }

    /// Append a write to the pending push queue.
    ///
    /// The caller is responsible for calling [`flush_pending`] (typically
    /// after a Sled batch commits, or on a timer).
    pub async fn record_op(&self, op: LogOp) -> SyncResult<()> {
        let seq = self.next_seq().await?;
        let now = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap_or_default();
        let entry = LogEntry {
            seq,
            timestamp_ms: now.as_millis() as u64,
            device_id: self.device_id.clone(),
            op,
        };
        self.pending.lock().await.push(entry);
        Ok(())
    }

    /// Push every pending entry to every peer's mailbox.
    ///
    /// Returns the number of (entry, peer) PUTs that succeeded. Failures
    /// are returned as the first error encountered — pending entries
    /// that have already gone to *some* peers are still drained, on the
    /// principle that the durable `SyncEngine` is the safety net (a peer
    /// that missed the p2p shortcut catches up via the next snapshot
    /// bootstrap).
    pub async fn flush_pending(&self) -> SyncResult<usize> {
        let entries: Vec<LogEntry> = {
            let mut pending = self.pending.lock().await;
            std::mem::take(&mut *pending)
        };
        if entries.is_empty() {
            return Ok(0);
        }
        let peers = self.config.lock().await.peer_device_ids.clone();
        if peers.is_empty() {
            // No peers configured — drop pending entries silently is
            // wrong (could leak an unbounded queue). Re-stuff them so a
            // future `set_peers` flush can pick them up.
            self.pending.lock().await.extend(entries);
            return Ok(0);
        }

        let mut puts = 0usize;
        let mut first_error: Option<SyncError> = None;

        for entry in &entries {
            let sealed = entry.seal(&self.crypto).await?;
            for peer in &peers {
                let urls = self
                    .auth
                    .presign_p2p_upload(&self.device_id, peer, &[entry.seq])
                    .await?;
                let url = urls.into_iter().next().ok_or_else(|| {
                    SyncError::Auth("presign_p2p_upload returned no URLs".to_string())
                })?;
                match self.s3.upload(&url, sealed.bytes.clone()).await {
                    Ok(()) => puts += 1,
                    Err(e) => {
                        if first_error.is_none() {
                            first_error = Some(e);
                        }
                    }
                }
            }
        }

        if let Some(e) = first_error {
            return Err(e);
        }
        Ok(puts)
    }

    /// Poll every configured peer's outbound mailbox and apply new entries.
    ///
    /// Returns the total number of entries applied across all peers.
    pub async fn poll_all_peers(&self) -> SyncResult<usize> {
        let peers = self.config.lock().await.peer_device_ids.clone();
        let mut applied = 0usize;
        for peer in peers {
            applied += self.poll_peer(&peer).await?;
        }
        Ok(applied)
    }

    /// Poll a single peer's outbound mailbox and apply new entries.
    pub async fn poll_peer(&self, peer_id: &str) -> SyncResult<usize> {
        let cursor = self
            .download_cursors
            .lock()
            .await
            .get(peer_id)
            .copied()
            .unwrap_or(0);

        let objects = self.auth.list_p2p_objects(peer_id, &self.device_id).await?;

        // Filter to keys with seq > cursor and parse them.
        // Keys come back as `p2p/<src>__<dst>/<seq>.enc` (relative).
        let mailbox_prefix = format!("p2p/{peer_id}__{}/", self.device_id);
        let mut new_seqs: Vec<u64> = objects
            .iter()
            .filter_map(|obj| {
                let suffix = obj.key.strip_prefix(&mailbox_prefix)?;
                let seq_str = suffix.strip_suffix(".enc")?;
                seq_str.parse::<u64>().ok()
            })
            .filter(|s| *s > cursor)
            .collect();
        new_seqs.sort_unstable();

        if new_seqs.is_empty() {
            return Ok(0);
        }

        // Batch the seqs for download. The server enforces a 1000-seq
        // ceiling per request; we chunk to stay well under it.
        const CHUNK: usize = 500;
        let mut applied = 0usize;
        let mut highest_applied = cursor;

        for chunk in new_seqs.chunks(CHUNK) {
            let urls = self
                .auth
                .presign_p2p_download(peer_id, &self.device_id, chunk)
                .await?;
            if urls.len() != chunk.len() {
                return Err(SyncError::S3(format!(
                    "presign_p2p_download returned {} URLs for {} seqs",
                    urls.len(),
                    chunk.len()
                )));
            }

            for (seq, url) in chunk.iter().zip(urls) {
                let bytes = match self.s3.download(&url).await? {
                    Some(b) => b,
                    // 24h lifecycle may have evicted between list and
                    // download — skip silently and let the durable sync
                    // catch us up.
                    None => continue,
                };
                let entry = LogEntry::unseal(&bytes, &self.crypto).await?;
                if entry.device_id != peer_id {
                    return Err(SyncError::CorruptEntry {
                        seq: *seq,
                        reason: format!(
                            "p2p entry device_id {} does not match mailbox sender {}",
                            entry.device_id, peer_id
                        ),
                    });
                }
                self.apply_entry(&entry).await?;
                applied += 1;
                if *seq > highest_applied {
                    highest_applied = *seq;
                }
            }
        }

        // Persist the advanced cursor.
        if highest_applied > cursor {
            self.write_cursor(peer_id, highest_applied).await?;
        }
        Ok(applied)
    }

    async fn write_cursor(&self, peer_id: &str, seq: u64) -> SyncResult<()> {
        let kv = self.store.open_namespace(P2P_STATE_NAMESPACE).await?;
        let mut key = Vec::with_capacity(CURSOR_KEY_PREFIX.len() + peer_id.len());
        key.extend_from_slice(CURSOR_KEY_PREFIX);
        key.extend_from_slice(peer_id.as_bytes());
        kv.put(&key, seq.to_be_bytes().to_vec()).await?;
        self.download_cursors
            .lock()
            .await
            .insert(peer_id.to_string(), seq);
        Ok(())
    }

    /// Apply a peer's log op to the local store.
    ///
    /// p2p deltas are not subject to molecule-merge / ref-key handling —
    /// that's the durable `SyncEngine`'s job. p2p is a fast unconditional
    /// shortcut; if two devices write the same key concurrently, the
    /// later durable replay rewrites the loser. (Per the design: explicit
    /// p2p ACK is intentionally not implemented.)
    async fn apply_entry(&self, entry: &LogEntry) -> SyncResult<()> {
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
                    .collect::<SyncResult<_>>()?;
                kv.batch_put(decoded).await?;
            }
            LogOp::BatchDelete { namespace, keys } => {
                let kv = self.store.open_namespace(namespace).await?;
                let decoded: Vec<Vec<u8>> = keys
                    .iter()
                    .map(|k| LogOp::decode_bytes(k))
                    .collect::<SyncResult<_>>()?;
                kv.batch_delete(decoded).await?;
            }
        }
        Ok(())
    }

    /// Returns the current download cursor for a peer (0 if never synced).
    pub async fn cursor_for_peer(&self, peer_id: &str) -> u64 {
        self.download_cursors
            .lock()
            .await
            .get(peer_id)
            .copied()
            .unwrap_or(0)
    }
}

/// Background polling loop. Wakes every `poll_interval_ms`, flushes any
/// pending pushes, then polls each configured peer.
///
/// Runs until the engine is dropped (the loop holds a `Weak` so it doesn't
/// keep the engine alive past its useful life).
pub fn spawn_polling_loop(engine: Arc<P2pSyncEngine>) -> tokio::task::JoinHandle<()> {
    use tracing::Instrument;
    let weak = Arc::downgrade(&engine);
    drop(engine);
    let span = tracing::info_span!("p2p_polling_loop");
    tokio::spawn(
        async move {
            // Resolve the actual interval once we can `.await`. If the
            // engine is already gone (caller dropped between spawn and
            // first tick), exit cleanly.
            let Some(first) = weak.upgrade() else { return };
            let interval = first.config.lock().await.poll_interval_ms.max(1);
            drop(first);

            let mut ticker = tokio::time::interval(std::time::Duration::from_millis(interval));
            // First tick fires immediately; skip it so we don't thrash on
            // startup.
            ticker.tick().await;
            loop {
                ticker.tick().await;
                let Some(eng) = weak.upgrade() else { break };
                if let Err(e) = eng.flush_pending().await {
                    tracing::warn!(error = %e, "p2p flush_pending failed");
                }
                if let Err(e) = eng.poll_all_peers().await {
                    tracing::warn!(error = %e, "p2p poll_all_peers failed");
                }
            }
        }
        .instrument(span),
    )
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::crypto::provider::LocalCryptoProvider;
    use crate::storage::inmemory_backend::InMemoryNamespacedStore;

    fn test_crypto() -> Arc<dyn CryptoProvider> {
        Arc::new(LocalCryptoProvider::from_key([0x42u8; 32]))
    }

    /// Build a p2p engine with a stub AuthClient — useful for tests that
    /// only exercise local state (cursor persistence, seq counter, apply).
    /// HTTP-bound paths (`flush_pending`, `poll_peer`) need a live mock
    /// server; those live in the integration tests file.
    fn local_engine(device_id: &str, store: Arc<dyn NamespacedStore>) -> P2pSyncEngine {
        use crate::sync::auth::SyncAuth;
        use reqwest::Client;
        // trace-egress: skip-3p (test stub, never sends)
        let http = Arc::new(Client::new());
        let auth = Arc::new(AuthClient::new(
            http.clone(),
            "http://127.0.0.1:0".to_string(),
            SyncAuth::ApiKey("test".into()),
        ));
        let s3 = S3Client::new(http);
        P2pSyncEngine::new(
            device_id.to_string(),
            test_crypto(),
            auth,
            s3,
            store,
            P2pConfig {
                peer_device_ids: vec!["peer-b".to_string()],
                poll_interval_ms: 30_000,
            },
        )
    }

    #[tokio::test]
    async fn record_op_appends_to_pending_with_monotonic_seq() {
        let store: Arc<dyn NamespacedStore> = Arc::new(InMemoryNamespacedStore::new());
        let engine = local_engine("device-a", store);

        engine
            .record_op(LogOp::Put {
                namespace: "main".into(),
                key: LogOp::encode_bytes(b"k1"),
                value: LogOp::encode_bytes(b"v1"),
            })
            .await
            .unwrap();
        engine
            .record_op(LogOp::Delete {
                namespace: "main".into(),
                key: LogOp::encode_bytes(b"k1"),
            })
            .await
            .unwrap();

        assert_eq!(engine.pending_count().await, 2);
        let pending = engine.pending.lock().await;
        // seqs must be strictly increasing within a single process even
        // when wall-clock nanos collide.
        assert!(
            pending[0].seq < pending[1].seq,
            "p2p seqs must be strictly increasing: {} vs {}",
            pending[0].seq,
            pending[1].seq
        );
    }

    #[tokio::test]
    async fn seq_counter_persists_across_engine_instances() {
        let store: Arc<dyn NamespacedStore> = Arc::new(InMemoryNamespacedStore::new());
        let engine1 = local_engine("device-a", store.clone());
        engine1
            .record_op(LogOp::Put {
                namespace: "main".into(),
                key: LogOp::encode_bytes(b"k"),
                value: LogOp::encode_bytes(b"v"),
            })
            .await
            .unwrap();
        let first_seq = engine1.pending.lock().await[0].seq;
        drop(engine1);

        // A fresh engine on the same store must observe a seq strictly
        // greater than any seq the previous instance minted — otherwise
        // a restarted device could overwrite a peer's older delta.
        let engine2 = local_engine("device-a", store);
        engine2.load_state().await.unwrap();
        let next = engine2.next_seq().await.unwrap();
        assert!(
            next > first_seq,
            "persisted seq counter must monotonically advance across restarts: {next} > {first_seq}"
        );
    }

    #[tokio::test]
    async fn cursor_round_trips_through_load_state() {
        let store: Arc<dyn NamespacedStore> = Arc::new(InMemoryNamespacedStore::new());
        let engine1 = local_engine("device-a", store.clone());
        engine1.write_cursor("peer-b", 12345).await.unwrap();
        drop(engine1);

        let engine2 = local_engine("device-a", store);
        engine2.load_state().await.unwrap();
        assert_eq!(engine2.cursor_for_peer("peer-b").await, 12345);
        assert_eq!(engine2.cursor_for_peer("unknown").await, 0);
    }

    #[tokio::test]
    async fn apply_entry_writes_put_and_delete_to_namespace() {
        let store: Arc<dyn NamespacedStore> = Arc::new(InMemoryNamespacedStore::new());
        let engine = local_engine("device-a", store.clone());

        let put = LogEntry {
            seq: 1,
            timestamp_ms: 1000,
            device_id: "peer-b".into(),
            op: LogOp::Put {
                namespace: "main".into(),
                key: LogOp::encode_bytes(b"foo"),
                value: LogOp::encode_bytes(b"bar"),
            },
        };
        engine.apply_entry(&put).await.unwrap();
        let kv = store.open_namespace("main").await.unwrap();
        assert_eq!(kv.get(b"foo").await.unwrap().as_deref(), Some(&b"bar"[..]));

        let del = LogEntry {
            seq: 2,
            timestamp_ms: 1001,
            device_id: "peer-b".into(),
            op: LogOp::Delete {
                namespace: "main".into(),
                key: LogOp::encode_bytes(b"foo"),
            },
        };
        engine.apply_entry(&del).await.unwrap();
        assert_eq!(kv.get(b"foo").await.unwrap(), None);
    }

    #[tokio::test]
    async fn apply_entry_handles_batch_ops() {
        let store: Arc<dyn NamespacedStore> = Arc::new(InMemoryNamespacedStore::new());
        let engine = local_engine("device-a", store.clone());

        let batch = LogEntry {
            seq: 5,
            timestamp_ms: 100,
            device_id: "peer-b".into(),
            op: LogOp::BatchPut {
                namespace: "main".into(),
                items: vec![
                    (LogOp::encode_bytes(b"a"), LogOp::encode_bytes(b"1")),
                    (LogOp::encode_bytes(b"b"), LogOp::encode_bytes(b"2")),
                ],
            },
        };
        engine.apply_entry(&batch).await.unwrap();
        let kv = store.open_namespace("main").await.unwrap();
        assert_eq!(kv.get(b"a").await.unwrap().as_deref(), Some(&b"1"[..]));
        assert_eq!(kv.get(b"b").await.unwrap().as_deref(), Some(&b"2"[..]));

        let del = LogEntry {
            seq: 6,
            timestamp_ms: 101,
            device_id: "peer-b".into(),
            op: LogOp::BatchDelete {
                namespace: "main".into(),
                keys: vec![LogOp::encode_bytes(b"a"), LogOp::encode_bytes(b"b")],
            },
        };
        engine.apply_entry(&del).await.unwrap();
        assert_eq!(kv.get(b"a").await.unwrap(), None);
        assert_eq!(kv.get(b"b").await.unwrap(), None);
    }

    #[tokio::test]
    async fn flush_pending_with_no_peers_keeps_entries_queued() {
        let store: Arc<dyn NamespacedStore> = Arc::new(InMemoryNamespacedStore::new());
        let engine = local_engine("device-a", store);
        engine.set_peers(Vec::new()).await;

        engine
            .record_op(LogOp::Put {
                namespace: "main".into(),
                key: LogOp::encode_bytes(b"k"),
                value: LogOp::encode_bytes(b"v"),
            })
            .await
            .unwrap();

        // No peers — flush is a no-op, but the entry must NOT be dropped
        // (silently dropping would lose data the moment a peer is added).
        let flushed = engine.flush_pending().await.unwrap();
        assert_eq!(flushed, 0);
        assert_eq!(engine.pending_count().await, 1);
    }

    #[test]
    fn p2p_config_default_uses_30s_interval() {
        let cfg = P2pConfig::default();
        assert_eq!(cfg.poll_interval_ms, 30_000);
        assert!(cfg.peer_device_ids.is_empty());
    }

    /// p2p reuses the durable log's `LogEntry::seal` / `unseal` wire
    /// format. Document the contract so any future change to `log.rs`
    /// that breaks p2p compatibility is caught here, not silently in
    /// production where peers stop being able to read each other.
    #[tokio::test]
    async fn seal_then_apply_round_trip_uses_log_entry_format() {
        let store: Arc<dyn NamespacedStore> = Arc::new(InMemoryNamespacedStore::new());
        let engine = local_engine("device-a", store.clone());

        // Producer side: seal a put exactly the way `flush_pending` does.
        let entry = LogEntry {
            seq: 99,
            timestamp_ms: 12345,
            device_id: "peer-b".into(),
            op: LogOp::Put {
                namespace: "main".into(),
                key: LogOp::encode_bytes(b"the_key"),
                value: LogOp::encode_bytes(b"the_value"),
            },
        };
        let sealed = entry.seal(&engine.crypto).await.unwrap();

        // Consumer side: unseal and apply exactly the way `poll_peer` does.
        let opened = LogEntry::unseal(&sealed.bytes, &engine.crypto)
            .await
            .unwrap();
        assert_eq!(opened.seq, 99);
        assert_eq!(opened.device_id, "peer-b");
        engine.apply_entry(&opened).await.unwrap();

        let kv = store.open_namespace("main").await.unwrap();
        assert_eq!(
            kv.get(b"the_key").await.unwrap().as_deref(),
            Some(&b"the_value"[..])
        );
    }
}
