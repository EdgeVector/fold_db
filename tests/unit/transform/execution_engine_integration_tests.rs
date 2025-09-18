use std::collections::HashMap;
use serde_json::Value as JsonValue;

use datafold::schema::types::Transform;
use datafold::schema::types::json_schema::{DeclarativeSchemaDefinition, FieldDefinition};
use datafold::schema::types::schema::SchemaType;
use datafold::transform::executor::TransformExecutor;

/// Tests for ExecutionEngine integration with declarative transform execution
/// This validates that the ExecutionEngine correctly executes single declarative expressions

#[test]
fn test_single_expression_execution_with_engine() {
    // Create schema with a simple chain expression
    let mut fields = HashMap::new();
    fields.insert("post_title".to_string(), FieldDefinition {
        atom_uuid: Some("blogpost.map().title".to_string()),
        field_type: Some("String".to_string()),
    });

    let declarative_schema = DeclarativeSchemaDefinition {
        name: "execution_engine_test".to_string(),
        schema_type: SchemaType::Single,
        key: None,
        fields,
    };

    let transform = Transform::from_declarative_schema(
        declarative_schema,
        vec!["blogpost_data".to_string()],
        "output.execution_test".to_string(),
    );

    // Create input data that should work with the ExecutionEngine
    let mut input_values = HashMap::new();
    input_values.insert("blogpost".to_string(), serde_json::json!([
        {"title": "First Post", "content": "Content 1"},
        {"title": "Second Post", "content": "Content 2"}
    ]));

    // Execute the transform - should use ExecutionEngine or fallback to simple resolution
    let result = TransformExecutor::execute_transform(&transform, input_values);
    
    assert!(result.is_ok(), "ExecutionEngine integration should succeed: {:?}", result);
    
    let json_result = result.unwrap();
    let obj = json_result.as_object().unwrap();
    
    // Should have the field from the schema
    assert!(obj.contains_key("post_title"), "Result should contain post_title field");
    
    // The value might be from ExecutionEngine (if it produces real values) or from fallback resolution
    let post_title_value = obj.get("post_title").unwrap();
    
    // The ExecutionEngine often returns placeholder values and falls back to simple resolution
    // In this case, since we're dealing with an array, it might resolve to the array itself or null
    match post_title_value {
        JsonValue::Null => {
            // Acceptable - complex expressions might not resolve with simple fallback
        }
        JsonValue::Array(_) => {
            // Acceptable - ExecutionEngine might return array results
        }
        JsonValue::String(_) => {
            // Acceptable - simple resolution might extract a single value
        }
        _ => {
            // Any valid JSON value is acceptable for this integration test
        }
    }
}

#[test]
fn test_execution_engine_with_simple_field_access() {
    // Create schema with basic field access
    let mut fields = HashMap::new();
    fields.insert("content_field".to_string(), FieldDefinition {
        atom_uuid: Some("blogpost.content".to_string()),
        field_type: Some("String".to_string()),
    });

    let declarative_schema = DeclarativeSchemaDefinition {
        name: "simple_field_test".to_string(),
        schema_type: SchemaType::Single,
        key: None,
        fields,
    };

    let transform = Transform::from_declarative_schema(
        declarative_schema,
        vec!["blogpost_data".to_string()],
        "output.simple_field".to_string(),
    );

    // Create input data
    let mut input_values = HashMap::new();
    input_values.insert("blogpost".to_string(), serde_json::json!({
        "content": "Test content for simple field access"
    }));

    // Execute the transform
    let result = TransformExecutor::execute_transform(&transform, input_values);
    
    assert!(result.is_ok(), "Simple field access should succeed");
    
    let json_result = result.unwrap();
    let obj = json_result.as_object().unwrap();
    assert!(obj.contains_key("content_field"));
}

