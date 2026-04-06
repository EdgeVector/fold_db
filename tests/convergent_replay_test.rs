//! Tests for convergent replay in the sync engine.
//!
//! Verifies that ref keys converge via molecule merge regardless of replay order,
//! while non-ref keys (atoms, history) are always accepted.

use fold_db::atom::{Molecule, MoleculeHash};
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
async fn ref_molecule_merge_single() {
    // Two Molecule values for the same ref key should merge via LWW on written_at
    let store = Arc::new(InMemoryNamespacedStore::new());
    let engine = make_engine(store.clone());

    let mol_a = Molecule::new("atom-aaa".to_string(), "TestSchema", "field1");
    let mol_b = Molecule::new("atom-bbb".to_string(), "TestSchema", "field1");

    let val_a = serde_json::to_vec(&mol_a).unwrap();
    let val_b = serde_json::to_vec(&mol_b).unwrap();

    // Replay first, then second — second has a later written_at (created after)
    let e1 = put_entry(1, 100, "dev-a", b"ref:mol-1", &val_a);
    engine.replay_entry(&e1).await.unwrap();

    let e2 = put_entry(2, 200, "dev-b", b"ref:mol-1", &val_b);
    engine.replay_entry(&e2).await.unwrap();

    let kv = store.open_namespace("main").await.unwrap();
    let stored = kv.get(b"ref:mol-1").await.unwrap().unwrap();
    let result: Molecule = serde_json::from_slice(&stored).unwrap();
    // mol_b was created later so has a later written_at — it should win
    assert_eq!(result.get_atom_uuid(), "atom-bbb");
}

#[tokio::test]
async fn ref_molecule_merge_hash() {
    // Two MoleculeHash values should merge their keys
    let store = Arc::new(InMemoryNamespacedStore::new());
    let engine = make_engine(store.clone());

    let mut mol_a = MoleculeHash::new("TestSchema", "field1");
    mol_a.set_atom_uuid("key1".to_string(), "atom-a1".to_string());

    let mut mol_b = MoleculeHash::new("TestSchema", "field1");
    mol_b.set_atom_uuid("key2".to_string(), "atom-b2".to_string());

    let val_a = serde_json::to_vec(&mol_a).unwrap();
    let val_b = serde_json::to_vec(&mol_b).unwrap();

    let e1 = put_entry(1, 100, "dev-a", b"ref:mol-hash-1", &val_a);
    engine.replay_entry(&e1).await.unwrap();

    let e2 = put_entry(2, 200, "dev-b", b"ref:mol-hash-1", &val_b);
    engine.replay_entry(&e2).await.unwrap();

    let kv = store.open_namespace("main").await.unwrap();
    let stored = kv.get(b"ref:mol-hash-1").await.unwrap().unwrap();
    let result: MoleculeHash = serde_json::from_slice(&stored).unwrap();

    // Both keys should be present after merge
    assert!(
        result.get_atom_uuid("key1").is_some(),
        "key1 should exist after merge"
    );
    assert!(
        result.get_atom_uuid("key2").is_some(),
        "key2 should exist after merge"
    );
}

#[tokio::test]
async fn ref_no_local_writes_incoming() {
    // When no local value exists, incoming is written as-is
    let store = Arc::new(InMemoryNamespacedStore::new());
    let engine = make_engine(store.clone());

    let mol = Molecule::new("atom-first".to_string(), "TestSchema", "field1");
    let val = serde_json::to_vec(&mol).unwrap();

    let e1 = put_entry(1, 100, "dev-a", b"ref:mol-new", &val);
    engine.replay_entry(&e1).await.unwrap();

    let kv = store.open_namespace("main").await.unwrap();
    let stored = kv.get(b"ref:mol-new").await.unwrap().unwrap();
    let result: Molecule = serde_json::from_slice(&stored).unwrap();
    assert_eq!(result.get_atom_uuid(), "atom-first");
}

