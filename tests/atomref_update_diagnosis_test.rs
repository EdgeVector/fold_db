//! Comprehensive integration test to diagnose AtomRef update failures
//!
//! This test covers the complete mutation→query flow to identify where
//! the AtomRef update is failing in the FieldValueSetRequest handler.

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
fn test_atomref_update_complete_flow() {
    println!("🔍 STARTING COMPREHENSIVE ATOMREF UPDATE DIAGNOSIS TEST");
    
    // Setup database
    let temp_dir = tempdir().expect("Failed to create temp dir");
    let db = sled::Config::new()
        .path(temp_dir.path())
        .temporary(true)
        .open()
        .expect("Failed to open database");
    
    let db_ops = DbOperations::new(db).expect("Failed to create DbOperations");
    let message_bus = Arc::new(MessageBus::new());
    
    // Create AtomManager with diagnostic logging
    let _atom_manager = AtomManager::new(db_ops, Arc::clone(&message_bus));
    
    // Subscribe to FieldValueSetResponse events
    let mut response_consumer = message_bus.subscribe::<FieldValueSetResponse>();
    
    println!("📝 STEP 1: Creating first field value (initial state)");
    
    // Create first FieldValueSetRequest for user.username field
    let request1 = FieldValueSetRequest::new(
        "test_correlation_001".to_string(),
        "user_schema".to_string(),
        "username".to_string(),
        json!("alice_v1"),
        "test_pubkey_001".to_string(),
    );
    
    message_bus.publish(request1).expect("Failed to publish first FieldValueSetRequest");
    thread::sleep(Duration::from_millis(300)); // Give handler time to process
    
    let response1 = response_consumer.recv_timeout(Duration::from_millis(500))
        .expect("Should receive first FieldValueSetResponse");
    
    assert!(response1.success, "First FieldValueSetRequest should succeed");
    let aref_uuid = response1.molecule_uuid.as_ref().expect("Should return AtomRef UUID");
    
    println!("✅ STEP 1 COMPLETE: AtomRef UUID: {}", aref_uuid);
    
    // Allow time for all logging to be processed
    thread::sleep(Duration::from_millis(200));
    
    println!("📝 STEP 2: Creating second field value (should update AtomRef)");
    
    // Create second FieldValueSetRequest for same field (should update AtomRef)
    let request2 = FieldValueSetRequest::new(
        "test_correlation_002".to_string(),
        "user_schema".to_string(),
        "username".to_string(), // Same schema.field combination
        json!("alice_v2"), // Different value
        "test_pubkey_002".to_string(),
    );
    
    message_bus.publish(request2).expect("Failed to publish second FieldValueSetRequest");
    thread::sleep(Duration::from_millis(300)); // Give handler time to process
    
    let response2 = response_consumer.recv_timeout(Duration::from_millis(500))
        .expect("Should receive second FieldValueSetResponse");
    
    assert!(response2.success, "Second FieldValueSetRequest should succeed");
    let aref_uuid_2 = response2.molecule_uuid.as_ref().expect("Should return AtomRef UUID");
    
    println!("✅ STEP 2 COMPLETE: AtomRef UUID: {}", aref_uuid_2);
    
    // Allow time for all logging to be processed
    thread::sleep(Duration::from_millis(200));
    
    println!("🔍 CRITICAL VALIDATION: AtomRef UUIDs should be identical (same field)");
    assert_eq!(aref_uuid, aref_uuid_2, 
        "AtomRef UUID should be the same for both requests (same schema.field): {} vs {}", 
        aref_uuid, aref_uuid_2);
    
    println!("📝 STEP 3: Creating third field value (final update test)");
    
    // Create third FieldValueSetRequest to test multiple updates
    let request3 = FieldValueSetRequest::new(
        "test_correlation_003".to_string(),
        "user_schema".to_string(),
        "username".to_string(), // Same schema.field combination
        json!("alice_v3"), // Different value
        "test_pubkey_003".to_string(),
    );
    
    message_bus.publish(request3).expect("Failed to publish third FieldValueSetRequest");
    thread::sleep(Duration::from_millis(300)); // Give handler time to process
    
    let response3 = response_consumer.recv_timeout(Duration::from_millis(500))
        .expect("Should receive third FieldValueSetResponse");
    
    assert!(response3.success, "Third FieldValueSetRequest should succeed");
    let aref_uuid_3 = response3.molecule_uuid.as_ref().expect("Should return AtomRef UUID");
    
    println!("✅ STEP 3 COMPLETE: AtomRef UUID: {}", aref_uuid_3);
    
    // Allow time for all logging to be processed  
    thread::sleep(Duration::from_millis(500));
    
    println!("🔍 FINAL VALIDATION: All AtomRef UUIDs should be identical");
    assert_eq!(aref_uuid, aref_uuid_3, 
        "All AtomRef UUIDs should match (same schema.field): {} vs {}", 
        aref_uuid, aref_uuid_3);
    
    println!("✅ ATOMREF UPDATE DIAGNOSIS TEST COMPLETED SUCCESSFULLY");
    println!("   - Created 3 atoms for same field");
    println!("   - Verified AtomRef UUID consistency: {}", aref_uuid);
    println!("   - Check logs above for detailed AtomRef update flow");
}

#[test]
fn test_atomref_update_different_fields() {
    println!("🔍 TESTING ATOMREF UPDATE FOR DIFFERENT FIELDS");
    
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
    let _atom_manager = AtomManager::new(db_ops, Arc::clone(&message_bus));
    
    // Subscribe to FieldValueSetResponse events
    let mut response_consumer = message_bus.subscribe::<FieldValueSetResponse>();
    
    // Create requests for different fields
    let request_username = FieldValueSetRequest::new(
        "test_username".to_string(),
        "user_schema".to_string(),
        "username".to_string(),
        json!("alice"),
        "test_pubkey".to_string(),
    );
    
    let request_email = FieldValueSetRequest::new(
        "test_email".to_string(),
        "user_schema".to_string(),
        "email".to_string(), // Different field
        json!("alice@example.com"),
        "test_pubkey".to_string(),
    );
    
    message_bus.publish(request_username).expect("Failed to publish username request");
    thread::sleep(Duration::from_millis(200));
    
    let response_username = response_consumer.recv_timeout(Duration::from_millis(500))
        .expect("Should receive username response");
    
    message_bus.publish(request_email).expect("Failed to publish email request");
    thread::sleep(Duration::from_millis(200));
    
    let response_email = response_consumer.recv_timeout(Duration::from_millis(500))
        .expect("Should receive email response");
    
    assert!(response_username.success);
    assert!(response_email.success);
    
    let username_aref = response_username.molecule_uuid.unwrap();
    let email_aref = response_email.molecule_uuid.unwrap();
    
    println!("✅ Different fields get different AtomRef UUIDs:");
    println!("   username AtomRef: {}", username_aref);
    println!("   email AtomRef: {}", email_aref);
    
    // Different fields should have different AtomRef UUIDs
    assert_ne!(username_aref, email_aref, 
        "Different fields should have different AtomRef UUIDs");
}