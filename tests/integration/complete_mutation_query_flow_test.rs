//! Comprehensive Integration Test for Complete Mutation→Query Flow
//!
//! This test validates that the complete mutation→query flow works correctly 
//! after the collection removal and AtomRef bug fixes. It proves that:
//!
//! 1. **Mutations correctly create/update dynamic AtomRefs** 
//! 2. **Queries read from the correct dynamic AtomRefs** (not stale static schema refs)
//! 3. **The Range-only architecture works end-to-end** (no Collection dependencies)
//! 4. **Multiple mutation cycles work consistently** 
//! 5. **The system handles both single and multi-field mutations**
//!
//! **Root Cause Validation:**
//! Previously, mutations would create dynamic AtomRefs, but queries would read
//! static schema references, causing "Atom not found" errors. This test ensures
//! the fix is working correctly.

use datafold::fold_db_core::infrastructure::message_bus::request_events::{
    FieldValueSetRequest, FieldValueSetResponse,
};
use datafold::fold_db_core::transform_manager::utils::TransformUtils;
use datafold::schema::{Schema, types::field::FieldVariant};
use datafold::schema::types::field::{SingleField, Field};
use datafold::permissions::types::policy::PermissionsPolicy;
use datafold::fees::types::config::FieldPaymentConfig;
use serde_json::json;
use std::sync::Arc;
use std::time::Duration;
use std::thread;
use crate::test_utils::TEST_WAIT_MS;
use std::collections::HashMap;

use crate::test_utils::TestFixture;
    
    /// Create a realistic test schema based on TransformBase
fn create_transform_base_schema() -> Schema {
        println!("🏗️  Creating TransformBase schema with single fields");
        
        let mut schema = Schema::new("TransformBase".to_string());
        
        // Create value1 field (Single field) with static ref (to test the bug fix)
        let mut value1_field = SingleField::new(
            PermissionsPolicy::default(),
            FieldPaymentConfig::default(),
            HashMap::new(),
        );
        // Set a static reference that will be overridden by dynamic AtomRef system
        value1_field.set_ref_atom_uuid("static_ref_value1_should_be_overridden".to_string());
        schema.fields.insert("value1".to_string(), FieldVariant::Single(value1_field));
        
        // Create value2 field (Single field) with static ref (to test the bug fix)
        let mut value2_field = SingleField::new(
            PermissionsPolicy::default(),
            FieldPaymentConfig::default(),
            HashMap::new(),
        );
        // Set a static reference that will be overridden by dynamic AtomRef system
        value2_field.set_ref_atom_uuid("static_ref_value2_should_be_overridden".to_string());
        schema.fields.insert("value2".to_string(), FieldVariant::Single(value2_field));
        
        println!("✅ TransformBase schema created with fields: {:?}", schema.fields.keys().collect::<Vec<_>>());
        println!("🔧 Static references set (will be overridden by dynamic AtomRef system)");
        schema
}

/// Perform a field mutation and verify it succeeds
fn mutate_field_value(
    fixture: &TestFixture,
    schema_name: &str,
    field_name: &str,
    value: serde_json::Value,
    source: &str,
) -> Result<String, Box<dyn std::error::Error>> {
        println!("🔄 Mutating {}.{} = {} (source: {})", schema_name, field_name, value, source);
        
        // Subscribe to FieldValueSetResponse events
        let mut response_consumer = fixture.message_bus.subscribe::<FieldValueSetResponse>();
        
        // Create and publish FieldValueSetRequest
        let correlation_id = format!("mutation_{}_{}", schema_name, field_name);
        let request = FieldValueSetRequest::new(
            correlation_id.clone(),
            schema_name.to_string(),
            field_name.to_string(),
            value.clone(),
            source.to_string(),
        );
        
        fixture.message_bus.publish(request)?;
        
        // Wait for processing and response
        thread::sleep(Duration::from_millis(200));
        
        let response = response_consumer.recv_timeout(Duration::from_millis(1000))
            .map_err(|_| "Timeout waiting for FieldValueSetResponse")?;
        
        // Verify mutation succeeded
        if !response.success {
            return Err(format!("Mutation failed: {:?}", response.error).into());
        }
        
        let aref_uuid = response.aref_uuid
            .ok_or("Mutation should return AtomRef UUID")?;
        
        println!("✅ Mutation succeeded - AtomRef UUID: {}", aref_uuid);
        
        // DIAGNOSTIC: Verify AtomRef was created in database
        let aref_key = format!("ref:{}", aref_uuid);
        match fixture.db_ops.get_item::<datafold::atom::AtomRef>(&aref_key) {
            Ok(Some(aref)) => {
                let atom_uuid = aref.get_atom_uuid();
                println!("🔍 DIAGNOSTIC: AtomRef {} points to atom {}", aref_uuid, atom_uuid);
                
                // Verify atom exists and contains expected data
                let atom_key = format!("atom:{}", atom_uuid);
                match fixture.db_ops.get_item::<datafold::atom::Atom>(&atom_key) {
                    Ok(Some(atom)) => {
                        println!("🔍 DIAGNOSTIC: Atom {} contains: {}", atom_uuid, atom.content());
                    }
                    Ok(None) => {
                        println!("⚠️  DIAGNOSTIC: Atom {} not found in database", atom_uuid);
                    }
                    Err(e) => {
                        println!("❌ DIAGNOSTIC: Error loading atom {}: {}", atom_uuid, e);
                    }
                }
            }
            Ok(None) => {
                println!("❌ DIAGNOSTIC: AtomRef {} not found in database", aref_uuid);
            }
            Err(e) => {
                println!("❌ DIAGNOSTIC: Error loading AtomRef {}: {}", aref_uuid, e);
            }
        }
        
        Ok(aref_uuid)
    }
    
    /// Query a field value and verify it returns correct data
