//! End-to-end test for org data sync with encryption.
//!
//! Unlike `org_sync_e2e_test.rs` which simulates sync by copying raw Sled keys,
//! this test exercises the **encryption path**: org data is sealed with the org's
//! E2E key, unsealed on a second node, and replayed via SyncEngine.replay_entry().
//!
//! This validates the critical property that:
//! 1. Org data written on node1 produces org-prefixed keys in Sled
//! 2. Those keys can be packaged into LogEntries and sealed with org crypto
//! 3. A second node with the same org E2E key can unseal and replay them
//! 4. The replayed data is queryable on node2 via the org schema

use fold_db::access::AccessContext;
use fold_db::crypto::provider::LocalCryptoProvider;
use fold_db::crypto::CryptoProvider;
use fold_db::fold_db_core::FoldDB;
use fold_db::org::operations as org_ops;
use fold_db::schema::types::operations::MutationType;
use fold_db::schema::types::operations::Query;
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

// ---------------------------------------------------------------------------
// Helpers
// ---------------------------------------------------------------------------

async fn make_folddb(tmp: &tempfile::TempDir) -> FoldDB {
    FoldDB::new(tmp.path().to_str().unwrap())
        .await
        .expect("Failed to create FoldDB")
}

async fn register_schema(db: &FoldDB, name: &str, org_hash: Option<&str>) {
    let mut builder = TestSchemaBuilder::new(name)
        .fields(&["body"])
        .hash_key("title")
        .range_key("date");
    if let Some(h) = org_hash {
        builder = builder.org_hash(h);
    }
    let json_str = builder.build_json();
    db.load_schema_from_json(&json_str).await.unwrap();
    db.schema_manager()
        .set_schema_state(name, SchemaState::Approved)
        .await
        .unwrap();
}

async fn write_mutation(db: &FoldDB, schema_name: &str, title: &str, date: &str, body: &str) {
    let mut fields = HashMap::new();
    fields.insert("title".to_string(), json!(title));
    fields.insert("body".to_string(), json!(body));
    fields.insert("date".to_string(), json!(date));

    let mutation = Mutation::new(
        schema_name.to_string(),
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

async fn query_field_values(db: &FoldDB, schema_name: &str, field: &str) -> Vec<serde_json::Value> {
    let query = Query::new(schema_name.to_string(), vec![field.to_string()]);
    let access = AccessContext::owner("test-owner");
    let result = db
        .query_executor()
        .query_with_access(query, &access, None)
        .await
        .expect("Query failed");

    match result.get(field) {
        Some(field_map) => field_map.values().map(|fv| fv.value.clone()).collect(),
        None => vec![],
    }
}

/// Build a dummy SyncEngine backed by the given NamespacedStore.
/// The S3/auth clients are dummies — we only use replay_entry() directly.
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
        "node2-device".to_string(),
        crypto,
        s3,
        auth,
        store,
        SyncConfig::default(),
        Arc::new(Ed25519KeyPair::generate().unwrap()),
    )
}

/// Extract all org-prefixed key-value pairs from a Sled tree and package them
/// as LogEntries with the given namespace.
fn extract_org_entries(
    pool: &Arc<fold_db::storage::SledPool>,
    tree_name: &str,
    org_prefix: &str,
) -> Vec<LogEntry> {
    let guard = pool.acquire_arc().unwrap();
    let tree = guard.db().open_tree(tree_name).unwrap();
    let mut entries = Vec::new();
    let mut seq = 1_000_000u64; // start high to avoid collisions

    for result in tree.iter() {
        let (key, value) = result.unwrap();
        let key_str = String::from_utf8_lossy(&key);
        if key_str.starts_with(org_prefix) {
            entries.push(LogEntry {
                seq,
                timestamp_ms: 1700000000000,
                device_id: "node1-device".to_string(),
                op: LogOp::Put {
                    namespace: tree_name.to_string(),
                    key: LogOp::encode_bytes(&key),
                    value: LogOp::encode_bytes(&value),
                },
            });
            seq += 1;
        }
    }
    entries
}

