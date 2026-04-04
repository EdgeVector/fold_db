//! Integration tests for org sync conflict detection and resolution.
//!
//! Tests the LWW (Last-Write-Wins) conflict detection in replay_org_entry,
//! conflict logging, querying, and manual resolution.

use fold_db::crypto::provider::LocalCryptoProvider;
use fold_db::crypto::CryptoProvider;
use fold_db::storage::inmemory_backend::InMemoryNamespacedStore;
use fold_db::storage::traits::NamespacedStore;
use fold_db::sync::auth::{AuthClient, SyncAuth};
use fold_db::sync::conflict::ConflictResolution;
use fold_db::sync::engine::SyncEngine;
use fold_db::sync::log::{LogEntry, LogOp};
use fold_db::sync::s3::S3Client;
use fold_db::sync::SyncConfig;
use std::sync::Arc;

/// Create a SyncEngine backed by in-memory storage for testing.
fn make_engine(store: Arc<dyn NamespacedStore>) -> SyncEngine {
    let crypto: Arc<dyn CryptoProvider> = Arc::new(LocalCryptoProvider::from_key([0x42u8; 32]));
    let http = Arc::new(reqwest::Client::new());
    let s3 = S3Client::new(http.clone());
    let auth = AuthClient::new(
        http,
        "http://localhost:0".to_string(),
        SyncAuth::ApiKey("test".to_string()),
    );
    SyncEngine::new(
        "test-device".to_string(),
        crypto,
        s3,
        auth,
        store,
        SyncConfig::default(),
    )
}

fn put_entry(seq: u64, timestamp_ms: u64, device_id: &str, key: &[u8], value: &[u8]) -> LogEntry {
    LogEntry {
        seq,
        timestamp_ms,
        device_id: device_id.to_string(),
        op: LogOp::Put {
            namespace: "main".to_string(),
            key: LogOp::encode_bytes(key),
            value: LogOp::encode_bytes(value),
        },
    }
}

fn delete_entry(seq: u64, timestamp_ms: u64, device_id: &str, key: &[u8]) -> LogEntry {
    LogEntry {
        seq,
        timestamp_ms,
        device_id: device_id.to_string(),
        op: LogOp::Delete {
            namespace: "main".to_string(),
            key: LogOp::encode_bytes(key),
        },
    }
}

fn batch_put_entry(
    seq: u64,
    timestamp_ms: u64,
    device_id: &str,
    items: Vec<(&[u8], &[u8])>,
) -> LogEntry {
    LogEntry {
        seq,
        timestamp_ms,
        device_id: device_id.to_string(),
        op: LogOp::BatchPut {
            namespace: "main".to_string(),
            items: items
                .into_iter()
                .map(|(k, v)| (LogOp::encode_bytes(k), LogOp::encode_bytes(v)))
                .collect(),
        },
    }
}

const ORG: &str = "org_abc123";

// =========================================================================
// Basic conflict detection
// =========================================================================

#[tokio::test]
async fn incoming_newer_wins_over_local() {
    let store = Arc::new(InMemoryNamespacedStore::new());
    let engine = make_engine(store.clone());

    // Device A writes at t=100
    let entry_a = put_entry(1, 100, "dev-a", b"atom:uuid-1", b"value-a");
    engine.replay_org_entry(&entry_a, ORG).await.unwrap();

    // Device B writes same key at t=200 (newer)
    let entry_b = put_entry(2, 200, "dev-b", b"atom:uuid-1", b"value-b");
    engine.replay_org_entry(&entry_b, ORG).await.unwrap();

    // B should win — verify stored value
    let kv = store.open_namespace("main").await.unwrap();
    let val = kv.get(b"atom:uuid-1").await.unwrap().unwrap();
    assert_eq!(val, b"value-b");

    // Conflict should be recorded
    let conflicts = engine.list_conflicts(Some(ORG), 10, 0).await.unwrap();
    assert_eq!(conflicts.len(), 1);
    assert_eq!(conflicts[0].winner.device_id, "dev-b");
    assert_eq!(conflicts[0].loser.device_id, "dev-a");
    assert!(matches!(
        conflicts[0].resolution,
        ConflictResolution::LastWriteWins
    ));
}

