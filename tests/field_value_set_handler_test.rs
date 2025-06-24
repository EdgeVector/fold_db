//! Integration test for FieldValueSetRequest handler in AtomManager
//!
//! This test verifies that the critical mutation bug fix is working correctly
//! by testing the new FieldValueSetRequest handler implementation.

use datafold::fold_db_core::infrastructure::message_bus::{
    MessageBus,
    request_events::{FieldValueSetRequest, FieldValueSetResponse},
};
use datafold::fold_db_core::managers::atom::AtomManager;
use datafold::db_operations::DbOperations;
use serde_json::json;
use std::sync::Arc;
use std::time::Duration;
use std::thread;
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
        json!("alice_test"),
        "test_pubkey_456".to_string(),
    );
    
    // Publish the request
    message_bus.publish(request).expect("Failed to publish FieldValueSetRequest");
    
    // Give the handler time to process the request
    thread::sleep(Duration::from_millis(200));
    
    // Check for the response
    let response = response_consumer.recv_timeout(Duration::from_millis(500))
        .expect("Should receive FieldValueSetResponse");
    
    // Verify the response
    assert_eq!(response.correlation_id, "test_correlation_123");
    assert!(response.success, "FieldValueSetRequest should succeed");
    assert!(response.molecule_uuid.is_some(), "Should return an AtomRef UUID");
    assert!(response.error.is_none(), "Should not have an error");
    
    // The AtomRef UUID should follow our naming convention
    let aref_uuid = response.molecule_uuid.unwrap();
    assert!(
        aref_uuid.contains("user_schema_username"), 
        "AtomRef UUID should contain schema and field name: {}", 
        aref_uuid
    );
    assert!(
        aref_uuid.contains("single") || aref_uuid.contains("range"),
        "AtomRef UUID should indicate field type: {}",
        aref_uuid
    );
    
    println!("✅ FieldValueSetRequest handler test passed!");
    println!("   Correlation ID: {}", response.correlation_id);
    println!("   AtomRef UUID: {}", aref_uuid);
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
        json!([1, 2, 3, 4, 5]),
        "test_range_pubkey_789".to_string(),
    );
    
    // Publish the request
    message_bus.publish(request).expect("Failed to publish FieldValueSetRequest");
    
    // Give the handler time to process the request
    thread::sleep(Duration::from_millis(200));
    
    // Check for the response
    let response = response_consumer.recv_timeout(Duration::from_millis(500))
        .expect("Should receive FieldValueSetResponse");
    
    // Verify the response
    assert_eq!(response.correlation_id, "test_range_456");
    assert!(response.success, "FieldValueSetRequest should succeed");
    assert!(response.molecule_uuid.is_some(), "Should return an AtomRef UUID");
    assert!(response.error.is_none(), "Should not have an error");
    
    // The AtomRef UUID should indicate it's a range field
    let aref_uuid = response.molecule_uuid.unwrap();
    assert!(
        aref_uuid.contains("range"), 
        "Range field should create AtomRefRange: {}", 
        aref_uuid
    );
    
    println!("✅ FieldValueSetRequest range field test passed!");
    println!("   Correlation ID: {}", response.correlation_id);
    println!("   AtomRef UUID: {}", aref_uuid);
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
    let message_bus = Arc::new(MessageBus::new());
    
    // Create AtomManager
    let atom_manager = AtomManager::new(db_ops, Arc::clone(&message_bus));
    
    // Get initial statistics
    let initial_stats = atom_manager.get_stats();
    let initial_requests = initial_stats.requests_processed;
    let initial_atoms = initial_stats.atoms_created;
    let initial_refs = initial_stats.atom_refs_created;
    
    // Subscribe to FieldValueSetResponse events
    let mut response_consumer = message_bus.subscribe::<FieldValueSetResponse>();
    
    // Create and publish a FieldValueSetRequest
    let request = FieldValueSetRequest::new(
        "stats_test_789".to_string(),
        "test_schema".to_string(),
        "test_field".to_string(),
        json!("test_value"),
        "stats_test_pubkey".to_string(),
    );
    
    message_bus.publish(request).expect("Failed to publish FieldValueSetRequest");
    
    // Wait for processing
    thread::sleep(Duration::from_millis(200));
    
    // Verify response received
    let _response = response_consumer.recv_timeout(Duration::from_millis(500))
        .expect("Should receive FieldValueSetResponse");
    
    // Check that statistics were updated
    let final_stats = atom_manager.get_stats();
    
    assert_eq!(final_stats.requests_processed, initial_requests + 1, "Should increment requests processed");
    assert_eq!(final_stats.atoms_created, initial_atoms + 1, "Should increment atoms created");
    assert_eq!(final_stats.atom_refs_created, initial_refs + 1, "Should increment atom refs created");
    
    println!("✅ FieldValueSetRequest statistics test passed!");
    println!("   Requests processed: {} -> {}", initial_requests, final_stats.requests_processed);
    println!("   Atoms created: {} -> {}", initial_atoms, final_stats.atoms_created);
    println!("   AtomRefs created: {} -> {}", initial_refs, final_stats.atom_refs_created);
}