/// Extract a specific key from a Sled tree as a LogEntry.
fn extract_key_entry(
    pool: &Arc<fold_db::storage::SledPool>,
    tree_name: &str,
    key: &str,
) -> Option<LogEntry> {
    let guard = pool.acquire_arc().unwrap();
    let tree = guard.db().open_tree(tree_name).unwrap();
    tree.get(key.as_bytes()).unwrap().map(|value| LogEntry {
        seq: 999_999,
        timestamp_ms: 1700000000000,
        device_id: "node1-device".to_string(),
        op: LogOp::Put {
            namespace: tree_name.to_string(),
            key: LogOp::encode_bytes(key.as_bytes()),
            value: LogOp::encode_bytes(&value),
        },
    })
}

// ---------------------------------------------------------------------------
// Test: Org data sealed with org crypto → unsealed on node2 → queryable
// ---------------------------------------------------------------------------

#[tokio::test]
async fn test_org_sync_with_encryption_roundtrip() {
    // --- Setup: two FoldDB instances + org membership ---
    let tmp1 = tempfile::tempdir().unwrap();
    let tmp2 = tempfile::tempdir().unwrap();
    let node1 = make_folddb(&tmp1).await;
    let node2 = make_folddb(&tmp2).await;

    let pool1 = node1.sled_pool().cloned().unwrap();

    // Create org on node1
    let membership =
        org_ops::create_org(&pool1, "Encrypted Corp", "pubkey_alice", "Alice").unwrap();
    let org_hash = &membership.org_hash;

    // Org E2E key — both nodes share this (received via invite bundle)
    let org_key_bytes: [u8; 32] = {
        use sha2::{Digest, Sha256};
        let mut hasher = Sha256::new();
        hasher.update(membership.org_e2e_secret.as_bytes());
        hasher.finalize().into()
    };
    let org_crypto: Arc<dyn CryptoProvider> =
        Arc::new(LocalCryptoProvider::from_key(org_key_bytes));

    // A different key (personal) should NOT be able to unseal org data
    let wrong_crypto: Arc<dyn CryptoProvider> =
        Arc::new(LocalCryptoProvider::from_key([0x99u8; 32]));

    // --- Node 1: register org schema and write data ---
    register_schema(&node1, "enc_notes", Some(org_hash)).await;

    write_mutation(
        &node1,
        "enc_notes",
        "note-1",
        "2026-04-01",
        "encrypted body 1",
    )
    .await;
    write_mutation(
        &node1,
        "enc_notes",
        "note-2",
        "2026-04-02",
        "encrypted body 2",
    )
    .await;

    // Verify node1 has the data
    let node1_bodies = query_field_values(&node1, "enc_notes", "body").await;
    assert_eq!(node1_bodies.len(), 2, "Node 1 should have 2 records");

    // --- Extract org-prefixed entries from node1's Sled ---
    let pool1_ref = node1.sled_pool().unwrap();
    let org_prefix = format!("{}:", org_hash);

    let main_entries = extract_org_entries(pool1_ref, "main", &org_prefix);
    assert!(
        !main_entries.is_empty(),
        "Should have org-prefixed keys in main"
    );

    // Also extract the schema and schema state
    let schema_entry =
        extract_key_entry(pool1_ref, "schemas", "enc_notes").expect("Schema should exist in sled");
    let state_entry = extract_key_entry(pool1_ref, "schema_states", "enc_notes")
        .expect("Schema state should exist in sled");

    // --- Seal all entries with org crypto (simulating upload) ---
    let mut sealed_entries = Vec::new();
    for entry in &main_entries {
        let sealed = entry.seal(&org_crypto).await.unwrap();
        sealed_entries.push(sealed);
    }
    let sealed_schema = schema_entry.seal(&org_crypto).await.unwrap();
    let sealed_state = state_entry.seal(&org_crypto).await.unwrap();

    // --- Verify wrong key cannot unseal ---
    let unseal_result = LogEntry::unseal(&sealed_entries[0].bytes, &wrong_crypto).await;
    assert!(
        unseal_result.is_err(),
        "Wrong key should fail to unseal org data"
    );

    // --- Node 2: unseal with correct org crypto and replay ---

    // Build a SyncEngine for node2 backed by node2's Sled
    let pool2 = node2.sled_pool().cloned().unwrap();
    let node2_store =
        Arc::new(fold_db::storage::SledNamespacedStore::new(pool2)) as Arc<dyn NamespacedStore>;
    let replay_engine = build_replay_engine(node2_store, org_crypto.clone());

    // Unseal and replay schema + state first
    let unsealed_schema = LogEntry::unseal(&sealed_schema.bytes, &org_crypto)
        .await
        .unwrap();
    replay_engine
        .replay_entry(&unsealed_schema, None)
        .await
        .unwrap();

    let unsealed_state = LogEntry::unseal(&sealed_state.bytes, &org_crypto)
        .await
        .unwrap();
    replay_engine
        .replay_entry(&unsealed_state, None)
        .await
        .unwrap();

    // Unseal and replay all org data entries
    for sealed in &sealed_entries {
        let unsealed = LogEntry::unseal(&sealed.bytes, &org_crypto).await.unwrap();
        replay_engine.replay_entry(&unsealed, None).await.unwrap();
    }

    // --- Load schema into node2's in-memory SchemaManager cache ---
    let pool2_ref = node2.sled_pool().unwrap();
    let guard2 = pool2_ref.acquire_arc().unwrap();
    let schemas_tree = guard2.db().open_tree("schemas").unwrap();
    let schema_bytes = schemas_tree
        .get("enc_notes".as_bytes())
        .unwrap()
        .expect("Schema should have been replayed to node2 sled");
    let schema: fold_db::schema::Schema =
        serde_json::from_slice(&schema_bytes).expect("Failed to deserialize schema");
    node2
        .schema_manager()
        .load_schema_internal(schema)
        .await
        .expect("Failed to load schema on node2");
    node2
        .schema_manager()
        .set_schema_state("enc_notes", SchemaState::Approved)
        .await
        .unwrap();

    // --- Verify node2 can query the synced data ---
    let node2_bodies = query_field_values(&node2, "enc_notes", "body").await;
    assert_eq!(
        node2_bodies.len(),
        2,
        "Node 2 should see 2 records after encrypted sync replay"
    );

    let mut node1_sorted: Vec<String> = node1_bodies
        .iter()
        .map(|v| v.as_str().unwrap().to_string())
        .collect();
    let mut node2_sorted: Vec<String> = node2_bodies
        .iter()
        .map(|v| v.as_str().unwrap().to_string())
        .collect();
    node1_sorted.sort();
    node2_sorted.sort();
    assert_eq!(
        node1_sorted, node2_sorted,
        "Node 1 and Node 2 should have identical data after encrypted sync"
    );
}