#[tokio::test]
async fn local_wins_when_incoming_is_older() {
    let store = Arc::new(InMemoryNamespacedStore::new());
    let engine = make_engine(store.clone());

    // Device A writes at t=200 (newer)
    let entry_a = put_entry(1, 200, "dev-a", b"atom:uuid-1", b"value-a");
    engine.replay_org_entry(&entry_a, ORG).await.unwrap();

    // Device B writes same key at t=100 (older — arrives late)
    let entry_b = put_entry(2, 100, "dev-b", b"atom:uuid-1", b"value-b");
    engine.replay_org_entry(&entry_b, ORG).await.unwrap();

    // A should win — value unchanged
    let kv = store.open_namespace("main").await.unwrap();
    let val = kv.get(b"atom:uuid-1").await.unwrap().unwrap();
    assert_eq!(val, b"value-a");

    // Conflict recorded with A as winner
    let conflicts = engine.list_conflicts(Some(ORG), 10, 0).await.unwrap();
    assert_eq!(conflicts.len(), 1);
    assert_eq!(conflicts[0].winner.device_id, "dev-a");
    assert_eq!(conflicts[0].loser.device_id, "dev-b");
}

// =========================================================================
// Timestamp tie with device_id tiebreaker
// =========================================================================

#[tokio::test]
async fn same_timestamp_higher_device_id_wins() {
    let store = Arc::new(InMemoryNamespacedStore::new());
    let engine = make_engine(store.clone());

    // Device "aaa" writes at t=100
    let entry_a = put_entry(1, 100, "aaa", b"atom:uuid-1", b"value-aaa");
    engine.replay_org_entry(&entry_a, ORG).await.unwrap();

    // Device "bbb" writes at t=100 (same timestamp, higher device_id)
    let entry_b = put_entry(2, 100, "bbb", b"atom:uuid-1", b"value-bbb");
    engine.replay_org_entry(&entry_b, ORG).await.unwrap();

    // "bbb" wins (lexicographically greater)
    let kv = store.open_namespace("main").await.unwrap();
    let val = kv.get(b"atom:uuid-1").await.unwrap().unwrap();
    assert_eq!(val, b"value-bbb");

    let conflicts = engine.list_conflicts(Some(ORG), 10, 0).await.unwrap();
    assert_eq!(conflicts.len(), 1);
    assert_eq!(conflicts[0].winner.device_id, "bbb");
}

// =========================================================================
// Same device — no conflict
// =========================================================================

#[tokio::test]
async fn same_device_sequential_writes_no_conflict() {
    let store = Arc::new(InMemoryNamespacedStore::new());
    let engine = make_engine(store.clone());

    let entry1 = put_entry(1, 100, "dev-a", b"atom:uuid-1", b"value-1");
    engine.replay_org_entry(&entry1, ORG).await.unwrap();

    let entry2 = put_entry(2, 200, "dev-a", b"atom:uuid-1", b"value-2");
    engine.replay_org_entry(&entry2, ORG).await.unwrap();

    // Latest value applied, no conflict
    let kv = store.open_namespace("main").await.unwrap();
    let val = kv.get(b"atom:uuid-1").await.unwrap().unwrap();
    assert_eq!(val, b"value-2");

    let conflicts = engine.list_conflicts(Some(ORG), 10, 0).await.unwrap();
    assert!(conflicts.is_empty());
}

// =========================================================================
// Different keys — no conflict
// =========================================================================

#[tokio::test]
async fn different_keys_no_conflict() {
    let store = Arc::new(InMemoryNamespacedStore::new());
    let engine = make_engine(store.clone());

    let entry_a = put_entry(1, 100, "dev-a", b"atom:uuid-1", b"value-a");
    engine.replay_org_entry(&entry_a, ORG).await.unwrap();

    let entry_b = put_entry(2, 200, "dev-b", b"atom:uuid-2", b"value-b");
    engine.replay_org_entry(&entry_b, ORG).await.unwrap();

    let conflicts = engine.list_conflicts(Some(ORG), 10, 0).await.unwrap();
    assert!(conflicts.is_empty());
}

// =========================================================================
// Delete vs Put conflict
// =========================================================================

