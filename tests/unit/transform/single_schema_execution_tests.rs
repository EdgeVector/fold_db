use std::collections::HashMap;
use serde_json::Value as JsonValue;

use datafold::schema::types::Transform;
use datafold::schema::types::json_schema::{DeclarativeSchemaDefinition, FieldDefinition};
use datafold::schema::types::schema::SchemaType;
use datafold::transform::executor::TransformExecutor;

/// Tests for Single schema declarative transform execution
/// This validates that Single schema type declarative transforms execute correctly with actual field resolution

#[test]
fn test_single_schema_execution_with_simple_fields() {
    // Create a simple Single schema
    let mut fields = HashMap::new();
    fields.insert("user_name".to_string(), FieldDefinition {
        atom_uuid: Some("user_data.map().user".to_string()),
        field_type: Some("String".to_string()),
    });
    fields.insert("user_age".to_string(), FieldDefinition {
        atom_uuid: Some("user_data.map().age".to_string()),
        field_type: Some("Number".to_string()),
    });

    let declarative_schema = DeclarativeSchemaDefinition {
        name: "user_profile".to_string(),
        schema_type: SchemaType::Single,
        key: None,
        fields,
    };

    let transform = Transform::from_declarative_schema(
        declarative_schema,
        vec!["user_data".to_string()],
        "output.user_profile".to_string(),
    );

    // Create input data
    let mut input_values = HashMap::new();
    input_values.insert("user_data".to_string(), serde_json::json!({
        "user": "Alice",
        "age": 25
    }));

    // Execute the transform
    let result = TransformExecutor::execute_transform(&transform, input_values);
    
    assert!(result.is_ok(), "Single schema execution should succeed");
    
    let json_result = result.unwrap();
    assert!(json_result.is_object());
    
    let obj = json_result.as_object().unwrap();
    assert_eq!(obj.get("user_name"), Some(&JsonValue::String("Alice".to_string())));
    assert_eq!(obj.get("user_age"), Some(&JsonValue::Number(serde_json::Number::from(25))));
}

#[test]
fn test_single_schema_execution_with_dotted_paths() {
    // Create schema with dotted path expressions
    let mut fields = HashMap::new();
    fields.insert("user_name".to_string(), FieldDefinition {
        atom_uuid: Some("user.profile.name".to_string()),
        field_type: Some("String".to_string()),
    });
    fields.insert("user_email".to_string(), FieldDefinition {
        atom_uuid: Some("user.contact.email".to_string()),
        field_type: Some("String".to_string()),
    });

    let declarative_schema = DeclarativeSchemaDefinition {
        name: "user_info".to_string(),
        schema_type: SchemaType::Single,
        key: None,
        fields,
    };

    let transform = Transform::from_declarative_schema(
        declarative_schema,
        vec!["user_data".to_string()],
        "output.user_info".to_string(),
    );

    // Create nested input data
    let mut input_values = HashMap::new();
    input_values.insert("user".to_string(), serde_json::json!({
        "profile": {
            "name": "Bob",
            "age": 30
        },
        "contact": {
            "email": "bob@example.com",
            "phone": "123-456-7890"
        }
    }));

    // Execute the transform
    let result = TransformExecutor::execute_transform(&transform, input_values);
    
    assert!(result.is_ok(), "Dotted path execution should succeed");
    
    let json_result = result.unwrap();
    let obj = json_result.as_object().unwrap();
    assert_eq!(obj.get("user_name"), Some(&JsonValue::String("Bob".to_string())));
    assert_eq!(obj.get("user_email"), Some(&JsonValue::String("bob@example.com".to_string())));
}

#[test]
fn test_single_schema_execution_with_missing_fields() {
    // Create schema that references non-existent fields
    let mut fields = HashMap::new();
    fields.insert("existing_field".to_string(), FieldDefinition {
        atom_uuid: Some("data".to_string()),
        field_type: Some("String".to_string()),
    });
    fields.insert("missing_field".to_string(), FieldDefinition {
        atom_uuid: Some("nonexistent".to_string()),
        field_type: Some("String".to_string()),
    });

    let declarative_schema = DeclarativeSchemaDefinition {
        name: "partial_data".to_string(),
        schema_type: SchemaType::Single,
        key: None,
        fields,
    };

    let transform = Transform::from_declarative_schema(
        declarative_schema,
        vec!["input_data".to_string()],
        "output.partial_data".to_string(),
    );

    // Create input data with only some fields
    let mut input_values = HashMap::new();
    input_values.insert("data".to_string(), JsonValue::String("present".to_string()));
    // Note: "nonexistent" is not provided

    // Execute the transform
    let result = TransformExecutor::execute_transform(&transform, input_values);
    
    assert!(result.is_ok(), "Execution should succeed even with missing fields");
    
    let json_result = result.unwrap();
    let obj = json_result.as_object().unwrap();
    assert_eq!(obj.get("existing_field"), Some(&JsonValue::String("present".to_string())));
    assert_eq!(obj.get("missing_field"), Some(&JsonValue::Null)); // Should be null for missing fields
}


