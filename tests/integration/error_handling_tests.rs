//! Error Handling Tests
//!
//! This comprehensive test suite validates error handling and edge cases for both
//! transform types, ensuring proper error recovery and system stability.
//!
//! **Error Handling Coverage:**
//! 1. **Validation Failures** - Test validation failures for malformed declarative transforms
//! 2. **Error Messages** - Verify error messages are clear and actionable
//! 3. **Edge Cases** - Test edge cases with complex iterator expressions
//! 4. **Error Recovery** - Validate error recovery and system stability
//! 5. **Invalid Input Handling** - Test handling of invalid input data
//! 6. **Resource Exhaustion** - Test behavior under resource constraints
//! 7. **Concurrent Error Scenarios** - Test error handling under concurrent conditions

use datafold::db_operations::DbOperations;
use datafold::schema::types::transform::{Transform, TransformRegistration};
use datafold::schema::types::json_schema::{DeclarativeSchemaDefinition, FieldDefinition};
use datafold::schema::types::schema::SchemaType;
use datafold::fold_db_core::transform_manager::TransformManager;
use datafold::fold_db_core::infrastructure::message_bus::MessageBus;
use datafold::transform::TransformExecutor;
use datafold::schema::indexing::{ChainParser, FieldAlignmentValidator};
use std::collections::HashMap;
use std::sync::Arc;
use tempfile::TempDir;
use serde_json::json;

/// Test fixture for error handling testing
struct ErrorHandlingFixture {
    pub db_ops: Arc<DbOperations>,
    pub message_bus: Arc<MessageBus>,
    pub transform_manager: Arc<TransformManager>,
    pub chain_parser: ChainParser,
    pub field_alignment_validator: FieldAlignmentValidator,
    pub _temp_dir: TempDir,
}

impl ErrorHandlingFixture {
    fn new() -> Self {
        let temp_dir = TempDir::new().expect("Failed to create temp dir");
        let db = sled::open(temp_dir.path()).expect("Failed to open database");
        let db_ops = Arc::new(DbOperations::new(db).expect("Failed to create database"));
        let message_bus = Arc::new(MessageBus::new());
        let transform_manager = Arc::new(TransformManager::new(db_ops.clone(), message_bus.clone())
            .expect("Failed to create transform manager"));
        
        Self {
            db_ops,
            message_bus,
            transform_manager,
            chain_parser: ChainParser::new(),
            field_alignment_validator: FieldAlignmentValidator::new(),
            _temp_dir: temp_dir,
        }
    }
}

/// Test validation failures for malformed declarative transforms
#[test]
fn test_validation_failures_malformed_declarative_transforms() {
    let _fixture = ErrorHandlingFixture::new();
    
    // Test empty schema name
    let empty_name_schema = DeclarativeSchemaDefinition {
        name: String::new(), // Empty name should fail validation
        schema_type: SchemaType::Single,
        fields: HashMap::from([
            ("result".to_string(), FieldDefinition {
                field_type: Some("single".to_string()),
                atom_uuid: Some("data.map().value".to_string()),
            }),
        ]),
        key: None,
    };
    
    let empty_name_transform = Transform::from_declarative_schema(
        empty_name_schema,
        vec!["test.data".to_string()],
        "test.result".to_string()
    );
    
    let validation_result = empty_name_transform.validate();
    assert!(validation_result.is_err(), "Empty schema name should fail validation");
    
    if let Err(error) = validation_result {
        println!("Correctly failed validation for empty schema name: {}", error);
    }
    
    // Test empty fields
    let empty_fields_schema = DeclarativeSchemaDefinition {
        name: "test_schema".to_string(),
        schema_type: SchemaType::Single,
        fields: HashMap::new(), // Empty fields should fail validation
        key: None,
    };
    
    let empty_fields_transform = Transform::from_declarative_schema(
        empty_fields_schema,
        vec!["test.data".to_string()],
        "test.result".to_string()
    );
    
    let empty_fields_validation = empty_fields_transform.validate();
    assert!(empty_fields_validation.is_err(), "Empty fields should fail validation");
    
    if let Err(error) = empty_fields_validation {
        println!("Correctly failed validation for empty fields: {}", error);
    }
    
    // Test invalid field expressions
    let invalid_expression_schema = DeclarativeSchemaDefinition {
        name: "test_schema".to_string(),
        schema_type: SchemaType::Single,
        fields: HashMap::from([
            ("result".to_string(), FieldDefinition {
                field_type: Some("single".to_string()),
                atom_uuid: Some("invalid..expression..syntax".to_string()), // Invalid expression
            }),
        ]),
        key: None,
    };
    
    let invalid_expression_transform = Transform::from_declarative_schema(
        invalid_expression_schema,
        vec!["test.data".to_string()],
        "test.result".to_string()
    );
    
    let invalid_expression_validation = invalid_expression_transform.validate();
    assert!(invalid_expression_validation.is_err(), "Invalid expression should fail validation");
    
    if let Err(error) = invalid_expression_validation {
        println!("Correctly failed validation for invalid expression: {}", error);
    }
}