// ---------------------------------------------------------------------------
// Test: Personal data sealed with personal crypto is NOT readable with org crypto
// ---------------------------------------------------------------------------

#[tokio::test]
async fn test_personal_data_not_readable_with_org_crypto() {
    let tmp1 = tempfile::tempdir().unwrap();
    let node1 = make_folddb(&tmp1).await;

    let pool1 = node1.sled_pool().cloned().unwrap();
    let membership =
        org_ops::create_org(&pool1, "Isolation Corp", "pubkey_owner", "Owner").unwrap();
    let org_hash = &membership.org_hash;

    // Register personal + org schemas
    register_schema(&node1, "personal_notes", None).await;
    register_schema(&node1, "org_notes", Some(org_hash)).await;

    // Write personal data
    write_mutation(
        &node1,
        "personal_notes",
        "secret",
        "2026-04-01",
        "my private data",
    )
    .await;

    // Write org data
    write_mutation(
        &node1,
        "org_notes",
        "shared",
        "2026-04-01",
        "org shared data",
    )
    .await;

    // Personal crypto (different from org crypto)
    let personal_crypto: Arc<dyn CryptoProvider> =
        Arc::new(LocalCryptoProvider::from_key([0xAA; 32]));

    // Org crypto
    let org_key_bytes: [u8; 32] = {
        use sha2::{Digest, Sha256};
        let mut hasher = Sha256::new();
        hasher.update(membership.org_e2e_secret.as_bytes());
        hasher.finalize().into()
    };
    let org_crypto: Arc<dyn CryptoProvider> =
        Arc::new(LocalCryptoProvider::from_key(org_key_bytes));

    // Extract personal keys (unprefixed)
    let pool1_ref = node1.sled_pool().unwrap();
    let guard1 = pool1_ref.acquire_arc().unwrap();
    let main_tree = guard1.db().open_tree("main").unwrap();
    let org_prefix = format!("{}:", org_hash);

    let mut personal_entries = Vec::new();
    for result in main_tree.iter() {
        let (key, value) = result.unwrap();
        let key_str = String::from_utf8_lossy(&key);
        if !key_str.starts_with(&org_prefix) {
            personal_entries.push(LogEntry {
                seq: 1,
                timestamp_ms: 1700000000000,
                device_id: "node1".to_string(),
                op: LogOp::Put {
                    namespace: "main".to_string(),
                    key: LogOp::encode_bytes(&key),
                    value: LogOp::encode_bytes(&value),
                },
            });
        }
    }
    assert!(
        !personal_entries.is_empty(),
        "Should have personal (unprefixed) entries"
    );

    // Seal personal data with personal crypto
    let sealed_personal = personal_entries[0].seal(&personal_crypto).await.unwrap();

    // Org crypto should NOT be able to unseal personal data
    let unseal_result = LogEntry::unseal(&sealed_personal.bytes, &org_crypto).await;
    assert!(
        unseal_result.is_err(),
        "Org crypto should not unseal personal data — encryption isolation is preserved"
    );
}

