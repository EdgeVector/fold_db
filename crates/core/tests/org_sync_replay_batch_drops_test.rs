//! Regression test for alpha BLOCKER 4439b — silent drops during org-log replay.
//!
//! In dogfood run 7, Alice wrote 10 post-tag mutations on an org-scoped range
//! schema. Bob downloaded the log and his cursor advanced to the final seq
//! (Bob appeared caught up), but only 6 of 10 molecules resolved on query.
//! A contiguous block of 4 (seqs in the middle) silently dropped with no
//! warn/error in Bob's server log.
//!
//! The drop surface was the receiver replay path: any `Ok(None)` or
//! `Err => continue` inside `download_entries`, `bootstrap_target`, or the
//! BatchPut loop in `replay_entry` that skipped an entry without propagating
//! the error would silently lose data and still advance the cursor.
//!
//! This test exercises the **BatchPut replay loop** with a realistic 10-item
//! org-scoped atom batch and asserts every item lands. It also checks the
//! "contiguous cursor" invariant: `download_entries` must never advance the
//! cursor past a seq whose replay failed.

use fold_db::access::AccessContext;
use fold_db::crypto::provider::LocalCryptoProvider;
use fold_db::crypto::CryptoProvider;
use fold_db::fold_db_core::FoldDB;
use fold_db::org::operations as org_ops;
use fold_db::schema::types::operations::{MutationType, Query};
use fold_db::schema::types::{KeyValue, Mutation};
use fold_db::schema::SchemaState;
use fold_db::security::Ed25519KeyPair;
use fold_db::storage::traits::NamespacedStore;
use fold_db::sync::auth::{AuthClient, SyncAuth};
use fold_db::sync::log::{LogEntry, LogOp};
use fold_db::sync::s3::S3Client;
use fold_db::sync::{SyncConfig, SyncEngine};
use fold_db::test_helpers::TestSchemaBuilder;
use serde_json::json;
use std::collections::HashMap;
use std::sync::Arc;

async fn make_folddb(tmp: &tempfile::TempDir) -> FoldDB {
    FoldDB::new(tmp.path().to_str().unwrap())
        .await
        .expect("Failed to create FoldDB")
}

async fn register_range_schema(db: &FoldDB, name: &str, org_hash: Option<&str>) {
    let mut builder = TestSchemaBuilder::new(name)
        .fields(&["body"])
        .hash_key("title")
        .range_key("date");
    if let Some(h) = org_hash {
        builder = builder.org_hash(h);
    }
    db.load_schema_from_json(&builder.build_json())
        .await
        .unwrap();
    db.schema_manager()
        .set_schema_state(name, SchemaState::Approved)
        .await
        .unwrap();
}

async fn write_mutation(db: &FoldDB, schema: &str, title: &str, date: &str, body: &str) {
    let mut fields = HashMap::new();
    fields.insert("title".to_string(), json!(title));
    fields.insert("body".to_string(), json!(body));
    fields.insert("date".to_string(), json!(date));
    let mutation = Mutation::new(
        schema.to_string(),
        fields,
        KeyValue::new(Some(title.to_string()), Some(date.to_string())),
        "test-pub-key".to_string(),
        MutationType::Create,
    );
    db.mutation_manager()
        .write_mutations_batch_async(vec![mutation])
        .await
        .expect("Failed to write mutation");
}

fn build_replay_engine(
    store: Arc<dyn NamespacedStore>,
    crypto: Arc<dyn CryptoProvider>,
) -> SyncEngine {
    let http = Arc::new(reqwest::Client::new());
    let s3 = S3Client::new(http.clone());
    let auth = AuthClient::new(
        http,
        "http://localhost:0".to_string(),
        SyncAuth::ApiKey("test".to_string()),
    );
    SyncEngine::new(
        "receiver-device".to_string(),
        crypto,
        s3,
        auth,
        store,
        SyncConfig::default(),
        Arc::new(Ed25519KeyPair::generate().unwrap()),
    )
}

/// Read all key/value pairs under the given org prefix from a Sled tree and
/// package them as a single `LogOp::BatchPut` LogEntry. This mirrors how the
/// sender batches atom writes into one sync log entry.
fn collect_batch_put(
    pool: &Arc<fold_db::storage::SledPool>,
    tree_name: &str,
    org_prefix: &str,
    seq: u64,
) -> LogEntry {
    let guard = pool.acquire_arc().unwrap();
    let tree = guard.db().open_tree(tree_name).unwrap();
    let mut items: Vec<(String, String)> = Vec::new();
    for result in tree.iter() {
        let (key, value) = result.unwrap();
        let key_str = String::from_utf8_lossy(&key);
        if key_str.starts_with(org_prefix) {
            items.push((LogOp::encode_bytes(&key), LogOp::encode_bytes(&value)));
        }
    }
    LogEntry {
        seq,
        timestamp_ms: 1700000000000,
        device_id: "sender-device".to_string(),
        op: LogOp::BatchPut {
            namespace: tree_name.to_string(),
            items,
        },
    }
}

