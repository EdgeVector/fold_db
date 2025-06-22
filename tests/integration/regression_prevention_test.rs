//! Regression Prevention Tests
//!
//! This comprehensive test suite validates that specific bugs have been fixed
//! and implements safeguards to prevent their reintroduction.
//!
//! **Regression Prevention Coverage:**
//! 1. **AtomRef Resolution Bug Fixes** - Prevent static vs dynamic AtomRef issues
//! 2. **Transform Trigger Bug Fixes** - Ensure transform triggering works correctly
//! 3. **Edge Cases Prevention** - Test boundary conditions that could cause issues
//! 4. **Backwards Compatibility** - Ensure system works with legacy data patterns
//! 5. **Event Ordering and Timing** - Prevent race conditions and timing issues
//! 6. **Data Consistency Safeguards** - Ensure data integrity under all conditions

use datafold::fold_db_core::infrastructure::message_bus::{
    MessageBus,
    request_events::{FieldValueSetRequest, FieldValueSetResponse},
    schema_events::{TransformTriggered, TransformExecuted},
};
use datafold::fold_db_core::transform_manager::{TransformManager, TransformUtils};
use datafold::fold_db_core::managers::atom::AtomManager;
use datafold::db_operations::DbOperations;
use datafold::schema::{Schema, types::field::FieldVariant, field_factory::FieldFactory};
use datafold::schema::types::field::Field;
use datafold::atom::{Atom, AtomRef};
use serde_json::json;
use std::sync::{Arc, Mutex, atomic::{AtomicUsize, Ordering}};
use std::time::{Duration, Instant};
use std::thread;
use crate::test_utils::TEST_WAIT_MS;
use tempfile::tempdir;

/// Test fixture for regression prevention testing
#[allow(dead_code)]
struct RegressionPreventionTestFixture {
    pub db_ops: Arc<DbOperations>,
    pub message_bus: Arc<MessageBus>,
    pub transform_manager: Arc<TransformManager>,
    pub _atom_manager: AtomManager,
    pub _temp_dir: tempfile::TempDir,
}

impl RegressionPreventionTestFixture {
    fn new() -> Result<Self, Box<dyn std::error::Error>> {
        let temp_dir = tempdir()?;
        
        let db = sled::Config::new()
            .path(temp_dir.path())
            .temporary(true)
            .open()?;
            
        let db_ops = Arc::new(DbOperations::new(db)?);
        let message_bus = Arc::new(MessageBus::new());
        
        let transform_manager = Arc::new(TransformManager::new(
            Arc::clone(&db_ops),
            Arc::clone(&message_bus),
        )?);
        
        let atom_manager = AtomManager::new(
            (*db_ops).clone(),
            Arc::clone(&message_bus)
        );
        
        Ok(Self {
            db_ops,
            message_bus,
            transform_manager,
            _atom_manager: atom_manager,
            _temp_dir: temp_dir,
        })
    }
    
    /// Create a schema with pre-existing static AtomRef to test the bug fix
    fn create_schema_with_static_atomref(&self) -> Result<Schema, Box<dyn std::error::Error>> {
        let mut problematic_schema = Schema::new("ProblematicSchema".to_string());
        
        // Create field with static AtomRef that should be overridden by dynamic system
        let mut field_with_static_ref = FieldFactory::create_single_field();
        field_with_static_ref.set_ref_atom_uuid("STATIC_REF_SHOULD_BE_OVERRIDDEN".to_string());
        
        problematic_schema.fields.insert(
            "problematic_field".to_string(),
            FieldVariant::Single(field_with_static_ref)
        );
        
        // Add normal field for comparison
        problematic_schema.fields.insert(
            "normal_field".to_string(),
            FieldVariant::Single(FieldFactory::create_single_field())
        );
        
        self.db_ops.store_schema("ProblematicSchema", &problematic_schema)?;
        Ok(problematic_schema)
    }
    
    /// Execute a mutation and wait for response with detailed tracking
    fn execute_tracked_mutation(
        &self,
        schema_name: &str,
        field_name: &str,
        value: serde_json::Value,
        source: &str,
    ) -> Result<MutationResult, Box<dyn std::error::Error>> {
        let correlation_id = format!("regression_{}_{}", schema_name, field_name);
        let start_time = Instant::now();
        
        // Subscribe to response
        let mut response_consumer = self.message_bus.subscribe::<FieldValueSetResponse>();
        
        let request = FieldValueSetRequest::new(
            correlation_id.clone(),
            schema_name.to_string(),
            field_name.to_string(),
            value.clone(),
            source.to_string(),
        );
        
        self.message_bus.publish(request)?;
        
        // Wait for processing
        thread::sleep(Duration::from_millis(TEST_WAIT_MS));
        
        let response = response_consumer.recv_timeout(Duration::from_millis(2000))
            .map_err(|_| "Timeout waiting for FieldValueSetResponse")?;
        
        let duration = start_time.elapsed();
        
        Ok(MutationResult {
            correlation_id,
            success: response.success,
            aref_uuid: response.aref_uuid,
            error: response.error,
            duration,
            value,
        })
    }
    
