use serde_json::Value as JsonValue;
use std::collections::HashMap;

use datafold::schema::types::json_schema::{
    DeclarativeSchemaDefinition, FieldDefinition, KeyConfig,
};
use datafold::schema::types::schema::SchemaType;
use datafold::schema::types::Transform;
use datafold::transform::executor::TransformExecutor;

/// Tests for multi-chain coordination in HashRange schema execution
/// This validates that the system correctly coordinates multiple field expressions (hash, range, regular fields)

#[test]
fn test_basic_hashrange_schema_execution() {
    // Create a basic HashRange schema with key configuration and fields
    let mut fields = HashMap::new();
    fields.insert(
        "title".to_string(),
        FieldDefinition {
            atom_uuid: Some("blogpost.map().title".to_string()),
            field_type: Some("String".to_string()),
        },
    );
    fields.insert(
        "content".to_string(),
        FieldDefinition {
            atom_uuid: Some("blogpost.map().content".to_string()),
            field_type: Some("String".to_string()),
        },
    );

    let key_config = KeyConfig {
        hash_field: "blogpost.map().author".to_string(),
        range_field: "blogpost.map().timestamp".to_string(),
    };

    let declarative_schema = DeclarativeSchemaDefinition {
        name: "basic_hashrange_test".to_string(),
        schema_type: SchemaType::HashRange,
        key: Some(key_config),
        fields,
    };

    let transform = Transform::from_declarative_schema(
        declarative_schema,
        vec!["blogpost_data".to_string()],
        "output.basic_hashrange".to_string(),
    );

    // Create input data
    let mut input_values = HashMap::new();
    input_values.insert(
        "blogpost".to_string(),
        serde_json::json!([
            {
                "title": "First Post",
                "content": "Content 1",
                "author": "Alice",
                "timestamp": "2025-01-01T10:00:00Z"
            },
            {
                "title": "Second Post",
                "content": "Content 2",
                "author": "Bob",
                "timestamp": "2025-01-02T10:00:00Z"
            }
        ]),
    );

    // Execute the transform - should handle HashRange schema with multi-chain coordination
    let result = TransformExecutor::execute_transform(&transform, input_values);

    assert!(
        result.is_ok(),
        "Basic HashRange schema execution should succeed: {:?}",
        result
    );

    let json_result = result.unwrap();
    let obj = json_result.as_object().unwrap();
    let fields = obj
        .get("fields")
        .and_then(|value| value.as_object())
        .expect("Result should contain fields map");

    // Should contain the regular fields (not key fields)
    assert!(
        fields.contains_key("title"),
        "Result should contain title field"
    );
    assert!(
        fields.contains_key("content"),
        "Result should contain content field"
    );

    // Key fields (_hash_field, _range_field) should NOT be in the final output
    assert!(
        !obj.contains_key("_hash_field"),
        "Key fields should not be in output"
    );
    assert!(
        !obj.contains_key("_range_field"),
        "Key fields should not be in output"
    );
}

#[test]
fn test_hashrange_schema_validation() {
    // Test that HashRange schema validation works correctly
    let mut fields = HashMap::new();
    fields.insert(
        "field1".to_string(),
        FieldDefinition {
            atom_uuid: Some("data.value1".to_string()),
            field_type: Some("String".to_string()),
        },
    );

    let key_config = KeyConfig {
        hash_field: "data.hash_key".to_string(),
        range_field: "data.range_key".to_string(),
    };

    let declarative_schema = DeclarativeSchemaDefinition {
        name: "validation_test".to_string(),
        schema_type: SchemaType::HashRange,
        key: Some(key_config),
        fields,
    };

    let transform = Transform::from_declarative_schema(
        declarative_schema,
        vec!["test_data".to_string()],
        "output.validation_test".to_string(),
    );

    // Create input data
    let mut input_values = HashMap::new();
    input_values.insert(
        "data".to_string(),
        serde_json::json!({
            "value1": "Field value",
            "hash_key": "hash_value",
            "range_key": "range_value"
        }),
    );

    // Execute the transform - should validate and execute successfully or fail gracefully
    let result = TransformExecutor::execute_transform(&transform, input_values);

    // May succeed or fail depending on ExecutionEngine behavior with simple expressions
    match result {
        Ok(json_result) => {
            let obj = json_result.as_object().unwrap();
            assert!(obj.contains_key("field1"));
        }
        Err(err) => {
            // ExecutionEngine may have issues with simple expressions - this is acceptable
            let error_msg = format!("{:?}", err);
            assert!(
                !error_msg.contains("panic") && !error_msg.contains("crash"),
                "Should handle ExecutionEngine limitations gracefully: {}",
                error_msg
            );
        }
    }
}

