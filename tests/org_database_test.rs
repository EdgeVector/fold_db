//! Comprehensive integration tests for the org database feature.
//!
//! Exercises the full lifecycle: org creation, schema registration, data CRUD,
//! multi-org isolation, key prefixing, sync partitioning, purge, and cleanup.

use fold_db::access::AccessContext;
use fold_db::fold_db_core::FoldDB;
use fold_db::org::{operations as org_ops, OrgMemberInfo};
use fold_db::schema::types::operations::{MutationType, Query, SortOrder};
use fold_db::schema::types::{KeyValue, Mutation};
use fold_db::schema::SchemaState;
use fold_db::sync::org_sync::{SyncDestination, SyncPartitioner};
use fold_db::test_helpers::TestSchemaBuilder;
use serde_json::json;
use std::collections::HashMap;

// ---------------------------------------------------------------------------
// Helpers (copied from org_key_prefixing_test.rs + new)
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

async fn write_mutation_update(
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
        MutationType::Update,
    );
    db.mutation_manager()
        .write_mutations_batch_async(vec![mutation])
        .await
        .expect("Failed to write update mutation")
}

/// Query a schema and return all values for a given field.
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

/// Query a schema and return the full result map.
async fn query_full(
    db: &FoldDB,
    schema_name: &str,
) -> HashMap<String, HashMap<KeyValue, fold_db::schema::types::field::FieldValue>> {
    let query = Query::new(schema_name.to_string(), vec![]);
    let access = AccessContext::owner("test-owner");
    db.query_executor()
        .query_with_access(query, &access, None)
        .await
        .expect("Query failed")
}

/// Count sled keys in the "main" tree that start with `{org_hash}:`.
fn count_org_prefixed_keys(db: &FoldDB, org_hash: &str) -> usize {
    let pool = db.sled_pool().expect("Expected sled backend");
    let guard = pool.acquire_arc().unwrap();
    let main_tree = guard.db().open_tree("main").unwrap();
    let prefix = format!("{}:", org_hash);
    main_tree
        .iter()
        .filter_map(|r| r.ok())
        .map(|(k, _)| String::from_utf8_lossy(&k).to_string())
        .filter(|k| k.starts_with(&prefix))
        .count()
}

/// Collect all sled keys in the "main" tree.
fn all_sled_keys(db: &FoldDB) -> Vec<String> {
    let pool = db.sled_pool().expect("Expected sled backend");
    let guard = pool.acquire_arc().unwrap();
    let main_tree = guard.db().open_tree("main").unwrap();
    main_tree
        .iter()
        .filter_map(|r| r.ok())
        .map(|(k, _)| String::from_utf8_lossy(&k).to_string())
        .collect()
}

// ---------------------------------------------------------------------------
// Test 1: Full org lifecycle
// ---------------------------------------------------------------------------

#[tokio::test]
async fn test_full_org_lifecycle() {
    let tmp = tempfile::tempdir().unwrap();
    let db = make_folddb(&tmp).await;
    let sled_pool = db.sled_pool().cloned().unwrap();

    // Create org
    let membership = org_ops::create_org(&sled_pool, "Test Corp", "pubkey_alice", "Alice").unwrap();
    let org_hash = &membership.org_hash;

    // Register org schema
    register_schema(&db, "corp_notes", Some(org_hash)).await;

    // Write initial data
    write_mutation(&db, "corp_notes", "meeting", "2026-01-15", "initial notes").await;

    // Query — should return 1 record
    let bodies = query_field_values(&db, "corp_notes", "body").await;
    assert_eq!(bodies.len(), 1);
    assert_eq!(bodies[0], json!("initial notes"));

    // Update same record
    write_mutation_update(&db, "corp_notes", "meeting", "2026-01-15", "updated notes").await;

    // Query — should return updated value
    let bodies = query_field_values(&db, "corp_notes", "body").await;
    assert_eq!(bodies.len(), 1);
    assert_eq!(bodies[0], json!("updated notes"));

    // Org-prefixed keys exist
    assert!(
        count_org_prefixed_keys(&db, org_hash) > 0,
        "Expected org-prefixed keys in sled"
    );

    // Purge org data + schemas
    let db_ops = db.get_db_ops();
    let purged = db_ops.purge_org_data(org_hash).await.unwrap();
    assert!(purged > 0, "Expected to purge at least 1 key");
    let removed_schemas = db.schema_manager().purge_org_schemas(org_hash).await.unwrap();
    assert_eq!(removed_schemas, vec!["corp_notes"]);

    // Schema should be gone from the manager
    let schema = db.schema_manager().get_schema("corp_notes").await.unwrap();
    assert!(schema.is_none(), "Schema should be purged");

    // No org-prefixed keys remain
    assert_eq!(
        count_org_prefixed_keys(&db, org_hash),
        0,
        "Expected zero org-prefixed keys after purge"
    );

    // Delete org membership
    org_ops::delete_org(&sled_pool, org_hash).unwrap();
    assert!(
        org_ops::get_org(&sled_pool, org_hash).unwrap().is_none(),
        "Org should be gone after delete"
    );
}