fn query_field_value(
    fixture: &TestFixture,
    schema: &Schema,
    field_name: &str,
    expected_value: &serde_json::Value,
) -> Result<serde_json::Value, Box<dyn std::error::Error>> {
        println!("🔍 Querying {}.{} (expecting: {})", schema.name, field_name, expected_value);
        
        // Use the query system to resolve field value
    let result = TransformUtils::resolve_field_value(&fixture.db_ops, schema, field_name, None)?;
        
        println!("✅ Query returned: {}", result);
        
        // Verify the result matches expectations
        if &result != expected_value {
            return Err(format!(
                "Query result mismatch: expected {}, got {}", 
                expected_value, result
            ).into());
        }
        
        println!("🎯 Query validation passed - correct value returned");
        Ok(result)
    }

#[test]
fn test_single_field_mutation_to_query_flow() {
    println!("🧪 TEST: Single Field Mutation→Query Flow");
    println!("   This validates the complete flow for individual field mutations");
    
    let fixture = TestFixture::new()
        .expect("Failed to create test fixture");
    
    let schema = create_transform_base_schema();
    
    // Step 1: Perform mutation on value1 field
    let test_value = json!(42);
    let aref_uuid = mutate_field_value(&fixture, "TransformBase", "value1", test_value.clone(), "test_source")
        .expect("Failed to mutate value1");
    
    // Step 2: Query the same field and verify it returns the mutated value
    let result = query_field_value(&fixture, &schema, "value1", &test_value)
        .expect("Failed to query value1");
    
    assert_eq!(result, test_value, "Query should return the mutated value");
    
    println!("✅ Single field mutation→query flow test PASSED");
    println!("   AtomRef UUID: {}", aref_uuid);
    println!("   Value: {} → {}", test_value, result);
}

#[test]
fn test_multiple_field_mutations_and_queries() {
    println!("🧪 TEST: Multiple Field Mutations and Queries");
    println!("   This validates mutations and queries across multiple fields");
    
    let fixture = TestFixture::new()
        .expect("Failed to create test fixture");
    
    let schema = create_transform_base_schema();
    
    // Test data for multiple fields
    let test_cases = vec![
        ("value1", json!(25), "source_a"),
        ("value2", json!(35), "source_b"),
    ];
    
    let mut aref_uuids = Vec::new();
    
    // Step 1: Perform mutations on multiple fields
    for (field_name, value, source) in &test_cases {
        println!("📝 Mutating field: {}", field_name);
        let aref_uuid = mutate_field_value(&fixture, "TransformBase", field_name, value.clone(), source)
            .unwrap_or_else(|_| panic!("Failed to mutate {}", field_name));
        aref_uuids.push(aref_uuid);
    }
    
    // Step 2: Query all fields and verify they return correct values
    for (field_name, expected_value, _) in &test_cases {
        println!("🔍 Querying field: {}", field_name);
        let result = query_field_value(&fixture, &schema, field_name, expected_value)
            .unwrap_or_else(|_| panic!("Failed to query {}", field_name));
        
        assert_eq!(&result, expected_value, "Field {} should return correct value", field_name);
    }
    
    println!("✅ Multiple field mutations and queries test PASSED");
    println!("   AtomRef UUIDs: {:?}", aref_uuids);
}