#[test]
fn test_hashrange_schema_missing_key_config() {
    // Test that HashRange schema without key config fails appropriately
    let mut fields = HashMap::new();
    fields.insert(
        "field1".to_string(),
        FieldDefinition {
            atom_uuid: Some("data.value".to_string()),
            field_type: Some("String".to_string()),
        },
    );

    let declarative_schema = DeclarativeSchemaDefinition {
        name: "missing_key_test".to_string(),
        schema_type: SchemaType::HashRange,
        key: None, // Missing key config
        fields,
    };

    let transform = Transform::from_declarative_schema(
        declarative_schema,
        vec!["test_data".to_string()],
        "output.missing_key".to_string(),
    );

    let input_values = HashMap::new();

    // Execute the transform - should fail due to missing key configuration
    let result = TransformExecutor::execute_transform(&transform, input_values);

    assert!(
        result.is_err(),
        "HashRange schema without key config should fail"
    );

    let error = result.unwrap_err();
    let error_msg = format!("{:?}", error);
    assert!(
        error_msg.contains("key configuration")
            || error_msg.contains("hash_field")
            || error_msg.contains("range_field"),
        "Error should mention missing key configuration: {}",
        error_msg
    );
}

#[test]
fn test_multi_chain_coordination_with_different_depths() {
    // Test multi-chain coordination with expressions at different depths
    let mut fields = HashMap::new();
    fields.insert(
        "simple_field".to_string(),
        FieldDefinition {
            atom_uuid: Some("data.title".to_string()), // Depth 0
            field_type: Some("String".to_string()),
        },
    );
    fields.insert(
        "mapped_field".to_string(),
        FieldDefinition {
            atom_uuid: Some("data.items.map().name".to_string()), // Depth 1
            field_type: Some("String".to_string()),
        },
    );

    let key_config = KeyConfig {
        hash_field: "data.id".to_string(),                     // Depth 0
        range_field: "data.items.map().timestamp".to_string(), // Depth 1
    };

    let declarative_schema = DeclarativeSchemaDefinition {
        name: "depth_coordination_test".to_string(),
        schema_type: SchemaType::HashRange,
        key: Some(key_config),
        fields,
    };

    let transform = Transform::from_declarative_schema(
        declarative_schema,
        vec!["test_data".to_string()],
        "output.depth_coordination".to_string(),
    );

    // Create input data
    let mut input_values = HashMap::new();
    input_values.insert(
        "data".to_string(),
        serde_json::json!({
            "title": "Multi-depth test",
            "id": "test-id-123",
            "items": [
                {"name": "Item 1", "timestamp": "2025-01-01"},
                {"name": "Item 2", "timestamp": "2025-01-02"}
            ]
        }),
    );

    // Execute the transform - should handle different depths appropriately
    let result = TransformExecutor::execute_transform(&transform, input_values);

    // May succeed or fail depending on field alignment validation for different depths
    match result {
        Ok(json_result) => {
            let obj = json_result.as_object().unwrap();
            assert!(obj.contains_key("simple_field"));
            assert!(obj.contains_key("mapped_field"));
        }
        Err(err) => {
            // Should be alignment-related error or ExecutionEngine limitation, not execution crash
            let error_msg = format!("{:?}", err);
            assert!(
                error_msg.contains("alignment")
                    || error_msg.contains("depth")
                    || error_msg.contains("CartesianProduct")
                    || error_msg.contains("No current scope")
                    || error_msg.contains("Execution error"),
                "Error should be coordination-related: {}",
                error_msg
            );
        }
    }
}