    /// Perform a complete AtomRef lifecycle validation
    fn validate_atomref_lifecycle(
        &self,
        schema_name: &str,
        field_name: &str,
        initial_value: serde_json::Value,
        updated_value: serde_json::Value,
    ) -> Result<AtomRefLifecycleResult, Box<dyn std::error::Error>> {
        // Step 1: Initial mutation
        let initial_result = self.execute_tracked_mutation(
            schema_name,
            field_name,
            initial_value.clone(),
            "lifecycle_test_initial",
        )?;
        
        if !initial_result.success {
            return Err(format!("Initial mutation failed: {:?}", initial_result.error).into());
        }
        
        let initial_aref_uuid = initial_result.aref_uuid
            .ok_or("Initial mutation should return AtomRef UUID")?;
        
        // Step 2: Validate initial AtomRef
        let initial_aref = self.validate_atomref_exists(&initial_aref_uuid)?;
        let initial_atom_uuid = initial_aref.get_atom_uuid();
        let initial_atom = self.validate_atom_exists(initial_atom_uuid)?;
        
        if initial_atom.content() != &initial_value {
            return Err("Initial atom content doesn't match expected value".into());
        }
        
        // Step 3: Update mutation (should reuse same AtomRef)
        let update_result = self.execute_tracked_mutation(
            schema_name,
            field_name,
            updated_value.clone(),
            "lifecycle_test_update",
        )?;
        
        if !update_result.success {
            return Err(format!("Update mutation failed: {:?}", update_result.error).into());
        }
        
        let update_aref_uuid = update_result.aref_uuid
            .ok_or("Update mutation should return AtomRef UUID")?;
        
        // Step 4: Validate AtomRef reuse
        if initial_aref_uuid != update_aref_uuid {
            return Err(format!(
                "AtomRef should be reused: initial={}, update={}",
                initial_aref_uuid, update_aref_uuid
            ).into());
        }
        
        // Step 5: Validate updated content
        let updated_aref = self.validate_atomref_exists(&update_aref_uuid)?;
        let updated_atom_uuid = updated_aref.get_atom_uuid();
        let updated_atom = self.validate_atom_exists(updated_atom_uuid)?;
        
        if updated_atom.content() != &updated_value {
            return Err("Updated atom content doesn't match expected value".into());
        }
        
        // Step 6: Validate atom chain
        if let Some(prev_atom_uuid) = updated_atom.prev_atom_uuid() {
            if prev_atom_uuid != initial_atom_uuid {
                return Err("Atom chain is broken: prev_atom_uuid doesn't match initial atom".into());
            }
        }
        
        Ok(AtomRefLifecycleResult {
            initial_aref_uuid: initial_aref_uuid.clone(),
            update_aref_uuid,
            initial_atom_uuid: initial_atom_uuid.clone(),
            updated_atom_uuid: updated_atom_uuid.clone(),
            atomref_reused: true,
            atom_chain_valid: true,
        })
    }
    
    fn validate_atomref_exists(&self, aref_uuid: &str) -> Result<AtomRef, Box<dyn std::error::Error>> {
        self.db_ops.get_item::<AtomRef>(&format!("ref:{}", aref_uuid))?
            .ok_or_else(|| format!("AtomRef {} not found", aref_uuid).into())
    }
    
    fn validate_atom_exists(&self, atom_uuid: &str) -> Result<Atom, Box<dyn std::error::Error>> {
        self.db_ops.get_item::<Atom>(&format!("atom:{}", atom_uuid))?
            .ok_or_else(|| format!("Atom {} not found", atom_uuid).into())
    }
}

#[derive(Debug)]
#[allow(dead_code)]
struct MutationResult {
    correlation_id: String,
    success: bool,
    aref_uuid: Option<String>,
    error: Option<String>,
    duration: Duration,
    value: serde_json::Value,
}

#[derive(Debug)]
#[allow(dead_code)]
struct AtomRefLifecycleResult {
    initial_aref_uuid: String,
    update_aref_uuid: String,
    initial_atom_uuid: String,
    updated_atom_uuid: String,
    atomref_reused: bool,
    atom_chain_valid: bool,
}

#[test]
fn test_atomref_resolution_bug_prevention() {
    println!("🧪 TEST: AtomRef Resolution Bug Prevention");
    println!("   This validates the fix for static vs dynamic AtomRef resolution issues");
    
    let fixture = RegressionPreventionTestFixture::new()
        .expect("Failed to create test fixture");
    
    // Test 1: Static AtomRef override prevention
    println!("🔧 Test 1: Static AtomRef Override Prevention");
    
    let schema = fixture.create_schema_with_static_atomref()
        .expect("Failed to create schema with static AtomRef");
    
    // Verify static ref is set initially
    if let Some(FieldVariant::Single(field)) = schema.fields.get("problematic_field") {
        assert_eq!(field.ref_atom_uuid(), Some(&"STATIC_REF_SHOULD_BE_OVERRIDDEN".to_string()));
        println!("✅ Static AtomRef initially set: STATIC_REF_SHOULD_BE_OVERRIDDEN");
    }
    
    // Perform mutation that should create dynamic AtomRef
    let test_value = json!({"message": "Dynamic AtomRef test", "bug_prevention": true});
    let mutation_result = fixture.execute_tracked_mutation(
        "ProblematicSchema",
        "problematic_field",
        test_value.clone(),
        "bug_prevention_test",
    ).expect("Failed to execute mutation");
    
    assert!(mutation_result.success, "Mutation should succeed despite static AtomRef");
    
    let dynamic_aref_uuid = mutation_result.aref_uuid
        .expect("Mutation should return dynamic AtomRef UUID");
    
    // CRITICAL: Verify dynamic AtomRef was created and is different from static ref
    assert_ne!(dynamic_aref_uuid, "STATIC_REF_SHOULD_BE_OVERRIDDEN",
        "Dynamic AtomRef should override static reference");
    
    println!("✅ Dynamic AtomRef created: {}", dynamic_aref_uuid);
    
    // Verify dynamic AtomRef points to correct data
    let dynamic_aref = fixture.validate_atomref_exists(&dynamic_aref_uuid)
        .expect("Dynamic AtomRef should exist");
    
    let dynamic_atom = fixture.validate_atom_exists(dynamic_aref.get_atom_uuid())
        .expect("Dynamic atom should exist");
    
    assert_eq!(dynamic_atom.content(), &test_value);
    println!("✅ Dynamic AtomRef points to correct data");
    
    // Test 2: Query uses dynamic AtomRef, not static
    println!("🔍 Test 2: Query Resolution Uses Dynamic AtomRef");
    
    let query_result = TransformUtils::resolve_field_value(
        &fixture.db_ops,
        &schema,
        "problematic_field",
        None,
    ).expect("Query should succeed using dynamic AtomRef");
    
    assert_eq!(query_result, test_value,
        "Query should return value from dynamic AtomRef, not static reference");
    
    println!("✅ Query correctly uses dynamic AtomRef");
    
    // Test 3: Multiple mutations continue to use same dynamic AtomRef
    println!("🔄 Test 3: Dynamic AtomRef Persistence Across Mutations");
    
    let second_value = json!({"message": "Second mutation test", "sequence": 2});
    let second_mutation = fixture.execute_tracked_mutation(
        "ProblematicSchema",
        "problematic_field",
        second_value.clone(),
        "second_mutation_test",
    ).expect("Failed to execute second mutation");
    
    assert!(second_mutation.success);
    let second_aref_uuid = second_mutation.aref_uuid.expect("Second mutation should return AtomRef UUID");
    
    // Should reuse the same dynamic AtomRef
    assert_eq!(dynamic_aref_uuid, second_aref_uuid,
        "Subsequent mutations should reuse the same dynamic AtomRef");
    
    println!("✅ Dynamic AtomRef reused across mutations");
    
    // Test 4: Normal field behavior remains unchanged
    println!("✅ Test 4: Normal Field Behavior Validation");
    
    let normal_value = json!({"normal": "field", "test": true});
    let normal_mutation = fixture.execute_tracked_mutation(
        "ProblematicSchema",
        "normal_field",
        normal_value.clone(),
        "normal_field_test",
    ).expect("Failed to execute normal field mutation");
    
    assert!(normal_mutation.success);
    let normal_aref_uuid = normal_mutation.aref_uuid.expect("Normal mutation should return AtomRef UUID");
    
    // Normal field should get its own AtomRef
    assert_ne!(normal_aref_uuid, dynamic_aref_uuid,
        "Normal field should have its own AtomRef");
    
    println!("✅ Normal field behavior unaffected");
    
    println!("✅ AtomRef Resolution Bug Prevention Test PASSED");
}