// ---------------------------------------------------------------------------
// Test: Partitioner correctly classifies org-prefixed keys from real mutations
// ---------------------------------------------------------------------------

#[tokio::test]
async fn test_partitioner_classifies_real_org_mutations() {
    let tmp1 = tempfile::tempdir().unwrap();
    let node1 = make_folddb(&tmp1).await;

    let pool1 = node1.sled_pool().cloned().unwrap();
    let membership =
        org_ops::create_org(&pool1, "Partition Corp", "pubkey_owner", "Owner").unwrap();
    let org_hash = &membership.org_hash;

    // Register both personal and org schemas
    register_schema(&node1, "personal_stuff", None).await;
    register_schema(&node1, "org_stuff", Some(org_hash)).await;

    // Write to both
    write_mutation(
        &node1,
        "personal_stuff",
        "my-note",
        "2026-04-01",
        "personal",
    )
    .await;
    write_mutation(
        &node1,
        "org_stuff",
        "team-note",
        "2026-04-01",
        "for the team",
    )
    .await;

    // Use the SyncPartitioner to classify all keys in main tree
    let partitioner = fold_db::sync::SyncPartitioner::new(std::slice::from_ref(&membership), &[]);

    let pool1_ref = node1.sled_pool().unwrap();
    let guard1 = pool1_ref.acquire_arc().unwrap();
    let main_tree = guard1.db().open_tree("main").unwrap();

    let mut personal_count = 0usize;
    let mut org_count = 0usize;

    for result in main_tree.iter() {
        let (key, _value) = result.unwrap();
        let key_str = String::from_utf8_lossy(&key);
        match partitioner.partition(&key_str) {
            fold_db::sync::SyncDestination::Personal => personal_count += 1,
            fold_db::sync::SyncDestination::Org { .. } => org_count += 1,
            fold_db::sync::SyncDestination::Share { .. } => {}
        }
    }

    assert!(personal_count > 0, "Should have personal keys (unprefixed)");
    assert!(org_count > 0, "Should have org keys (prefixed)");

    // Verify org keys all start with the org hash
    let org_prefix = format!("{}:", org_hash);
    for result in main_tree.iter() {
        let (key, _) = result.unwrap();
        let key_str = String::from_utf8_lossy(&key);
        if let fold_db::sync::SyncDestination::Org { org_hash: h, .. } =
            partitioner.partition(&key_str)
        {
            assert_eq!(h, *org_hash, "Partitioned org hash should match");
            assert!(
                key_str.starts_with(&org_prefix),
                "Org-classified key should start with org prefix"
            );
        }
    }
}

// ---------------------------------------------------------------------------
// Test: Org-prefix replay via replay_entry(target=Some) — the real sync path.
//
// `test_org_sync_with_encryption_roundtrip` uses `replay_entry(…, None)` which
// bypasses `rewrite_key_if_needed`, so it never exercises the af4ba strip-on-
// replay path that real `download_entries` takes. This test drives the full
// sender → sealed → unsealed → replay_entry(Some(&org_target)) → schema
// reloader flow and asserts the receiver sees the schema under its bare name
// and can resolve molecules through it.
//
// Alpha BLOCKER 2767c: without this coverage the af4ba replay regressed on
// real dev Exemem because schema-namespace rows stripped to a bare key never
// made it into the receiver's SchemaCore.
// ---------------------------------------------------------------------------

