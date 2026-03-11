//! Integration tests for the S3 sync system.
//!
//! These tests exercise the full local sync flow without a real S3 backend:
//! - SyncingNamespacedStore recording writes to SyncEngine
//! - Encrypted log entry seal/unseal roundtrips
//! - Snapshot create → seal → unseal → restore
//! - Full write-through-sync-restore cycle

use fold_db::crypto::provider::LocalCryptoProvider;
use fold_db::crypto::CryptoProvider;
use fold_db::storage::encrypting_namespaced_store::EncryptingNamespacedStore;
use fold_db::storage::inmemory_backend::InMemoryNamespacedStore;
use fold_db::storage::syncing_namespaced_store::SyncingNamespacedStore;
use fold_db::storage::traits::NamespacedStore;
use fold_db::sync::auth::{AuthClient, SyncAuth};
use fold_db::sync::s3::S3Client;
use fold_db::sync::snapshot::Snapshot;
use fold_db::sync::{SyncConfig, SyncEngine, SyncState};
use std::sync::Arc;

fn test_crypto() -> Arc<dyn CryptoProvider> {
    Arc::new(LocalCryptoProvider::from_key([0x42u8; 32]))
}

/// Build the full storage stack: InMemory → Syncing → Encrypting
async fn build_stack() -> (
    Arc<dyn NamespacedStore>, // top-level store (encrypting)
    Arc<SyncEngine>,
    Arc<InMemoryNamespacedStore>, // raw base store (for snapshot verification)
) {
    let base = Arc::new(InMemoryNamespacedStore::new());
    let crypto = test_crypto();

    let http = Arc::new(reqwest::Client::new());
    let s3 = S3Client::new(http.clone());
    let auth = AuthClient::new(
        http,
        "http://localhost:0".to_string(), // not used in these tests
        SyncAuth::ApiKey("test".to_string()),
    );

    let engine = Arc::new(SyncEngine::new(
        "test-device".to_string(),
        crypto.clone(),
        s3,
        auth,
        base.clone() as Arc<dyn NamespacedStore>,
        SyncConfig::default(),
    ));

    // Stack: base → syncing → encrypting
    let syncing = SyncingNamespacedStore::new(
        base.clone() as Arc<dyn NamespacedStore>,
        engine.clone(),
    );
    let syncing_arc = Arc::new(syncing) as Arc<dyn NamespacedStore>;

    let encrypting = EncryptingNamespacedStore::new(syncing_arc, crypto, false);
    let top = Arc::new(encrypting) as Arc<dyn NamespacedStore>;

    (top, engine, base)
}

#[tokio::test]
async fn writes_flow_through_full_stack() {
    let (store, engine, _base) = build_stack().await;

    assert_eq!(engine.state().await, SyncState::Idle);
    assert_eq!(engine.pending_count().await, 0);

    // Write through the full stack (encrypting → syncing → inmemory)
    let main = store.open_namespace("main").await.unwrap();
    main.put(b"key1", b"value1".to_vec()).await.unwrap();

    // SyncEngine should have recorded the write
    assert_eq!(engine.state().await, SyncState::Dirty);
    assert_eq!(engine.pending_count().await, 1);

    // Data should be readable back through the encrypting layer
    let val = main.get(b"key1").await.unwrap();
    assert_eq!(val, Some(b"value1".to_vec()));
}

#[tokio::test]
async fn multiple_namespaces_all_recorded() {
    let (store, engine, _base) = build_stack().await;

    let main = store.open_namespace("main").await.unwrap();
    let meta = store.open_namespace("metadata").await.unwrap();

    main.put(b"atom:1", b"data1".to_vec()).await.unwrap();
    main.put(b"atom:2", b"data2".to_vec()).await.unwrap();
    meta.put(b"schema:foo", b"schema".to_vec()).await.unwrap();

    assert_eq!(engine.pending_count().await, 3);
    assert_eq!(engine.state().await, SyncState::Dirty);
}

#[tokio::test]
async fn reads_dont_trigger_sync() {
    let (store, engine, _base) = build_stack().await;

    let main = store.open_namespace("main").await.unwrap();
    main.put(b"key1", b"val1".to_vec()).await.unwrap();
    assert_eq!(engine.pending_count().await, 1);

    // These reads should NOT add pending entries
    main.get(b"key1").await.unwrap();
    main.exists(b"key1").await.unwrap();
    main.scan_prefix(b"key").await.unwrap();

    assert_eq!(engine.pending_count().await, 1);
}

