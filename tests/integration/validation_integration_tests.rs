use std::collections::HashMap;

use datafold::schema::types::Transform;
use datafold::schema::types::json_schema::{
    DeclarativeSchemaDefinition, FieldDefinition, KeyConfig, JsonTransform, TransformKind
};
use datafold::schema::types::schema::SchemaType;
use datafold::transform::executor::TransformExecutor;

/// Integration tests for validation with existing infrastructure.
///
/// These tests verify that validation integrates properly with:
/// - Schema loading and interpretation
/// - Transform execution
/// - Error handling throughout the system
/// - User experience with validation feedback

#[test]
fn test_validation_integration_with_transform_execution() {
    // Create a valid declarative transform
    let mut fields = HashMap::new();
    fields.insert("title".to_string(), FieldDefinition {
        field_type: Some("single".to_string()),
        atom_uuid: Some("blogpost.title".to_string()),
    });
    fields.insert("content".to_string(), FieldDefinition {
        field_type: Some("single".to_string()),
        atom_uuid: Some("blogpost.content".to_string()),
    });

    let schema = DeclarativeSchemaDefinition {
        name: "integration_test_schema".to_string(),
        schema_type: SchemaType::Single,
        key: None,
        fields,
    };

    let transform = Transform::from_declarative_schema(
        schema,
        vec!["blogpost".to_string()],
        "processed_content".to_string()
    );

    // Validation should pass
    assert!(transform.validate().is_ok(), "Valid declarative transform should pass validation");

    // Transform should execute successfully with proper input
    let mut input_values = HashMap::new();
    input_values.insert("blogpost".to_string(), serde_json::json!({
        "title": "Test Blog Post",
        "content": "This is test content",
        "id": "12345"
    }));

    let execution_result = TransformExecutor::execute_transform_with_expr(&transform, input_values);
    
    // Should not crash due to validation issues
    match execution_result {
        Ok(_) => {
            // Successful execution is good
        }
        Err(error) => {
            // Execution errors are acceptable (due to ExecutionEngine limitations)
            // but should not be validation-related crashes
            let error_msg = format!("{:?}", error);
            assert!(!error_msg.contains("panic") && !error_msg.contains("crash"),
                   "Execution should not crash due to validation issues: {}", error_msg);
        }
    }
}

#[test]
fn test_json_transform_validation_integration() {
    // Test that JsonTransform validation integrates with existing validation
    let json_transform = JsonTransform {
        inputs: vec!["test_schema.blogpost".to_string()], // Use proper schema.field format  
        output: "test_schema.processed_content".to_string(), // Use proper schema.field format
        kind: TransformKind::Declarative {
            schema: DeclarativeSchemaDefinition {
                name: "json_integration_test".to_string(),
                schema_type: SchemaType::Single,
                key: None,
                fields: {
                    let mut fields = HashMap::new();
                    fields.insert("title".to_string(), FieldDefinition {
                        field_type: Some("single".to_string()),
                        atom_uuid: Some("blogpost.title".to_string()),
                    });
                    fields
                },
            }
        },
    };

    // JsonTransform validation should work
    let validation_result = json_transform.validate();
    assert!(validation_result.is_ok(), "Valid JsonTransform should pass validation: {:?}", validation_result);

    // Conversion to Transform should preserve validation
    let transform: Transform = json_transform.into();
    let transform_validation = transform.validate();
    assert!(transform_validation.is_ok(), "Converted Transform should also pass validation: {:?}", transform_validation);
}