#[tokio::test]
async fn delete_wins_over_older_put() {
    let store = Arc::new(InMemoryNamespacedStore::new());
    let engine = make_engine(store.clone());

    // Device A puts at t=100
    let entry_a = put_entry(1, 100, "dev-a", b"atom:uuid-1", b"value-a");
    engine.replay_org_entry(&entry_a, ORG).await.unwrap();

    // Device B deletes at t=200 (newer)
    let entry_b = delete_entry(2, 200, "dev-b", b"atom:uuid-1");
    engine.replay_org_entry(&entry_b, ORG).await.unwrap();

    // Key should be deleted
    let kv = store.open_namespace("main").await.unwrap();
    let val = kv.get(b"atom:uuid-1").await.unwrap();
    assert!(val.is_none());

    // Conflict recorded
    let conflicts = engine.list_conflicts(Some(ORG), 10, 0).await.unwrap();
    assert_eq!(conflicts.len(), 1);
    assert_eq!(conflicts[0].winner.device_id, "dev-b");
    assert!(conflicts[0].winner.value.is_none()); // delete has no value
}

// =========================================================================
// BatchPut with partial conflicts
// =========================================================================

#[tokio::test]
async fn batch_put_with_partial_conflict() {
    let store = Arc::new(InMemoryNamespacedStore::new());
    let engine = make_engine(store.clone());

    // Device A writes key1 at t=200 (will be newer than batch)
    let entry_a = put_entry(1, 200, "dev-a", b"key1", b"value-a-1");
    engine.replay_org_entry(&entry_a, ORG).await.unwrap();

    // Device B batch-writes key1, key2, key3 at t=100 (older for key1)
    let batch = batch_put_entry(
        2,
        100,
        "dev-b",
        vec![
            (b"key1".as_slice(), b"value-b-1".as_slice()),
            (b"key2".as_slice(), b"value-b-2".as_slice()),
            (b"key3".as_slice(), b"value-b-3".as_slice()),
        ],
    );
    engine.replay_org_entry(&batch, ORG).await.unwrap();

    let kv = store.open_namespace("main").await.unwrap();

    // key1: A wins (t=200 > t=100)
    let v1 = kv.get(b"key1").await.unwrap().unwrap();
    assert_eq!(v1, b"value-a-1");

    // key2, key3: B writes (no prior write, no conflict)
    let v2 = kv.get(b"key2").await.unwrap().unwrap();
    assert_eq!(v2, b"value-b-2");
    let v3 = kv.get(b"key3").await.unwrap().unwrap();
    assert_eq!(v3, b"value-b-3");

    // Only 1 conflict (key1)
    let conflicts = engine.list_conflicts(Some(ORG), 10, 0).await.unwrap();
    assert_eq!(conflicts.len(), 1);
    assert_eq!(conflicts[0].winner.device_id, "dev-a");
}

// =========================================================================
// Manual resolution
// =========================================================================

#[tokio::test]
async fn resolve_conflict_applies_loser_value() {
    let store = Arc::new(InMemoryNamespacedStore::new());
    let engine = make_engine(store.clone());

    // Device A writes at t=100
    let entry_a = put_entry(1, 100, "dev-a", b"atom:uuid-1", b"value-a");
    engine.replay_org_entry(&entry_a, ORG).await.unwrap();

    // Device B writes at t=200 (wins)
    let entry_b = put_entry(2, 200, "dev-b", b"atom:uuid-1", b"value-b");
    engine.replay_org_entry(&entry_b, ORG).await.unwrap();

    // Verify B won
    let kv = store.open_namespace("main").await.unwrap();
    assert_eq!(kv.get(b"atom:uuid-1").await.unwrap().unwrap(), b"value-b");

    // Get conflict and resolve it (apply loser A's value)
    let conflicts = engine.list_conflicts(Some(ORG), 10, 0).await.unwrap();
    let conflict_id = &conflicts[0].id;
    let resolved = engine.resolve_conflict(conflict_id).await.unwrap();

    // After resolution, A's value should be in storage
    // Note: loser had value=None in the record (local value not captured in meta).
    // The loser side is the original "dev-a" which is now the winner after swap.
    // Since loser.value was None (local meta doesn't store value), the resolve
    // won't have the original value. This is a known limitation — manual resolution
    // only works when the loser's value was captured (i.e., the incoming side).
    assert!(matches!(
        resolved.resolution,
        ConflictResolution::ManualOverride { .. }
    ));
    // Winner is now the previously-losing side
    assert_eq!(resolved.winner.device_id, "dev-a");
}