// ---------------------------------------------------------------------------
// Test 2: Multi-org data isolation
// ---------------------------------------------------------------------------

#[tokio::test]
async fn test_multi_org_data_isolation() {
    let tmp = tempfile::tempdir().unwrap();
    let db = make_folddb(&tmp).await;
    let sled_pool = db.sled_pool().cloned().unwrap();

    // Create two orgs
    let org_alpha = org_ops::create_org(&sled_pool, "Org Alpha", "pubkey_alice", "Alice").unwrap();
    let org_beta = org_ops::create_org(&sled_pool, "Org Beta", "pubkey_bob", "Bob").unwrap();

    // Register schemas
    register_schema(&db, "alpha_notes", Some(&org_alpha.org_hash)).await;
    register_schema(&db, "beta_notes", Some(&org_beta.org_hash)).await;

    // Write 3 records to each org
    for i in 1..=3 {
        write_mutation(
            &db,
            "alpha_notes",
            &format!("a{i}"),
            &format!("2026-01-{i:02}"),
            &format!("alpha body {i}"),
        )
        .await;
        write_mutation(
            &db,
            "beta_notes",
            &format!("b{i}"),
            &format!("2026-02-{i:02}"),
            &format!("beta body {i}"),
        )
        .await;
    }

    // Alpha query returns only alpha data
    let alpha_bodies = query_field_values(&db, "alpha_notes", "body").await;
    assert_eq!(alpha_bodies.len(), 3);
    assert!(alpha_bodies
        .iter()
        .all(|v| v.as_str().unwrap().contains("alpha")));
    assert!(!alpha_bodies
        .iter()
        .any(|v| v.as_str().unwrap().contains("beta")));

    // Beta query returns only beta data
    let beta_bodies = query_field_values(&db, "beta_notes", "body").await;
    assert_eq!(beta_bodies.len(), 3);
    assert!(beta_bodies
        .iter()
        .all(|v| v.as_str().unwrap().contains("beta")));

    // Both orgs have prefixed keys
    assert!(count_org_prefixed_keys(&db, &org_alpha.org_hash) > 0);
    assert!(count_org_prefixed_keys(&db, &org_beta.org_hash) > 0);

    // Purge alpha (data + schemas)
    let db_ops = db.get_db_ops();
    db_ops.purge_org_data(&org_alpha.org_hash).await.unwrap();
    db.schema_manager()
        .purge_org_schemas(&org_alpha.org_hash)
        .await
        .unwrap();

    // Alpha is fully purged — sled keys gone and schema removed
    assert_eq!(count_org_prefixed_keys(&db, &org_alpha.org_hash), 0);
    assert!(
        db.schema_manager()
            .get_schema("alpha_notes")
            .await
            .unwrap()
            .is_none(),
        "Alpha schema should be purged"
    );

    // Beta is untouched — both at sled level and via query
    assert!(count_org_prefixed_keys(&db, &org_beta.org_hash) > 0);
    let beta_bodies = query_field_values(&db, "beta_notes", "body").await;
    assert_eq!(beta_bodies.len(), 3, "Beta data should survive alpha purge");
}

// ---------------------------------------------------------------------------
// Test 3: Org and personal coexistence at scale
// ---------------------------------------------------------------------------