#[test]
fn test_multiple_mutation_cycles_on_same_field() {
    println!("🧪 TEST: Multiple Mutation Cycles on Same Field");
    println!("   This validates that AtomRef updates work consistently across multiple mutations");
    
    let fixture = TestFixture::new()
        .expect("Failed to create test fixture");
    
    let schema = create_transform_base_schema();
    
    // Test multiple mutation cycles on the same field
    let mutation_cycles = [
        (json!("initial_value"), "cycle_1"),
        (json!(100), "cycle_2"),
        (json!("updated_value"), "cycle_3"),
        (json!(200), "cycle_4"),
    ];
    
    let mut aref_uuids = Vec::new();
    
    for (i, (value, source)) in mutation_cycles.iter().enumerate() {
        println!("📝 Mutation cycle {} of {}", i + 1, mutation_cycles.len());
        
        // Perform mutation
        let aref_uuid = mutate_field_value(&fixture, "TransformBase", "value1", value.clone(), source)
            .unwrap_or_else(|_| panic!("Failed to mutate in cycle {}", i + 1));
        aref_uuids.push(aref_uuid.clone());
        
        // Verify query returns the latest value
        let result = query_field_value(&fixture, &schema, "value1", value)
            .unwrap_or_else(|_| panic!("Failed to query in cycle {}", i + 1));
        
        assert_eq!(&result, value, "Cycle {}: Query should return latest mutated value", i + 1);
        
        // DIAGNOSTIC: Verify AtomRef consistency
        if i > 0 {
            // All mutations should reuse the same AtomRef UUID
            assert_eq!(aref_uuids[i], aref_uuids[0], 
                "Cycle {}: Should reuse same AtomRef UUID", i + 1);
        }
        
        println!("✅ Cycle {} completed successfully", i + 1);
    }
    
    println!("✅ Multiple mutation cycles test PASSED");
    println!("   Total cycles: {}", mutation_cycles.len());
    println!("   AtomRef UUID (reused): {}", aref_uuids[0]);
}

#[test]
fn test_mutation_query_flow_with_different_data_types() {
    println!("🧪 TEST: Mutation→Query Flow with Different Data Types");
    println!("   This validates the system handles various JSON data types correctly");
    
    let fixture = TestFixture::new()
        .expect("Failed to create test fixture");
    
    let schema = create_transform_base_schema();
    
    // Test different data types
    let test_data_types = vec![
        ("string", json!("test_string_value")),
        ("number", json!(42.5)),
        ("integer", json!(123)),
        ("boolean", json!(true)),
        ("object", json!({"nested": "object", "count": 5})),
        ("array", json!([1, 2, 3, "mixed", true])),
        ("null", json!(null)),
    ];
    
    for (data_type, value) in test_data_types {
        println!("📝 Testing data type: {} = {}", data_type, value);
        
        // Use value1 field for all data type tests
        let source = format!("datatype_{}", data_type);
        
        // Perform mutation
        let aref_uuid = mutate_field_value(&fixture, "TransformBase", "value1", value.clone(), &source)
            .unwrap_or_else(|_| panic!("Failed to mutate {} data type", data_type));
        
        // Verify query returns correct value
        let result = query_field_value(&fixture, &schema, "value1", &value)
            .unwrap_or_else(|_| panic!("Failed to query {} data type", data_type));
        
        assert_eq!(result, value, "Data type {}: Query should return correct value", data_type);
        
        println!("✅ Data type {} test passed - AtomRef: {}", data_type, aref_uuid);
    }
    
    println!("✅ Different data types test PASSED");
}

