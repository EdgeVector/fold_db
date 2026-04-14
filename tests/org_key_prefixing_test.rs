//! Tests for org key prefixing in the mutation and query paths.
//!
//! When a schema has `org_hash = Some(hash)`, all Sled keys for data in that
//! schema should be prefixed with `{org_hash}:`.

use fold_db::access::AccessContext;
use fold_db::fold_db_core::FoldDB;
use fold_db::schema::types::field::{build_storage_key, Field};
use fold_db::schema::types::operations::{MutationType, Query};
use fold_db::schema::types::{
    DeclarativeSchemaDefinition, KeyConfig, KeyValue, Mutation, SchemaType,
};
use fold_db::schema::SchemaState;
use fold_db::test_helpers::TestSchemaBuilder;
use serde_json::json;
use std::collections::HashMap;

const ORG_HASH: &str = "abc123def456";

/// Helper: create a FoldDB instance backed by a temp sled directory.
async fn make_folddb(tmp: &tempfile::TempDir) -> FoldDB {
    FoldDB::new(tmp.path().to_str().unwrap())
        .await
        .expect("Failed to create FoldDB")
}

/// Helper: register a fake org membership under `ORG_HASH` so mutations
/// against org-scoped schemas pass the membership check in MutationManager.
fn register_test_org(db: &FoldDB, org_hash: &str) {
    let pool = db.sled_pool().expect("Expected sled backend").clone();
    fold_db::org::operations::insert_test_membership(&pool, org_hash)
        .expect("Failed to insert test org membership");
}

/// Helper: register a HashRange schema with optional org_hash via JSON.
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

/// Helper: write a mutation to a schema.
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

// === Unit tests for build_storage_key ===

#[test]
fn test_build_storage_key_personal() {
    assert_eq!(build_storage_key(None, "atom:uuid-1"), "atom:uuid-1");
    assert_eq!(build_storage_key(None, "ref:mol-1"), "ref:mol-1");
    assert_eq!(
        build_storage_key(None, "history:mol-1:00000"),
        "history:mol-1:00000"
    );
}

#[test]
fn test_build_storage_key_org() {
    assert_eq!(
        build_storage_key(Some(ORG_HASH), "atom:uuid-1"),
        format!("{ORG_HASH}:atom:uuid-1")
    );
    assert_eq!(
        build_storage_key(Some(ORG_HASH), "ref:mol-1"),
        format!("{ORG_HASH}:ref:mol-1")
    );
    assert_eq!(
        build_storage_key(Some(ORG_HASH), "history:mol-1:00000"),
        format!("{ORG_HASH}:history:mol-1:00000")
    );
}

// === Integration tests: org mutation produces prefixed keys ===

#[tokio::test]
async fn test_org_mutation_produces_prefixed_keys() {
    let tmp = tempfile::tempdir().unwrap();
    let db = make_folddb(&tmp).await;

    register_test_org(&db, ORG_HASH);
    register_schema(&db, "org_notes", Some(ORG_HASH)).await;
    write_mutation(&db, "org_notes", "meeting", "2026-01-01", "org body").await;

    // The schema should have org_hash set
    let schema = db
        .schema_manager()
        .get_schema("org_notes")
        .await
        .unwrap()
        .unwrap();
    assert_eq!(schema.org_hash.as_deref(), Some(ORG_HASH));

    // The underlying sled keys should be org-prefixed.
    let pool = db.sled_pool().expect("Expected sled backend");
    let guard = pool.acquire_arc().unwrap();
    let main_tree = guard.db().open_tree("main").unwrap();

    let org_prefix = format!("{ORG_HASH}:");
    let org_keys: Vec<String> = main_tree
        .iter()
        .filter_map(|r| r.ok())
        .map(|(k, _)| String::from_utf8_lossy(&k).to_string())
        .filter(|k| k.starts_with(&org_prefix))
        .collect();

    // There should be at least one atom and one ref key with the org prefix
    assert!(
        org_keys.iter().any(|k| k.contains(":atom:")),
        "Expected org-prefixed atom key, found: {:?}",
        org_keys
    );
    assert!(
        org_keys.iter().any(|k| k.contains(":ref:")),
        "Expected org-prefixed ref key, found: {:?}",
        org_keys
    );
}

// === Integration tests: org query reads from prefixed keys ===