#[tokio::test]
async fn test_org_and_personal_coexistence_at_scale() {
    let tmp = tempfile::tempdir().unwrap();
    let db = make_folddb(&tmp).await;
    let sled_pool = db.sled_pool().cloned().unwrap();

    let org = org_ops::create_org(&sled_pool, "My Org", "pubkey_owner", "Owner").unwrap();

    // Register personal and org schemas
    register_schema(&db, "personal_journal", None).await;
    register_schema(&db, "org_journal", Some(&org.org_hash)).await;

    // Write 12 personal records
    for i in 1..=12 {
        write_mutation(
            &db,
            "personal_journal",
            &format!("p{i:02}"),
            &format!("2026-01-{i:02}"),
            &format!("personal entry {i}"),
        )
        .await;
    }

    // Write 15 org records
    for i in 1..=15 {
        write_mutation(
            &db,
            "org_journal",
            &format!("o{i:02}"),
            &format!("2026-02-{i:02}"),
            &format!("org entry {i}"),
        )
        .await;
    }

    // Verify personal count and content
    let personal_bodies = query_field_values(&db, "personal_journal", "body").await;
    assert_eq!(personal_bodies.len(), 12);
    assert!(personal_bodies
        .iter()
        .all(|v| v.as_str().unwrap().contains("personal")));
    assert!(!personal_bodies
        .iter()
        .any(|v| v.as_str().unwrap().contains("org entry")));

    // Verify org count and content
    let org_bodies = query_field_values(&db, "org_journal", "body").await;
    assert_eq!(org_bodies.len(), 15);
    assert!(org_bodies
        .iter()
        .all(|v| v.as_str().unwrap().contains("org entry")));
    assert!(!org_bodies
        .iter()
        .any(|v| v.as_str().unwrap().contains("personal")));

    // Verify sled-level separation
    let all_keys = all_sled_keys(&db);
    let org_prefix = format!("{}:", org.org_hash);
    let personal_atom_keys: Vec<_> = all_keys
        .iter()
        .filter(|k| !k.starts_with(&org_prefix) && k.starts_with("atom:"))
        .collect();
    let org_atom_keys: Vec<_> = all_keys
        .iter()
        .filter(|k| k.starts_with(&org_prefix) && k.contains(":atom:"))
        .collect();
    assert!(
        !personal_atom_keys.is_empty(),
        "Expected personal atom keys"
    );
    assert!(!org_atom_keys.is_empty(), "Expected org-prefixed atom keys");
}

// ---------------------------------------------------------------------------
// Test 4: Org mutation history prefixing
// ---------------------------------------------------------------------------

#[tokio::test]
async fn test_org_mutation_history_prefixing() {
    let tmp = tempfile::tempdir().unwrap();
    let db = make_folddb(&tmp).await;
    let sled_pool = db.sled_pool().cloned().unwrap();

    let org = org_ops::create_org(&sled_pool, "History Org", "pubkey_alice", "Alice").unwrap();
    let org_hash = &org.org_hash;

    register_schema(&db, "org_tasks", Some(org_hash)).await;

    // Write initial
    write_mutation(&db, "org_tasks", "task1", "2026-03-01", "v1").await;

    // Small delay to ensure different timestamps
    tokio::time::sleep(std::time::Duration::from_millis(10)).await;

    // Update same record
    write_mutation_update(&db, "org_tasks", "task1", "2026-03-01", "v2").await;

    // Verify latest query value
    let bodies = query_field_values(&db, "org_tasks", "body").await;
    assert_eq!(bodies.len(), 1);
    assert_eq!(bodies[0], json!("v2"));

    // Get molecule_uuid from query result
    let result = query_full(&db, "org_tasks").await;
    let body_map = result.get("body").expect("Missing body field");
    let (_, fv) = body_map.iter().next().expect("No body entries");
    let molecule_uuid = fv
        .molecule_uuid
        .as_ref()
        .expect("Missing molecule_uuid on FieldValue");

    // Retrieve mutation history
    let db_ops = db.get_db_ops();
    let events = db_ops
        .get_mutation_events(molecule_uuid, Some(org_hash))
        .await
        .unwrap();
    assert!(
        events.len() >= 2,
        "Expected at least 2 mutation events, got {}",
        events.len()
    );

    // Events should be in chronological order (ascending timestamp)
    for w in events.windows(2) {
        assert!(
            w[0].timestamp <= w[1].timestamp,
            "Events should be in chronological order"
        );
    }

    // Verify at sled level: history keys are org-prefixed
    let all_keys = all_sled_keys(&db);
    let org_history_keys: Vec<_> = all_keys
        .iter()
        .filter(|k| k.starts_with(&format!("{}:history:", org_hash)))
        .collect();
    assert!(
        !org_history_keys.is_empty(),
        "Expected org-prefixed history keys"
    );

    // No unprefixed history keys should exist
    let personal_history_keys: Vec<_> = all_keys
        .iter()
        .filter(|k| k.starts_with("history:") && !k.starts_with(&format!("{}:", org_hash)))
        .collect();
    assert!(
        personal_history_keys.is_empty(),
        "Expected no unprefixed history keys, found: {:?}",
        personal_history_keys
    );
}

// ---------------------------------------------------------------------------
// Test 5: Org query with sort order
// ---------------------------------------------------------------------------