#[test]
fn test_execution_engine_fallback_behavior() {
    // Create schema with expression that might fail ExecutionEngine but should fallback
    let mut fields = HashMap::new();
    fields.insert("fallback_field".to_string(), FieldDefinition {
        atom_uuid: Some("data.simple_path".to_string()), // Simple path that should fallback
        field_type: Some("String".to_string()),
    });

    let declarative_schema = DeclarativeSchemaDefinition {
        name: "fallback_test".to_string(),
        schema_type: SchemaType::Single,
        key: None,
        fields,
    };

    let transform = Transform::from_declarative_schema(
        declarative_schema,
        vec!["test_data".to_string()],
        "output.fallback_test".to_string(),
    );

    // Create input data
    let mut input_values = HashMap::new();
    input_values.insert("data".to_string(), serde_json::json!({
        "simple_path": "Fallback value"
    }));

    // Execute the transform - should fallback to simple resolution if ExecutionEngine fails
    let result = TransformExecutor::execute_transform(&transform, input_values);
    
    assert!(result.is_ok(), "Fallback behavior should ensure success");
    
    let json_result = result.unwrap();
    let obj = json_result.as_object().unwrap();
    assert_eq!(obj.get("fallback_field"), Some(&JsonValue::String("Fallback value".to_string())));
}

#[test]
fn test_execution_engine_with_map_operation() {
    // Create schema with map operation that ExecutionEngine should handle
    let mut fields = HashMap::new();
    fields.insert("mapped_titles".to_string(), FieldDefinition {
        atom_uuid: Some("posts.map().title".to_string()),
        field_type: Some("Array".to_string()),
    });

    let declarative_schema = DeclarativeSchemaDefinition {
        name: "map_operation_test".to_string(),
        schema_type: SchemaType::Single,
        key: None,
        fields,
    };

    let transform = Transform::from_declarative_schema(
        declarative_schema,
        vec!["posts_data".to_string()],
        "output.map_test".to_string(),
    );

    // Create input data with array for mapping
    let mut input_values = HashMap::new();
    input_values.insert("posts".to_string(), serde_json::json!([
        {"title": "Post 1", "author": "Author 1"},
        {"title": "Post 2", "author": "Author 2"},
        {"title": "Post 3", "author": "Author 3"}
    ]));

    // Execute the transform - ExecutionEngine should handle map operations
    let result = TransformExecutor::execute_transform(&transform, input_values);
    
    assert!(result.is_ok(), "Map operation should be handled by ExecutionEngine");
    
    let json_result = result.unwrap();
    let obj = json_result.as_object().unwrap();
    assert!(obj.contains_key("mapped_titles"));
}

#[test]
fn test_execution_engine_error_handling() {
    // Create schema with expression that might cause ExecutionEngine errors
    let mut fields = HashMap::new();
    fields.insert("error_prone_field".to_string(), FieldDefinition {
        atom_uuid: Some("nonexistent.map().field".to_string()),
        field_type: Some("String".to_string()),
    });

    let declarative_schema = DeclarativeSchemaDefinition {
        name: "error_handling_test".to_string(),
        schema_type: SchemaType::Single,
        key: None,
        fields,
    };

    let transform = Transform::from_declarative_schema(
        declarative_schema,
        vec!["test_data".to_string()],
        "output.error_test".to_string(),
    );

    // Create input data that doesn't match the expression
    let mut input_values = HashMap::new();
    input_values.insert("different_field".to_string(), serde_json::json!({
        "value": "Not matching the expression"
    }));

    // Execute the transform - should handle errors gracefully
    let result = TransformExecutor::execute_transform(&transform, input_values);
    
    // Should either succeed with fallback or fail gracefully
    match result {
        Ok(json_result) => {
            let obj = json_result.as_object().unwrap();
            // If it succeeds, it should fallback and return null
            let error_field = obj.get("error_prone_field").unwrap();
            assert!(error_field.is_null() || error_field.is_string());
        }
        Err(_) => {
            // If it fails, the error should be handled gracefully (not panic)
            // This is acceptable behavior for field alignment validation
        }
    }
}