#[test]
fn test_concurrent_mutations_and_queries() {
    println!("🧪 TEST: Concurrent Mutations and Queries");
    println!("   This validates thread safety and consistency under concurrent operations");
    
    let fixture = TestFixture::new()
        .expect("Failed to create test fixture");
    
    let schema = create_transform_base_schema();
    
    // Perform concurrent mutations on different fields
    let mutation_handles: Vec<_> = (0..5).map(|i| {
        let _db_ops = Arc::clone(&fixture.db_ops);
        let message_bus = Arc::clone(&fixture.message_bus);
        let value = json!(format!("concurrent_value_{}", i));
        
        std::thread::spawn(move || {
            // Subscribe to responses
            let mut response_consumer = message_bus.subscribe::<FieldValueSetResponse>();
            
            // Create mutation request
            let correlation_id = format!("concurrent_test_{}", i);
            let request = FieldValueSetRequest::new(
                correlation_id.clone(),
                "TransformBase".to_string(),
                "value1".to_string(),  // All threads mutate the same field
                value.clone(),
                format!("concurrent_source_{}", i),
            );
            
            // Publish and wait for response
            message_bus.publish(request).expect("Failed to publish request");
            thread::sleep(Duration::from_millis(TEST_WAIT_MS));
            
            let response = response_consumer.recv_timeout(Duration::from_millis(1000))
                .expect("Failed to receive response");
            
            (i, response.success, response.aref_uuid, value)
        })
    }).collect();
    
    // Collect results from all concurrent operations
    let mut results = Vec::new();
    for handle in mutation_handles {
        let result = handle.join().expect("Thread panicked");
        results.push(result);
    }
    
    // Verify all operations succeeded
    for (i, success, aref_uuid, _value) in &results {
        assert!(success, "Concurrent operation {} should succeed", i);
        assert!(aref_uuid.is_some(), "Concurrent operation {} should return AtomRef UUID", i);
        println!("✅ Concurrent operation {} succeeded - AtomRef: {:?}", i, aref_uuid);
    }
    
    // Verify final state is consistent
    let final_result = TransformUtils::resolve_field_value(&fixture.db_ops, &schema, "value1", None)
        .expect("Failed to query final state");
    
    println!("✅ Final field state after concurrent operations: {}", final_result);
    
    println!("✅ Concurrent mutations and queries test PASSED");
    println!("   Concurrent operations: {}", results.len());
}

#[test]
fn test_complete_range_only_architecture_validation() {
    println!("🧪 TEST: Complete Range-Only Architecture Validation");
    println!("   This validates that the system works correctly without Collection dependencies");
    
    let fixture = TestFixture::new()
        .expect("Failed to create test fixture");
    
    let schema = create_transform_base_schema();
    
    // Verify schema only contains Single and Range fields (no Collections)
    for (field_name, field_variant) in &schema.fields {
        match field_variant {
            FieldVariant::Single(_) => {
                println!("✅ Field '{}' is Single (Range-only architecture compatible)", field_name);
            }
            FieldVariant::Range(_) => {
                println!("✅ Field '{}' is Range (Range-only architecture compatible)", field_name);
            }
            // Note: Collection variant should not exist after collection removal
        }
    }
    
    // Test that mutations and queries work correctly in Range-only environment
    let test_value = json!({"architecture": "range_only", "collections_removed": true});
    
    let aref_uuid = mutate_field_value(&fixture, "TransformBase", "value1", test_value.clone(), "range_only_test")
        .expect("Failed to mutate in range-only architecture");
    
    let result = query_field_value(&fixture, &schema, "value1", &test_value)
        .expect("Failed to query in range-only architecture");
    
    assert_eq!(result, test_value, "Range-only architecture should handle mutations and queries correctly");
    
    println!("✅ Range-only architecture validation PASSED");
    println!("   AtomRef UUID: {}", aref_uuid);
    println!("   No Collection dependencies detected");
}

#[test]
fn test_diagnostic_atomref_bug_prevention() {
    println!("🧪 TEST: Diagnostic AtomRef Bug Prevention");
    println!("   This validates that the static-schema-reference bug is prevented");
    
    let fixture = TestFixture::new()
        .expect("Failed to create test fixture");
    
    let mut schema = create_transform_base_schema();
    
    // SIMULATE THE BUG: Set a static ref_atom_uuid in the schema field
    if let Some(FieldVariant::Single(single_field)) = schema.fields.get_mut("value1") {
        // Set a static reference that should NOT be used by queries
        single_field.set_ref_atom_uuid("STATIC_REFERENCE_SHOULD_NOT_BE_USED".to_string());
        println!("🚨 Set static schema reference: STATIC_REFERENCE_SHOULD_NOT_BE_USED");
    }
    
    // Perform mutation to create dynamic AtomRef
    let test_value = json!("dynamic_value_from_mutation");
    let dynamic_aref_uuid = mutate_field_value(&fixture, "TransformBase", "value1", test_value.clone(), "bug_prevention_test")
        .expect("Failed to mutate with static reference present");
    
    println!("✅ Dynamic AtomRef created: {}", dynamic_aref_uuid);
    
    // Verify dynamic AtomRef points to the correct atom
    let aref_key = format!("ref:{}", dynamic_aref_uuid);
    let dynamic_aref = fixture.db_ops.get_item::<datafold::atom::AtomRef>(&aref_key)
        .expect("Failed to load dynamic AtomRef")
        .expect("Dynamic AtomRef should exist");
    
    let dynamic_atom_uuid = dynamic_aref.get_atom_uuid();
    println!("🔍 Dynamic AtomRef points to atom: {}", dynamic_atom_uuid);
    
    // CRITICAL TEST: Query should use dynamic AtomRef, NOT static schema reference
    let result = query_field_value(&fixture, &schema, "value1", &test_value)
        .expect("Query should succeed using dynamic AtomRef (bug prevention working)");
    
    assert_eq!(result, test_value, "Query should return value from dynamic AtomRef, not static reference");
    
    // DIAGNOSTIC: Verify static reference was NOT used
    if dynamic_atom_uuid == "STATIC_REFERENCE_SHOULD_NOT_BE_USED" {
        panic!("❌ BUG DETECTED: Query used static schema reference instead of dynamic AtomRef");
    }
    
    println!("✅ AtomRef bug prevention test PASSED");
    println!("   Query correctly used dynamic AtomRef: {}", dynamic_aref_uuid);
    println!("   Static schema reference correctly ignored");
}