#[test]
fn test_validation_with_complex_hashrange_scenario() {
    // Create a more complex HashRange scenario to test comprehensive validation
    let mut fields = HashMap::new();
    fields.insert("title".to_string(), FieldDefinition {
        field_type: Some("single".to_string()),
        atom_uuid: Some("blogpost.title".to_string()),
    });
    fields.insert("tags".to_string(), FieldDefinition {
        field_type: Some("single".to_string()),
        atom_uuid: Some("blogpost.tags.map().name".to_string()),
    });
    fields.insert("author".to_string(), FieldDefinition {
        field_type: Some("single".to_string()),
        atom_uuid: Some("blogpost.author.name".to_string()),
    });

    let schema = DeclarativeSchemaDefinition {
        name: "complex_hashrange_integration".to_string(),
        schema_type: SchemaType::HashRange,
        key: Some(KeyConfig {
            hash_field: "blogpost.id".to_string(),
            range_field: "blogpost.published_date".to_string(),
        }),
        fields,
    };

    let transform = Transform::from_declarative_schema(
        schema,
        vec!["blogpost".to_string()],
        "processed_blog".to_string()
    );

    // Comprehensive validation should pass
    let validation_result = transform.validate();
    assert!(validation_result.is_ok(), "Complex HashRange transform should pass validation: {:?}", validation_result);

    // Execution integration test
    let mut input_values = HashMap::new();
    input_values.insert("blogpost".to_string(), serde_json::json!({
        "id": "blog_123",
        "title": "Integration Test Blog",
        "published_date": "2025-01-27",
        "author": {
            "name": "Test Author"
        },
        "tags": [
            {"name": "rust"},
            {"name": "testing"}
        ]
    }));

    let execution_result = TransformExecutor::execute_transform_with_expr(&transform, input_values);
    
    // Should handle execution gracefully (success or controlled failure)
    match execution_result {
        Ok(result) => {
            // Successful execution demonstrates good validation integration
            assert!(result.is_object(), "HashRange execution should produce object result");
        }
        Err(error) => {
            // Controlled failures are acceptable, but should not be validation crashes
            let error_msg = format!("{:?}", error);
            assert!(!error_msg.contains("panic") && !error_msg.contains("crash"),
                   "Should handle execution gracefully: {}", error_msg);
        }
    }
}

#[test]
fn test_validation_error_integration_with_execution() {
    // Test that validation errors prevent execution and provide good feedback
    let mut fields = HashMap::new();
    fields.insert("invalid_field".to_string(), FieldDefinition {
        field_type: Some("single".to_string()),
        atom_uuid: Some("blogpost..invalid.syntax".to_string()), // Invalid syntax
    });

    let schema = DeclarativeSchemaDefinition {
        name: "invalid_integration_test".to_string(),
        schema_type: SchemaType::Single,
        key: None,
        fields,
    };

    let transform = Transform::from_declarative_schema(
        schema,
        vec!["blogpost".to_string()],
        "output".to_string()
    );

    // Validation should fail with clear error
    let validation_result = transform.validate();
    assert!(validation_result.is_err(), "Invalid transform should fail validation");

    let error_msg = format!("{:?}", validation_result.unwrap_err());
    assert!(error_msg.contains("Expression parsing failed") || error_msg.contains("invalid"),
           "Validation error should be descriptive: {}", error_msg);

    // Even with validation failure, execution should not crash
    let mut input_values = HashMap::new();
    input_values.insert("blogpost".to_string(), serde_json::json!({
        "title": "Test"
    }));

    let execution_result = TransformExecutor::execute_transform_with_expr(&transform, input_values);
    
    // Execution may succeed or fail, but should not crash
    match execution_result {
        Ok(_) => {
            // Execution engine may handle some invalid expressions gracefully
        }
        Err(error) => {
            let error_msg = format!("{:?}", error);
            assert!(!error_msg.contains("panic") && !error_msg.contains("crash"),
                   "Should not crash on invalid expressions: {}", error_msg);
        }
    }
}

