//! Comprehensive integration test to diagnose Molecule update failures
//!
//! This test covers the complete mutation→query flow to identify where
//! the Molecule update is failing in the FieldValueSetRequest handler.

use datafold::db_operations::DbOperations;
use datafold::fees::types::config::FieldPaymentConfig;
use datafold::fold_db_core::infrastructure::message_bus::{
    request_events::FieldValueSetResponse, MessageBus,
};
use datafold::fold_db_core::managers::atom::AtomManager;
use datafold::permissions::types::policy::PermissionsPolicy;
use datafold::schema::types::field::SingleField;
use datafold::schema::types::{FieldVariant, Schema};
use serde_json::json;
use std::collections::HashMap;
use std::sync::Arc;
use std::thread;
use std::time::Duration;
use tempfile::tempdir;

#[path = "test_utils.rs"]
mod shared_test_utils;

use shared_test_utils::normalized_field_value_request;

fn register_user_schema(db_ops: &DbOperations) {
    let mut schema = Schema::new("user_schema".to_string());
    schema.fields.insert(
        "username".to_string(),
        FieldVariant::Single(SingleField::new(
            PermissionsPolicy::default(),
            FieldPaymentConfig::default(),
            HashMap::new(),
        )),
    );
    schema.fields.insert(
        "email".to_string(),
        FieldVariant::Single(SingleField::new(
            PermissionsPolicy::default(),
            FieldPaymentConfig::default(),
            HashMap::new(),
        )),
    );

    db_ops
        .store_schema("user_schema", &schema)
        .expect("Failed to store user schema");
}

#[test]
fn test_molecule_update_complete_flow() {
    println!("🔍 STARTING COMPREHENSIVE MOLECULE UPDATE DIAGNOSIS TEST");

    // Setup database
    let temp_dir = tempdir().expect("Failed to create temp dir");
    let db = sled::Config::new()
        .path(temp_dir.path())
        .temporary(true)
        .open()
        .expect("Failed to open database");

    let db_ops = DbOperations::new(db).expect("Failed to create DbOperations");
    register_user_schema(&db_ops);
    let message_bus = Arc::new(MessageBus::new());

    // Create AtomManager with diagnostic logging
    let _atom_manager = AtomManager::new(db_ops, Arc::clone(&message_bus));

    // Subscribe to FieldValueSetResponse events
    let mut response_consumer = message_bus.subscribe::<FieldValueSetResponse>();

    println!("📝 STEP 1: Creating first field value (initial state)");

    // Create first FieldValueSetRequest for user.username field
    let request1 = normalized_field_value_request(
        "test_correlation_001",
        "user_schema",
        "username",
        json!("alice_v1"),
        "test_pubkey_001",
    );

    message_bus
        .publish(request1)
        .expect("Failed to publish first FieldValueSetRequest");
    thread::sleep(Duration::from_millis(300)); // Give handler time to process

    let response1 = response_consumer
        .recv_timeout(Duration::from_millis(500))
        .expect("Should receive first FieldValueSetResponse");

    assert!(
        response1.success,
        "First FieldValueSetRequest should succeed"
    );
    let molecule_uuid = response1
        .molecule_uuid
        .as_ref()
        .expect("Should return Molecule UUID");

    println!("✅ STEP 1 COMPLETE: Molecule UUID: {}", molecule_uuid);

    // Allow time for all logging to be processed
    thread::sleep(Duration::from_millis(200));

    println!("📝 STEP 2: Creating second field value (should update Molecule)");

    // Create second FieldValueSetRequest for same field (should update Molecule)
    let request2 = normalized_field_value_request(
        "test_correlation_002",
        "user_schema",
        "username",
        json!("alice_v2"),
        "test_pubkey_002",
    );

    message_bus
        .publish(request2)
        .expect("Failed to publish second FieldValueSetRequest");
    thread::sleep(Duration::from_millis(300)); // Give handler time to process

    let response2 = response_consumer
        .recv_timeout(Duration::from_millis(500))
        .expect("Should receive second FieldValueSetResponse");

    assert!(
        response2.success,
        "Second FieldValueSetRequest should succeed"
    );
    let molecule_uuid_2 = response2
        .molecule_uuid
        .as_ref()
        .expect("Should return Molecule UUID");

    println!("✅ STEP 2 COMPLETE: Molecule UUID: {}", molecule_uuid_2);

    // Allow time for all logging to be processed
    thread::sleep(Duration::from_millis(200));

    println!("🔍 CRITICAL VALIDATION: Molecule UUIDs should be identical (same field)");
    assert_eq!(
        molecule_uuid, molecule_uuid_2,
        "Molecule UUID should be the same for both requests (same schema.field): {} vs {}",
        molecule_uuid, molecule_uuid_2
    );

    println!("📝 STEP 3: Creating third field value (final update test)");

    // Create third FieldValueSetRequest to test multiple updates
    let request3 = normalized_field_value_request(
        "test_correlation_003",
        "user_schema",
        "username",
        json!("alice_v3"),
        "test_pubkey_003",
    );

    message_bus
        .publish(request3)
        .expect("Failed to publish third FieldValueSetRequest");
    thread::sleep(Duration::from_millis(300)); // Give handler time to process

    let response3 = response_consumer
        .recv_timeout(Duration::from_millis(500))
        .expect("Should receive third FieldValueSetResponse");

    assert!(
        response3.success,
        "Third FieldValueSetRequest should succeed"
    );
    let molecule_uuid_3 = response3
        .molecule_uuid
        .as_ref()
        .expect("Should return Molecule UUID");

    println!("✅ STEP 3 COMPLETE: Molecule UUID: {}", molecule_uuid_3);

    // Allow time for all logging to be processed
    thread::sleep(Duration::from_millis(500));

    println!("🔍 FINAL VALIDATION: All Molecule UUIDs should be identical");
    assert_eq!(
        molecule_uuid, molecule_uuid_3,
        "All Molecule UUIDs should match (same schema.field): {} vs {}",
        molecule_uuid, molecule_uuid_3
    );

    println!("✅ MOLECULE UPDATE DIAGNOSIS TEST COMPLETED SUCCESSFULLY");
    println!("   - Created 3 atoms for same field");
    println!("   - Verified Molecule UUID consistency: {}", molecule_uuid);
    println!("   - Check logs above for detailed Molecule update flow");
}