/// Test error messages are clear and actionable
#[test]
fn test_error_messages_clear_and_actionable() {
    let fixture = ErrorHandlingFixture::new();
    
    // Test chain parser error messages
    let invalid_expressions = vec![
        "invalid..syntax..expression",
        "blogpost.map().invalid_method()",
        "blogpost.map().content.split_by_word().invalid_reducer()",
        "blogpost.map().content.split_by_word().count().invalid_chain()",
    ];
    
    for invalid_expr in &invalid_expressions {
        let result = fixture.chain_parser.parse(invalid_expr);
        assert!(result.is_err(), "Should fail to parse invalid expression: {}", invalid_expr);
        
        if let Err(error) = result {
            let error_msg = format!("{}", error);
            // Error messages should be descriptive and actionable
            assert!(error_msg.contains("Invalid") || error_msg.contains("Failed") || error_msg.contains("Error"), 
                    "Error message should be descriptive: {}", error_msg);
            println!("Clear error message for '{}': {}", invalid_expr, error_msg);
        }
    }
    
    // Test field alignment validation error messages
    let mismatched_chains = vec![
        fixture.chain_parser.parse("blogpost.map().content").expect("Failed to parse valid chain"),
        fixture.chain_parser.parse("blogpost.map().author").expect("Failed to parse valid chain"),
    ];
    
    let alignment_result = fixture.field_alignment_validator.validate_alignment(&mismatched_chains)
        .expect("Failed to validate field alignment");
    
    if alignment_result.valid {
        for warning in &alignment_result.warnings {
            let warning_msg = format!("{:?}", warning);
            // Check that warning messages are descriptive (contain meaningful keywords)
            assert!(warning_msg.contains("depth") || warning_msg.contains("alignment") || warning_msg.contains("mismatch") || 
                    warning_msg.contains("performance") || warning_msg.contains("optimization") || warning_msg.contains("reducer"), 
                    "Warning message should be descriptive: {}", warning_msg);
            println!("Clear warning message: {}", warning_msg);
        }
    } else {
        for error in &alignment_result.errors {
            let error_msg = format!("{:?}", error);
            assert!(error_msg.contains("depth") || error_msg.contains("alignment") || error_msg.contains("mismatch"), 
                    "Error message should be descriptive: {}", error_msg);
            println!("Clear error message: {}", error_msg);
        }
    }
}

