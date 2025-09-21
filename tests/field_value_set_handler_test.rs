//! Integration test for FieldValueSetRequest handler in AtomManager
//!
//! This test verifies that the critical mutation bug fix is working correctly
//! by testing the new FieldValueSetRequest handler implementation.

use datafold::db_operations::DbOperations;
use datafold::fees::types::config::FieldPaymentConfig;
use datafold::fold_db_core::infrastructure::message_bus::{
    request_events::{FieldValueSetRequest, FieldValueSetResponse},
    MessageBus,
};
use datafold::fold_db_core::managers::atom::AtomManager;
use datafold::permissions::types::policy::PermissionsPolicy;
use datafold::schema::json_schema::KeyConfig;
use datafold::schema::types::field::{RangeField, SingleField};
use datafold::schema::types::{FieldVariant, Schema};
use serde_json::json;
use std::collections::HashMap;
use std::sync::Arc;
use std::thread;
use std::time::Duration;
use tempfile::tempdir;

#[test]
fn test_field_value_set_request_handler() {
    // Setup database
    let temp_dir = tempdir().expect("Failed to create temp dir");
    let db = sled::Config::new()
        .path(temp_dir.path())
        .temporary(true)
        .open()
        .expect("Failed to open database");

    let db_ops = DbOperations::new(db).expect("Failed to create DbOperations");

    let mut schema = Schema::new("user_schema".to_string());
    schema.fields.insert(
        "username".to_string(),
        FieldVariant::Single(SingleField::new(
            PermissionsPolicy::default(),
            FieldPaymentConfig::default(),
            HashMap::new(),
        )),
    );
    db_ops
        .store_schema("user_schema", &schema)
        .expect("Failed to store user schema");
    let message_bus = Arc::new(MessageBus::new());

    // Create AtomManager with the new FieldValueSetRequest handler
    let _atom_manager = AtomManager::new(db_ops, Arc::clone(&message_bus));

    // Subscribe to FieldValueSetResponse events
    let mut response_consumer = message_bus.subscribe::<FieldValueSetResponse>();

    // Create a FieldValueSetRequest
    let request = FieldValueSetRequest::new(
        "test_correlation_123".to_string(),
        "user_schema".to_string(),
        "username".to_string(),
        json!({
            "username": "alice_test"
        }),
        "test_pubkey_456".to_string(),
    );

    // Publish the request
    message_bus
        .publish(request)
        .expect("Failed to publish FieldValueSetRequest");

    // Give the handler time to process the request
    thread::sleep(Duration::from_millis(200));

    // Check for the response
    let response = response_consumer
        .recv_timeout(Duration::from_millis(500))
        .expect("Should receive FieldValueSetResponse");

    // Verify the response
    assert_eq!(response.correlation_id, "test_correlation_123");
    assert!(response.success, "FieldValueSetRequest should succeed");
    assert!(
        response.molecule_uuid.is_some(),
        "Should return an Molecule UUID"
    );
    assert!(response.error.is_none(), "Should not have an error");

    // The Molecule UUID should follow our naming convention
    let molecule_uuid = response.molecule_uuid.unwrap();
    assert!(
        molecule_uuid.contains("user_schema_username"),
        "Molecule UUID should contain schema and field name: {}",
        molecule_uuid
    );
    assert!(
        molecule_uuid.contains("single") || molecule_uuid.contains("range"),
        "Molecule UUID should indicate field type: {}",
        molecule_uuid
    );

    println!("✅ FieldValueSetRequest handler test passed!");
    println!("   Correlation ID: {}", response.correlation_id);
    println!("   Molecule UUID: {}", molecule_uuid);
    println!("   Success: {}", response.success);
}