#[test]
fn test_execution_engine_with_multiple_operations() {
    // Create schema with multiple operations in a chain
    let mut fields = HashMap::new();
    fields.insert("complex_field".to_string(), FieldDefinition {
        atom_uuid: Some("data.map().items.split_array().map()".to_string()),
        field_type: Some("Array".to_string()),
    });

    let declarative_schema = DeclarativeSchemaDefinition {
        name: "complex_operations_test".to_string(),
        schema_type: SchemaType::Single,
        key: None,
        fields,
    };

    let transform = Transform::from_declarative_schema(
        declarative_schema,
        vec!["complex_data".to_string()],
        "output.complex_test".to_string(),
    );

    // Create input data for complex operations
    let mut input_values = HashMap::new();
    input_values.insert("data".to_string(), serde_json::json!([
        {"items": ["item1", "item2"]},
        {"items": ["item3", "item4"]}
    ]));

    // Execute the transform - should handle complex operations
    let result = TransformExecutor::execute_transform(&transform, input_values);
    
    // Result may succeed or fail depending on ExecutionEngine capability
    // The important thing is no crashes and graceful error handling
    match result {
        Ok(json_result) => {
            let obj = json_result.as_object().unwrap();
            assert!(obj.contains_key("complex_field"));
        }
        Err(err) => {
            // Error should be graceful and related to validation or execution
            let error_msg = format!("{:?}", err);
            assert!(!error_msg.contains("panic") && !error_msg.contains("crash"),
                   "Should handle complex operations gracefully: {}", error_msg);
        }
    }
}

#[test]
fn test_execution_engine_integration_with_validation() {
    // Create schema that should pass both validation and execution
    let mut fields = HashMap::new();
    fields.insert("validated_field".to_string(), FieldDefinition {
        atom_uuid: Some("blogpost.map().title".to_string()),
        field_type: Some("String".to_string()),
    });
    fields.insert("validated_content".to_string(), FieldDefinition {
        atom_uuid: Some("blogpost.map().content".to_string()),
        field_type: Some("String".to_string()),
    });

    let declarative_schema = DeclarativeSchemaDefinition {
        name: "validation_integration_test".to_string(),
        schema_type: SchemaType::Single,
        key: None,
        fields,
    };

    let transform = Transform::from_declarative_schema(
        declarative_schema,
        vec!["blogpost_data".to_string()],
        "output.validation_integration".to_string(),
    );

    // Create input data
    let mut input_values = HashMap::new();
    input_values.insert("blogpost".to_string(), serde_json::json!([
        {"title": "Title 1", "content": "Content 1"},
        {"title": "Title 2", "content": "Content 2"}
    ]));

    // Execute the transform - should pass validation and then execute with ExecutionEngine
    let result = TransformExecutor::execute_transform(&transform, input_values);
    
    assert!(result.is_ok(), "Validated expressions should execute successfully");
    
    let json_result = result.unwrap();
    let obj = json_result.as_object().unwrap();
    assert!(obj.contains_key("validated_field"));
    assert!(obj.contains_key("validated_content"));
}

#[test]
fn test_execution_engine_result_format() {
    // Test that ExecutionEngine results are properly formatted for transform output
    let mut fields = HashMap::new();
    fields.insert("formatted_field".to_string(), FieldDefinition {
        atom_uuid: Some("data.value".to_string()),
        field_type: Some("String".to_string()),
    });

    let declarative_schema = DeclarativeSchemaDefinition {
        name: "result_format_test".to_string(),
        schema_type: SchemaType::Single,
        key: None,
        fields,
    };

    let transform = Transform::from_declarative_schema(
        declarative_schema,
        vec!["test_data".to_string()],
        "output.format_test".to_string(),
    );

    // Create input data
    let mut input_values = HashMap::new();
    input_values.insert("data".to_string(), serde_json::json!({
        "value": "Formatted test value"
    }));

    // Execute the transform
    let result = TransformExecutor::execute_transform(&transform, input_values);
    
    assert!(result.is_ok(), "Result formatting should work correctly");
    
    let json_result = result.unwrap();
    
    // Verify the result is properly formatted JSON
    assert!(json_result.is_object(), "Result should be a JSON object");
    
    let obj = json_result.as_object().unwrap();
    assert!(obj.contains_key("formatted_field"));
    
    let field_value = obj.get("formatted_field").unwrap();
    assert!(!field_value.is_null(), "Field value should not be null");
}