#[tokio::test]
async fn delete_recorded() {
    let (store, engine, _base) = build_stack().await;

    let main = store.open_namespace("main").await.unwrap();
    main.put(b"key1", b"val1".to_vec()).await.unwrap();
    assert_eq!(engine.pending_count().await, 1);

    main.delete(b"key1").await.unwrap();
    assert_eq!(engine.pending_count().await, 2); // put + delete

    let val = main.get(b"key1").await.unwrap();
    assert_eq!(val, None);
}

#[tokio::test]
async fn batch_operations_recorded() {
    let (store, engine, _base) = build_stack().await;

    let main = store.open_namespace("main").await.unwrap();
    let items = vec![
        (b"k1".to_vec(), b"v1".to_vec()),
        (b"k2".to_vec(), b"v2".to_vec()),
        (b"k3".to_vec(), b"v3".to_vec()),
    ];
    main.batch_put(items).await.unwrap();

    assert_eq!(engine.pending_count().await, 1); // one batch op

    // Verify all items readable
    assert_eq!(main.get(b"k1").await.unwrap(), Some(b"v1".to_vec()));
    assert_eq!(main.get(b"k2").await.unwrap(), Some(b"v2".to_vec()));
    assert_eq!(main.get(b"k3").await.unwrap(), Some(b"v3".to_vec()));
}

#[tokio::test]
async fn snapshot_captures_encrypted_data_and_restores() {
    let (store, _engine, base) = build_stack().await;
    let crypto = test_crypto();

    // Write data through the encrypting stack
    let main = store.open_namespace("main").await.unwrap();
    main.put(b"atom:1", b"hello".to_vec()).await.unwrap();
    main.put(b"atom:2", b"world".to_vec()).await.unwrap();

    let meta = store.open_namespace("metadata").await.unwrap();
    meta.put(b"schema:test", b"schema_data".to_vec())
        .await
        .unwrap();

    // Create snapshot from the BASE store (contains encrypted data)
    let snapshot = Snapshot::create(base.as_ref(), "test-device", 5)
        .await
        .unwrap();

    assert_eq!(snapshot.device_id, "test-device");
    assert_eq!(snapshot.last_seq, 5);
    assert!(snapshot.namespaces.len() >= 2); // main + metadata

    // Seal and unseal the snapshot
    let sealed = snapshot.seal(&crypto).await.unwrap();
    let restored_snapshot = Snapshot::unseal(&sealed, &crypto).await.unwrap();

    assert_eq!(restored_snapshot.device_id, "test-device");
    assert_eq!(restored_snapshot.last_seq, 5);

    // Restore to a fresh store
    let target = InMemoryNamespacedStore::new();
    restored_snapshot.restore(&target).await.unwrap();

    // The restored data is encrypted (it was stored encrypted in the base store).
    // Wrap target with EncryptingNamespacedStore to decrypt and read back.
    let target_enc = EncryptingNamespacedStore::new(
        Arc::new(target) as Arc<dyn NamespacedStore>,
        crypto,
        false,
    );

    let restored_main = target_enc.open_namespace("main").await.unwrap();
    assert_eq!(
        restored_main.get(b"atom:1").await.unwrap(),
        Some(b"hello".to_vec())
    );
    assert_eq!(
        restored_main.get(b"atom:2").await.unwrap(),
        Some(b"world".to_vec())
    );

    let restored_meta = target_enc.open_namespace("metadata").await.unwrap();
    assert_eq!(
        restored_meta.get(b"schema:test").await.unwrap(),
        Some(b"schema_data".to_vec())
    );
}

#[tokio::test]
async fn snapshot_wrong_key_fails() {
    let (store, _engine, base) = build_stack().await;
    let crypto = test_crypto();
    let wrong_crypto: Arc<dyn CryptoProvider> =
        Arc::new(LocalCryptoProvider::from_key([0x99u8; 32]));

    let main = store.open_namespace("main").await.unwrap();
    main.put(b"secret", b"data".to_vec()).await.unwrap();

    let snapshot = Snapshot::create(base.as_ref(), "dev-1", 1)
        .await
        .unwrap();
    let sealed = snapshot.seal(&crypto).await.unwrap();

    // Unseal with wrong key should fail
    let result = Snapshot::unseal(&sealed, &wrong_crypto).await;
    assert!(result.is_err());
}

#[tokio::test]
async fn list_namespaces_works_through_stack() {
    let (store, _engine, _base) = build_stack().await;

    store.open_namespace("main").await.unwrap();
    store.open_namespace("metadata").await.unwrap();
    store.open_namespace("schemas").await.unwrap();

    let namespaces = store.list_namespaces().await.unwrap();
    assert!(namespaces.contains(&"main".to_string()));
    assert!(namespaces.contains(&"metadata".to_string()));
    assert!(namespaces.contains(&"schemas".to_string()));
}
