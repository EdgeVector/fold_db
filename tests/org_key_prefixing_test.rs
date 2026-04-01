//! Tests for org key prefixing in the mutation and query paths.
//!
//! When a schema has `org_hash = Some(hash)`, all Sled keys for data in that
//! schema should be prefixed with `{org_hash}:`.

use fold_db::access::AccessContext;
use fold_db::fold_db_core::FoldDB;
use fold_db::schema::types::field::{build_storage_key, Field};
use fold_db::schema::types::operations::{MutationType, Query};
use fold_db::schema::types::{DeclarativeSchemaDefinition, KeyConfig, KeyValue, Mutation, SchemaType};
use fold_db::schema::SchemaState;
use serde_json::json;
use std::collections::HashMap;

const ORG_HASH: &str = "abc123def456";

/// Helper: create a FoldDB instance backed by a temp sled directory.
async fn make_folddb(tmp: &tempfile::TempDir) -> FoldDB {
    FoldDB::new(tmp.path().to_str().unwrap())
        .await
        .expect("Failed to create FoldDB")
}

/// Helper: register a HashRange schema with optional org_hash via JSON.
async fn register_schema(db: &mut FoldDB, name: &str, org_hash: Option<&str>) {
    let org_hash_json = match org_hash {
        Some(h) => format!(r#", "org_hash": "{}""#, h),
        None => String::new(),
    };
    let json_str = format!(
        r#"{{
            "name": "{}",
            "key": {{ "hash_field": "title", "range_field": "date" }},
            "fields": {{ "title": {{}}, "body": {{}}, "date": {{}} }}
            {}
        }}"#,
        name, org_hash_json
    );
    db.load_schema_from_json(&json_str).await.unwrap();
    db.schema_manager
        .set_schema_state(name, SchemaState::Approved)
        .await
        .unwrap();
}

/// Helper: write a mutation to a schema.
async fn write_mutation(
    db: &mut FoldDB,
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
    db.mutation_manager
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
    let mut db = make_folddb(&tmp).await;

    register_schema(&mut db, "org_notes", Some(ORG_HASH)).await;
    write_mutation(&mut db, "org_notes", "meeting", "2026-01-01", "org body").await;

    // The schema should have org_hash set
    let schema = db
        .schema_manager
        .get_schema("org_notes")
        .await
        .unwrap()
        .unwrap();
    assert_eq!(schema.org_hash.as_deref(), Some(ORG_HASH));

    // The underlying sled keys should be org-prefixed.
    let sled_db = db.sled_db().expect("Expected sled backend");
    let main_tree = sled_db.open_tree("main").unwrap();

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
    let mut db = make_folddb(&tmp).await;

    register_schema(&mut db, "org_events", Some(ORG_HASH)).await;
    write_mutation(
        &mut db,
        "org_events",
        "standup",
        "2026-03-01",
        "org event body",
    )
    .await;

    // Query it back
    let query = Query::new("org_events".to_string(), vec![]);
    let access = AccessContext::owner("test-owner");
    let result = db
        .query_executor
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
    let mut db = make_folddb(&tmp).await;

    // Register personal schema
    register_schema(&mut db, "notes", None).await;
    write_mutation(&mut db, "notes", "personal-key", "2026-01-01", "personal body").await;

    // Register org schema with different name
    register_schema(&mut db, "org_notes", Some(ORG_HASH)).await;
    write_mutation(
        &mut db,
        "org_notes",
        "org-key",
        "2026-01-01",
        "org body",
    )
    .await;

    let access = AccessContext::owner("test-owner");

    // Query personal schema
    let personal_query = Query::new("notes".to_string(), vec!["body".to_string()]);
    let personal_result = db
        .query_executor
        .query_with_access(personal_query, &access, None)
        .await
        .expect("Personal query failed");

    // Query org schema
    let org_query = Query::new("org_notes".to_string(), vec!["body".to_string()]);
    let org_result = db
        .query_executor
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
    let sled_db = db.sled_db().expect("Expected sled backend");
    let main_tree = sled_db.open_tree("main").unwrap();

    let org_prefix = format!("{ORG_HASH}:");
    let all_keys: Vec<String> = main_tree
        .iter()
        .filter_map(|r| r.ok())
        .map(|(k, _)| String::from_utf8_lossy(&k).to_string())
        .collect();

    // Personal keys should NOT have the org prefix
    let personal_keys: Vec<&String> = all_keys
        .iter()
        .filter(|k| !k.starts_with(&org_prefix) && (k.starts_with("atom:") || k.starts_with("ref:")))
        .collect();
    assert!(
        !personal_keys.is_empty(),
        "Expected personal (non-prefixed) keys"
    );

    // Org keys should have the org prefix
    let org_keys: Vec<&String> = all_keys.iter().filter(|k| k.starts_with(&org_prefix)).collect();
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