#[test]
fn test_molecule_update_different_fields() {
    println!("🔍 TESTING MOLECULE UPDATE FOR DIFFERENT FIELDS");

    // Setup database
    let temp_dir = tempdir().expect("Failed to create temp dir");
    let db = sled::Config::new()
        .path(temp_dir.path())
        .temporary(true)
        .open()
        .expect("Failed to open database");

    let db_ops = DbOperations::new(db).expect("Failed to create DbOperations");
    register_user_schema(&db_ops);
    let message_bus = Arc::new(MessageBus::new());

    // Create AtomManager
    let _atom_manager = AtomManager::new(db_ops, Arc::clone(&message_bus));

    // Subscribe to FieldValueSetResponse events
    let mut response_consumer = message_bus.subscribe::<FieldValueSetResponse>();

    // Create requests for different fields
    let request_username = normalized_field_value_request(
        "test_username",
        "user_schema",
        "username",
        json!("alice"),
        "test_pubkey",
    );

    let request_email = normalized_field_value_request(
        "test_email",
        "user_schema",
        "email", // Different field
        json!("alice@example.com"),
        "test_pubkey",
    );

    message_bus
        .publish(request_username)
        .expect("Failed to publish username request");
    thread::sleep(Duration::from_millis(200));

    let response_username = response_consumer
        .recv_timeout(Duration::from_millis(500))
        .expect("Should receive username response");

    message_bus
        .publish(request_email)
        .expect("Failed to publish email request");
    thread::sleep(Duration::from_millis(200));

    let response_email = response_consumer
        .recv_timeout(Duration::from_millis(500))
        .expect("Should receive email response");

    assert!(response_username.success);
    assert!(response_email.success);

    let username_molecule = response_username.molecule_uuid.unwrap();
    let email_molecule = response_email.molecule_uuid.unwrap();

    println!("✅ Different fields get different Molecule UUIDs:");
    println!("   username Molecule: {}", username_molecule);
    println!("   email Molecule: {}", email_molecule);

    // Different fields should have different Molecule UUIDs
    assert_ne!(
        username_molecule, email_molecule,
        "Different fields should have different Molecule UUIDs"
    );
}