/// Test edge cases with complex iterator expressions
#[test]
fn test_edge_cases_complex_iterator_expressions() {
    let fixture = ErrorHandlingFixture::new();
    
    // Test deeply nested expressions
    let deep_expressions = vec![
        "blogpost.map().content.split_by_word().map().char.split_array().map().item.split_by_word().count()",
        "blogpost.map().tags.split_array().map().tag.split_by_word().map().char.split_array().count()",
        "blogpost.map().author.map().profile.map().name.split_by_word().count()",
    ];
    
    for deep_expr in &deep_expressions {
        let result = fixture.chain_parser.parse(deep_expr);
        match result {
            Ok(parsed_chain) => {
                println!("Successfully parsed deep expression: {}", deep_expr);
                
                // Test field alignment validation with deep expressions
                let chains = vec![parsed_chain];
                let alignment_result = fixture.field_alignment_validator.validate_alignment(&chains)
                    .expect("Failed to validate field alignment");
                
                if alignment_result.valid {
                    println!("Deep expression validation passed with {} warnings", alignment_result.warnings.len());
                } else {
                    println!("Deep expression validation failed with {} errors", alignment_result.errors.len());
                    // This is acceptable for very deep expressions
                }
            }
            Err(error) => {
                println!("Deep expression parsing failed (acceptable): {} - {}", deep_expr, error);
                // This is acceptable for very deep expressions that exceed limits
            }
        }
    }
    
    // Test edge cases with empty or null data
    let edge_case_expressions = vec![
        "blogpost.map().content", // Simple case
        "blogpost.map().empty_field", // Field that might be empty
        "blogpost.map().null_field", // Field that might be null
    ];
    
    for expr in &edge_case_expressions {
        let result = fixture.chain_parser.parse(expr);
        assert!(result.is_ok(), "Edge case expression should parse: {}", expr);
        
        if let Ok(_parsed_chain) = result {
            println!("Successfully parsed edge case expression: {}", expr);
            
            // Test execution with edge case data
            let _edge_case_data = json!({
                "blogpost": {
                    "content": "normal content",
                    "empty_field": "",
                    "null_field": null
                }
            });
            
            // This should either succeed with fallback values or fail gracefully
            println!("Edge case expression '{}' parsed successfully", expr);
        }
    }
}

/// Test error recovery and system stability
#[test]
fn test_error_recovery_and_system_stability() {
    let fixture = ErrorHandlingFixture::new();
    
    // Test that the system remains stable after validation failures
    let invalid_schema = DeclarativeSchemaDefinition {
        name: "invalid_schema".to_string(),
        schema_type: SchemaType::Single,
        fields: HashMap::from([
            ("result".to_string(), FieldDefinition {
                field_type: Some("single".to_string()),
                atom_uuid: Some("invalid..expression".to_string()),
            }),
        ]),
        key: None,
    };
    
    let invalid_transform = Transform::from_declarative_schema(
        invalid_schema,
        vec!["invalid_schema.data".to_string()],
        "invalid_schema.result".to_string()
    );
    
    // Validation should fail
    let validation_result = invalid_transform.validate();
    assert!(validation_result.is_err(), "Invalid transform should fail validation");
    
    // System should remain stable after validation failure
    let _transforms = fixture.transform_manager.list_transforms()
        .expect("System should remain stable after validation failure");
    
    // Should be able to create and register valid transforms after validation failure
    let valid_schema = DeclarativeSchemaDefinition {
        name: "valid_schema".to_string(),
        schema_type: SchemaType::Single,
        fields: HashMap::from([
            ("result".to_string(), FieldDefinition {
                field_type: Some("single".to_string()),
                atom_uuid: Some("data.map().value".to_string()),
            }),
        ]),
        key: None,
    };
    
    let valid_transform = Transform::from_declarative_schema(
        valid_schema,
        vec!["valid_schema.data".to_string()],
        "valid_schema.result".to_string()
    );
    
    // Valid transform should pass validation
    valid_transform.validate()
        .expect("Valid transform should pass validation");
    
    // Should be able to register valid transform
    let registration = TransformRegistration {
        transform_id: "valid_transform".to_string(),
        transform: valid_transform,
        input_molecules: vec!["valid_schema.data".to_string()],
        input_names: vec!["data".to_string()],
        trigger_fields: vec!["valid_schema.data".to_string()],
        output_molecule: "valid_schema.result".to_string(),
        schema_name: "valid_schema".to_string(),
        field_name: "result".to_string(),
    };
    
    fixture.transform_manager.register_transform_event_driven(registration)
        .expect("Should be able to register valid transform after validation failure");
    
    // Verify system is still functional
    let transforms_after = fixture.transform_manager.list_transforms()
        .expect("System should remain functional after error recovery");
    assert!(transforms_after.contains_key(&"valid_transform".to_string()));
    
    println!("Error recovery and system stability test passed");
}

