//! End-to-end test for org data sharing across two FoldDB instances.
//!
//! Simulates the sync flow: Node 1 writes org data → extract from sled →
//! replay into Node 2's sled → Node 2 queries and sees the data.
//! No network calls — sync is simulated by copying sled key-value pairs.

use fold_db::access::AccessContext;
use fold_db::fold_db_core::FoldDB;
use fold_db::org::operations as org_ops;
use fold_db::schema::types::operations::MutationType;
use fold_db::schema::types::operations::Query;
use fold_db::schema::types::{KeyValue, Mutation};
use fold_db::schema::SchemaState;
use fold_db::test_helpers::TestSchemaBuilder;
use serde_json::json;
use std::collections::HashMap;

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

async fn write_mutation(
    db: &FoldDB,
    schema_name: &str,
    title: &str,
    date: &str,
    body: &str,
) -> Vec<String> {
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
        .expect("Failed to write mutation")
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

/// Copy all keys with the given prefix from one sled tree to another.
fn copy_prefixed_keys(src_tree: &sled::Tree, dst_tree: &sled::Tree, prefix: &str) -> usize {
    let mut count = 0;
    for result in src_tree.iter() {
        let (key, value) = result.expect("Failed to read sled key");
        let key_str = String::from_utf8_lossy(&key);
        if key_str.starts_with(prefix) {
            dst_tree.insert(&key, value).expect("Failed to insert key");
            count += 1;
        }
    }
    count
}

/// Copy a specific key from one sled tree to another.
fn copy_key(src_tree: &sled::Tree, dst_tree: &sled::Tree, key: &str) -> bool {
    if let Some(value) = src_tree.get(key.as_bytes()).unwrap() {
        dst_tree
            .insert(key.as_bytes(), value)
            .expect("Failed to insert key");
        true
    } else {
        false
    }
}

// ---------------------------------------------------------------------------
// Test: Node 1 writes org data → simulated sync → Node 2 reads it
// ---------------------------------------------------------------------------

#[tokio::test]
async fn test_org_data_sync_between_two_nodes() {
    // --- Setup: two independent FoldDB instances ---
    let tmp1 = tempfile::tempdir().unwrap();
    let tmp2 = tempfile::tempdir().unwrap();
    let node1 = make_folddb(&tmp1).await;
    let node2 = make_folddb(&tmp2).await;

    let pool1 = node1.sled_pool().cloned().unwrap();

    // --- Node 1: create org and write data ---
    let membership = org_ops::create_org(&pool1, "Sync Corp", "pubkey_alice", "Alice").unwrap();
    let org_hash = &membership.org_hash;

    register_schema(&node1, "sync_notes", Some(org_hash)).await;

    // Write 3 records on node1
    for i in 1..=3 {
        write_mutation(
            &node1,
            "sync_notes",
            &format!("note-{i}"),
            &format!("2026-04-{i:02}"),
            &format!("body from node1 #{i}"),
        )
        .await;
    }

    // Verify node1 can query all 3
    let node1_bodies = query_field_values(&node1, "sync_notes", "body").await;
    assert_eq!(node1_bodies.len(), 3, "Node 1 should have 3 records");
    for val in &node1_bodies {
        assert!(
            val.as_str().unwrap().contains("body from node1"),
            "Node 1 data should contain expected body text"
        );
    }

    // --- Simulate sync: copy org-prefixed keys from node1 sled → node2 sled ---

    let pool1 = node1.sled_pool().unwrap();
    let pool2 = node2.sled_pool().unwrap();
    let guard1 = pool1.acquire_arc().unwrap();
    let guard2 = pool2.acquire_arc().unwrap();
    let sled1 = guard1.db();
    let sled2 = guard2.db();
    let org_prefix = format!("{}:", org_hash);

    // 1. Copy org-prefixed keys from "main" namespace (atom, ref, history data)
    let main_tree1 = sled1.open_tree("main").unwrap();
    let main_tree2 = sled2.open_tree("main").unwrap();
    let main_count = copy_prefixed_keys(&main_tree1, &main_tree2, &org_prefix);
    assert!(
        main_count > 0,
        "Expected org-prefixed keys in main namespace"
    );

    // 2. Copy the schema (stored under bare key) to node2
    let schemas_tree1 = sled1.open_tree("schemas").unwrap();
    let schemas_tree2 = sled2.open_tree("schemas").unwrap();
    assert!(
        copy_key(&schemas_tree1, &schemas_tree2, "sync_notes"),
        "Schema should exist under bare key"
    );

    // 3. Copy the schema state so node2 sees it as Approved
    let states_tree1 = sled1.open_tree("schema_states").unwrap();
    let states_tree2 = sled2.open_tree("schema_states").unwrap();
    copy_key(&states_tree1, &states_tree2, "sync_notes");

    // 4. Load the schema into node2's in-memory SchemaManager cache
    let schema_bytes = schemas_tree1
        .get("sync_notes".as_bytes())
        .unwrap()
        .expect("Schema should exist on node1");
    let schema: fold_db::schema::Schema =
        serde_json::from_slice(&schema_bytes).expect("Failed to deserialize schema");
    node2
        .schema_manager()
        .load_schema_internal(schema)
        .await
        .expect("Failed to load schema on node2");
    node2
        .schema_manager()
        .set_schema_state("sync_notes", SchemaState::Approved)
        .await
        .unwrap();

    // --- Node 2: verify it can query the synced data ---
    let node2_bodies = query_field_values(&node2, "sync_notes", "body").await;
    assert_eq!(
        node2_bodies.len(),
        3,
        "Node 2 should see all 3 records after sync replay"
    );
    for val in &node2_bodies {
        assert!(
            val.as_str().unwrap().contains("body from node1"),
            "Node 2 should see node1's data after sync"
        );
    }

    // Verify the values match exactly (sort both for deterministic comparison)
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
        "Node 1 and Node 2 should have identical data after sync"
    );
}