#[tokio::test]
async fn test_org_prefix_replay_dispatches_schema_and_molecules() {
    let tmp1 = tempfile::tempdir().unwrap();
    let tmp2 = tempfile::tempdir().unwrap();
    let node1 = make_folddb(&tmp1).await;
    let node2 = make_folddb(&tmp2).await;

    // Alice creates an org and derives its E2E crypto.
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

    // Alice registers an org-tagged schema and writes 5 molecules against it.
    register_schema(&node1, "replay_notes", Some(&org_hash)).await;
    for i in 1..=5 {
        write_mutation(
            &node1,
            "replay_notes",
            &format!("note-{i}"),
            &format!("2026-04-{i:02}"),
            &format!("body {i}"),
        )
        .await;
    }

    // Pull every org-prefixed log entry that a real SyncEngine would partition
    // onto the org prefix: schema body, schema state, and all main-namespace
    // atom/ref/history writes.
    let pool1_ref = node1.sled_pool().unwrap();
    let org_prefix = format!("{}:", org_hash);
    let schema_entries = extract_org_entries(pool1_ref, "schemas", &org_prefix);
    let state_entries = extract_org_entries(pool1_ref, "schema_states", &org_prefix);
    let main_entries = extract_org_entries(pool1_ref, "main", &org_prefix);
    assert!(
        !schema_entries.is_empty(),
        "Alice must have org-prefixed schema entry after af4ba dual-write"
    );
    assert!(
        !main_entries.is_empty(),
        "Alice must have org-prefixed molecule entries"
    );

    // Seal every entry with the org crypto — this is what `upload_entries`
    // produces.
    let mut sealed = Vec::new();
    for e in schema_entries
        .iter()
        .chain(state_entries.iter())
        .chain(main_entries.iter())
    {
        sealed.push(e.seal(&org_crypto).await.unwrap());
    }

    // Bob: build a replay engine over his Sled and a SyncTarget that names the
    // org prefix. `replay_entry(Some(&org_target))` must trigger the
    // af4ba strip-on-replay for the schemas/schema_states namespaces.
    let pool2 = node2.sled_pool().cloned().unwrap();
    let node2_store =
        Arc::new(fold_db::storage::SledNamespacedStore::new(pool2)) as Arc<dyn NamespacedStore>;
    let replay_engine = build_replay_engine(node2_store, org_crypto.clone());

    let org_target = fold_db::sync::org_sync::SyncTarget {
        label: membership.org_name.clone(),
        prefix: org_hash.clone(),
        crypto: org_crypto.clone(),
    };

    for s in &sealed {
        let unsealed = LogEntry::unseal(&s.bytes, &org_crypto).await.unwrap();
        replay_engine
            .replay_entry(&unsealed, Some(&org_target))
            .await
            .expect("org-prefix replay must dispatch schema/main entries");
    }

    // Fire the schema reloader callback — `download_entries` triggers this
    // when any replayed entry wrote to the schemas namespace.
    node2.schema_manager().reload_from_store().await.unwrap();
    // Auto-approve so the query path resolves against the schema.
    node2
        .schema_manager()
        .set_schema_state("replay_notes", SchemaState::Approved)
        .await
        .unwrap();

    // `/api/schemas` must expose the schema under its bare name — not under
    // `{org_hash}:replay_notes` — so UIs and queries can find it.
    let visible: Vec<String> = node2
        .schema_manager()
        .get_schemas()
        .unwrap()
        .keys()
        .cloned()
        .collect();
    assert!(
        visible.contains(&"replay_notes".to_string()),
        "Bob must surface replay_notes under bare name after org-prefix replay — got {:?}",
        visible
    );
    assert!(
        visible.iter().all(|k| !k.starts_with(&org_prefix)),
        "Bob must not surface org-prefixed schema names — got {:?}",
        visible
    );

    // Molecules must resolve against the replayed schema — the alpha-thesis
    // assertion for 2767c.
    let bodies = query_field_values(&node2, "replay_notes", "body").await;
    assert_eq!(
        bodies.len(),
        5,
        "Bob must query all 5 org molecules through the propagated schema"
    );
}

// ---------------------------------------------------------------------------
// Guard for the `schema_states` leg of the reloader dispatch.
//
// If a peer flips an org schema's state in isolation (approve / block) the org
// log receives ONLY a `schema_states` entry — no accompanying `schemas` entry.
// `download_entries` must still fire the schema reloader so the in-memory
// state cache converges. Without this, UIs and permission checks lag the
// on-disk truth until the next `schemas`-namespace write lands.
// ---------------------------------------------------------------------------

