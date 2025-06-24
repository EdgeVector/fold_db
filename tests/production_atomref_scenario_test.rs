//! Test that simulates the actual production scenario described in the logs
//!
//! This test attempts to reproduce the exact failure pattern where:
//! - Handler creates atom: f52f7401-017d-4998-813e-96946e0ffc65  
//! - Query looks for atom: f079c516-b08f-4105-a609-7f55ccdfcf0a (old/wrong)
//! - AtomRef UUID: affc56e5-17fb-48ad-b73e-353fe1739d7e (should point to new atom)

use datafold::fold_db_core::infrastructure::message_bus::{
    MessageBus,
    request_events::{FieldValueSetRequest, FieldValueSetResponse},
};
use datafold::fold_db_core::managers::atom::AtomManager;
use datafold::db_operations::DbOperations;
use datafold::atom::Molecule;
use serde_json::json;
use std::sync::Arc;
use std::time::Duration;
use std::thread;
use tempfile::tempdir;

#[test]
fn test_production_atomref_scenario() {
    println!("🔍 TESTING PRODUCTION ATOMREF SCENARIO");
    println!("   Reproducing: AtomRef points to old atom UUID after update");
    
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
    
    // STEP 1: Create initial field value (simulating existing data)
    println!("📝 STEP 1: Creating initial field value");
    let request1 = FieldValueSetRequest::new(
        "initial_value".to_string(),
        "user_schema".to_string(),
        "profile_data".to_string(),
        json!({"name": "alice", "version": 1}),
        "pubkey_001".to_string(),
    );
    
    message_bus.publish(request1).expect("Failed to publish initial request");
    thread::sleep(Duration::from_millis(200));
    
    let response1 = response_consumer.recv_timeout(Duration::from_millis(500))
        .expect("Should receive initial response");
    
    assert!(response1.success, "Initial request should succeed");
    let aref_uuid = response1.molecule_uuid.as_ref().expect("Should return Molecule UUID");
    println!("✅ Initial Molecule created: {}", aref_uuid);
    
    // STEP 2: Directly inspect the database to see current state
    println!("🔍 STEP 2: Inspecting database state after initial creation");
    let stored_aref = db_ops.get_item::<Molecule>(&format!("ref:{}", aref_uuid))
        .expect("Should be able to query AtomRef")
        .expect("AtomRef should exist");
    
    let initial_atom_uuid = stored_aref.get_atom_uuid().clone();
    println!("✅ Initial atom UUID in AtomRef: {}", initial_atom_uuid);
    
    // STEP 3: Create second field value (should update AtomRef to new atom)
    println!("📝 STEP 3: Creating updated field value (critical update)");
    let request2 = FieldValueSetRequest::new(
        "updated_value".to_string(),
        "user_schema".to_string(),
        "profile_data".to_string(), // Same field
        json!({"name": "alice", "version": 2}), // Updated content
        "pubkey_002".to_string(),
    );
    
    message_bus.publish(request2).expect("Failed to publish update request");
    thread::sleep(Duration::from_millis(200));
    
    let response2 = response_consumer.recv_timeout(Duration::from_millis(500))
        .expect("Should receive update response");
    
    assert!(response2.success, "Update request should succeed");
    let aref_uuid_2 = response2.molecule_uuid.as_ref().expect("Should return Molecule UUID");
    
    // Verify same AtomRef UUID is used
    assert_eq!(aref_uuid, aref_uuid_2, "Should reuse same Molecule UUID");
    println!("✅ Update used same Molecule: {}", aref_uuid_2);
    
    // STEP 4: CRITICAL VALIDATION - Check if AtomRef points to new atom
    println!("🔍 STEP 4: CRITICAL VALIDATION - Checking AtomRef update");
    let updated_aref = db_ops.get_item::<Molecule>(&format!("ref:{}", aref_uuid))
        .expect("Should be able to query updated AtomRef")
        .expect("Updated AtomRef should exist");
    
    let final_atom_uuid = updated_aref.get_atom_uuid().clone();
    println!("✅ Final atom UUID in AtomRef: {}", final_atom_uuid);
    
    // CRITICAL TEST: AtomRef should point to NEW atom, not old one
    if final_atom_uuid == initial_atom_uuid {
        panic!("🚨 ATOMREF UPDATE FAILURE DETECTED! 
               AtomRef still points to old atom UUID: {}
               This is the exact bug described in the logs!", final_atom_uuid);
    }
    
    println!("✅ SUCCESS: AtomRef correctly updated to new atom UUID");
    
    // STEP 5: Verify the new atom exists and old one is different
    println!("🔍 STEP 5: Verifying atom chain integrity");
    
    // Check that new atom exists
    let new_atom_key = format!("atom:{}", final_atom_uuid);
    let new_atom_exists = db.contains_key(&new_atom_key)
        .expect("Should be able to check atom existence");
    assert!(new_atom_exists, "New atom should exist in database");
    
    // Check that old atom is different
    assert_ne!(initial_atom_uuid, final_atom_uuid, 
        "Initial and final atom UUIDs should be different");
    
    println!("✅ PRODUCTION SCENARIO TEST PASSED");
    println!("   Initial atom: {}", initial_atom_uuid);
    println!("   Final atom:   {}", final_atom_uuid);
    println!("   AtomRef:      {}", aref_uuid);
    println!("   AtomRef correctly points to latest atom");
}

#[test] 
fn test_concurrent_atomref_updates() {
    println!("🔍 TESTING CONCURRENT ATOMREF UPDATES");
    println!("   Checking for race conditions in AtomRef updates");
    
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
    
    // Launch multiple concurrent requests for the same field
    let handles = (0..5).map(|i| {
        let bus_clone = Arc::clone(&message_bus);
        thread::spawn(move || {
            let request = FieldValueSetRequest::new(
                format!("concurrent_{}", i),
                "test_schema".to_string(),
                "concurrent_field".to_string(),
                json!({"update": i, "timestamp": chrono::Utc::now()}),
                format!("pubkey_{}", i),
            );
            
            bus_clone.publish(request).expect("Failed to publish concurrent request");
        })
    }).collect::<Vec<_>>();
    
    // Wait for all requests to be sent
    for handle in handles {
        handle.join().expect("Thread should complete");
    }
    
    // Collect all responses
    let mut responses = Vec::new();
    for i in 0..5 {
        match response_consumer.recv_timeout(Duration::from_millis(1000)) {
            Ok(response) => {
                assert!(response.success, "Concurrent request {} should succeed", i);
                responses.push(response);
            }
            Err(_) => panic!("Failed to receive response for concurrent request {}", i),
        }
    }
    
    // All responses should have the same AtomRef UUID (same field)
    let first_aref = responses[0].molecule_uuid.as_ref().unwrap();
    for (i, response) in responses.iter().enumerate() {
        let aref_uuid = response.molecule_uuid.as_ref().unwrap();
        assert_eq!(first_aref, aref_uuid,
            "All concurrent updates should use same Molecule UUID (response {})", i);
    }
    
    println!("✅ CONCURRENT UPDATE TEST PASSED");
    println!("   All {} concurrent updates used same Molecule: {}", responses.len(), first_aref);
    
    // Final validation: Check final state of AtomRef
    let final_aref = db_ops.get_item::<Molecule>(&format!("ref:{}", first_aref))
        .expect("Should be able to query final AtomRef")
        .expect("Final AtomRef should exist");
    
    let final_atom_uuid = final_aref.get_atom_uuid();
    println!("   Final AtomRef points to atom: {}", final_atom_uuid);
    
    // Verify the final atom exists
    let final_atom_key = format!("atom:{}", final_atom_uuid);
    let final_atom_exists = db.contains_key(&final_atom_key)
        .expect("Should be able to check final atom existence");
    assert!(final_atom_exists, "Final atom should exist in database");
}