// ---------------------------------------------------------------------------
// Test: Multiple records with updates — sync carries latest state
// ---------------------------------------------------------------------------

#[tokio::test]
async fn test_org_sync_with_updates() {
    let tmp1 = tempfile::tempdir().unwrap();
    let tmp2 = tempfile::tempdir().unwrap();
    let node1 = make_folddb(&tmp1).await;
    let node2 = make_folddb(&tmp2).await;

    let pool1 = node1.sled_pool().cloned().unwrap();
    let membership = org_ops::create_org(&pool1, "Update Corp", "pubkey_owner", "Owner").unwrap();
    let org_hash = &membership.org_hash;

    register_schema(&node1, "update_notes", Some(org_hash)).await;

    // Write initial record
    write_mutation(&node1, "update_notes", "doc1", "2026-04-01", "version-1").await;

    // Update the same record
    let mut fields = HashMap::new();
    fields.insert("title".to_string(), json!("doc1"));
    fields.insert("body".to_string(), json!("version-2"));
    fields.insert("date".to_string(), json!("2026-04-01"));
    let update = Mutation::new(
        "update_notes".to_string(),
        fields,
        KeyValue::new(Some("doc1".to_string()), Some("2026-04-01".to_string())),
        "test-pub-key".to_string(),
        MutationType::Update,
    );
    node1
        .mutation_manager()
        .write_mutations_batch_async(vec![update])
        .await
        .unwrap();

    // Node 1 should see updated value
    let node1_bodies = query_field_values(&node1, "update_notes", "body").await;
    assert_eq!(node1_bodies.len(), 1);
    assert_eq!(node1_bodies[0], json!("version-2"));

    // Simulate sync to node2
    let pool1 = node1.sled_pool().unwrap();
    let pool2 = node2.sled_pool().unwrap();
    let guard1 = pool1.acquire_arc().unwrap();
    let guard2 = pool2.acquire_arc().unwrap();
    let sled1 = guard1.db();
    let sled2 = guard2.db();
    let org_prefix = format!("{}:", org_hash);

    let main1 = sled1.open_tree("main").unwrap();
    let main2 = sled2.open_tree("main").unwrap();
    copy_prefixed_keys(&main1, &main2, &org_prefix);

    let schemas1 = sled1.open_tree("schemas").unwrap();
    let schemas2 = sled2.open_tree("schemas").unwrap();

    let schema_bytes = schemas1
        .get("update_notes".as_bytes())
        .unwrap()
        .expect("Schema key should exist");
    schemas2
        .insert("update_notes".as_bytes(), schema_bytes.clone())
        .unwrap();

    let states1 = sled1.open_tree("schema_states").unwrap();
    let states2 = sled2.open_tree("schema_states").unwrap();
    if let Some(state_bytes) = states1.get("update_notes".as_bytes()).unwrap() {
        states2
            .insert("update_notes".as_bytes(), state_bytes)
            .unwrap();
    }

    let schema: fold_db::schema::Schema =
        serde_json::from_slice(&schema_bytes).expect("Failed to deserialize schema");
    node2
        .schema_manager()
        .load_schema_internal(schema)
        .await
        .unwrap();
    node2
        .schema_manager()
        .set_schema_state("update_notes", SchemaState::Approved)
        .await
        .unwrap();

    // Node 2 should see the latest (updated) value
    let node2_bodies = query_field_values(&node2, "update_notes", "body").await;
    assert_eq!(node2_bodies.len(), 1);
    assert_eq!(
        node2_bodies[0],
        json!("version-2"),
        "Node 2 should see the updated value after sync"
    );
}

// ---------------------------------------------------------------------------
// Test: Personal data does NOT leak to node2 during org sync
// ---------------------------------------------------------------------------