#[tokio::test]
async fn test_org_prefix_replay_state_only_triggers_reload() {
    let tmp1 = tempfile::tempdir().unwrap();
    let tmp2 = tempfile::tempdir().unwrap();
    let node1 = make_folddb(&tmp1).await;
    let node2 = make_folddb(&tmp2).await;

    let pool1 = node1.sled_pool().cloned().unwrap();
    let membership = org_ops::create_org(&pool1, "State Corp", "pubkey_alice", "Alice").unwrap();
    let org_hash = membership.org_hash.clone();

    let org_key_bytes: [u8; 32] = {
        use sha2::{Digest, Sha256};
        let mut hasher = Sha256::new();
        hasher.update(membership.org_e2e_secret.as_bytes());
        hasher.finalize().into()
    };
    let org_crypto: Arc<dyn CryptoProvider> =
        Arc::new(LocalCryptoProvider::from_key(org_key_bytes));

    // Pre-propagate the schema + initial Approved state to Bob so we can
    // isolate the state-only replay path.
    register_schema(&node1, "state_notes", Some(&org_hash)).await;
    let pool1_ref = node1.sled_pool().unwrap();
    let org_prefix = format!("{}:", org_hash);
    let schema_entries = extract_org_entries(pool1_ref, "schemas", &org_prefix);
    let initial_state_entries = extract_org_entries(pool1_ref, "schema_states", &org_prefix);

    let pool2 = node2.sled_pool().cloned().unwrap();
    let node2_store =
        Arc::new(fold_db::storage::SledNamespacedStore::new(pool2)) as Arc<dyn NamespacedStore>;
    let replay_engine = build_replay_engine(node2_store, org_crypto.clone());
    let org_target = fold_db::sync::org_sync::SyncTarget {
        label: membership.org_name.clone(),
        prefix: org_hash.clone(),
        crypto: org_crypto.clone(),
    };
    for e in schema_entries.iter().chain(initial_state_entries.iter()) {
        let sealed = e.seal(&org_crypto).await.unwrap();
        let unsealed = LogEntry::unseal(&sealed.bytes, &org_crypto).await.unwrap();
        replay_engine
            .replay_entry(&unsealed, Some(&org_target))
            .await
            .unwrap();
    }
    node2.schema_manager().reload_from_store().await.unwrap();
    // Bob must start from `Available` so the next replay demonstrably flips
    // the state.
    node2
        .schema_manager()
        .set_schema_state("state_notes", SchemaState::Available)
        .await
        .unwrap();

    // Alice blocks the schema — triggers a `schema_states`-namespace dual-
    // write. No `schemas` entry is produced for a pure state flip.
    node1
        .schema_manager()
        .set_schema_state("state_notes", SchemaState::Blocked)
        .await
        .unwrap();

    let block_state_entries = extract_org_entries(pool1_ref, "schema_states", &org_prefix);
    // The newly-captured state must encode `Blocked`; find the entry whose
    // decoded value is `Blocked` (it may share the org-prefixed key with the
    // earlier `Available` write if the writer reused it).
    let blocked_entry = block_state_entries
        .iter()
        .find(|e| match &e.op {
            LogOp::Put { value, .. } => {
                let bytes = LogOp::decode_bytes(value).unwrap_or_default();
                serde_json::from_slice::<SchemaState>(&bytes)
                    .map(|s| matches!(s, SchemaState::Blocked))
                    .unwrap_or(false)
            }
            _ => false,
        })
        .expect("blocked state entry must be present on the org prefix");

    // Simulate a download cycle carrying ONLY the state entry. If the
    // reloader dispatch omits `schema_states`, Bob's cache stays stuck at
    // whatever it was before — surfacing the regression this guards against.
    let sealed = blocked_entry.seal(&org_crypto).await.unwrap();
    let unsealed = LogEntry::unseal(&sealed.bytes, &org_crypto).await.unwrap();
    replay_engine
        .replay_entry(&unsealed, Some(&org_target))
        .await
        .unwrap();

    // Drive the reloader the way `download_entries` would — per this PR, a
    // `schema_states`-namespace entry counts as a schema replay.
    node2.schema_manager().reload_from_store().await.unwrap();

    let states = node2.schema_manager().get_schema_states().unwrap();
    assert_eq!(
        states.get("state_notes"),
        Some(&SchemaState::Blocked),
        "Bob's schema_states cache must converge after a state-only org replay"
    );
}
