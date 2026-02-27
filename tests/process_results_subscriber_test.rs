//! Integration test for the ProcessResultsSubscriber.
//!
//! Verifies that when mutations carry a `progress_id` in their metadata,
//! the ProcessResultsSubscriber writes the actual stored key_value to the
//! `process_results` namespace, and `FoldNode::get_process_results` returns
//! the correct outcomes.

mod common;

use fold_db::fold_node::node::FoldNode;
use fold_db::logging::core::run_with_user;
use fold_db::schema::types::operations::MutationType;
use fold_db::schema::types::{KeyValue, Mutation};
use fold_db::testing_utils::TestDatabaseFactory;
use serde_json::json;
use std::collections::HashMap;

/// Helper: create a FoldNode with a fresh temp database and generated identity.
async fn create_test_node() -> (FoldNode, String) {
    let mut config = TestDatabaseFactory::create_test_node_config();
    let keypair = fold_db::security::Ed25519KeyPair::generate().unwrap();
    let user_id = keypair.public_key_base64();
    config = config.with_identity(&user_id, &keypair.secret_key_base64());
    let node = FoldNode::new(config).await.expect("Failed to create test node");
    (node, user_id)
}

/// Helper: load and approve a simple Hash-key schema into the node.
async fn load_test_schema(node: &FoldNode) {
    let schema_json = r#"{
        "name": "TestContact",
        "type": "Single",
        "key": {
            "hash_field": "email"
        },
        "fields": ["email", "name", "phone"],
        "field_topologies": {
            "email": {
                "root": { "type": "Primitive", "value": "String" }
            },
            "name": {
                "root": { "type": "Primitive", "value": "String" }
            },
            "phone": {
                "root": { "type": "Primitive", "value": "String" }
            }
        }
    }"#;

    let mut db = node.get_fold_db().await.unwrap();
    db.load_schema_from_json(schema_json).await.unwrap();
    db.schema_manager().approve("TestContact").await.unwrap();
}

/// Helper: load and approve a Range-key schema into the node.
async fn load_range_schema(node: &FoldNode) {
    let schema_json = r#"{
        "name": "EventLog",
        "type": "Single",
        "key": {
            "range_field": "timestamp"
        },
        "fields": ["timestamp", "message", "level"],
        "field_topologies": {
            "timestamp": {
                "root": { "type": "Primitive", "value": "String" }
            },
            "message": {
                "root": { "type": "Primitive", "value": "String" }
            },
            "level": {
                "root": { "type": "Primitive", "value": "String" }
            }
        }
    }"#;

    let mut db = node.get_fold_db().await.unwrap();
    db.load_schema_from_json(schema_json).await.unwrap();
    db.schema_manager().approve("EventLog").await.unwrap();
}

/// Build a mutation with metadata containing the given progress_id.
fn build_mutation_with_progress(
    schema_name: &str,
    fields_and_values: HashMap<String, serde_json::Value>,
    key_value: KeyValue,
    pub_key: &str,
    progress_id: &str,
) -> Mutation {
    let mut metadata = HashMap::new();
    metadata.insert("progress_id".to_string(), progress_id.to_string());

    Mutation::new(
        schema_name.to_string(),
        fields_and_values,
        key_value,
        pub_key.to_string(),
        MutationType::Create,
    )
    .with_metadata(metadata)
}

/// Core test: a single mutation with progress_id produces one ProcessResult entry.
#[tokio::test(flavor = "multi_thread")]
async fn test_process_results_single_mutation() {
    let (node, user_id) = create_test_node().await;
    load_test_schema(&node).await;

    let progress_id = "test-progress-001";

    let mut fields = HashMap::new();
    fields.insert("email".to_string(), json!("alice@example.com"));
    fields.insert("name".to_string(), json!("Alice"));
    fields.insert("phone".to_string(), json!("555-0100"));

    let mutation = build_mutation_with_progress(
        "TestContact",
        fields,
        KeyValue::new(Some("alice@example.com".to_string()), None),
        &user_id,
        progress_id,
    );

    // Execute within user context (required for multi-tenant isolation)
    let mutation_ids = run_with_user(&user_id, node.mutate_batch(vec![mutation]))
        .await
        .expect("mutate_batch failed");
    assert_eq!(mutation_ids.len(), 1);

    // Give the async subscriber a moment to process the event
    tokio::time::sleep(std::time::Duration::from_millis(200)).await;

    // Query process results
    let results = node
        .get_process_results(progress_id)
        .await
        .expect("get_process_results failed");

    assert_eq!(results.len(), 1, "Expected exactly one process result");
    let outcome = &results[0];
    assert_eq!(outcome.schema_name, "TestContact");
    // The key_value should have the hash field set by KeyValue::from_mutation
    assert!(
        outcome.key_value.hash.is_some(),
        "Expected hash key to be set"
    );
    assert_eq!(
        outcome.key_value.hash.as_deref(),
        Some("alice@example.com"),
        "Hash key should match the email field value"
    );
}