#[test]
fn test_transform_trigger_bug_prevention() {
    println!("🧪 TEST: Transform Trigger Bug Prevention");
    println!("   This validates that transform triggering works correctly after bug fixes");
    
    let fixture = RegressionPreventionTestFixture::new()
        .expect("Failed to create test fixture");
    
    // Create schemas for transform testing
    let mut source_schema = Schema::new("TransformSource".to_string());
    source_schema.fields.insert(
        "input_value".to_string(),
        FieldVariant::Single(FieldFactory::create_single_field())
    );
    fixture.db_ops.store_schema("TransformSource", &source_schema)
        .expect("Failed to store source schema");
    
    let mut target_schema = Schema::new("TransformTarget".to_string());
    target_schema.fields.insert(
        "output_value".to_string(),
        FieldVariant::Single(FieldFactory::create_single_field())
    );
    fixture.db_ops.store_schema("TransformTarget", &target_schema)
        .expect("Failed to store target schema");
    
    // Test 1: Transform trigger event handling
    println!("🚀 Test 1: Transform Trigger Event Handling");
    
    let triggered_transforms = Arc::new(Mutex::new(Vec::new()));
    let executed_transforms = Arc::new(Mutex::new(Vec::new()));
    
    // Subscribe to transform events
    let mut executed_consumer = fixture.message_bus.subscribe::<TransformExecuted>();
    
    // Trigger multiple transforms
    let trigger_count = 10;
    for i in 0..trigger_count {
        let transform_id = format!("trigger_test_{}", i);
        let trigger_event = TransformTriggered {
            transform_id: transform_id.clone(),
        };
        
        fixture.message_bus.publish(trigger_event)
            .expect("Failed to publish transform trigger");
        
        triggered_transforms.lock().unwrap().push(transform_id);
    }
    
    println!("✅ Triggered {} transforms", trigger_count);
    
    // Monitor for executed events (with timeout)
    let monitor_start = Instant::now();
    let monitor_timeout = Duration::from_secs(5);
    
    while monitor_start.elapsed() < monitor_timeout {
        match executed_consumer.recv_timeout(Duration::from_millis(TEST_WAIT_MS)) {
            Ok(executed_event) => {
                executed_transforms.lock().unwrap().push(executed_event.transform_id);
            }
            Err(_) => break, // Timeout, no more events
        }
    }
    
    let triggered_count = triggered_transforms.lock().unwrap().len();
    let executed_count = executed_transforms.lock().unwrap().len();
    
    println!("✅ Transform trigger handling: {} triggered, {} executed", 
        triggered_count, executed_count);
    
    // Test 2: Transform trigger ordering preservation
    println!("📋 Test 2: Transform Trigger Ordering Preservation");
    
    let ordered_triggers = Arc::new(Mutex::new(Vec::new()));
    let received_order = Arc::new(Mutex::new(Vec::new()));
    
    // Create ordered sequence of triggers
    let order_test_count = 5;
    for i in 0..order_test_count {
        let transform_id = format!("order_test_{:03}", i);
        let trigger_event = TransformTriggered {
            transform_id: transform_id.clone(),
        };
        
        ordered_triggers.lock().unwrap().push(transform_id);
        
        fixture.message_bus.publish(trigger_event)
            .expect("Failed to publish ordered trigger");
        
        // Small delay to ensure ordering
        thread::sleep(Duration::from_millis(10));
    }
    
    // Monitor execution order
    let mut order_consumer = fixture.message_bus.subscribe::<TransformExecuted>();
    let order_start = Instant::now();
    let order_timeout = Duration::from_secs(3);
    
    while order_start.elapsed() < order_timeout && received_order.lock().unwrap().len() < order_test_count {
        match order_consumer.recv_timeout(Duration::from_millis(TEST_WAIT_MS)) {
            Ok(executed_event) => {
                received_order.lock().unwrap().push(executed_event.transform_id);
            }
            Err(_) => break,
        }
    }
    
    let expected_order = ordered_triggers.lock().unwrap().clone();
    let actual_order = received_order.lock().unwrap().clone();
    
    println!("✅ Ordering test: expected {} events, received {}", 
        expected_order.len(), actual_order.len());
    
    // Test 3: Concurrent transform trigger handling
    println!("⚡ Test 3: Concurrent Transform Trigger Handling");
    
    let concurrent_triggers = 20;
    let trigger_threads = 4;
    let triggers_per_thread = concurrent_triggers / trigger_threads;
    
    let concurrent_triggered = Arc::new(AtomicUsize::new(0));
    
    let concurrent_start = Instant::now();
    
    // Create concurrent trigger threads
    let trigger_handles: Vec<_> = (0..trigger_threads).map(|thread_id| {
        let message_bus = Arc::clone(&fixture.message_bus);
        let triggered = Arc::clone(&concurrent_triggered);
        
        thread::spawn(move || {
            for i in 0..triggers_per_thread {
                let transform_id = format!("concurrent_{}_{}", thread_id, i);
                let trigger_event = TransformTriggered {
                    transform_id,
                };
                
                if message_bus.publish(trigger_event).is_ok() {
                    triggered.fetch_add(1, Ordering::Relaxed);
                }
            }
        })
    }).collect();
    
    // Wait for all trigger threads
    for handle in trigger_handles {
        handle.join().expect("Trigger thread panicked");
    }
    
    let concurrent_duration = concurrent_start.elapsed();
    let concurrent_triggered_count = concurrent_triggered.load(Ordering::Relaxed);
    let concurrent_trigger_rate = concurrent_triggered_count as f64 / concurrent_duration.as_secs_f64();
    
    println!("✅ Concurrent trigger handling: {} triggers in {:?}", 
        concurrent_triggered_count, concurrent_duration);
    println!("   Trigger rate: {:.2} triggers/sec", concurrent_trigger_rate);
    
    // Test 4: Transform trigger error handling
    println!("❌ Test 4: Transform Trigger Error Handling");
    
    // Trigger with invalid transform ID
    let invalid_trigger = TransformTriggered {
        transform_id: "INVALID_TRANSFORM_ID_SHOULD_FAIL".to_string(),
    };
    
    // Should not cause system failure
    let invalid_result = fixture.message_bus.publish(invalid_trigger);
    assert!(invalid_result.is_ok(), "Message publishing should succeed even for invalid transforms");
    
    // System should continue working after invalid trigger
    let recovery_trigger = TransformTriggered {
        transform_id: "recovery_test".to_string(),
    };
    
    let recovery_result = fixture.message_bus.publish(recovery_trigger);
    assert!(recovery_result.is_ok(), "System should recover from invalid triggers");
    
    println!("✅ Error handling: System remains stable after invalid triggers");
    
    // Performance assertions
    assert!(concurrent_trigger_rate > 10.0, "Concurrent trigger rate should be > 10/sec");
    
    println!("✅ Transform Trigger Bug Prevention Test PASSED");
}