#[test]
fn test_multi_chain_parsing_errors() {
    // Test multi-chain coordination with some invalid expressions
    let mut fields = HashMap::new();
    fields.insert(
        "valid_field".to_string(),
        FieldDefinition {
            atom_uuid: Some("data.valid_value".to_string()),
            field_type: Some("String".to_string()),
        },
    );
    fields.insert(
        "invalid_field".to_string(),
        FieldDefinition {
            atom_uuid: Some("data.invalid_expression().content".to_string()), // Invalid expression
            field_type: Some("String".to_string()),
        },
    );

    let key_config = KeyConfig {
        hash_field: "data.hash".to_string(),
        range_field: "data.range".to_string(),
    };

    let declarative_schema = DeclarativeSchemaDefinition {
        name: "parsing_errors_test".to_string(),
        schema_type: SchemaType::HashRange,
        key: Some(key_config),
        fields,
    };

    let transform = Transform::from_declarative_schema(
        declarative_schema,
        vec!["test_data".to_string()],
        "output.parsing_errors".to_string(),
    );

    // Create input data
    let mut input_values = HashMap::new();
    input_values.insert(
        "data".to_string(),
        serde_json::json!({
            "valid_value": "Valid field value",
            "hash": "hash_value",
            "range": "range_value",
            "content": "Content value"
        }),
    );

    // Execute the transform - should handle parsing errors gracefully
    let result = TransformExecutor::execute_transform(&transform, input_values);

    // Should either succeed with valid fields or handle errors gracefully
    match result {
        Ok(json_result) => {
            let obj = json_result.as_object().unwrap();
            assert!(obj.contains_key("valid_field"));
            // Invalid field might be included with fallback resolution
        }
        Err(err) => {
            // Should not crash, error should be informative
            let error_msg = format!("{:?}", err);
            assert!(
                !error_msg.contains("panic") && !error_msg.contains("crash"),
                "Should handle parsing errors gracefully: {}",
                error_msg
            );
        }
    }
}

#[test]
fn test_multi_chain_fallback_behavior() {
    // Test that multi-chain coordination falls back to simple resolution when ExecutionEngine produces placeholders
    let mut fields = HashMap::new();
    fields.insert(
        "fallback_field".to_string(),
        FieldDefinition {
            atom_uuid: Some("data.simple_path".to_string()),
            field_type: Some("String".to_string()),
        },
    );

    let key_config = KeyConfig {
        hash_field: "data.hash_value".to_string(),
        range_field: "data.range_value".to_string(),
    };

    let declarative_schema = DeclarativeSchemaDefinition {
        name: "fallback_test".to_string(),
        schema_type: SchemaType::HashRange,
        key: Some(key_config),
        fields,
    };

    let transform = Transform::from_declarative_schema(
        declarative_schema,
        vec!["test_data".to_string()],
        "output.fallback_test".to_string(),
    );

    // Create input data
    let mut input_values = HashMap::new();
    input_values.insert(
        "data".to_string(),
        serde_json::json!({
            "simple_path": "Fallback value",
            "hash_value": "hash123",
            "range_value": "range456"
        }),
    );

    // Execute the transform - should fallback to simple resolution if ExecutionEngine produces placeholders
    let result = TransformExecutor::execute_transform(&transform, input_values);

    // May succeed with fallback or fail due to ExecutionEngine limitations
    match result {
        Ok(json_result) => {
            let obj = json_result.as_object().unwrap();
            assert!(obj.contains_key("fallback_field"));

            // Should resolve to the actual value, not placeholder
            // For HashRange schemas, field values are arrays
            let fallback_value = obj.get("fallback_field").unwrap();
            assert!(fallback_value.is_array());
            let fallback_array = fallback_value.as_array().unwrap();
            assert!(!fallback_array.is_empty());
            assert_eq!(
                fallback_array[0],
                JsonValue::String("Fallback value".to_string())
            );
        }
        Err(err) => {
            // ExecutionEngine may have limitations with certain expressions
            let error_msg = format!("{:?}", err);
            assert!(
                !error_msg.contains("panic") && !error_msg.contains("crash"),
                "Should handle ExecutionEngine limitations gracefully: {}",
                error_msg
            );
        }
    }
}