/// Multiple mutations in a single batch, all sharing the same progress_id.
#[tokio::test(flavor = "multi_thread")]
async fn test_process_results_batch_mutations() {
    let (node, user_id) = create_test_node().await;
    load_test_schema(&node).await;

    let progress_id = "test-progress-batch";

    let contacts = vec![
        ("bob@example.com", "Bob", "555-0200"),
        ("carol@example.com", "Carol", "555-0300"),
        ("dave@example.com", "Dave", "555-0400"),
    ];

    let mutations: Vec<Mutation> = contacts
        .iter()
        .map(|(email, name, phone)| {
            let mut fields = HashMap::new();
            fields.insert("email".to_string(), json!(email));
            fields.insert("name".to_string(), json!(name));
            fields.insert("phone".to_string(), json!(phone));

            build_mutation_with_progress(
                "TestContact",
                fields,
                KeyValue::new(Some(email.to_string()), None),
                &user_id,
                progress_id,
            )
        })
        .collect();

    let ids = run_with_user(&user_id, node.mutate_batch(mutations))
        .await
        .expect("mutate_batch failed");
    assert_eq!(ids.len(), 3);

    tokio::time::sleep(std::time::Duration::from_millis(200)).await;

    let results = node
        .get_process_results(progress_id)
        .await
        .expect("get_process_results failed");

    assert_eq!(results.len(), 3, "Expected 3 process results for the batch");

    // All should be for the TestContact schema
    for outcome in &results {
        assert_eq!(outcome.schema_name, "TestContact");
        assert!(outcome.key_value.hash.is_some());
    }

    // Verify all emails are present
    let mut emails: Vec<String> = results
        .iter()
        .filter_map(|r| r.key_value.hash.clone())
        .collect();
    emails.sort();
    assert_eq!(
        emails,
        vec![
            "bob@example.com".to_string(),
            "carol@example.com".to_string(),
            "dave@example.com".to_string(),
        ]
    );
}

/// Mutations without progress_id in metadata should NOT produce process results.
#[tokio::test(flavor = "multi_thread")]
async fn test_process_results_no_metadata_no_entry() {
    let (node, user_id) = create_test_node().await;
    load_test_schema(&node).await;

    let mut fields = HashMap::new();
    fields.insert("email".to_string(), json!("eve@example.com"));
    fields.insert("name".to_string(), json!("Eve"));
    fields.insert("phone".to_string(), json!("555-0500"));

    // Mutation WITHOUT metadata
    let mutation = Mutation::new(
        "TestContact".to_string(),
        fields,
        KeyValue::new(Some("eve@example.com".to_string()), None),
        user_id.clone(),
        MutationType::Create,
    );

    run_with_user(&user_id, node.mutate_batch(vec![mutation]))
        .await
        .expect("mutate_batch failed");

    tokio::time::sleep(std::time::Duration::from_millis(200)).await;

    // Query with a made-up progress_id — should be empty
    let results = node
        .get_process_results("nonexistent-progress-id")
        .await
        .expect("get_process_results failed");
    assert!(
        results.is_empty(),
        "No process results should exist for mutations without progress_id"
    );
}