#[tokio::test]
async fn test_org_sync_does_not_leak_personal_data() {
    let tmp1 = tempfile::tempdir().unwrap();
    let tmp2 = tempfile::tempdir().unwrap();
    let node1 = make_folddb(&tmp1).await;
    let node2 = make_folddb(&tmp2).await;

    let pool1 = node1.sled_pool().cloned().unwrap();
    let membership =
        org_ops::create_org(&pool1, "Isolation Corp", "pubkey_owner", "Owner").unwrap();
    let org_hash = &membership.org_hash;

    // Register both personal and org schemas on node1
    register_schema(&node1, "personal_diary", None).await;
    register_schema(&node1, "org_shared", Some(org_hash)).await;

    // Write personal data
    write_mutation(
        &node1,
        "personal_diary",
        "secret",
        "2026-04-01",
        "my private thoughts",
    )
    .await;

    // Write org data
    write_mutation(
        &node1,
        "org_shared",
        "meeting",
        "2026-04-01",
        "org meeting notes",
    )
    .await;

    // Simulate sync: only copy org-prefixed keys
    let pool1 = node1.sled_pool().unwrap();
    let pool2 = node2.sled_pool().unwrap();
    let guard1 = pool1.acquire_arc().unwrap();
    let guard2 = pool2.acquire_arc().unwrap();
    let sled1 = guard1.db();
    let sled2 = guard2.db();
    let org_prefix = format!("{}:", org_hash);

    let main1 = sled1.open_tree("main").unwrap();
    let main2 = sled2.open_tree("main").unwrap();
    let synced_main = copy_prefixed_keys(&main1, &main2, &org_prefix);
    assert!(synced_main > 0, "Should have synced org data");

    // Verify no personal (unprefixed) atom/ref keys were copied
    let personal_leaked: Vec<String> = main2
        .iter()
        .filter_map(|r| r.ok())
        .map(|(k, _)| String::from_utf8_lossy(&k).to_string())
        .filter(|k| !k.starts_with(&org_prefix))
        .collect();
    assert!(
        personal_leaked.is_empty(),
        "No personal keys should be synced to node2, but found: {:?}",
        personal_leaked
    );

    // Set up the org schema on node2 so we can query
    let schemas1 = sled1.open_tree("schemas").unwrap();
    let schemas2 = sled2.open_tree("schemas").unwrap();

    let schema_bytes = schemas1
        .get("org_shared".as_bytes())
        .unwrap()
        .expect("Schema should exist");
    schemas2
        .insert("org_shared".as_bytes(), schema_bytes.clone())
        .unwrap();

    let states1 = sled1.open_tree("schema_states").unwrap();
    let states2 = sled2.open_tree("schema_states").unwrap();
    if let Some(state_bytes) = states1.get("org_shared".as_bytes()).unwrap() {
        states2
            .insert("org_shared".as_bytes(), state_bytes)
            .unwrap();
    }

    let schema: fold_db::schema::Schema =
        serde_json::from_slice(&schema_bytes).expect("Failed to deserialize");
    node2
        .schema_manager()
        .load_schema_internal(schema)
        .await
        .unwrap();
    node2
        .schema_manager()
        .set_schema_state("org_shared", SchemaState::Approved)
        .await
        .unwrap();

    // Node 2 should see org data
    let org_bodies = query_field_values(&node2, "org_shared", "body").await;
    assert_eq!(org_bodies.len(), 1);
    assert_eq!(org_bodies[0], json!("org meeting notes"));

    // Node 2 should NOT have the personal schema or data
    let personal_schema = node2
        .schema_manager()
        .get_schema("personal_diary")
        .await
        .unwrap();
    assert!(
        personal_schema.is_none(),
        "Personal schema should not exist on node2"
    );
}

// ---------------------------------------------------------------------------
// Test: org schema propagates to peer via org-prefixed sync routing
//
// Regression cover for alpha BLOCKER af4ba: before the `SchemaStore`
// dual-write + `SyncEngine::rewrite_key_if_needed` org-prefix strip,
// org-tagged schemas were only stored under the bare key. The
// SyncPartitioner then routed them to the writer's personal log, so
// peers downloading from the org log never received the schema and any
// molecules they did receive were orphaned.
//
// This test simulates the full sync path without the manual
// `load_schema_internal` shortcut used by earlier tests:
//
// 1. Node 1 registers a schema with `org_hash` set — the dual-write puts
//    both `sync_notes` and `{org_hash}:sync_notes` into the schemas
//    namespace.
// 2. The org-log sync is simulated by copying ONLY `{org_hash}:`-prefixed
//    keys across the `schemas`, `schema_states`, and `main` namespaces to
//    node 2, then applying the replay-side rewrite (strip org prefix for
//    the schemas and schema_states namespaces) so node 2's store matches
//    what `SyncEngine::replay_entry` would produce.
// 3. Node 2 refreshes its SchemaCore via `reload_from_store` (the same
//    callback the sync engine fires on schema replay) and must surface
//    the schema under its bare name and return all 3 molecules.
// ---------------------------------------------------------------------------