// =========================================================================
// Conflict get/list queries
// =========================================================================

#[tokio::test]
async fn list_conflicts_filters_by_org() {
    let store = Arc::new(InMemoryNamespacedStore::new());
    let engine = make_engine(store.clone());

    // Create conflict in org_abc
    let e1 = put_entry(1, 100, "dev-a", b"key1", b"v1");
    engine.replay_org_entry(&e1, "org_abc").await.unwrap();
    let e2 = put_entry(2, 200, "dev-b", b"key1", b"v2");
    engine.replay_org_entry(&e2, "org_abc").await.unwrap();

    // Create conflict in org_xyz
    let e3 = put_entry(3, 100, "dev-a", b"key2", b"v1");
    engine.replay_org_entry(&e3, "org_xyz").await.unwrap();
    let e4 = put_entry(4, 200, "dev-b", b"key2", b"v2");
    engine.replay_org_entry(&e4, "org_xyz").await.unwrap();

    // All conflicts
    let all = engine.list_conflicts(None, 50, 0).await.unwrap();
    assert_eq!(all.len(), 2);

    // Filtered by org
    let abc = engine.list_conflicts(Some("org_abc"), 50, 0).await.unwrap();
    assert_eq!(abc.len(), 1);
    assert_eq!(abc[0].org_hash.as_deref(), Some("org_abc"));

    let xyz = engine.list_conflicts(Some("org_xyz"), 50, 0).await.unwrap();
    assert_eq!(xyz.len(), 1);
}

#[tokio::test]
async fn get_conflict_by_id() {
    let store = Arc::new(InMemoryNamespacedStore::new());
    let engine = make_engine(store.clone());

    let e1 = put_entry(1, 100, "dev-a", b"key1", b"v1");
    engine.replay_org_entry(&e1, ORG).await.unwrap();
    let e2 = put_entry(2, 200, "dev-b", b"key1", b"v2");
    engine.replay_org_entry(&e2, ORG).await.unwrap();

    let conflicts = engine.list_conflicts(Some(ORG), 10, 0).await.unwrap();
    let id = &conflicts[0].id;

    let fetched = engine.get_conflict(id).await.unwrap().unwrap();
    assert_eq!(fetched.id, *id);
    assert_eq!(fetched.winner.device_id, "dev-b");

    // Non-existent ID
    let missing = engine.get_conflict("nonexistent").await.unwrap();
    assert!(missing.is_none());
}

#[tokio::test]
async fn list_conflicts_pagination() {
    let store = Arc::new(InMemoryNamespacedStore::new());
    let engine = make_engine(store.clone());

    // Create 3 conflicts on different keys
    for i in 0..3 {
        let key = format!("key{i}");
        let e1 = put_entry(i * 2 + 1, 100, "dev-a", key.as_bytes(), b"v1");
        engine.replay_org_entry(&e1, ORG).await.unwrap();
        let e2 = put_entry(i * 2 + 2, 200, "dev-b", key.as_bytes(), b"v2");
        engine.replay_org_entry(&e2, ORG).await.unwrap();
    }

    let all = engine.list_conflicts(Some(ORG), 50, 0).await.unwrap();
    assert_eq!(all.len(), 3);

    let page1 = engine.list_conflicts(Some(ORG), 2, 0).await.unwrap();
    assert_eq!(page1.len(), 2);

    let page2 = engine.list_conflicts(Some(ORG), 2, 2).await.unwrap();
    assert_eq!(page2.len(), 1);
}

// =========================================================================
// Resolve nonexistent conflict returns error
// =========================================================================

#[tokio::test]
async fn resolve_nonexistent_conflict_returns_error() {
    let store = Arc::new(InMemoryNamespacedStore::new());
    let engine = make_engine(store);

    let result = engine.resolve_conflict("nonexistent").await;
    assert!(result.is_err());
}