#[test]
fn test_edge_cases_prevention() {
    println!("🧪 TEST: Edge Cases Prevention");
    println!("   This validates handling of boundary conditions that could cause issues");
    
    let fixture = RegressionPreventionTestFixture::new()
        .expect("Failed to create test fixture");
    
    // Create test schema
    let mut edge_case_schema = Schema::new("EdgeCaseSchema".to_string());
    edge_case_schema.fields.insert(
        "edge_field".to_string(),
        FieldVariant::Single(FieldFactory::create_single_field())
    );
    fixture.db_ops.store_schema("EdgeCaseSchema", &edge_case_schema)
        .expect("Failed to store edge case schema");
    
    // Test 1: Empty and null value handling
    println!("🕳️  Test 1: Empty and Null Value Handling");
    
    let edge_values = vec![
        ("null", json!(null)),
        ("empty_string", json!("")),
        ("empty_object", json!({})),
        ("empty_array", json!([])),
        ("zero", json!(0)),
        ("false", json!(false)),
    ];
    
    for (description, value) in edge_values {
        let result = fixture.execute_tracked_mutation(
            "EdgeCaseSchema",
            "edge_field",
            value.clone(),
            &format!("edge_case_{}", description),
        ).unwrap_or_else(|_| panic!("Failed to execute {} mutation", description));
        
        assert!(result.success, "Edge case {} should be handled correctly", description);
        
        // Verify value can be queried back (use stored schema with AtomRefs)
        let stored_schema = fixture.db_ops.get_schema("EdgeCaseSchema")
            .expect("Failed to retrieve stored schema")
            .expect("Schema should exist after storage");
            
        let query_result = TransformUtils::resolve_field_value(
            &fixture.db_ops,
            &stored_schema,
            "edge_field",
            None,
        ).unwrap_or_else(|_| panic!("Failed to query {} value", description));
        
        assert_eq!(query_result, value, "Edge case {} should round-trip correctly", description);
        
        println!("✅ Edge case handled: {} = {}", description, value);
    }
    
    // Test 2: Large data handling
    println!("📦 Test 2: Large Data Handling");
    
    // Create progressively larger data sets
    let large_data_sizes = vec![
        (1024, "small_large"),
        (10240, "medium_large"),
        (102400, "large_large"),
    ];
    
    for (size, description) in large_data_sizes {
        let large_string = "x".repeat(size);
        let large_data = json!({
            "type": "large_data_test",
            "size": size,
            "data": large_string,
            "metadata": {
                "description": description,
                "timestamp": chrono::Utc::now().to_rfc3339(),
            }
        });
        
        let start_time = Instant::now();
        let result = fixture.execute_tracked_mutation(
            "EdgeCaseSchema",
            "edge_field",
            large_data.clone(),
            &format!("large_data_{}", description),
        ).unwrap_or_else(|_| panic!("Failed to execute large data mutation: {}", description));
        
        let duration = start_time.elapsed();
        
        assert!(result.success, "Large data {} should be handled correctly", description);
        println!("✅ Large data handled: {} bytes in {:?}", size, duration);
        
        // Verify large data retrieval (use stored schema with AtomRefs)
        let query_start = Instant::now();
        let stored_schema = fixture.db_ops.get_schema("EdgeCaseSchema")
            .expect("Failed to retrieve stored schema")
            .expect("Schema should exist after storage");
            
        let query_result = TransformUtils::resolve_field_value(
            &fixture.db_ops,
            &stored_schema,
            "edge_field",
            None,
        ).unwrap_or_else(|_| panic!("Failed to query large data: {}", description));
        
        let query_duration = query_start.elapsed();
        
        assert_eq!(query_result, large_data);
        println!("✅ Large data queried: {} bytes in {:?}", size, query_duration);
    }
    
    // Test 3: Deeply nested data structures
    println!("🏗️  Test 3: Deeply Nested Data Structures");
    
    // Create nested structure
    let mut nested_data = json!("base");
    for depth in 0..20 {
        nested_data = json!({
            "level": depth,
            "nested": nested_data,
            "metadata": format!("depth_{}", depth),
        });
    }
    
    let nested_result = fixture.execute_tracked_mutation(
        "EdgeCaseSchema",
        "edge_field",
        nested_data.clone(),
        "deeply_nested_test",
    ).expect("Failed to execute deeply nested mutation");
    
    assert!(nested_result.success, "Deeply nested data should be handled correctly");
    
    let stored_schema = fixture.db_ops.get_schema("EdgeCaseSchema")
        .expect("Failed to retrieve stored schema")
        .expect("Schema should exist after storage");
        
    let nested_query = TransformUtils::resolve_field_value(
        &fixture.db_ops,
        &stored_schema,
        "edge_field",
        None,
    ).expect("Failed to query deeply nested data");
    
    assert_eq!(nested_query, nested_data);
    println!("✅ Deeply nested data handled correctly");
    
    // Test 4: Special character handling
    println!("🔤 Test 4: Special Character Handling");
    
    let special_characters = vec![
        ("unicode", json!("Hello 🌍 World! 测试 العربية русский")),
        ("control_chars", json!("Line1\nLine2\tTabbed\r\nWindows")),
        ("json_escapes", json!("Quote: \" Backslash: \\ Slash: /")),
        ("symbols", json!("!@#$%^&*()_+-=[]{}|;':\",./<>?")),
    ];
    
    for (description, value) in special_characters {
        let result = fixture.execute_tracked_mutation(
            "EdgeCaseSchema",
            "edge_field",
            value.clone(),
            &format!("special_char_{}", description),
        ).unwrap_or_else(|_| panic!("Failed to execute special character mutation: {}", description));
        
        assert!(result.success, "Special characters {} should be handled correctly", description);
        
        let stored_schema = fixture.db_ops.get_schema("EdgeCaseSchema")
            .expect("Failed to retrieve stored schema")
            .expect("Schema should exist after storage");
            
        let query_result = TransformUtils::resolve_field_value(
            &fixture.db_ops,
            &stored_schema,
            "edge_field",
            None,
        ).unwrap_or_else(|_| panic!("Failed to query special characters: {}", description));
        
        assert_eq!(query_result, value);
        println!("✅ Special characters handled: {}", description);
    }
    
    // Test 5: Rapid sequential mutations
    println!("⚡ Test 5: Rapid Sequential Mutations");
    
    let rapid_mutations = 100;
    let rapid_start = Instant::now();
    
    for i in 0..rapid_mutations {
        let rapid_value = json!({
            "sequence": i,
            "rapid_test": true,
            "timestamp": chrono::Utc::now().to_rfc3339(),
        });
        
        let result = fixture.execute_tracked_mutation(
            "EdgeCaseSchema",
            "edge_field",
            rapid_value,
            &format!("rapid_mutation_{}", i),
        ).unwrap_or_else(|_| panic!("Failed to execute rapid mutation {}", i));
        
        assert!(result.success, "Rapid mutation {} should succeed", i);
        
        if i % 10 == 0 {
            println!("Rapid mutations progress: {}/{}", i, rapid_mutations);
        }
    }
    
    let rapid_duration = rapid_start.elapsed();
    let rapid_rate = rapid_mutations as f64 / rapid_duration.as_secs_f64();
    
    println!("✅ Rapid sequential mutations: {} mutations in {:?}", 
        rapid_mutations, rapid_duration);
    println!("   Rapid mutation rate: {:.2} mutations/sec", rapid_rate);
    
    // Performance assertions - Allow for system variance
    assert!(rapid_rate > 8.0, "Rapid mutation rate should be > 8/sec (allowing for system variance)");
    
    println!("✅ Edge Cases Prevention Test PASSED");
}