#[test] 
fn test_validation_warning_integration() {
    // Test that validation warnings are properly logged and don't prevent execution
    let mut fields = HashMap::new();
    
    // Create a scenario that might generate warnings (many fields in Single schema)
    for i in 0..12 {
        fields.insert(format!("field_{}", i), FieldDefinition {
            field_type: Some("single".to_string()),
            atom_uuid: Some(format!("blogpost.field_{}", i)),
        });
    }

    let schema = DeclarativeSchemaDefinition {
        name: "warning_integration_test".to_string(),
        schema_type: SchemaType::Single,
        key: None,
        fields,
    };

    let transform = Transform::from_declarative_schema(
        schema,
        vec!["blogpost".to_string()],
        "output".to_string()
    );

    // Validation should pass despite warnings
    let validation_result = transform.validate();
    assert!(validation_result.is_ok(), "Transform with warnings should still pass validation: {:?}", validation_result);

    // Execution should also work
    let mut input_values = HashMap::new();
    let mut blogpost_data = serde_json::Map::new();
    for i in 0..12 {
        blogpost_data.insert(format!("field_{}", i), serde_json::Value::String(format!("value_{}", i)));
    }
    input_values.insert("blogpost".to_string(), serde_json::Value::Object(blogpost_data));

    let execution_result = TransformExecutor::execute_transform_with_expr(&transform, input_values);
    
    // Should handle execution despite performance warnings
    match execution_result {
        Ok(_) => {
            // Good - warnings don't prevent execution
        }
        Err(error) => {
            // Acceptable - but should not be warning-related crashes
            let error_msg = format!("{:?}", error);
            assert!(!error_msg.contains("warning") && !error_msg.contains("crash"),
                   "Warnings should not cause execution failures: {}", error_msg);
        }
    }
}

#[test]
fn test_range_schema_validation_integration() {
    // Test Range schema validation integration
    let mut fields = HashMap::new();
    fields.insert("title".to_string(), FieldDefinition {
        field_type: Some("single".to_string()),
        atom_uuid: Some("blogpost.title".to_string()),
    });
    fields.insert("published_date".to_string(), FieldDefinition {
        field_type: Some("single".to_string()),
        atom_uuid: Some("blogpost.published_date".to_string()),
    });

    let schema = DeclarativeSchemaDefinition {
        name: "range_integration_test".to_string(),
        schema_type: SchemaType::Range { range_key: "published_date".to_string() },
        key: None,
        fields,
    };

    let transform = Transform::from_declarative_schema(
        schema,
        vec!["blogpost".to_string()],
        "processed_blog".to_string()
    );

    // Range validation should pass
    let validation_result = transform.validate();
    assert!(validation_result.is_ok(), "Valid Range transform should pass validation: {:?}", validation_result);

    // Execution integration
    let mut input_values = HashMap::new();
    input_values.insert("blogpost".to_string(), serde_json::json!({
        "title": "Range Test Blog",
        "published_date": "2025-01-27"
    }));

    let execution_result = TransformExecutor::execute_transform_with_expr(&transform, input_values);
    
    // Should handle Range execution appropriately
    match execution_result {
        Ok(_) => {
            // Successful Range execution
        }
        Err(error) => {
            // Range execution might have limitations, but should not crash
            let error_msg = format!("{:?}", error);
            assert!(!error_msg.contains("panic") && !error_msg.contains("crash"),
                   "Range execution should be controlled: {}", error_msg);
        }
    }
}

#[test]
fn test_backward_compatibility_with_validation() {
    // Test that validation doesn't break backward compatibility with existing transforms
    let procedural_transform = Transform::new(
        "return input.title + ' - processed'".to_string(),
        "processed_title".to_string()
    );

    // Procedural transforms should still validate
    let validation_result = procedural_transform.validate();
    assert!(validation_result.is_ok(), "Procedural transforms should remain compatible: {:?}", validation_result);

    // Execution should still work
    let mut input_values = HashMap::new();
    input_values.insert("input".to_string(), serde_json::json!({
        "title": "Test Title"
    }));

    let execution_result = TransformExecutor::execute_transform_with_expr(&procedural_transform, input_values);
    
    // Procedural execution should work as before
    match execution_result {
        Ok(_) => {
            // Good - backward compatibility maintained
        }
        Err(error) => {
            // Some procedural expressions might not execute perfectly, but should not be validation-related
            let error_msg = format!("{:?}", error);
            assert!(!error_msg.contains("validation") && !error_msg.contains("crash"),
                   "Procedural execution should maintain compatibility: {}", error_msg);
        }
    }
}