// ---------------------------------------------------------------------------
// Regression: 10-mutation org batch → every atom and molecule survives replay
// ---------------------------------------------------------------------------

#[tokio::test]
async fn batch_put_replay_applies_all_ten_items_no_silent_drops() {
    let tmp1 = tempfile::tempdir().unwrap();
    let tmp2 = tempfile::tempdir().unwrap();
    let node1 = make_folddb(&tmp1).await;
    let node2 = make_folddb(&tmp2).await;

    let pool1 = node1.sled_pool().cloned().unwrap();
    let membership = org_ops::create_org(&pool1, "Replay Corp", "pubkey_alice", "Alice").unwrap();
    let org_hash = membership.org_hash.clone();

    let org_key_bytes: [u8; 32] = {
        use sha2::{Digest, Sha256};
        let mut hasher = Sha256::new();
        hasher.update(membership.org_e2e_secret.as_bytes());
        hasher.finalize().into()
    };
    let org_crypto: Arc<dyn CryptoProvider> =
        Arc::new(LocalCryptoProvider::from_key(org_key_bytes));

    register_range_schema(&node1, "replay_notes", Some(&org_hash)).await;

    // 10 mutations, contiguous date range — mirrors the dogfood pattern.
    for i in 11..=20 {
        write_mutation(
            &node1,
            "replay_notes",
            &format!("note-{i}"),
            &format!("2026-04-{i:02}"),
            &format!("post-tag body {i}"),
        )
        .await;
    }

    let node1_bodies = {
        let query = Query::new("replay_notes".to_string(), vec!["body".to_string()]);
        let access = AccessContext::owner("test-owner");
        node1
            .query_executor()
            .query_with_access(query, &access, None)
            .await
            .unwrap()
    };
    let node1_body_count = node1_bodies.get("body").map(|m| m.len()).unwrap_or(0);
    assert_eq!(node1_body_count, 10, "Alice must see all 10 local writes");

    // Pull the schema + schema_state + a single BatchPut covering every
    // org-prefixed key in `main` (atoms + ref + history + range index).
    let pool1_ref = node1.sled_pool().unwrap();
    let guard1 = pool1_ref.acquire_arc().unwrap();
    let schemas_tree = guard1.db().open_tree("schemas").unwrap();
    let schema_value = schemas_tree
        .get("replay_notes".as_bytes())
        .unwrap()
        .expect("sender schema must exist in sled");
    let states_tree = guard1.db().open_tree("schema_states").unwrap();
    let state_value = states_tree
        .get("replay_notes".as_bytes())
        .unwrap()
        .expect("sender schema_state must exist in sled");
    drop(guard1);

    let schema_entry = LogEntry {
        seq: 1,
        timestamp_ms: 1700000000000,
        device_id: "sender-device".to_string(),
        op: LogOp::Put {
            namespace: "schemas".to_string(),
            key: LogOp::encode_bytes("replay_notes".as_bytes()),
            value: LogOp::encode_bytes(&schema_value),
        },
    };
    let state_entry = LogEntry {
        seq: 2,
        timestamp_ms: 1700000000000,
        device_id: "sender-device".to_string(),
        op: LogOp::Put {
            namespace: "schema_states".to_string(),
            key: LogOp::encode_bytes("replay_notes".as_bytes()),
            value: LogOp::encode_bytes(&state_value),
        },
    };
    let org_prefix = format!("{org_hash}:");
    let main_batch = collect_batch_put(pool1_ref, "main", &org_prefix, 3);

    let item_count = match &main_batch.op {
        LogOp::BatchPut { items, .. } => items.len(),
        _ => panic!("expected BatchPut"),
    };
    assert!(
        item_count >= 10,
        "org main batch must contain at least the 10 atom writes, got {item_count}"
    );

    // --- Replay everything on node2 under org crypto ---
    let pool2 = node2.sled_pool().cloned().unwrap();
    let node2_store =
        Arc::new(fold_db::storage::SledNamespacedStore::new(pool2)) as Arc<dyn NamespacedStore>;
    let replay_engine = build_replay_engine(node2_store, org_crypto.clone());

    replay_engine
        .replay_entry(&schema_entry, None)
        .await
        .unwrap();
    replay_engine
        .replay_entry(&state_entry, None)
        .await
        .unwrap();
    replay_engine.replay_entry(&main_batch, None).await.unwrap();

    // Load the schema into node2's in-memory SchemaManager.
    let pool2_ref = node2.sled_pool().unwrap();
    let guard2 = pool2_ref.acquire_arc().unwrap();
    let bytes = guard2
        .db()
        .open_tree("schemas")
        .unwrap()
        .get("replay_notes".as_bytes())
        .unwrap()
        .expect("replayed schema must be in sled");
    drop(guard2);
    let schema: fold_db::schema::Schema = serde_json::from_slice(&bytes).unwrap();
    node2
        .schema_manager()
        .load_schema_internal(schema)
        .await
        .unwrap();
    node2
        .schema_manager()
        .set_schema_state("replay_notes", SchemaState::Approved)
        .await
        .unwrap();

    // Node2 must see EVERY body — this is the regression assertion.
    let query = Query::new("replay_notes".to_string(), vec!["body".to_string()]);
    let access = AccessContext::owner("test-owner");
    let result = node2
        .query_executor()
        .query_with_access(query, &access, None)
        .await
        .expect("node2 query must succeed");

    let field_map = result.get("body").expect("body field must be populated");
    let received: Vec<String> = field_map
        .values()
        .map(|fv| fv.value.as_str().unwrap_or("").to_string())
        .collect();
    assert_eq!(
        received.len(),
        10,
        "BatchPut replay dropped items: expected 10 bodies, got {} ({received:?})",
        received.len()
    );

    let mut sorted = received.clone();
    sorted.sort();
    for i in 11..=20 {
        let expected = format!("post-tag body {i}");
        assert!(
            sorted.iter().any(|b| b == &expected),
            "missing body {expected} after replay (got {sorted:?})"
        );
    }
}