#[tokio::test]
async fn test_org_schema_propagates_via_org_prefixed_sync() {
    let tmp1 = tempfile::tempdir().unwrap();
    let tmp2 = tempfile::tempdir().unwrap();
    let node1 = make_folddb(&tmp1).await;
    let node2 = make_folddb(&tmp2).await;

    let pool1 = node1.sled_pool().cloned().unwrap();
    let membership =
        org_ops::create_org(&pool1, "Propagation Corp", "pubkey_alice", "Alice").unwrap();
    let org_hash = membership.org_hash.clone();

    register_schema(&node1, "propagation_notes", Some(&org_hash)).await;
    for i in 1..=3 {
        write_mutation(
            &node1,
            "propagation_notes",
            &format!("note-{i}"),
            &format!("2026-04-{i:02}"),
            &format!("body from node1 #{i}"),
        )
        .await;
    }

    // The dual-write must leave the schema in node 1's Sled under both the
    // bare name (so local lookup works) and the org-prefixed name (so
    // partitioning routes the sync copy to the org log).
    let pool1 = node1.sled_pool().unwrap();
    let pool2 = node2.sled_pool().unwrap();
    let guard1 = pool1.acquire_arc().unwrap();
    let guard2 = pool2.acquire_arc().unwrap();
    let sled1 = guard1.db();
    let sled2 = guard2.db();

    let org_prefix = format!("{}:", org_hash);
    let schemas1 = sled1.open_tree("schemas").unwrap();
    let states1 = sled1.open_tree("schema_states").unwrap();
    let main1 = sled1.open_tree("main").unwrap();

    assert!(
        schemas1
            .get("propagation_notes".as_bytes())
            .unwrap()
            .is_some(),
        "bare schema key must exist on node 1 for local lookup"
    );
    assert!(
        schemas1
            .get(format!("{}propagation_notes", org_prefix).as_bytes())
            .unwrap()
            .is_some(),
        "org-prefixed schema key must exist on node 1 so SyncPartitioner routes it to the org log"
    );

    // Simulate the org-log replay: only `{org_hash}:`-prefixed entries
    // reach node 2. For schemas and schema_states, the replay rewrite
    // strips the org prefix before writing, so the schema lands under its
    // bare name. For the main namespace (atoms, refs), the org prefix is
    // preserved — peers store org molecules under the same key Alice did.
    let schemas2 = sled2.open_tree("schemas").unwrap();
    let states2 = sled2.open_tree("schema_states").unwrap();
    let main2 = sled2.open_tree("main").unwrap();

    let org_schema_key = format!("{}propagation_notes", org_prefix);
    let schema_bytes = schemas1
        .get(org_schema_key.as_bytes())
        .unwrap()
        .expect("org-prefixed schema must exist on node 1");
    schemas2
        .insert("propagation_notes".as_bytes(), schema_bytes)
        .unwrap();

    let org_state_key = format!("{}propagation_notes", org_prefix);
    if let Some(state_bytes) = states1.get(org_state_key.as_bytes()).unwrap() {
        states2
            .insert("propagation_notes".as_bytes(), state_bytes)
            .unwrap();
    }

    let synced_main = copy_prefixed_keys(&main1, &main2, &org_prefix);
    assert!(
        synced_main > 0,
        "expected org-prefixed atom/ref entries to sync"
    );

    // The schema reloader callback is what `SyncEngine` fires after a
    // successful schema replay. Exercise the same entrypoint.
    node2.schema_manager().reload_from_store().await.unwrap();

    // Node 2 must now see the schema under its bare name — not under the
    // org-prefixed name, not both. get_all_schemas filters the companion
    // on the writer side, and the replay strip ensures the peer only has
    // the bare entry to begin with.
    let visible: Vec<String> = node2
        .schema_manager()
        .get_schemas()
        .unwrap()
        .keys()
        .cloned()
        .collect();
    assert!(
        visible.contains(&"propagation_notes".to_string()),
        "node 2 schemas must include propagation_notes by its bare name — got {:?}",
        visible
    );
    assert!(
        visible.iter().all(|k| !k.starts_with(&org_prefix)),
        "node 2 must not surface org-prefixed schema names, got {:?}",
        visible
    );

    // Molecules must resolve against the replayed schema — the alpha
    // thesis in one assertion.
    let node2_bodies = query_field_values(&node2, "propagation_notes", "body").await;
    assert_eq!(
        node2_bodies.len(),
        3,
        "node 2 must query all 3 org molecules through the propagated schema"
    );
}