/// Different progress_ids are isolated: querying one doesn't return the other's results.
#[tokio::test(flavor = "multi_thread")]
async fn test_process_results_isolation_between_progress_ids() {
    let (node, user_id) = create_test_node().await;
    load_test_schema(&node).await;

    let progress_a = "progress-aaa";
    let progress_b = "progress-bbb";

    // Mutation for progress A
    let mut fields_a = HashMap::new();
    fields_a.insert("email".to_string(), json!("frank@example.com"));
    fields_a.insert("name".to_string(), json!("Frank"));
    fields_a.insert("phone".to_string(), json!("555-0600"));
    let mut_a = build_mutation_with_progress(
        "TestContact",
        fields_a,
        KeyValue::new(Some("frank@example.com".to_string()), None),
        &user_id,
        progress_a,
    );

    // Mutation for progress B
    let mut fields_b = HashMap::new();
    fields_b.insert("email".to_string(), json!("grace@example.com"));
    fields_b.insert("name".to_string(), json!("Grace"));
    fields_b.insert("phone".to_string(), json!("555-0700"));
    let mut_b = build_mutation_with_progress(
        "TestContact",
        fields_b,
        KeyValue::new(Some("grace@example.com".to_string()), None),
        &user_id,
        progress_b,
    );

    run_with_user(&user_id, node.mutate_batch(vec![mut_a, mut_b]))
        .await
        .expect("mutate_batch failed");

    tokio::time::sleep(std::time::Duration::from_millis(200)).await;

    let results_a = node.get_process_results(progress_a).await.unwrap();
    let results_b = node.get_process_results(progress_b).await.unwrap();

    assert_eq!(results_a.len(), 1, "Progress A should have exactly 1 result");
    assert_eq!(results_b.len(), 1, "Progress B should have exactly 1 result");

    assert_eq!(
        results_a[0].key_value.hash.as_deref(),
        Some("frank@example.com")
    );
    assert_eq!(
        results_b[0].key_value.hash.as_deref(),
        Some("grace@example.com")
    );
}

/// Test with a Range-key schema to verify the subscriber captures the correct range key.
#[tokio::test(flavor = "multi_thread")]
async fn test_process_results_range_key_schema() {
    let (node, user_id) = create_test_node().await;
    load_range_schema(&node).await;

    let progress_id = "test-progress-range";

    let mut fields = HashMap::new();
    fields.insert("timestamp".to_string(), json!("2024-06-15T10:30:00Z"));
    fields.insert("message".to_string(), json!("Server started"));
    fields.insert("level".to_string(), json!("info"));

    let mutation = build_mutation_with_progress(
        "EventLog",
        fields,
        KeyValue::new(None, Some("2024-06-15T10:30:00Z".to_string())),
        &user_id,
        progress_id,
    );

    run_with_user(&user_id, node.mutate_batch(vec![mutation]))
        .await
        .expect("mutate_batch failed");

    tokio::time::sleep(std::time::Duration::from_millis(200)).await;

    let results = node
        .get_process_results(progress_id)
        .await
        .expect("get_process_results failed");

    assert_eq!(results.len(), 1);
    assert_eq!(results[0].schema_name, "EventLog");
    // Range key should be set
    assert!(
        results[0].key_value.range.is_some(),
        "Expected range key to be set for EventLog"
    );
}

/// Test that mutations across different schemas in the same progress_id
/// produce correct results for each schema.
#[tokio::test(flavor = "multi_thread")]
async fn test_process_results_multiple_schemas() {
    let (node, user_id) = create_test_node().await;
    load_test_schema(&node).await;
    load_range_schema(&node).await;

    let progress_id = "test-progress-multi-schema";

    // Contact mutation
    let mut contact_fields = HashMap::new();
    contact_fields.insert("email".to_string(), json!("heidi@example.com"));
    contact_fields.insert("name".to_string(), json!("Heidi"));
    contact_fields.insert("phone".to_string(), json!("555-0800"));
    let contact_mut = build_mutation_with_progress(
        "TestContact",
        contact_fields,
        KeyValue::new(Some("heidi@example.com".to_string()), None),
        &user_id,
        progress_id,
    );

    // Event mutation
    let mut event_fields = HashMap::new();
    event_fields.insert("timestamp".to_string(), json!("2024-07-01T12:00:00Z"));
    event_fields.insert("message".to_string(), json!("User logged in"));
    event_fields.insert("level".to_string(), json!("info"));
    let event_mut = build_mutation_with_progress(
        "EventLog",
        event_fields,
        KeyValue::new(None, Some("2024-07-01T12:00:00Z".to_string())),
        &user_id,
        progress_id,
    );

    // Execute both in same batch (they'll be grouped by schema in mutation_manager)
    run_with_user(&user_id, node.mutate_batch(vec![contact_mut, event_mut]))
        .await
        .expect("mutate_batch failed");

    tokio::time::sleep(std::time::Duration::from_millis(200)).await;

    let results = node
        .get_process_results(progress_id)
        .await
        .expect("get_process_results failed");

    assert_eq!(
        results.len(),
        2,
        "Expected 2 results (one per schema) under the same progress_id"
    );

    let schemas: Vec<&str> = results.iter().map(|r| r.schema_name.as_str()).collect();
    assert!(
        schemas.contains(&"TestContact"),
        "Results should include TestContact"
    );
    assert!(
        schemas.contains(&"EventLog"),
        "Results should include EventLog"
    );
}