#[tokio::test]
async fn org_prefixed_ref_merges() {
    // Org-prefixed ref keys should also use molecule merge
    let store = Arc::new(InMemoryNamespacedStore::new());
    let engine = make_engine(store.clone());

    let mol_a = Molecule::new("atom-org-a".to_string(), "OrgSchema", "field1");
    let mol_b = Molecule::new("atom-org-b".to_string(), "OrgSchema", "field1");

    let val_a = serde_json::to_vec(&mol_a).unwrap();
    let val_b = serde_json::to_vec(&mol_b).unwrap();

    let e1 = put_entry(1, 100, "dev-a", b"org_abc:ref:mol-1", &val_a);
    engine.replay_entry(&e1).await.unwrap();

    let e2 = put_entry(2, 200, "dev-b", b"org_abc:ref:mol-1", &val_b);
    engine.replay_entry(&e2).await.unwrap();

    let kv = store.open_namespace("main").await.unwrap();
    let stored = kv.get(b"org_abc:ref:mol-1").await.unwrap().unwrap();
    let result: Molecule = serde_json::from_slice(&stored).unwrap();
    // mol_b was created later (later written_at) so it wins
    assert_eq!(result.get_atom_uuid(), "atom-org-b");
}

#[tokio::test]
async fn ref_non_molecule_bytes_uses_incoming() {
    // When stored bytes aren't valid molecule JSON, incoming is used as-is
    let store = Arc::new(InMemoryNamespacedStore::new());
    let engine = make_engine(store.clone());

    // Write raw bytes first (not valid molecule JSON)
    let e1 = put_entry(1, 100, "dev-a", b"ref:mol-raw", b"not-json");
    engine.replay_entry(&e1).await.unwrap();

    // Write more raw bytes — should overwrite since merge can't parse either
    let e2 = put_entry(2, 200, "dev-b", b"ref:mol-raw", b"also-not-json");
    engine.replay_entry(&e2).await.unwrap();

    let kv = store.open_namespace("main").await.unwrap();
    let stored = kv.get(b"ref:mol-raw").await.unwrap().unwrap();
    assert_eq!(stored, b"also-not-json");
}

#[tokio::test]
async fn merge_conflict_stored_as_sync_conflict() {
    // When two molecules have different atoms for the same key, a SyncConflict record is stored
    use fold_db::sync::SyncConflict;

    let store = Arc::new(InMemoryNamespacedStore::new());
    let engine = make_engine(store.clone());

    // Create two Molecule values pointing to different atoms for the same ref key
    let mol_a = Molecule::new("atom-aaa".to_string(), "S", "f");
    std::thread::sleep(std::time::Duration::from_millis(2));
    let mol_b = Molecule::new("atom-bbb".to_string(), "S", "f");

    let val_a = serde_json::to_vec(&mol_a).unwrap();
    let val_b = serde_json::to_vec(&mol_b).unwrap();

    let mol_uuid = mol_a.uuid().to_string();
    let ref_key = format!("ref:{}", mol_uuid);

    let e1 = put_entry(1, 100, "dev-a", ref_key.as_bytes(), &val_a);
    engine.replay_entry(&e1).await.unwrap();

    let e2 = put_entry(2, 200, "dev-b", ref_key.as_bytes(), &val_b);
    engine.replay_entry(&e2).await.unwrap();

    // Verify the merge result: mol_b (later written_at) should win
    let kv = store.open_namespace("main").await.unwrap();
    let stored = kv.get(ref_key.as_bytes()).await.unwrap().unwrap();
    let result: Molecule = serde_json::from_slice(&stored).unwrap();
    assert_eq!(result.get_atom_uuid(), "atom-bbb");

    // Verify a SyncConflict record was stored at conflict:{mol_uuid}:*
    let conflict_prefix = format!("conflict:{}:", mol_uuid);
    let keys = kv
        .scan_prefix(conflict_prefix.as_bytes())
        .await
        .unwrap();
    assert_eq!(keys.len(), 1, "expected exactly one conflict record");

    let conflict_bytes = &keys[0].1;
    let conflict: SyncConflict = serde_json::from_slice(conflict_bytes).unwrap();

    assert_eq!(conflict.molecule_uuid, mol_uuid);
    assert_eq!(conflict.winner_atom, "atom-bbb");
    assert_eq!(conflict.loser_atom, "atom-aaa");
    assert!(!conflict.resolved);
}