#[test]
fn test_field_value_set_request_range_field() {
    // Setup database
    let temp_dir = tempdir().expect("Failed to create temp dir");
    let db = sled::Config::new()
        .path(temp_dir.path())
        .temporary(true)
        .open()
        .expect("Failed to open database");

    let db_ops = DbOperations::new(db).expect("Failed to create DbOperations");

    let mut schema = Schema::new_range(
        "analytics_schema".to_string(),
        "score_timestamp".to_string(),
    );
    schema.key = Some(KeyConfig {
        hash_field: "user_id".to_string(),
        range_field: "score_timestamp".to_string(),
    });
    schema.fields.insert(
        "score_range".to_string(),
        FieldVariant::Range(RangeField::new(
            PermissionsPolicy::default(),
            FieldPaymentConfig::default(),
            HashMap::new(),
        )),
    );
    db_ops
        .store_schema("analytics_schema", &schema)
        .expect("Failed to store analytics schema");
    let message_bus = Arc::new(MessageBus::new());

    // Create AtomManager with the new FieldValueSetRequest handler
    let _atom_manager = AtomManager::new(db_ops, Arc::clone(&message_bus));

    // Subscribe to FieldValueSetResponse events
    let mut response_consumer = message_bus.subscribe::<FieldValueSetResponse>();

    // Create a FieldValueSetRequest for a range field (field name contains "range")
    let request = FieldValueSetRequest::new(
        "test_range_456".to_string(),
        "analytics_schema".to_string(),
        "score_range".to_string(), // This should trigger Range field type
        json!({
            "score_range": [1, 2, 3, 4, 5],
            "user_id": "user-123",
            "score_timestamp": "2023-01-01T00:00:00Z"
        }),
        "test_range_pubkey_789".to_string(),
    );

    // Publish the request
    message_bus
        .publish(request)
        .expect("Failed to publish FieldValueSetRequest");

    // Give the handler time to process the request
    thread::sleep(Duration::from_millis(200));

    // Check for the response
    let response = response_consumer
        .recv_timeout(Duration::from_millis(500))
        .expect("Should receive FieldValueSetResponse");

    // Verify the response
    assert_eq!(response.correlation_id, "test_range_456");
    assert!(response.success, "FieldValueSetRequest should succeed");
    assert!(
        response.molecule_uuid.is_some(),
        "Should return an Molecule UUID"
    );
    assert!(response.error.is_none(), "Should not have an error");

    // The Molecule UUID should indicate it's a range field
    let molecule_uuid = response.molecule_uuid.unwrap();
    assert!(
        molecule_uuid.contains("range"),
        "Range field should create MoleculeRange: {}",
        molecule_uuid
    );

    println!("✅ FieldValueSetRequest range field test passed!");
    println!("   Correlation ID: {}", response.correlation_id);
    println!("   Molecule UUID: {}", molecule_uuid);
    println!("   Success: {}", response.success);
}

#[test]
fn test_field_value_set_statistics() {
    // Setup database
    let temp_dir = tempdir().expect("Failed to create temp dir");
    let db = sled::Config::new()
        .path(temp_dir.path())
        .temporary(true)
        .open()
        .expect("Failed to open database");

    let db_ops = DbOperations::new(db).expect("Failed to create DbOperations");

    let mut schema = Schema::new("test_schema".to_string());
    schema.fields.insert(
        "test_field".to_string(),
        FieldVariant::Single(SingleField::new(
            PermissionsPolicy::default(),
            FieldPaymentConfig::default(),
            HashMap::new(),
        )),
    );
    db_ops
        .store_schema("test_schema", &schema)
        .expect("Failed to store test schema");
    let message_bus = Arc::new(MessageBus::new());

    // Create AtomManager
    let atom_manager = AtomManager::new(db_ops, Arc::clone(&message_bus));

    // Get initial statistics
    let initial_stats = atom_manager.get_stats();
    let initial_requests = initial_stats.requests_processed;
    let initial_atoms = initial_stats.atoms_created;
    let initial_refs = initial_stats.molecules_created;

    // Subscribe to FieldValueSetResponse events
    let mut response_consumer = message_bus.subscribe::<FieldValueSetResponse>();

    // Create and publish a FieldValueSetRequest
    let request = FieldValueSetRequest::new(
        "stats_test_789".to_string(),
        "test_schema".to_string(),
        "test_field".to_string(),
        json!({
            "test_field": "test_value"
        }),
        "stats_test_pubkey".to_string(),
    );

    message_bus
        .publish(request)
        .expect("Failed to publish FieldValueSetRequest");

    // Wait for processing
    thread::sleep(Duration::from_millis(200));

    // Verify response received
    let _response = response_consumer
        .recv_timeout(Duration::from_millis(500))
        .expect("Should receive FieldValueSetResponse");

    // Check that statistics were updated
    let final_stats = atom_manager.get_stats();

    assert_eq!(
        final_stats.requests_processed,
        initial_requests + 1,
        "Should increment requests processed"
    );
    assert_eq!(
        final_stats.atoms_created,
        initial_atoms + 1,
        "Should increment atoms created"
    );
    assert_eq!(
        final_stats.molecules_created,
        initial_refs + 1,
        "Should increment molecules created"
    );

    println!("✅ FieldValueSetRequest statistics test passed!");
    println!(
        "   Requests processed: {} -> {}",
        initial_requests, final_stats.requests_processed
    );
    println!(
        "   Atoms created: {} -> {}",
        initial_atoms, final_stats.atoms_created
    );
    println!(
        "   Molecules created: {} -> {}",
        initial_refs, final_stats.molecules_created
    );
}