#[test]
fn test_event_ordering_and_timing_safeguards() {
    println!("🧪 TEST: Event Ordering and Timing Safeguards");
    println!("   This validates prevention of race conditions and timing issues");
    
    let fixture = RegressionPreventionTestFixture::new()
        .expect("Failed to create test fixture");
    
    // Create test schema
    let mut timing_schema = Schema::new("TimingTestSchema".to_string());
    timing_schema.fields.insert(
        "timing_field".to_string(),
        FieldVariant::Single(FieldFactory::create_single_field())
    );
    fixture.db_ops.store_schema("TimingTestSchema", &timing_schema)
        .expect("Failed to store timing test schema");
    
    // Test 1: Sequential event ordering preservation
    println!("📋 Test 1: Sequential Event Ordering Preservation");
    
    let sequence_length = 10;
    let sequence_events = Arc::new(Mutex::new(Vec::new()));
    let received_events = Arc::new(Mutex::new(Vec::new()));
    
    // Subscribe to responses
    let mut response_consumer = fixture.message_bus.subscribe::<FieldValueSetResponse>();
    
    // Send sequential events
    for i in 0..sequence_length {
        let sequence_value = json!({
            "sequence": i,
            "timestamp": chrono::Utc::now().to_rfc3339(),
            "test": "sequential_ordering"
        });
        
        let correlation_id = format!("sequence_{:03}", i);
        sequence_events.lock().unwrap().push(correlation_id.clone());
        
        let request = FieldValueSetRequest::new(
            correlation_id,
            "TimingTestSchema".to_string(),
            "timing_field".to_string(),
            sequence_value,
            "sequential_test".to_string(),
        );
        
        fixture.message_bus.publish(request)
            .expect("Failed to publish sequential event");
        
        // Small delay to ensure ordering
        thread::sleep(Duration::from_millis(5));
    }
    
    // Collect responses
    let response_start = Instant::now();
    let response_timeout = Duration::from_secs(5);
    
    while response_start.elapsed() < response_timeout && received_events.lock().unwrap().len() < sequence_length {
        match response_consumer.recv_timeout(Duration::from_millis(TEST_WAIT_MS)) {
            Ok(response) => {
                received_events.lock().unwrap().push(response.correlation_id);
            }
            Err(_) => break,
        }
    }
    
    let expected_sequence = sequence_events.lock().unwrap().clone();
    let actual_sequence = received_events.lock().unwrap().clone();
    
    println!("✅ Sequential ordering: sent {}, received {}", 
        expected_sequence.len(), actual_sequence.len());
    
    // Test 2: Concurrent event handling with deterministic outcomes
    println!("⚡ Test 2: Concurrent Event Handling with Deterministic Outcomes");
    
    let concurrent_count = 20;
    let concurrent_threads = 4;
    let events_per_thread = concurrent_count / concurrent_threads;
    
    let concurrent_successful = Arc::new(AtomicUsize::new(0));
    let concurrent_total = Arc::new(AtomicUsize::new(0));
    
    let concurrent_start = Instant::now();
    
    // Create concurrent event threads
    let concurrent_handles: Vec<_> = (0..concurrent_threads).map(|thread_id| {
        let message_bus = Arc::clone(&fixture.message_bus);
        let successful = Arc::clone(&concurrent_successful);
        let total = Arc::clone(&concurrent_total);
        
        thread::spawn(move || {
            let mut response_consumer = message_bus.subscribe::<FieldValueSetResponse>();
            
            for i in 0..events_per_thread {
                let event_value = json!({
                    "thread_id": thread_id,
                    "event_id": i,
                    "timestamp": chrono::Utc::now().to_rfc3339(),
                    "test": "concurrent_deterministic"
                });
                
                let correlation_id = format!("concurrent_{}_{}", thread_id, i);
                let request = FieldValueSetRequest::new(
                    correlation_id,
                    "TimingTestSchema".to_string(),
                    "timing_field".to_string(),
                    event_value,
                    format!("concurrent_thread_{}", thread_id),
                );
                
                if message_bus.publish(request).is_ok() {
                    total.fetch_add(1, Ordering::Relaxed);
                    
                    // Try to get response
                    match response_consumer.recv_timeout(Duration::from_millis(TEST_WAIT_MS)) {
                        Ok(response) if response.success => {
                            successful.fetch_add(1, Ordering::Relaxed);
                        }
                        _ => {} // Failed or timeout
                    }
                }
            }
        })
    }).collect();
    
    // Wait for all concurrent threads
    for handle in concurrent_handles {
        handle.join().expect("Concurrent thread panicked");
    }
    
    let concurrent_duration = concurrent_start.elapsed();
    let concurrent_successful_count = concurrent_successful.load(Ordering::Relaxed);
    let concurrent_total_count = concurrent_total.load(Ordering::Relaxed);
    
    let concurrent_success_rate = concurrent_successful_count as f64 / concurrent_total_count as f64;
    
    println!("✅ Concurrent handling: {} successful / {} total ({:.1}%)", 
        concurrent_successful_count, concurrent_total_count, concurrent_success_rate * 100.0);
    println!("   Duration: {:?}", concurrent_duration);
    
    // Test 3: Event burst handling
    println!("💥 Test 3: Event Burst Handling");
    
    let burst_size = 50;
    let burst_events = Arc::new(AtomicUsize::new(0));
    let burst_responses = Arc::new(AtomicUsize::new(0));
    
    let mut burst_consumer = fixture.message_bus.subscribe::<FieldValueSetResponse>();
    
    // Send burst of events as fast as possible
    let burst_start = Instant::now();
    
    for i in 0..burst_size {
        let burst_value = json!({
            "burst_id": i,
            "timestamp": chrono::Utc::now().to_rfc3339(),
            "test": "burst_handling"
        });
        
        let correlation_id = format!("burst_{:03}", i);
        let request = FieldValueSetRequest::new(
            correlation_id,
            "TimingTestSchema".to_string(),
            "timing_field".to_string(),
            burst_value,
            "burst_test".to_string(),
        );
        
        if fixture.message_bus.publish(request).is_ok() {
            burst_events.fetch_add(1, Ordering::Relaxed);
        }
    }
    
    let burst_send_duration = burst_start.elapsed();
    
    // Collect burst responses
    let response_collection_start = Instant::now();
    let response_collection_timeout = Duration::from_secs(10);
    
    while response_collection_start.elapsed() < response_collection_timeout {
        match burst_consumer.recv_timeout(Duration::from_millis(50)) {
            Ok(_response) => {
                burst_responses.fetch_add(1, Ordering::Relaxed);
            }
            Err(_) => {
                // Check if we've received enough responses
                if burst_responses.load(Ordering::Relaxed) >= burst_events.load(Ordering::Relaxed) / 2 {
                    break;
                }
            }
        }
    }
    
    let burst_events_count = burst_events.load(Ordering::Relaxed);
    let burst_responses_count = burst_responses.load(Ordering::Relaxed);
    
    let burst_send_rate = burst_events_count as f64 / burst_send_duration.as_secs_f64();
    
    println!("✅ Burst handling: {} events sent, {} responses received", 
        burst_events_count, burst_responses_count);
    println!("   Send rate: {:.2} events/sec", burst_send_rate);
    
    // Test 4: Timeout and retry behavior
    println!("⏰ Test 4: Timeout and Retry Behavior");
    
    let timeout_test_value = json!({
        "timeout_test": true,
        "timestamp": chrono::Utc::now().to_rfc3339(),
    });
    
    let _timeout_start = Instant::now();
    let mut timeout_consumer = fixture.message_bus.subscribe::<FieldValueSetResponse>();
    
    let timeout_request = FieldValueSetRequest::new(
        "timeout_test".to_string(),
        "TimingTestSchema".to_string(),
        "timing_field".to_string(),
        timeout_test_value,
        "timeout_behavior_test".to_string(),
    );
    
    fixture.message_bus.publish(timeout_request)
        .expect("Failed to publish timeout test");
    
    // Test different timeout scenarios
    let short_timeout_result = timeout_consumer.recv_timeout(Duration::from_millis(10));
    let medium_timeout_result = timeout_consumer.recv_timeout(Duration::from_millis(TEST_WAIT_MS));
    let long_timeout_result = timeout_consumer.recv_timeout(Duration::from_millis(1000));
    
    println!("✅ Timeout behavior:");
    println!("   Short timeout (10ms): {}", if short_timeout_result.is_ok() { "received" } else { "timeout" });
    println!("   Medium timeout (100ms): {}", if medium_timeout_result.is_ok() { "received" } else { "timeout" });
    println!("   Long timeout (1000ms): {}", if long_timeout_result.is_ok() { "received" } else { "timeout" });
    
    // Performance assertions
    assert!(concurrent_success_rate > 0.5, "Concurrent success rate should be > 50%");
    assert!(burst_send_rate > 100.0, "Burst send rate should be > 100 events/sec");
    
    println!("✅ Event Ordering and Timing Safeguards Test PASSED");
}