#[tokio::test]
async fn merge_no_conflict_for_same_atom() {
    // When both sides have the same atom, no conflict record should be stored
    let store = Arc::new(InMemoryNamespacedStore::new());
    let engine = make_engine(store.clone());

    let mol_a = Molecule::new("atom-same".to_string(), "S", "f");
    let mol_b = Molecule::new("atom-same".to_string(), "S", "f");

    let val_a = serde_json::to_vec(&mol_a).unwrap();
    let val_b = serde_json::to_vec(&mol_b).unwrap();

    let mol_uuid = mol_a.uuid().to_string();
    let ref_key = format!("ref:{}", mol_uuid);

    let e1 = put_entry(1, 100, "dev-a", ref_key.as_bytes(), &val_a);
    engine.replay_entry(&e1).await.unwrap();

    let e2 = put_entry(2, 200, "dev-b", ref_key.as_bytes(), &val_b);
    engine.replay_entry(&e2).await.unwrap();

    // No conflict should be stored
    let kv = store.open_namespace("main").await.unwrap();
    let conflict_prefix = format!("conflict:{}:", mol_uuid);
    let entries = kv
        .scan_prefix(conflict_prefix.as_bytes())
        .await
        .unwrap();
    assert!(entries.is_empty(), "no conflict expected for same atom");
}

#[tokio::test]
async fn hash_molecule_merge_conflict_stored() {
    // MoleculeHash with same key but different atoms should produce a conflict
    use fold_db::sync::SyncConflict;

    let store = Arc::new(InMemoryNamespacedStore::new());
    let engine = make_engine(store.clone());

    let mut mol_a = MoleculeHash::new("S", "f");
    mol_a.set_atom_uuid("shared_key".to_string(), "atom-old".to_string());

    std::thread::sleep(std::time::Duration::from_millis(2));

    let mut mol_b = MoleculeHash::new("S", "f");
    mol_b.set_atom_uuid("shared_key".to_string(), "atom-new".to_string());

    let val_a = serde_json::to_vec(&mol_a).unwrap();
    let val_b = serde_json::to_vec(&mol_b).unwrap();

    let mol_uuid = mol_a.uuid().to_string();
    let ref_key = format!("ref:{}", mol_uuid);

    let e1 = put_entry(1, 100, "dev-a", ref_key.as_bytes(), &val_a);
    engine.replay_entry(&e1).await.unwrap();

    let e2 = put_entry(2, 200, "dev-b", ref_key.as_bytes(), &val_b);
    engine.replay_entry(&e2).await.unwrap();

    // Verify merge result
    let kv = store.open_namespace("main").await.unwrap();
    let stored = kv.get(ref_key.as_bytes()).await.unwrap().unwrap();
    let result: MoleculeHash = serde_json::from_slice(&stored).unwrap();
    assert_eq!(
        result.get_atom_uuid("shared_key"),
        Some(&"atom-new".to_string())
    );

    // Verify conflict record
    let conflict_prefix = format!("conflict:{}:", mol_uuid);
    let entries = kv
        .scan_prefix(conflict_prefix.as_bytes())
        .await
        .unwrap();
    assert_eq!(entries.len(), 1);

    let conflict_bytes = &entries[0].1;
    let conflict: SyncConflict = serde_json::from_slice(conflict_bytes).unwrap();
    assert_eq!(conflict.molecule_uuid, mol_uuid);
    assert_eq!(conflict.winner_atom, "atom-new");
    assert_eq!(conflict.loser_atom, "atom-old");
    assert_eq!(conflict.conflict_key, "shared_key");
}
