//! Integration tests for the S3 sync system.
//!
//! These tests exercise the full local sync flow without a real S3 backend:
//! - SyncingNamespacedStore recording writes to SyncEngine
//! - Encrypted log entry seal/unseal roundtrips
//! - Snapshot create → seal → unseal → restore
//! - Full write-through-sync-restore cycle

use fold_db::crypto::provider::LocalCryptoProvider;
use fold_db::crypto::CryptoProvider;
use fold_db::security::Ed25519KeyPair;
use fold_db::storage::encrypting_namespaced_store::EncryptingNamespacedStore;
use fold_db::storage::inmemory_backend::InMemoryNamespacedStore;
use fold_db::storage::syncing_namespaced_store::SyncingNamespacedStore;
use fold_db::storage::traits::NamespacedStore;
use fold_db::sync::auth::{AuthClient, SyncAuth};
use fold_db::sync::s3::S3Client;
use fold_db::sync::snapshot::Snapshot;
use fold_db::sync::{SyncConfig, SyncEngine, SyncState};
use std::sync::Arc;

fn test_signer() -> Arc<Ed25519KeyPair> {
    Arc::new(Ed25519KeyPair::generate().unwrap())
}

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
        test_signer(),
    ));

    // Stack: base → syncing → encrypting
    let syncing =
        SyncingNamespacedStore::new(base.clone() as Arc<dyn NamespacedStore>, engine.clone());
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
    let target_enc =
        EncryptingNamespacedStore::new(Arc::new(target) as Arc<dyn NamespacedStore>, crypto, false);

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

    let snapshot = Snapshot::create(base.as_ref(), "dev-1", 1).await.unwrap();
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

#[tokio::test]
async fn empty_store_has_no_user_data() {
    let base = Arc::new(InMemoryNamespacedStore::new());
    let namespaces = base.list_namespaces().await.unwrap();
    let has_user_data = namespaces.iter().any(|ns| ns != "__sled__default");
    assert!(!has_user_data, "fresh store should have no user data");
}

#[tokio::test]
async fn store_with_data_detected_as_non_empty() {
    let base = Arc::new(InMemoryNamespacedStore::new());
    let main = base.open_namespace("main").await.unwrap();
    main.put(b"key", b"val".to_vec()).await.unwrap();

    let namespaces = base.list_namespaces().await.unwrap();
    let has_user_data = namespaces.iter().any(|ns| ns != "__sled__default");
    assert!(
        has_user_data,
        "store with data should be detected as non-empty"
    );
}

#[tokio::test]
async fn reconfigure_sharing_replaces_extra_targets_atomically() {
    use fold_db::sharing::types::{ShareRule, ShareScope};
    use fold_db::sync::org_sync::{SyncPartitioner, SyncTarget};

    let base = Arc::new(InMemoryNamespacedStore::new());
    let crypto = test_crypto();
    let http = Arc::new(reqwest::Client::new());
    let s3 = S3Client::new(http.clone());
    let auth = AuthClient::new(
        http,
        "http://localhost:0".to_string(),
        SyncAuth::ApiKey("test".to_string()),
    );

    let engine = SyncEngine::new(
        "test-device".to_string(),
        crypto.clone(),
        s3,
        auth,
        base.clone() as Arc<dyn NamespacedStore>,
        SyncConfig::default(),
        test_signer(),
    );

    // Personal-only at startup.
    let prefixes = engine.target_prefixes().await;
    assert_eq!(prefixes.len(), 1, "only personal target at startup");
    assert!(!engine.has_org_sync().await);

    // Simulate runtime creation of a share rule (sender side).
    let rule = ShareRule {
        rule_id: "r1".to_string(),
        recipient_pubkey: "alice_pubkey".to_string(),
        recipient_display_name: "Alice".to_string(),
        scope: ShareScope::AllSchemas,
        share_prefix: "share:me:alice".to_string(),
        share_e2e_secret: vec![7u8; 32],
        active: true,
        created_at: 0,
        writer_pubkey: "me".to_string(),
        signature: String::new(),
    };

    let mut key = [0u8; 32];
    key.copy_from_slice(&rule.share_e2e_secret);
    let share_crypto = Arc::new(LocalCryptoProvider::from_key(key));
    let share_target = SyncTarget {
        label: format!("share -> {}", rule.recipient_display_name),
        prefix: rule.share_prefix.clone(),
        crypto: share_crypto,
    };

    let partitioner = SyncPartitioner::new(&[], std::slice::from_ref(&rule));
    engine
        .reconfigure_sharing(partitioner, vec![share_target])
        .await;

    let prefixes = engine.target_prefixes().await;
    assert_eq!(prefixes.len(), 2, "personal + share target");
    assert!(prefixes.contains(&"share:me:alice".to_string()));
    assert!(engine.has_org_sync().await);

    // Deactivating the rule: caller rebuilds with no extra targets.
    let empty_partitioner = SyncPartitioner::new(&[], &[]);
    engine.reconfigure_sharing(empty_partitioner, vec![]).await;

    let prefixes = engine.target_prefixes().await;
    assert_eq!(prefixes.len(), 1, "back to personal-only after dectivation");
    assert!(!engine.has_org_sync().await);
}