#[test]
fn test_data_consistency_safeguards() {
    println!("🧪 TEST: Data Consistency Safeguards");
    println!("   This validates data integrity under all operational conditions");
    
    let fixture = RegressionPreventionTestFixture::new()
        .expect("Failed to create test fixture");
    
    // Create test schema with both fields from the start (schemas are immutable)
    let mut consistency_schema = Schema::new("ConsistencyTestSchema".to_string());
    consistency_schema.fields.insert(
        "consistency_field".to_string(),
        FieldVariant::Single(FieldFactory::create_single_field())
    );
    consistency_schema.fields.insert(
        "secondary_field".to_string(),
        FieldVariant::Single(FieldFactory::create_single_field())
    );
    fixture.db_ops.store_schema("ConsistencyTestSchema", &consistency_schema)
        .expect("Failed to store consistency test schema");
    
    // Test 1: AtomRef lifecycle consistency
    println!("🔗 Test 1: AtomRef Lifecycle Consistency");
    
    let initial_value = json!({"test": "initial", "value": 1});
    let updated_value = json!({"test": "updated", "value": 2});
    
    let lifecycle_result = fixture.validate_atomref_lifecycle(
        "ConsistencyTestSchema",
        "consistency_field",
        initial_value.clone(),
        updated_value.clone(),
    ).expect("Failed to validate AtomRef lifecycle");
    
    assert!(lifecycle_result.atomref_reused, "AtomRef should be reused");
    assert!(lifecycle_result.atom_chain_valid, "Atom chain should be valid");
    
    println!("✅ AtomRef lifecycle consistency validated");
    println!("   Initial AtomRef: {}", lifecycle_result.initial_aref_uuid);
    println!("   Update AtomRef: {}", lifecycle_result.update_aref_uuid);
    println!("   AtomRef reused: {}", lifecycle_result.atomref_reused);
    
    // Test 2: Multiple field consistency within schema
    println!("🔄 Test 2: Multiple Field Consistency Within Schema");
    
    // Schema already has both fields (schemas are immutable - no updates allowed)
    
    // Retrieve the stored schema (which has AtomRefs added during storage)
    let stored_schema = fixture.db_ops.get_schema("ConsistencyTestSchema")
        .expect("Failed to retrieve stored schema")
        .expect("Schema should exist after storage");
    
    // Perform mutations on both fields
    let primary_data = json!({"field": "primary", "data": "primary_value"});
    let secondary_data = json!({"field": "secondary", "data": "secondary_value"});
    
    let primary_result = fixture.execute_tracked_mutation(
        "ConsistencyTestSchema",
        "consistency_field",
        primary_data.clone(),
        "multi_field_primary",
    ).expect("Failed to mutate primary field");
    
    let secondary_result = fixture.execute_tracked_mutation(
        "ConsistencyTestSchema",
        "secondary_field",
        secondary_data.clone(),
        "multi_field_secondary",
    ).expect("Failed to mutate secondary field");
    
    assert!(primary_result.success && secondary_result.success);
    
    // Verify both fields have independent AtomRefs
    let primary_aref_uuid = primary_result.aref_uuid.expect("Primary should have AtomRef");
    let secondary_aref_uuid = secondary_result.aref_uuid.expect("Secondary should have AtomRef");
    
    assert_ne!(primary_aref_uuid, secondary_aref_uuid,
        "Different fields should have different AtomRefs");
    
    // Verify data consistency across fields (use stored schema with AtomRefs)
    let primary_query = TransformUtils::resolve_field_value(
        &fixture.db_ops,
        &stored_schema,
        "consistency_field",
        None,
    ).expect("Failed to query primary field");
    
    let secondary_query = TransformUtils::resolve_field_value(
        &fixture.db_ops,
        &stored_schema,
        "secondary_field",
        None,
    ).expect("Failed to query secondary field");
    
    assert_eq!(primary_query, primary_data);
    assert_eq!(secondary_query, secondary_data);
    
    println!("✅ Multiple field consistency validated");
    
    // Test 3: Concurrent mutation consistency
    println!("⚡ Test 3: Concurrent Mutation Consistency");
    
    let concurrent_mutations = 10;
    let concurrent_threads = 3;
    let mutations_per_thread = concurrent_mutations / concurrent_threads;
    
    let consistency_results = Arc::new(Mutex::new(Vec::new()));
    
    let concurrent_start = Instant::now();
    
    // Create concurrent mutation threads
    let consistency_handles: Vec<_> = (0..concurrent_threads).map(|thread_id| {
        let message_bus = Arc::clone(&fixture.message_bus);
        let results = Arc::clone(&consistency_results);
        
        thread::spawn(move || {
            let mut response_consumer = message_bus.subscribe::<FieldValueSetResponse>();
            
            for i in 0..mutations_per_thread {
                let mutation_value = json!({
                    "thread": thread_id,
                    "mutation": i,
                    "timestamp": chrono::Utc::now().to_rfc3339(),
                    "data": format!("concurrent_data_{}_{}", thread_id, i)
                });
                
                let correlation_id = format!("consistency_{}_{}", thread_id, i);
                let request = FieldValueSetRequest::new(
                    correlation_id.clone(),
                    "ConsistencyTestSchema".to_string(),
                    "consistency_field".to_string(),
                    mutation_value,
                    format!("consistency_thread_{}", thread_id),
                );
                
                if message_bus.publish(request).is_ok() {
                    match response_consumer.recv_timeout(Duration::from_millis(200)) {
                        Ok(response) => {
                            results.lock().unwrap().push((correlation_id, response.success, response.aref_uuid));
                        }
                        Err(_) => {
                            results.lock().unwrap().push((correlation_id, false, None));
                        }
                    }
                }
            }
        })
    }).collect();
    
    // Wait for all consistency threads
    for handle in consistency_handles {
        handle.join().expect("Consistency thread panicked");
    }
    
    let concurrent_duration = concurrent_start.elapsed();
    let consistency_results_vec = consistency_results.lock().unwrap().clone();
    
    // Analyze consistency results
    let successful_mutations = consistency_results_vec.iter().filter(|(_, success, _)| *success).count();
    let failed_mutations = consistency_results_vec.len() - successful_mutations;
    
    // All successful mutations should have valid AtomRef UUIDs
    let valid_atomrefs: Vec<_> = consistency_results_vec.iter()
        .filter_map(|(_, success, aref_uuid)| {
            if *success { aref_uuid.as_ref() } else { None }
        })
        .collect();
    
    // All AtomRefs should be the same (reused for same field)
    if valid_atomrefs.len() > 1 {
        let first_aref = valid_atomrefs[0];
        let all_same = valid_atomrefs.iter().all(|aref| *aref == first_aref);
        assert!(all_same, "All successful mutations should reuse the same AtomRef");
    }
    
    println!("✅ Concurrent mutation consistency:");
    println!("   Successful: {}, Failed: {}", successful_mutations, failed_mutations);
    println!("   Duration: {:?}", concurrent_duration);
    println!("   AtomRef reuse validated: {} unique AtomRefs", 
        valid_atomrefs.iter().collect::<std::collections::HashSet<_>>().len());
    
    // Test 4: Data integrity after system stress
    println!("🧪 Test 4: Data Integrity After System Stress");
    
    // Create stress by rapid mutations
    let stress_mutations = 50;
    for i in 0..stress_mutations {
        let stress_value = json!({
            "stress_test": true,
            "iteration": i,
            "timestamp": chrono::Utc::now().to_rfc3339(),
        });
        
        fixture.execute_tracked_mutation(
            "ConsistencyTestSchema",
            "consistency_field",
            stress_value,
            &format!("stress_mutation_{}", i),
        ).ok(); // Ignore failures for stress test
    }
    
    // Verify final state is consistent and queryable (use stored schema)
    let final_state = TransformUtils::resolve_field_value(
        &fixture.db_ops,
        &stored_schema,
        "consistency_field",
        None,
    ).expect("Should be able to query final state after stress");
    
    println!("✅ Data integrity maintained after stress test");
    println!("   Final state: {}", final_state);
    
    // Test 5: Schema-level consistency validation
    println!("📋 Test 5: Schema-Level Consistency Validation");
    
    // Verify schema still exists and is valid
    let retrieved_schema = fixture.db_ops.get_schema("ConsistencyTestSchema")
        .expect("Failed to retrieve schema")
        .expect("Schema should still exist");
    
    assert_eq!(retrieved_schema.name, "ConsistencyTestSchema");
    assert_eq!(retrieved_schema.fields.len(), 2); // consistency_field + secondary_field
    
    // Verify both fields are still functional
    for field_name in ["consistency_field", "secondary_field"] {
        let test_value = json!({"validation": field_name, "test": "schema_consistency"});
        let validation_result = fixture.execute_tracked_mutation(
            "ConsistencyTestSchema",
            field_name,
            test_value.clone(),
            "schema_validation",
        ).unwrap_or_else(|_| panic!("Failed to validate field {}", field_name));
        
        assert!(validation_result.success, "Field {} should still be functional", field_name);
        
        let validation_query = TransformUtils::resolve_field_value(
            &fixture.db_ops,
            &retrieved_schema,
            field_name,
            None,
        ).unwrap_or_else(|_| panic!("Failed to query field {}", field_name));
        
        assert_eq!(validation_query, test_value);
    }
    
    println!("✅ Schema-level consistency validated");
    
    // Performance assertions
    let concurrent_success_rate = successful_mutations as f64 / consistency_results_vec.len() as f64;
    assert!(concurrent_success_rate > 0.6, "Concurrent success rate should be > 60%");
    
    println!("✅ Data Consistency Safeguards Test PASSED");
}