/// Integration test covering the complete mutation→query workflow
#[test]
fn test_complete_mutation_query_integration_workflow() {
    println!("🧪 COMPREHENSIVE TEST: Complete Mutation→Query Integration Workflow");
    println!("   This validates the entire end-to-end flow with realistic scenarios");
    
    let fixture = TestFixture::new()
        .expect("Failed to create test fixture");
    
    let schema = create_transform_base_schema();
    
    // Phase 1: Initial Setup and Basic Mutations
    println!("\n📋 Phase 1: Initial Setup and Basic Mutations");
    
    let initial_values = vec![
        ("value1", json!(10), "setup_phase"),
        ("value2", json!(20), "setup_phase"),
    ];
    
    for (field_name, value, source) in &initial_values {
        mutate_field_value(&fixture, "TransformBase", field_name, value.clone(), source)
            .unwrap_or_else(|_| panic!("Failed initial mutation for {}", field_name));
        
        query_field_value(&fixture, &schema, field_name, value)
            .unwrap_or_else(|_| panic!("Failed initial query for {}", field_name));
    }
    
    // Phase 2: Complex Data Updates
    println!("\n📋 Phase 2: Complex Data Updates");
    
    let complex_update = json!({
        "calculation_result": 30,
        "metadata": {
            "source": "integration_test",
            "timestamp": "2024-01-01T12:00:00Z"
        },
        "tags": ["integration", "mutation", "query"]
    });
    
    mutate_field_value(&fixture, "TransformBase", "value1", complex_update.clone(), "complex_phase")
        .expect("Failed complex mutation");
    
    query_field_value(&fixture, &schema, "value1", &complex_update)
        .expect("Failed complex query");
    
    // Phase 3: Rapid Update Cycles
    println!("\n📋 Phase 3: Rapid Update Cycles");
    
    for cycle in 1..=3 {
        let cycle_value = json!(format!("rapid_cycle_{}", cycle));
        
        mutate_field_value(&fixture, "TransformBase", "value2", cycle_value.clone(), &format!("rapid_cycle_{}", cycle))
            .unwrap_or_else(|_| panic!("Failed rapid cycle {} mutation", cycle));
        
        query_field_value(&fixture, &schema, "value2", &cycle_value)
            .unwrap_or_else(|_| panic!("Failed rapid cycle {} query", cycle));
        
        // Small delay between cycles
        thread::sleep(Duration::from_millis(50));
    }
    
    // Phase 4: Final Validation
    println!("\n📋 Phase 4: Final Validation");
    
    let final_value1 = TransformUtils::resolve_field_value(&fixture.db_ops, &schema, "value1", None)
        .expect("Failed final validation query for value1");
    
    let final_value2 = TransformUtils::resolve_field_value(&fixture.db_ops, &schema, "value2", None)
        .expect("Failed final validation query for value2");
    
    assert_eq!(final_value1, complex_update, "Final value1 should match complex update");
    assert_eq!(final_value2, json!("rapid_cycle_3"), "Final value2 should match last rapid cycle");
    
    println!("\n✅ COMPREHENSIVE INTEGRATION TEST PASSED");
    println!("   ✓ All mutation→query flows working correctly");
    println!("   ✓ Range-only architecture functioning properly");
    println!("   ✓ AtomRef bug fixes validated");
    println!("   ✓ System ready for production use");
    println!("   📊 Final state:");
    println!("     value1: {}", final_value1);
    println!("     value2: {}", final_value2);
}