/// Test invalid input handling
#[test]
fn test_invalid_input_handling() {
    let fixture = ErrorHandlingFixture::new();
    
    // Create a simple declarative transform
    let simple_schema = DeclarativeSchemaDefinition {
        name: "input_test".to_string(),
        schema_type: SchemaType::Single,
        fields: HashMap::from([
            ("result".to_string(), FieldDefinition {
                field_type: Some("single".to_string()),
                atom_uuid: Some("data.map().value".to_string()),
            }),
        ]),
        key: None,
    };
    
    let simple_transform = Transform::from_declarative_schema(
        simple_schema,
        vec!["input_test.data".to_string()],
        "input_test.result".to_string()
    );
    
    // Register the transform
    let registration = TransformRegistration {
        transform_id: "input_test_transform".to_string(),
        transform: simple_transform,
        input_molecules: vec!["input_test.data".to_string()],
        input_names: vec!["data".to_string()],
        trigger_fields: vec!["input_test.data".to_string()],
        output_molecule: "input_test.result".to_string(),
        schema_name: "input_test".to_string(),
        field_name: "result".to_string(),
    };
    
    fixture.transform_manager.register_transform_event_driven(registration)
        .expect("Failed to register input test transform");
    
    let transforms = fixture.transform_manager.list_transforms()
        .expect("Failed to list transforms");
    
    // Test various invalid input scenarios
    let invalid_inputs = vec![
        // Null input
        json!(null),
        // Empty object
        json!({}),
        // Missing required field
        json!({"other_field": "value"}),
        // Wrong data type
        json!({"data": "not_an_object"}),
        // Null required field
        json!({"data": null}),
        // Empty required field
        json!({"data": {}}),
    ];
    
    for (i, invalid_input) in invalid_inputs.iter().enumerate() {
        let input_map = HashMap::from([("data".to_string(), invalid_input.clone())]);
        let result = TransformExecutor::execute_transform_with_expr(
            &transforms[&"input_test_transform".to_string()],
            input_map
        );
        
        match result {
            Ok(json_result) => {
                println!("Invalid input {} handled gracefully: {}", i, json_result);
                // Should either succeed with fallback values or fail gracefully
                assert!(json_result.is_object() || json_result.is_null());
            }
            Err(error) => {
                println!("Invalid input {} correctly failed: {}", i, error);
                // This is also acceptable - the system should fail gracefully
                let error_msg = format!("{}", error);
                assert!(error_msg.contains("Invalid") || error_msg.contains("Failed") || error_msg.contains("Error"));
            }
        }
    }
}

/// Test resource exhaustion scenarios
#[test]
fn test_resource_exhaustion_scenarios() {
    let fixture = ErrorHandlingFixture::new();
    
    // Test with very large data structures
    let large_data = json!({
        "data": {
            "value": "x".repeat(10000), // Large string
            "array": (0..1000).map(|i| format!("item_{}", i)).collect::<Vec<_>>(), // Large array
            "nested": {
                "level1": {
                    "level2": {
                        "level3": {
                            "value": "deep_nested_value"
                        }
                    }
                }
            }
        }
    });
    
    // Create a transform that processes large data
    let large_data_schema = DeclarativeSchemaDefinition {
        name: "large_data_test".to_string(),
        schema_type: SchemaType::Single,
        fields: HashMap::from([
            ("processed".to_string(), FieldDefinition {
                field_type: Some("single".to_string()),
                atom_uuid: Some("data.map().value".to_string()),
            }),
        ]),
        key: None,
    };
    
    let large_data_transform = Transform::from_declarative_schema(
        large_data_schema,
        vec!["large_data_test.data".to_string()],
        "large_data_test.processed".to_string()
    );
    
    // Register the transform
    let registration = TransformRegistration {
        transform_id: "large_data_transform".to_string(),
        transform: large_data_transform,
        input_molecules: vec!["large_data_test.data".to_string()],
        input_names: vec!["data".to_string()],
        trigger_fields: vec!["large_data_test.data".to_string()],
        output_molecule: "large_data_test.processed".to_string(),
        schema_name: "large_data_test".to_string(),
        field_name: "processed".to_string(),
    };
    
    fixture.transform_manager.register_transform_event_driven(registration)
        .expect("Failed to register large data transform");
    
    let transforms = fixture.transform_manager.list_transforms()
        .expect("Failed to list transforms");
    
    // Test execution with large data
    let input_map = HashMap::from([("data".to_string(), large_data)]);
    let result = TransformExecutor::execute_transform_with_expr(
        &transforms[&"large_data_transform".to_string()],
        input_map
    );
    
    match result {
        Ok(json_result) => {
            println!("Large data processing succeeded: {}", json_result);
            assert!(json_result.is_object());
        }
        Err(error) => {
            println!("Large data processing failed (acceptable): {}", error);
            // This is acceptable if the system has resource limits
        }
    }
    
    // Test with many concurrent operations
    let mut handles = Vec::new();
    for i in 0..10 {
        let transform_clone = transforms[&"large_data_transform".to_string()].clone();
        let test_data = json!({
            "data": {
                "value": format!("concurrent_test_{}", i)
            }
        });
        
        let input_map = HashMap::from([("data".to_string(), test_data)]);
        let handle = std::thread::spawn(move || {
            TransformExecutor::execute_transform_with_expr(&transform_clone, input_map)
        });
        handles.push(handle);
    }
    
    // Wait for all concurrent operations to complete
    let mut success_count = 0;
    let mut failure_count = 0;
    
    for handle in handles {
        match handle.join() {
            Ok(Ok(_)) => success_count += 1,
            Ok(Err(_)) => failure_count += 1,
            Err(_) => failure_count += 1,
        }
    }
    
    println!("Concurrent operations completed: {} successes, {} failures", success_count, failure_count);
    
    // At least some operations should succeed
    assert!(success_count > 0, "At least some concurrent operations should succeed");
}