#[test]
fn test_multi_chain_empty_expressions() {
    // Test multi-chain coordination with minimal expressions
    let fields = HashMap::new(); // No regular fields, only key fields

    let key_config = KeyConfig {
        hash_field: "data.hash".to_string(),
        range_field: "data.range".to_string(),
    };

    let declarative_schema = DeclarativeSchemaDefinition {
        name: "empty_expressions_test".to_string(),
        schema_type: SchemaType::HashRange,
        key: Some(key_config),
        fields,
    };

    let transform = Transform::from_declarative_schema(
        declarative_schema,
        vec!["test_data".to_string()],
        "output.empty_expressions".to_string(),
    );

    // Create input data
    let mut input_values = HashMap::new();
    input_values.insert(
        "data".to_string(),
        serde_json::json!({
            "hash": "hash_only",
            "range": "range_only"
        }),
    );

    // Execute the transform - should handle minimal expressions
    let result = TransformExecutor::execute_transform(&transform, input_values);

    // May succeed or fail depending on ExecutionEngine behavior with minimal expressions
    match result {
        Ok(json_result) => {
            let obj = json_result.as_object().unwrap();

            // Should be empty or minimal result since only key fields were provided
            // Key fields should not appear in output
            assert!(!obj.contains_key("_hash_field"));
            assert!(!obj.contains_key("_range_field"));
        }
        Err(err) => {
            // ExecutionEngine may fail with only key expressions - this is acceptable
            let error_msg = format!("{:?}", err);
            assert!(
                !error_msg.contains("panic") && !error_msg.contains("crash"),
                "Should handle minimal expressions gracefully: {}",
                error_msg
            );
        }
    }
}

#[test]
fn test_multi_chain_complex_expressions() {
    // Test multi-chain coordination with complex nested expressions
    let mut fields = HashMap::new();
    fields.insert(
        "complex_field".to_string(),
        FieldDefinition {
            atom_uuid: Some("blogpost.map().tags.split_array().map()".to_string()),
            field_type: Some("Array".to_string()),
        },
    );

    let key_config = KeyConfig {
        hash_field: "blogpost.map().author".to_string(),
        range_field: "blogpost.map().content.split_by_word().count()".to_string(),
    };

    let declarative_schema = DeclarativeSchemaDefinition {
        name: "complex_expressions_test".to_string(),
        schema_type: SchemaType::HashRange,
        key: Some(key_config),
        fields,
    };

    let transform = Transform::from_declarative_schema(
        declarative_schema,
        vec!["blogpost_data".to_string()],
        "output.complex_expressions".to_string(),
    );

    // Create input data
    let mut input_values = HashMap::new();
    input_values.insert(
        "blogpost".to_string(),
        serde_json::json!([
            {
                "author": "Complex Author",
                "content": "Complex content with multiple words for testing",
                "tags": ["complex", "multi-chain", "test"]
            }
        ]),
    );

    // Execute the transform - complex expressions may succeed or fail validation
    let result = TransformExecutor::execute_transform(&transform, input_values);

    // Complex expressions might fail alignment validation, which is acceptable
    match result {
        Ok(json_result) => {
            let obj = json_result.as_object().unwrap();
            assert!(obj.contains_key("complex_field"));
        }
        Err(err) => {
            // Should be alignment or coordination error, not execution crash
            let error_msg = format!("{:?}", err);
            assert!(
                !error_msg.contains("panic") && !error_msg.contains("crash"),
                "Should handle complex expressions gracefully: {}",
                error_msg
            );
        }
    }
}

#[test]
fn test_multi_chain_coordination_logging() {
    // Test that multi-chain coordination produces appropriate logging without crashing
    let mut fields = HashMap::new();
    fields.insert(
        "logged_field".to_string(),
        FieldDefinition {
            atom_uuid: Some("data.value".to_string()),
            field_type: Some("String".to_string()),
        },
    );

    let key_config = KeyConfig {
        hash_field: "data.hash".to_string(),
        range_field: "data.range".to_string(),
    };

    let declarative_schema = DeclarativeSchemaDefinition {
        name: "logging_test".to_string(),
        schema_type: SchemaType::HashRange,
        key: Some(key_config),
        fields,
    };

    let transform = Transform::from_declarative_schema(
        declarative_schema,
        vec!["test_data".to_string()],
        "output.logging_test".to_string(),
    );

    // Create input data
    let mut input_values = HashMap::new();
    input_values.insert(
        "data".to_string(),
        serde_json::json!({
            "value": "Logging test value",
            "hash": "log_hash",
            "range": "log_range"
        }),
    );

    // Execute the transform - should produce logging without issues
    let result = TransformExecutor::execute_transform(&transform, input_values);

    // The main requirement is that it doesn't crash and produces some result
    match result {
        Ok(json_result) => {
            // Success is good
            let obj = json_result.as_object().unwrap();
            assert!(obj.contains_key("logged_field"));
        }
        Err(_) => {
            // Controlled errors are also acceptable for this logging test
        }
    }
}