#[tokio::test]
async fn test_org_query_with_sort_order() {
    let tmp = tempfile::tempdir().unwrap();
    let db = make_folddb(&tmp).await;
    let sled_pool = db.sled_pool().cloned().unwrap();

    let org = org_ops::create_org(&sled_pool, "Sort Org", "pubkey_owner", "Owner").unwrap();

    register_schema(&db, "org_events", Some(&org.org_hash)).await;

    // Write 5 records with out-of-order dates
    let dates = [
        "2026-05-03",
        "2026-05-01",
        "2026-05-05",
        "2026-05-02",
        "2026-05-04",
    ];
    for (i, date) in dates.iter().enumerate() {
        write_mutation(
            &db,
            "org_events",
            &format!("event{}", i + 1),
            date,
            &format!("body for {date}"),
        )
        .await;
    }

    // Query returns all 5 records
    let result = query_full(&db, "org_events").await;
    let body_map = result.get("body").expect("Missing body field");
    assert_eq!(body_map.len(), 5);

    // Extract range keys and verify they can be sorted
    let mut range_values: Vec<String> = body_map.keys().filter_map(|kv| kv.range.clone()).collect();
    range_values.sort();
    assert_eq!(
        range_values,
        vec![
            "2026-05-01",
            "2026-05-02",
            "2026-05-03",
            "2026-05-04",
            "2026-05-05",
        ]
    );

    // Descending
    range_values.reverse();
    assert_eq!(range_values[0], "2026-05-05");
    assert_eq!(range_values[4], "2026-05-01");

    // Verify SortOrder serde round-trip
    let query = Query {
        schema_name: "org_events".to_string(),
        fields: vec!["body".to_string()],
        filter: None,
        as_of: None,
        rehydrate_depth: None,
        sort_order: Some(SortOrder::Desc),
        value_filters: None,
    };
    let json_val = serde_json::to_value(&query).unwrap();
    assert_eq!(json_val["sort_order"], json!("desc"));
    let deserialized: Query = serde_json::from_value(json_val).unwrap();
    assert_eq!(deserialized.sort_order, Some(SortOrder::Desc));
}

// ---------------------------------------------------------------------------
// Test 6: Org purge leaves no residual keys
// ---------------------------------------------------------------------------

#[tokio::test]
async fn test_org_purge_leaves_no_residual_keys() {
    let tmp = tempfile::tempdir().unwrap();
    let db = make_folddb(&tmp).await;
    let sled_pool = db.sled_pool().cloned().unwrap();

    let org = org_ops::create_org(&sled_pool, "Purge Org", "pubkey_admin", "Admin").unwrap();
    let org_hash = &org.org_hash;

    // Register 3 org schemas + 1 personal schema
    register_schema(&db, "org_notes", Some(org_hash)).await;
    register_schema(&db, "org_tasks", Some(org_hash)).await;
    register_schema(&db, "org_events", Some(org_hash)).await;
    register_schema(&db, "my_notes", None).await;

    // Write 5 records to each org schema (15 total)
    for schema in &["org_notes", "org_tasks", "org_events"] {
        for i in 1..=5 {
            write_mutation(
                &db,
                schema,
                &format!("{schema}-{i}"),
                &format!("2026-01-{i:02}"),
                &format!("{schema} body {i}"),
            )
            .await;
        }
    }

    // Write 5 personal records
    for i in 1..=5 {
        write_mutation(
            &db,
            "my_notes",
            &format!("personal-{i}"),
            &format!("2026-03-{i:02}"),
            &format!("personal body {i}"),
        )
        .await;
    }

    // Verify org data exists
    assert!(
        count_org_prefixed_keys(&db, org_hash) > 0,
        "Expected org-prefixed keys before purge"
    );

    // Purge
    let db_ops = db.get_db_ops();
    let purged = db_ops.purge_org_data(org_hash).await.unwrap();
    assert!(purged > 0, "Expected to purge at least 1 key");

    // Zero org-prefixed keys remain across entire sled
    let all_keys = all_sled_keys(&db);
    let org_prefix = format!("{}:", org_hash);
    let residual: Vec<_> = all_keys
        .iter()
        .filter(|k| k.starts_with(&org_prefix))
        .collect();
    assert!(
        residual.is_empty(),
        "Expected zero residual org keys, found {}: {:?}",
        residual.len(),
        &residual[..residual.len().min(5)]
    );

    // Personal data intact
    let personal_bodies = query_field_values(&db, "my_notes", "body").await;
    assert_eq!(
        personal_bodies.len(),
        5,
        "Personal data should survive org purge"
    );
}

// ---------------------------------------------------------------------------
// Test 7: Sync partitioner routes org writes
// ---------------------------------------------------------------------------

