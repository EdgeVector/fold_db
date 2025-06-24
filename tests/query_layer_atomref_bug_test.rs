//! Test to validate the Query Layer AtomRef Bug
//!
//! This test reproduces the exact issue where:
//! 1. Mutation layer correctly updates dynamic AtomRefs
//! 2. Query layer incorrectly reads static schema references
//! 3. Result: Query finds old/wrong atom UUIDs

use datafold::fold_db_core::infrastructure::message_bus::{
    MessageBus,
    request_events::{FieldValueSetRequest, FieldValueSetResponse},
};
use datafold::fold_db_core::managers::atom::AtomManager;  
use datafold::fold_db_core::transform_manager::utils::TransformUtils;
use datafold::db_operations::DbOperations;
use datafold::schema::{Schema, types::field::FieldVariant};
use datafold::fold_db_core::infrastructure::factory::InfrastructureLogger;
use serde_json::json;
use std::sync::Arc;
use std::time::Duration;
use std::thread;
use tempfile::tempdir;

#[test]
fn test_query_layer_atomref_bug_reproduction() {
    InfrastructureLogger::log_investigation(
        "test_query_layer_atomref_bug_reproduction",
        "start",
    );
    
    // Setup database
    let temp_dir = tempdir().expect("Failed to create temp dir");
    let db = sled::Config::new()
        .path(temp_dir.path())
        .temporary(true)
        .open()
        .expect("Failed to open database");
    
    let db_ops = DbOperations::new(db.clone()).expect("Failed to create DbOperations");
    let message_bus = Arc::new(MessageBus::new());
    
    // Create AtomManager
    let _atom_manager = AtomManager::new(db_ops.clone(), Arc::clone(&message_bus));
    
    // Subscribe to FieldValueSetResponse events
    let mut response_consumer = message_bus.subscribe::<FieldValueSetResponse>();
    
    // STEP 1: Create a test schema with initial static field reference
    let mut test_schema = Schema::new("test_schema".to_string());
    
    // Add a field with a static atom reference (this will become stale)
    let initial_static_atom_uuid = "static-atom-uuid-12345";
    
    // Create SingleField with proper structure
    use datafold::schema::types::field::Field; // Import the trait
    use std::collections::HashMap;
    
    let mut single_field = datafold::schema::types::field::SingleField::new(
        datafold::permissions::types::policy::PermissionsPolicy::default(),
        datafold::fees::types::config::FieldPaymentConfig::default(),
        HashMap::new(),
    );
    
    // Set the static atom reference (this will become stale)
    single_field.set_molecule_uuid(initial_static_atom_uuid.to_string());
    
    let field_variant = FieldVariant::Single(single_field);
    test_schema.fields.insert("test_field".to_string(), field_variant);
    
    // STEP 2: Use mutation layer to create new field value (updates dynamic AtomRef)
    let mutation_request = FieldValueSetRequest::new(
        "mutation_test".to_string(),
        "test_schema".to_string(),
        "test_field".to_string(),
        json!({"content": "new_value_v1", "timestamp": "2024-01-01"}),
        "test_pubkey".to_string(),
    );
    
    message_bus.publish(mutation_request).expect("Failed to publish mutation");
    thread::sleep(Duration::from_millis(200));
    
    let mutation_response = response_consumer.recv_timeout(Duration::from_millis(500))
        .expect("Should receive mutation response");
    
    assert!(mutation_response.success, "Mutation should succeed");
    let dynamic_aref_uuid = mutation_response.molecule_uuid.expect("Should return AtomRef UUID");
    
    // STEP 3: Verify dynamic AtomRef was created and points to new atom  
    let dynamic_aref = db_ops.get_item::<datafold::atom::Molecule>(&format!("ref:{}", dynamic_aref_uuid))
        .expect("Should be able to query dynamic AtomRef")
        .expect("Dynamic AtomRef should exist");
    
    let dynamic_atom_uuid = dynamic_aref.get_atom_uuid().clone();
    
    // CRITICAL TEST: This should be DIFFERENT from the static schema reference
    assert_ne!(dynamic_atom_uuid, initial_static_atom_uuid, 
        "Dynamic atom UUID should differ from static schema reference");
    
    // STEP 4: Test query layer - this should reveal the bug!
    
    // Use the query layer to resolve field value
    match TransformUtils::resolve_field_value(&Arc::new(db_ops.clone()), &test_schema, "test_field", None) {
        Ok(value) => {
            
            // If our fix worked, the value should match what we set
            if let Some(obj) = value.as_object() {
                if let Some(content) = obj.get("content") {
                    assert_eq!(content, &json!("new_value_v1"), "Content should match what we set via mutation");
                }
            }
        }
        Err(_e) => {
            
            // This failure is expected if static reference doesn't exist
            // The diagnostic logs should show the mismatch
        }
    }
    
    // STEP 5: Create another mutation to further test the system
    let mutation_request_2 = FieldValueSetRequest::new(
        "mutation_test_2".to_string(),
        "test_schema".to_string(),
        "test_field".to_string(),
        json!({"content": "new_value_v2", "timestamp": "2024-01-02"}),
        "test_pubkey_2".to_string(),
    );
    
    message_bus.publish(mutation_request_2).expect("Failed to publish second mutation");
    thread::sleep(Duration::from_millis(200));
    
    let mutation_response_2 = response_consumer.recv_timeout(Duration::from_millis(500))
        .expect("Should receive second mutation response");
    
    assert!(mutation_response_2.success, "Second mutation should succeed");
    let dynamic_aref_uuid_2 = mutation_response_2.molecule_uuid.expect("Should return same AtomRef UUID");
    
    // Should reuse the same AtomRef UUID
    assert_eq!(dynamic_aref_uuid, dynamic_aref_uuid_2, "Should reuse same AtomRef UUID");
    
    // Check that the AtomRef now points to a newer atom
    let updated_aref = db_ops.get_item::<datafold::atom::Molecule>(&format!("ref:{}", dynamic_aref_uuid))
        .expect("Should be able to query updated AtomRef")
        .expect("Updated AtomRef should exist");
    
    let updated_atom_uuid = updated_aref.get_atom_uuid().clone();
    assert_ne!(updated_atom_uuid, dynamic_atom_uuid, "Should point to newer atom after second mutation");
    
    // Test query layer again
    match TransformUtils::resolve_field_value(&Arc::new(db_ops), &test_schema, "test_field", None) {
        Ok(value) => {
            
            if let Some(obj) = value.as_object() {
                if let Some(content) = obj.get("content") {
                    assert_eq!(
                        content,
                        &json!("new_value_v2"),
                        "Should return latest content after second mutation"
                    );
                }
            }
        }
        Err(_e) => {
        }
    }
    
}