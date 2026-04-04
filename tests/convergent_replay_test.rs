//! Tests for convergent replay in the sync engine.
//!
//! Verifies that ref keys converge via LWW regardless of replay order,
//! while non-ref keys (atoms, history) are always accepted.

use fold_db::crypto::provider::LocalCryptoProvider;
use fold_db::crypto::CryptoProvider;
use fold_db::storage::inmemory_backend::InMemoryNamespacedStore;
use fold_db::storage::traits::NamespacedStore;
use fold_db::sync::auth::{AuthClient, SyncAuth};
use fold_db::sync::engine::SyncEngine;
use fold_db::sync::log::{LogEntry, LogOp};
use fold_db::sync::s3::S3Client;
use fold_db::sync::SyncConfig;
use std::sync::Arc;

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

#[tokio::test]
async fn atoms_always_accepted() {
    let store = Arc::new(InMemoryNamespacedStore::new());
    let engine = make_engine(store.clone());

    let e1 = put_entry(1, 200, "dev-a", b"atom:uuid-aaa", b"atom-data-a");
    let e2 = put_entry(2, 100, "dev-b", b"atom:uuid-bbb", b"atom-data-b");

    engine.replay_entry(&e1).await.unwrap();
    engine.replay_entry(&e2).await.unwrap();

    let kv = store.open_namespace("main").await.unwrap();
    assert_eq!(
        kv.get(b"atom:uuid-aaa").await.unwrap().unwrap(),
        b"atom-data-a"
    );
    assert_eq!(
        kv.get(b"atom:uuid-bbb").await.unwrap().unwrap(),
        b"atom-data-b"
    );
}

#[tokio::test]
async fn history_always_accepted() {
    let store = Arc::new(InMemoryNamespacedStore::new());
    let engine = make_engine(store.clone());

    let e1 = put_entry(1, 100, "dev-a", b"history:mol1:00100", b"event-a");
    let e2 = put_entry(2, 200, "dev-b", b"history:mol1:00200", b"event-b");

    engine.replay_entry(&e1).await.unwrap();
    engine.replay_entry(&e2).await.unwrap();

    let kv = store.open_namespace("main").await.unwrap();
    assert!(kv.get(b"history:mol1:00100").await.unwrap().is_some());
    assert!(kv.get(b"history:mol1:00200").await.unwrap().is_some());
}

#[tokio::test]
async fn ref_newer_overwrites_older() {
    let store = Arc::new(InMemoryNamespacedStore::new());
    let engine = make_engine(store.clone());

    let e1 = put_entry(1, 100, "dev-a", b"ref:mol-1", b"atom-aaa");
    engine.replay_entry(&e1).await.unwrap();

    let e2 = put_entry(2, 200, "dev-b", b"ref:mol-1", b"atom-bbb");
    engine.replay_entry(&e2).await.unwrap();

    let kv = store.open_namespace("main").await.unwrap();
    assert_eq!(kv.get(b"ref:mol-1").await.unwrap().unwrap(), b"atom-bbb");
}

#[tokio::test]
async fn ref_older_does_not_overwrite_newer() {
    let store = Arc::new(InMemoryNamespacedStore::new());
    let engine = make_engine(store.clone());

    let e1 = put_entry(1, 200, "dev-a", b"ref:mol-1", b"atom-aaa");
    engine.replay_entry(&e1).await.unwrap();

    let e2 = put_entry(2, 100, "dev-b", b"ref:mol-1", b"atom-bbb");
    engine.replay_entry(&e2).await.unwrap();

    let kv = store.open_namespace("main").await.unwrap();
    assert_eq!(kv.get(b"ref:mol-1").await.unwrap().unwrap(), b"atom-aaa");
}

#[tokio::test]
async fn ref_same_timestamp_tiebreak_by_device_id() {
    let store = Arc::new(InMemoryNamespacedStore::new());
    let engine = make_engine(store.clone());

    let e1 = put_entry(1, 100, "aaa", b"ref:mol-1", b"val-aaa");
    engine.replay_entry(&e1).await.unwrap();

    let e2 = put_entry(2, 100, "bbb", b"ref:mol-1", b"val-bbb");
    engine.replay_entry(&e2).await.unwrap();

    let kv = store.open_namespace("main").await.unwrap();
    assert_eq!(kv.get(b"ref:mol-1").await.unwrap().unwrap(), b"val-bbb");
}

#[tokio::test]
async fn convergence_independent_of_replay_order() {
    let entries = vec![
        put_entry(1, 100, "dev-a", b"ref:mol-1", b"val-100"),
        put_entry(2, 300, "dev-b", b"ref:mol-1", b"val-300"),
        put_entry(3, 200, "dev-c", b"ref:mol-1", b"val-200"),
    ];

    let store1 = Arc::new(InMemoryNamespacedStore::new());
    let engine1 = make_engine(store1.clone());
    for e in &entries {
        engine1.replay_entry(e).await.unwrap();
    }

    let store2 = Arc::new(InMemoryNamespacedStore::new());
    let engine2 = make_engine(store2.clone());
    for e in entries.iter().rev() {
        engine2.replay_entry(e).await.unwrap();
    }

    let kv1 = store1.open_namespace("main").await.unwrap();
    let kv2 = store2.open_namespace("main").await.unwrap();

    let v1 = kv1.get(b"ref:mol-1").await.unwrap().unwrap();
    let v2 = kv2.get(b"ref:mol-1").await.unwrap().unwrap();

    assert_eq!(v1, b"val-300");
    assert_eq!(v2, b"val-300");
}

#[tokio::test]
async fn org_prefixed_ref_converges() {
    let store = Arc::new(InMemoryNamespacedStore::new());
    let engine = make_engine(store.clone());

    let e1 = put_entry(1, 200, "dev-a", b"org_abc:ref:mol-1", b"val-new");
    engine.replay_entry(&e1).await.unwrap();

    let e2 = put_entry(2, 100, "dev-b", b"org_abc:ref:mol-1", b"val-old");
    engine.replay_entry(&e2).await.unwrap();

    let kv = store.open_namespace("main").await.unwrap();
    assert_eq!(
        kv.get(b"org_abc:ref:mol-1").await.unwrap().unwrap(),
        b"val-new"
    );
}