#[tokio::test]
async fn test_sync_partitioner_routes_org_writes() {
    let tmp = tempfile::tempdir().unwrap();
    let db = make_folddb(&tmp).await;
    let sled_pool = db.sled_pool().cloned().unwrap();

    let org = org_ops::create_org(&sled_pool, "Sync Org", "pubkey_owner", "Owner").unwrap();

    // Register personal and org schemas
    register_schema(&db, "my_docs", None).await;
    register_schema(&db, "shared_docs", Some(&org.org_hash)).await;

    // Write 1 record to each
    write_mutation(
        &db,
        "my_docs",
        "personal-doc",
        "2026-01-01",
        "personal content",
    )
    .await;
    write_mutation(
        &db,
        "shared_docs",
        "shared-doc",
        "2026-01-01",
        "shared content",
    )
    .await;

    // Create SyncPartitioner from the real membership
    let memberships = vec![org];
    let partitioner = SyncPartitioner::new(&memberships);

    // Collect all sled keys and classify
    let all_keys = all_sled_keys(&db);
    let org_prefix = format!("{}:", memberships[0].org_hash);

    let mut routed_to_org = 0;
    let mut routed_to_personal = 0;

    for key in &all_keys {
        let dest = partitioner.partition(key);
        if key.starts_with(&org_prefix) {
            assert_eq!(
                dest,
                SyncDestination::Org {
                    org_hash: memberships[0].org_hash.clone(),
                    org_e2e_secret: memberships[0].org_e2e_secret.clone(),
                },
                "Org-prefixed key '{}' should route to Org destination",
                key
            );
            routed_to_org += 1;
        } else if key.starts_with("atom:") || key.starts_with("ref:") || key.starts_with("history:")
        {
            assert_eq!(
                dest,
                SyncDestination::Personal,
                "Personal key '{}' should route to Personal destination",
                key
            );
            routed_to_personal += 1;
        }
    }

    assert!(routed_to_org > 0, "Expected at least 1 key routed to Org");
    assert!(
        routed_to_personal > 0,
        "Expected at least 1 key routed to Personal"
    );
}

// ---------------------------------------------------------------------------
// Test 8: Org member operations during data lifecycle
// ---------------------------------------------------------------------------

#[tokio::test]
async fn test_org_member_operations_during_data_lifecycle() {
    let tmp = tempfile::tempdir().unwrap();
    let db = make_folddb(&tmp).await;
    let sled_pool = db.sled_pool().cloned().unwrap();

    // Alice creates org
    let org = org_ops::create_org(&sled_pool, "Team Org", "pubkey_alice", "Alice").unwrap();
    let org_hash = &org.org_hash;

    // Register org schema
    register_schema(&db, "team_notes", Some(org_hash)).await;

    // Alice writes 3 records
    for i in 1..=3 {
        write_mutation(
            &db,
            "team_notes",
            &format!("alice-{i}"),
            &format!("2026-01-{i:02}"),
            &format!("alice note {i}"),
        )
        .await;
    }

    // Add Bob
    let bob = OrgMemberInfo {
        node_public_key: "pubkey_bob".to_string(),
        display_name: "Bob".to_string(),
        added_at: 1000,
        added_by: "pubkey_alice".to_string(),
    };
    org_ops::add_member(&sled_pool, org_hash, bob).unwrap();

    // Verify 2 members
    let org_state = org_ops::get_org(&sled_pool, org_hash).unwrap().unwrap();
    assert_eq!(org_state.members.len(), 2);

    // Write 2 more records (simulating Bob's writes via different pub_key)
    for i in 4..=5 {
        // Using the same write_mutation helper (pub_key in mutation is "test-pub-key"
        // but the important thing is that data goes to the org-scoped schema)
        write_mutation(
            &db,
            "team_notes",
            &format!("bob-{i}"),
            &format!("2026-01-{i:02}"),
            &format!("bob note {i}"),
        )
        .await;
    }

    // Verify 5 total records
    let bodies = query_field_values(&db, "team_notes", "body").await;
    assert_eq!(bodies.len(), 5);

    // Remove Bob
    org_ops::remove_member(&sled_pool, org_hash, "pubkey_bob").unwrap();

    // Verify 1 member remains
    let org_state = org_ops::get_org(&sled_pool, org_hash).unwrap().unwrap();
    assert_eq!(org_state.members.len(), 1);
    assert_eq!(org_state.members[0].display_name, "Alice");

    // Data survives member removal — all 5 records still accessible
    let bodies = query_field_values(&db, "team_notes", "body").await;
    assert_eq!(bodies.len(), 5, "All data should survive member removal");

    // Org-prefixed keys still intact
    assert!(
        count_org_prefixed_keys(&db, org_hash) > 0,
        "Org data should not be affected by membership changes"
    );
}