#[tokio::test]
async fn test_org_query_reads_from_prefixed_keys() {
    let tmp = tempfile::tempdir().unwrap();
    let db = make_folddb(&tmp).await;

    register_test_org(&db, ORG_HASH);
    register_schema(&db, "org_events", Some(ORG_HASH)).await;
    write_mutation(&db, "org_events", "standup", "2026-03-01", "org event body").await;

    // Query it back
    let query = Query::new("org_events".to_string(), vec![]);
    let access = AccessContext::owner("test-owner");
    let result = db
        .query_executor()
        .query_with_access(query, &access, None)
        .await
        .expect("Query failed");

    // We should get at least one field with data
    assert!(
        !result.is_empty(),
        "Expected query results, got empty HashMap"
    );
    // The body field should contain "org event body"
    let body_values = result.get("body").expect("Missing 'body' field in results");
    let found = body_values
        .values()
        .any(|fv| fv.value == json!("org event body"));
    assert!(found, "Expected to find 'org event body' in query results");
}

// === Integration tests: personal and org data with same schema name don't collide ===

#[tokio::test]
async fn test_personal_and_org_data_do_not_collide() {
    let tmp = tempfile::tempdir().unwrap();
    let db = make_folddb(&tmp).await;

    // Register personal schema
    register_schema(&db, "notes", None).await;
    write_mutation(&db, "notes", "personal-key", "2026-01-01", "personal body").await;

    // Register org membership + org schema with different name
    register_test_org(&db, ORG_HASH);
    register_schema(&db, "org_notes", Some(ORG_HASH)).await;
    write_mutation(&db, "org_notes", "org-key", "2026-01-01", "org body").await;

    let access = AccessContext::owner("test-owner");

    // Query personal schema
    let personal_query = Query::new("notes".to_string(), vec!["body".to_string()]);
    let personal_result = db
        .query_executor()
        .query_with_access(personal_query, &access, None)
        .await
        .expect("Personal query failed");

    // Query org schema
    let org_query = Query::new("org_notes".to_string(), vec!["body".to_string()]);
    let org_result = db
        .query_executor()
        .query_with_access(org_query, &access, None)
        .await
        .expect("Org query failed");

    // Personal should only have "personal body"
    let personal_bodies = personal_result
        .get("body")
        .expect("Missing body in personal");
    assert!(personal_bodies
        .values()
        .any(|fv| fv.value == json!("personal body")));
    assert!(!personal_bodies
        .values()
        .any(|fv| fv.value == json!("org body")));

    // Org should only have "org body"
    let org_bodies = org_result.get("body").expect("Missing body in org");
    assert!(org_bodies.values().any(|fv| fv.value == json!("org body")));
    assert!(!org_bodies
        .values()
        .any(|fv| fv.value == json!("personal body")));

    // Verify at the Sled level that org keys are prefixed
    let pool = db.sled_pool().expect("Expected sled backend");
    let guard = pool.acquire_arc().unwrap();
    let main_tree = guard.db().open_tree("main").unwrap();

    let org_prefix = format!("{ORG_HASH}:");
    let all_keys: Vec<String> = main_tree
        .iter()
        .filter_map(|r| r.ok())
        .map(|(k, _)| String::from_utf8_lossy(&k).to_string())
        .collect();

    // Personal keys should NOT have the org prefix
    let personal_keys: Vec<&String> = all_keys
        .iter()
        .filter(|k| {
            !k.starts_with(&org_prefix) && (k.starts_with("atom:") || k.starts_with("ref:"))
        })
        .collect();
    assert!(
        !personal_keys.is_empty(),
        "Expected personal (non-prefixed) keys"
    );

    // Org keys should have the org prefix
    let org_keys: Vec<&String> = all_keys
        .iter()
        .filter(|k| k.starts_with(&org_prefix))
        .collect();
    assert!(!org_keys.is_empty(), "Expected org-prefixed keys");
}

// === Test that FieldCommon.org_hash is propagated during populate_runtime_fields ===

#[test]
fn test_org_hash_propagated_to_runtime_fields() {
    let mut schema = DeclarativeSchemaDefinition::new(
        "test_schema".to_string(),
        SchemaType::HashRange,
        Some(KeyConfig::new(
            Some("key".to_string()),
            Some("range".to_string()),
        )),
        Some(vec!["key".to_string(), "value".to_string()]),
        None,
        None,
    );
    schema.org_hash = Some(ORG_HASH.to_string());
    schema
        .populate_runtime_fields()
        .expect("Failed to populate runtime fields");

    for (field_name, field) in &schema.runtime_fields {
        assert_eq!(
            field.common().org_hash(),
            Some(ORG_HASH),
            "Field '{}' should have org_hash set",
            field_name
        );
    }
}