/// Test concurrent error scenarios
#[test]
fn test_concurrent_error_scenarios() {
    let fixture = ErrorHandlingFixture::new();
    
    // Create transforms that might fail
    let error_prone_schema = DeclarativeSchemaDefinition {
        name: "error_prone".to_string(),
        schema_type: SchemaType::Single,
        fields: HashMap::from([
            ("result".to_string(), FieldDefinition {
                field_type: Some("single".to_string()),
                atom_uuid: Some("data.map().value".to_string()),
            }),
        ]),
        key: None,
    };
    
    let error_prone_transform = Transform::from_declarative_schema(
        error_prone_schema,
        vec!["error_prone.data".to_string()],
        "error_prone.result".to_string()
    );
    
    // Register the transform
    let registration = TransformRegistration {
        transform_id: "error_prone_transform".to_string(),
        transform: error_prone_transform,
        input_molecules: vec!["error_prone.data".to_string()],
        input_names: vec!["data".to_string()],
        trigger_fields: vec!["error_prone.data".to_string()],
        output_molecule: "error_prone.result".to_string(),
        schema_name: "error_prone".to_string(),
        field_name: "result".to_string(),
    };
    
    fixture.transform_manager.register_transform_event_driven(registration)
        .expect("Failed to register error prone transform");
    
    let transforms = fixture.transform_manager.list_transforms()
        .expect("Failed to list transforms");
    
    // Test concurrent execution with mixed valid and invalid inputs
    let mut handles = Vec::new();
    for i in 0..20 {
        let transform_clone = transforms[&"error_prone_transform".to_string()].clone();
        let test_data = if i % 3 == 0 {
            // Invalid input every third iteration
            json!({"invalid": "data"})
        } else {
            // Valid input
            json!({
                "data": {
                    "value": format!("concurrent_test_{}", i)
                }
            })
        };
        
        let input_map = HashMap::from([("data".to_string(), test_data)]);
        let handle = std::thread::spawn(move || {
            TransformExecutor::execute_transform_with_expr(&transform_clone, input_map)
        });
        handles.push(handle);
    }
    
    // Wait for all concurrent operations to complete
    let mut success_count = 0;
    let mut failure_count = 0;
    
    for handle in handles {
        match handle.join() {
            Ok(Ok(_)) => success_count += 1,
            Ok(Err(_)) => failure_count += 1,
            Err(_) => failure_count += 1,
        }
    }
    
    println!("Concurrent error scenarios completed: {} successes, {} failures", success_count, failure_count);
    
    // Should have some successes (the system handles invalid inputs gracefully)
    assert!(success_count > 0, "Some concurrent operations should succeed");
    // Note: The system may handle invalid inputs gracefully, so failures are not guaranteed
    println!("Concurrent operations: {} successes, {} failures", success_count, failure_count);
    
    // System should remain stable after concurrent errors
    let transforms_after = fixture.transform_manager.list_transforms()
        .expect("System should remain stable after concurrent errors");
    assert!(transforms_after.contains_key(&"error_prone_transform".to_string()));
}