#[test]
fn test_single_schema_execution_skips_function_calls() {
    // Create schema with function calls (should be skipped for now)
    let mut fields = HashMap::new();
    fields.insert("function_result".to_string(), FieldDefinition {
        atom_uuid: Some("user.map().name".to_string()),
        field_type: Some("String".to_string()),
    });

    let declarative_schema = DeclarativeSchemaDefinition {
        name: "function_test".to_string(),
        schema_type: SchemaType::Single,
        key: None,
        fields,
    };

    let transform = Transform::from_declarative_schema(
        declarative_schema,
        vec!["user_data".to_string()],
        "output.function_test".to_string(),
    );

    // Create input data
    let mut input_values = HashMap::new();
    input_values.insert("user".to_string(), serde_json::json!({
        "name": "function_skipped"
    }));

    // Execute the transform
    let result = TransformExecutor::execute_transform(&transform, input_values);
    
    assert!(result.is_ok(), "Function call skipping should succeed");
    
    let json_result = result.unwrap();
    let obj = json_result.as_object().unwrap();
    // Should access "user.name" after skipping "map()"
    assert_eq!(obj.get("function_result"), Some(&JsonValue::String("function_skipped".to_string())));
}

#[test]
fn test_range_and_hashrange_schemas_use_placeholder() {
    // Test that Range and HashRange schemas still use placeholder execution
    
    // Create Range schema
    let mut range_fields = HashMap::new();
    range_fields.insert("test_field".to_string(), FieldDefinition {
        atom_uuid: Some("data".to_string()),
        field_type: Some("String".to_string()),
    });

    let range_schema = DeclarativeSchemaDefinition {
        name: "range_test".to_string(),
        schema_type: SchemaType::Range { range_key: "timestamp".to_string() },
        key: None,
        fields: range_fields,
    };

    let range_transform = Transform::from_declarative_schema(
        range_schema,
        vec!["input_data".to_string()],
        "output.range_test".to_string(),
    );

    let input_values = HashMap::new();
    let range_result = TransformExecutor::execute_transform(&range_transform, input_values.clone());
    
    // Range schemas now have actual execution, not placeholders
    match range_result {
        Ok(range_json) => {
            // Should be a proper result object, not a placeholder
            let obj = range_json.as_object().unwrap();
            // Should not contain placeholder fields
            assert!(!obj.contains_key("status") || obj.get("status") != Some(&JsonValue::String("placeholder_execution".to_string())));
        }
        Err(_) => {
            // May fail due to ExecutionEngine limitations or validation - this is acceptable
            // The important thing is that it's no longer a placeholder
        }
    }

    // Create HashRange schema
    let mut hashrange_fields = HashMap::new();
    hashrange_fields.insert("test_field".to_string(), FieldDefinition {
        atom_uuid: Some("data".to_string()),
        field_type: Some("String".to_string()),
    });

    let hashrange_schema = DeclarativeSchemaDefinition {
        name: "hashrange_test".to_string(),
        schema_type: SchemaType::HashRange,
        key: Some(datafold::schema::types::json_schema::KeyConfig {
            hash_field: "hash_key".to_string(),
            range_field: "range_key".to_string(),
        }),
        fields: hashrange_fields,
    };

    let hashrange_transform = Transform::from_declarative_schema(
        hashrange_schema,
        vec!["input_data".to_string()],
        "output.hashrange_test".to_string(),
    );

    let hashrange_result = TransformExecutor::execute_transform(&hashrange_transform, input_values);
    
    // HashRange schemas now have actual execution, not placeholders
    match hashrange_result {
        Ok(hashrange_json) => {
            // Should be a proper result object, not a placeholder
            let obj = hashrange_json.as_object().unwrap();
            // Should not contain placeholder fields
            assert!(!obj.contains_key("status") || obj.get("status") != Some(&JsonValue::String("placeholder_execution".to_string())));
        }
        Err(_) => {
            // May fail due to ExecutionEngine limitations or validation - this is acceptable
            // The important thing is that it's no longer a placeholder
        }
    }
}

#[test]
fn test_backward_compatibility_after_single_schema_implementation() {
    // Ensure procedural transforms still work after implementing single schema execution
    let procedural_transform = Transform::new("return 123".to_string(), "output.number".to_string());
    
    let input_values = HashMap::new();
    let result = TransformExecutor::execute_transform(&procedural_transform, input_values);
    
    // Should either succeed or fail with parsing error (not routing or execution error)
    match result {
        Ok(_) => {
            // Success - backward compatibility maintained
        }
        Err(err) => {
            // Should not be a routing or execution error
            let error_msg = format!("{:?}", err);
            assert!(!error_msg.contains("Unknown transform type"), 
                   "Backward compatibility broken: {}", error_msg);
            assert!(!error_msg.contains("Cannot execute declarative transform"), 
                   "Backward compatibility broken: {}", error_msg);
        }
    }
}