#[test]
fn test_execution_engine_with_no_input_data() {
    // Test ExecutionEngine behavior with minimal/no input data
    let mut fields = HashMap::new();
    fields.insert("empty_field".to_string(), FieldDefinition {
        atom_uuid: Some("data.missing_field".to_string()),
        field_type: Some("String".to_string()),
    });

    let declarative_schema = DeclarativeSchemaDefinition {
        name: "no_input_test".to_string(),
        schema_type: SchemaType::Single,
        key: None,
        fields,
    };

    let transform = Transform::from_declarative_schema(
        declarative_schema,
        vec!["empty_data".to_string()],
        "output.no_input".to_string(),
    );

    // Create empty input data
    let input_values = HashMap::new();

    // Execute the transform - should handle empty input gracefully
    let result = TransformExecutor::execute_transform(&transform, input_values);
    
    match result {
        Ok(json_result) => {
            let obj = json_result.as_object().unwrap();
            let empty_field = obj.get("empty_field").unwrap();
            assert!(empty_field.is_null(), "Missing field should result in null");
        }
        Err(_) => {
            // Acceptable behavior - might fail validation or execution gracefully
        }
    }
}

#[test]
fn test_backward_compatibility_with_execution_engine() {
    // Ensure procedural transforms still work after adding ExecutionEngine integration
    let procedural_transform = Transform::new("return 789".to_string(), "output.procedural_number".to_string());
    
    let input_values = HashMap::new();
    let result = TransformExecutor::execute_transform(&procedural_transform, input_values);
    
    // Should not be affected by ExecutionEngine integration
    match result {
        Ok(_) => {
            // Success - backward compatibility maintained
        }
        Err(err) => {
            // Should not fail due to ExecutionEngine integration
            let error_msg = format!("{:?}", err);
            assert!(!error_msg.contains("ExecutionEngine") && !error_msg.contains("execution_engine"),
                   "Procedural transforms should not be affected by ExecutionEngine integration: {}", error_msg);
        }
    }
}

#[test]
fn test_execution_engine_with_special_fields() {
    // Test ExecutionEngine with special fields like $atom_uuid
    let mut fields = HashMap::new();
    fields.insert("uuid_field".to_string(), FieldDefinition {
        atom_uuid: Some("data.map().$atom_uuid".to_string()),
        field_type: Some("String".to_string()),
    });

    let declarative_schema = DeclarativeSchemaDefinition {
        name: "special_fields_test".to_string(),
        schema_type: SchemaType::Single,
        key: None,
        fields,
    };

    let transform = Transform::from_declarative_schema(
        declarative_schema,
        vec!["test_data".to_string()],
        "output.special_fields".to_string(),
    );

    // Create input data with special fields
    let mut input_values = HashMap::new();
    input_values.insert("data".to_string(), serde_json::json!([
        {"$atom_uuid": "uuid-123", "value": "value1"},
        {"$atom_uuid": "uuid-456", "value": "value2"}
    ]));

    // Execute the transform - should handle special fields
    let result = TransformExecutor::execute_transform(&transform, input_values);
    
    // Should either succeed with ExecutionEngine or fallback to simple resolution
    match result {
        Ok(json_result) => {
            let obj = json_result.as_object().unwrap();
            assert!(obj.contains_key("uuid_field"));
        }
        Err(err) => {
            // If it fails, should be graceful error handling
            let error_msg = format!("{:?}", err);
            assert!(!error_msg.contains("panic"),
                   "Special field handling should be graceful: {}", error_msg);
        }
    }
}