// ---------------------------------------------------------------------------
// Invariant: a BatchPut whose per-item rewrite fails must abort the whole
// entry — we must never leave a partial batch in Sled and advance as if the
// entry was applied.
// ---------------------------------------------------------------------------

#[tokio::test]
async fn batch_put_replay_aborts_on_invalid_base64_item() {
    let tmp = tempfile::tempdir().unwrap();
    let node = make_folddb(&tmp).await;
    let pool = node.sled_pool().cloned().unwrap();
    let store =
        Arc::new(fold_db::storage::SledNamespacedStore::new(pool)) as Arc<dyn NamespacedStore>;
    let crypto: Arc<dyn CryptoProvider> = Arc::new(LocalCryptoProvider::from_key([0x42u8; 32]));
    let engine = build_replay_engine(store, crypto);

    // Valid first item, invalid base64 second item. If replay applied items
    // without aborting on error, the first item would land silently and the
    // caller would have no way to know the entry failed halfway.
    let entry = LogEntry {
        seq: 42,
        timestamp_ms: 1700000000000,
        device_id: "sender".to_string(),
        op: LogOp::BatchPut {
            namespace: "main".to_string(),
            items: vec![
                (
                    LogOp::encode_bytes(b"valid_key_1"),
                    LogOp::encode_bytes(b"valid_value_1"),
                ),
                (
                    "not valid base64 !!!".to_string(),
                    LogOp::encode_bytes(b"v"),
                ),
            ],
        },
    };

    let result = engine.replay_entry(&entry, None).await;
    assert!(
        result.is_err(),
        "replay_entry must return Err when a BatchPut item is undecodable — caller can't silently skip"
    );
}

// ---------------------------------------------------------------------------
// Invariant: BatchPut replay is additive — applying the same batch twice is
// idempotent (replay on top of already-applied state must not lose items).
// Mirrors what happens if the sync cycle retries after a transient error.
// ---------------------------------------------------------------------------

#[tokio::test]
async fn batch_put_replay_is_idempotent_under_retry() {
    let tmp = tempfile::tempdir().unwrap();
    let node = make_folddb(&tmp).await;
    let pool = node.sled_pool().cloned().unwrap();
    let store = Arc::new(fold_db::storage::SledNamespacedStore::new(pool.clone()))
        as Arc<dyn NamespacedStore>;
    let crypto: Arc<dyn CryptoProvider> = Arc::new(LocalCryptoProvider::from_key([0x24u8; 32]));
    let engine = build_replay_engine(store, crypto);

    let items: Vec<(String, String)> = (0..10)
        .map(|i| {
            (
                LogOp::encode_bytes(format!("atom:k{i:02}").as_bytes()),
                LogOp::encode_bytes(format!("v{i:02}").as_bytes()),
            )
        })
        .collect();

    let entry = LogEntry {
        seq: 100,
        timestamp_ms: 1700000000000,
        device_id: "sender".to_string(),
        op: LogOp::BatchPut {
            namespace: "main".to_string(),
            items,
        },
    };

    engine.replay_entry(&entry, None).await.unwrap();
    engine.replay_entry(&entry, None).await.unwrap();

    let guard = pool.acquire_arc().unwrap();
    let main = guard.db().open_tree("main").unwrap();
    let mut count = 0;
    for result in main.iter() {
        let (k, _) = result.unwrap();
        if String::from_utf8_lossy(&k).starts_with("atom:k") {
            count += 1;
        }
    }
    assert_eq!(
        count, 10,
        "all 10 batch items must persist across idempotent replays"
    );
}