#[test]
fn test_personal_schema_has_no_org_hash_on_fields() {
    let mut schema = DeclarativeSchemaDefinition::new(
        "test_personal".to_string(),
        SchemaType::HashRange,
        Some(KeyConfig::new(
            Some("key".to_string()),
            Some("range".to_string()),
        )),
        Some(vec!["key".to_string(), "value".to_string()]),
        None,
        None,
    );
    schema
        .populate_runtime_fields()
        .expect("Failed to populate runtime fields");

    for (field_name, field) in &schema.runtime_fields {
        assert_eq!(
            field.common().org_hash(),
            None,
            "Personal field '{}' should NOT have org_hash",
            field_name
        );
    }
}

// === Security test: org membership is required for org-scoped mutations ===

/// Low-level helper that returns the raw Result so tests can assert on errors.
async fn try_write_mutation(
    db: &FoldDB,
    schema_name: &str,
    title: &str,
    date: &str,
    body: &str,
) -> Result<Vec<String>, fold_db::schema::SchemaError> {
    let mut fields = HashMap::new();
    fields.insert("title".to_string(), json!(title));
    fields.insert("body".to_string(), json!(body));
    fields.insert("date".to_string(), json!(date));

    let mutation = Mutation::new(
        schema_name.to_string(),
        fields,
        KeyValue::new(Some(title.to_string()), Some(date.to_string())),
        "attacker-pub-key".to_string(),
        MutationType::Create,
    );
    db.mutation_manager()
        .write_mutations_batch_async(vec![mutation])
        .await
}

/// A local attacker must not be able to inject writes against an org-scoped
/// schema for an org they are not a member of. Those writes would otherwise
/// be prefixed with `{org_hash}:` and queued for sync, polluting local Sled
/// state and attempting to upload under an unauthorized prefix.
#[tokio::test]
async fn test_org_mutation_denied_when_not_a_member() {
    let tmp = tempfile::tempdir().unwrap();
    let db = make_folddb(&tmp).await;

    // NOTE: no register_test_org — the node is deliberately not a member.
    register_schema(&db, "stranger_notes", Some(ORG_HASH)).await;

    let err = try_write_mutation(&db, "stranger_notes", "meeting", "2026-01-01", "leaked")
        .await
        .expect_err("mutation must be rejected — node is not a member of the org");

    match err {
        fold_db::schema::SchemaError::PermissionDenied(msg) => {
            assert!(
                msg.contains(ORG_HASH),
                "PermissionDenied message should mention the org hash: {}",
                msg
            );
            assert!(
                msg.to_lowercase().contains("not a member"),
                "PermissionDenied message should explain membership failure: {}",
                msg
            );
        }
        other => panic!("expected SchemaError::PermissionDenied, got {:?}", other),
    }

    // Also verify no org-prefixed keys were written to the underlying sled store.
    let pool = db.sled_pool().expect("Expected sled backend");
    let guard = pool.acquire_arc().unwrap();
    let main_tree = guard.db().open_tree("main").unwrap();

    let org_prefix = format!("{ORG_HASH}:");
    let leaked: Vec<String> = main_tree
        .iter()
        .filter_map(|r| r.ok())
        .map(|(k, _)| String::from_utf8_lossy(&k).to_string())
        .filter(|k| k.starts_with(&org_prefix))
        .collect();
    assert!(
        leaked.is_empty(),
        "Rejected mutation must not leave any org-prefixed keys in sled: {:?}",
        leaked
    );
}

/// Once a membership is registered for the same org hash, previously-denied
/// writes are accepted. This verifies the gate is gating on membership state,
/// not on schema shape.
#[tokio::test]
async fn test_org_mutation_allowed_after_joining_org() {
    let tmp = tempfile::tempdir().unwrap();
    let db = make_folddb(&tmp).await;

    register_schema(&db, "joined_notes", Some(ORG_HASH)).await;

    // Before joining — denied.
    let err = try_write_mutation(&db, "joined_notes", "m", "2026-01-01", "pre-join").await;
    assert!(
        matches!(err, Err(fold_db::schema::SchemaError::PermissionDenied(_))),
        "expected PermissionDenied before membership, got {:?}",
        err
    );

    // Join.
    register_test_org(&db, ORG_HASH);

    // After joining — allowed.
    let ids = try_write_mutation(&db, "joined_notes", "m", "2026-01-01", "post-join")
        .await
        .expect("mutation must succeed after org membership is registered");
    assert_eq!(ids.len(), 1);
}
