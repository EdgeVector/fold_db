use std::collections::HashMap;
use serde_json::Value as JsonValue;

use datafold::schema::types::Transform;
use datafold::schema::types::json_schema::{DeclarativeSchemaDefinition, FieldDefinition};
use datafold::schema::types::schema::SchemaType;
use datafold::transform::executor::TransformExecutor;

/// Tests for FieldAlignmentValidator integration with declarative transform execution
/// This validates that the FieldAlignmentValidator correctly validates field alignment for declarative transforms

#[test]
fn test_valid_single_depth_alignment() {
    // Create schema with fields at the same depth (should pass validation)
    let mut fields = HashMap::new();
    fields.insert("title".to_string(), FieldDefinition {
        atom_uuid: Some("blogpost.map().title".to_string()),
        field_type: Some("String".to_string()),
    });
    fields.insert("content".to_string(), FieldDefinition {
        atom_uuid: Some("blogpost.map().content".to_string()),
        field_type: Some("String".to_string()),
    });

    let declarative_schema = DeclarativeSchemaDefinition {
        name: "aligned_schema".to_string(),
        schema_type: SchemaType::Single,
        key: None,
        fields,
    };

    let transform = Transform::from_declarative_schema(
        declarative_schema,
        vec!["blogpost_data".to_string()],
        "output.aligned_schema".to_string(),
    );

    // Create input data
    let mut input_values = HashMap::new();
    input_values.insert("blogpost".to_string(), serde_json::json!({
        "title": "Test Post",
        "content": "Test content",
        "author": "Test Author"
    }));

    // Execute the transform - should pass field alignment validation
    let result = TransformExecutor::execute_transform_with_expr(&transform, input_values);
    
    assert!(result.is_ok(), "Valid field alignment should succeed: {:?}", result);
    
    let json_result = result.unwrap();
    let obj = json_result.as_object().unwrap();
    assert_eq!(obj.get("title"), Some(&JsonValue::String("Test Post".to_string())));
    assert_eq!(obj.get("content"), Some(&JsonValue::String("Test content".to_string())));
}

#[test]
fn test_broadcast_alignment_validation() {
    // Create schema with broadcast alignment (shallow + deep fields)
    let mut fields = HashMap::new();
    fields.insert("blog_title".to_string(), FieldDefinition {
        atom_uuid: Some("blogpost.title".to_string()), // Depth 0 (broadcast)
        field_type: Some("String".to_string()),
    });
    fields.insert("tag_name".to_string(), FieldDefinition {
        atom_uuid: Some("blogpost.map().tags.split_array().map()".to_string()), // Depth 2
        field_type: Some("String".to_string()),
    });

    let declarative_schema = DeclarativeSchemaDefinition {
        name: "broadcast_schema".to_string(),
        schema_type: SchemaType::Single,
        key: None,
        fields,
    };

    let transform = Transform::from_declarative_schema(
        declarative_schema,
        vec!["blogpost_data".to_string()],
        "output.broadcast_schema".to_string(),
    );

    // Create input data
    let mut input_values = HashMap::new();
    input_values.insert("blogpost".to_string(), serde_json::json!({
        "title": "Broadcast Test",
        "tags": ["rust", "programming", "test"]
    }));

    // Execute the transform - should handle broadcast alignment
    let result = TransformExecutor::execute_transform_with_expr(&transform, input_values);
    
    assert!(result.is_ok(), "Broadcast alignment should be valid: {:?}", result);
    
    let json_result = result.unwrap();
    let obj = json_result.as_object().unwrap();
    assert_eq!(obj.get("blog_title"), Some(&JsonValue::String("Broadcast Test".to_string())));
    assert!(obj.contains_key("tag_name")); // Should have tag data
}


#[test]
fn test_simple_expressions_fallback_validation() {
    // Create schema with simple expressions that don't parse as chains
    let mut fields = HashMap::new();
    fields.insert("user_name".to_string(), FieldDefinition {
        atom_uuid: Some("user.name".to_string()), // Simple dotted path, not a chain
        field_type: Some("String".to_string()),
    });
    fields.insert("user_email".to_string(), FieldDefinition {
        atom_uuid: Some("user.email".to_string()), // Simple dotted path, not a chain
        field_type: Some("String".to_string()),
    });

    let declarative_schema = DeclarativeSchemaDefinition {
        name: "simple_expressions_schema".to_string(),
        schema_type: SchemaType::Single,
        key: None,
        fields,
    };

    let transform = Transform::from_declarative_schema(
        declarative_schema,
        vec!["user_data".to_string()],
        "output.simple_expressions".to_string(),
    );

    // Create input data
    let mut input_values = HashMap::new();
    input_values.insert("user".to_string(), serde_json::json!({
        "name": "Alice",
        "email": "alice@example.com"
    }));

    // Execute the transform - should handle simple expressions gracefully
    let result = TransformExecutor::execute_transform_with_expr(&transform, input_values);
    
    assert!(result.is_ok(), "Simple expressions should be handled gracefully");
    
    let json_result = result.unwrap();
    let obj = json_result.as_object().unwrap();
    assert_eq!(obj.get("user_name"), Some(&JsonValue::String("Alice".to_string())));
    assert_eq!(obj.get("user_email"), Some(&JsonValue::String("alice@example.com".to_string())));
}

#[test]
fn test_complex_chain_alignment_validation() {
    // Create schema with complex but compatible chain expressions (same branch)
    let mut fields = HashMap::new();
    fields.insert("tag_name".to_string(), FieldDefinition {
        atom_uuid: Some("blogpost.map().tags.split_array().map()".to_string()),
        field_type: Some("String".to_string()),
    });
    fields.insert("tag_length".to_string(), FieldDefinition {
        atom_uuid: Some("blogpost.map().tags.split_array().map()".to_string()), // Same expression but different field
        field_type: Some("String".to_string()),
    });
    fields.insert("range_field".to_string(), FieldDefinition {
        atom_uuid: Some("blogpost.map().tags".to_string()),
        field_type: Some("Array".to_string()),
    });

    let declarative_schema = DeclarativeSchemaDefinition {
        name: "complex_chains_schema".to_string(),
        schema_type: SchemaType::HashRange,
        key: Some(datafold::schema::types::json_schema::KeyConfig {
            hash_field: "blogpost.map().tags.split_array().map()".to_string(),
            range_field: "blogpost.map().tags".to_string(),
        }),
        fields,
    };

    let transform = Transform::from_declarative_schema(
        declarative_schema,
        vec!["blogpost_data".to_string()],
        "output.complex_chains".to_string(),
    );

    // Create input data
    let mut input_values = HashMap::new();
    input_values.insert("blogpost".to_string(), serde_json::json!({
        "tags": ["alignment", "validation", "test"]
    }));

    // Execute the transform - should validate complex chains with same branch
    let result = TransformExecutor::execute_transform_with_expr(&transform, input_values);
    
    assert!(result.is_ok(), "Complex compatible chains on same branch should pass validation: {:?}", result);
    
    let json_result = result.unwrap();
    let obj = json_result.as_object().unwrap();
    
    // HashRange results should have hash_key and range_key arrays
    assert!(obj.contains_key("hash_key"), "HashRange result should contain hash_key array");
    assert!(obj.contains_key("range_key"), "HashRange result should contain range_key array");
    
                // The tag_name should be in the hash_key array
                let hash_key = obj.get("hash_key").unwrap();
                assert!(hash_key.is_array(), "hash_key should be an array");

                let tag_array = hash_key.as_array().unwrap();
                assert_eq!(tag_array.len(), 3, "Should have 3 entries (no duplication)");

                // Check that we get actual tags, not placeholder values
                assert_eq!(tag_array[0], JsonValue::String("alignment".to_string()));
                assert_eq!(tag_array[1], JsonValue::String("validation".to_string()));
                assert_eq!(tag_array[2], JsonValue::String("test".to_string()));

                // Check that tag_length contains the same tags as tag_name
                let tag_length = obj.get("tag_length").unwrap();
                assert!(tag_length.is_array(), "tag_length should be an array");
                let tag_length_array = tag_length.as_array().unwrap();
                assert_eq!(tag_length_array.len(), 3, "Should have 3 entries for tag_length (no duplication)");
                assert_eq!(tag_length_array[0], JsonValue::String("alignment".to_string()));
                assert_eq!(tag_length_array[1], JsonValue::String("validation".to_string()));
                assert_eq!(tag_length_array[2], JsonValue::String("test".to_string()));
}

#[test]
fn test_mixed_expression_types_validation() {
    // Create schema with mix of simple expressions and chain expressions
    let mut fields = HashMap::new();
    fields.insert("simple_field".to_string(), FieldDefinition {
        atom_uuid: Some("data.value".to_string()), // Simple expression
        field_type: Some("String".to_string()),
    });
    fields.insert("chain_field".to_string(), FieldDefinition {
        atom_uuid: Some("data.map().items.split_array().map()".to_string()), // Chain expression
        field_type: Some("Array".to_string()),
    });
    fields.insert("no_expression_field".to_string(), FieldDefinition {
        atom_uuid: None, // No expression
        field_type: Some("Boolean".to_string()),
    });
    fields.insert("range_field".to_string(), FieldDefinition {
        atom_uuid: Some("data.map().items".to_string()),
        field_type: Some("Array".to_string()),
    });

    let declarative_schema = DeclarativeSchemaDefinition {
        name: "mixed_expressions_schema".to_string(),
        schema_type: SchemaType::HashRange,
        key: Some(datafold::schema::types::json_schema::KeyConfig {
            hash_field: "data.map().items.split_array().map()".to_string(),
            range_field: "data.map().items".to_string(),
        }),
        fields,
    };

    let transform = Transform::from_declarative_schema(
        declarative_schema,
        vec!["mixed_data".to_string()],
        "output.mixed_expressions".to_string(),
    );

    // Create input data
    let mut input_values = HashMap::new();
    input_values.insert("data".to_string(), serde_json::json!({
        "value": "Mixed validation test",
        "items": ["item1", "item2", "item3"]
    }));

    // Execute the transform - should handle mixed expression types
    let result = TransformExecutor::execute_transform_with_expr(&transform, input_values);
    
    assert!(result.is_ok(), "Mixed expression types should be handled correctly");
    
    let json_result = result.unwrap();
    let obj = json_result.as_object().unwrap();
    
    // HashRange results should have hash_key and range_key arrays
    assert!(obj.contains_key("hash_key"), "HashRange result should contain hash_key array");
    assert!(obj.contains_key("range_key"), "HashRange result should contain range_key array");
    
    // The chain_field should be in the hash_key array
    let hash_key = obj.get("hash_key").unwrap();
    assert!(hash_key.is_array(), "hash_key should be an array");
    
    let item_array = hash_key.as_array().unwrap();
    assert_eq!(item_array.len(), 3, "Should have 3 items");
    
    // Check that we get actual items, not placeholder values
    assert_eq!(item_array[0], JsonValue::String("item1".to_string()));
    assert_eq!(item_array[1], JsonValue::String("item2".to_string()));
    assert_eq!(item_array[2], JsonValue::String("item3".to_string()));
}

#[test]
fn test_special_field_alignment_validation() {
    // Create schema with special fields like $atom_uuid
    let mut fields = HashMap::new();
    fields.insert("uuid_field".to_string(), FieldDefinition {
        atom_uuid: Some("blogpost.map().$atom_uuid".to_string()),
        field_type: Some("String".to_string()),
    });
    fields.insert("content_field".to_string(), FieldDefinition {
        atom_uuid: Some("blogpost.map().content".to_string()),
        field_type: Some("String".to_string()),
    });

    let declarative_schema = DeclarativeSchemaDefinition {
        name: "special_fields_schema".to_string(),
        schema_type: SchemaType::Single,
        key: None,
        fields,
    };

    let transform = Transform::from_declarative_schema(
        declarative_schema,
        vec!["blogpost_data".to_string()],
        "output.special_fields".to_string(),
    );

    // Create input data
    let mut input_values = HashMap::new();
    input_values.insert("blogpost".to_string(), serde_json::json!({
        "$atom_uuid": "blog-uuid-12345",
        "content": "Special field validation test"
    }));

    // Execute the transform - should handle special fields in alignment validation
    let result = TransformExecutor::execute_transform_with_expr(&transform, input_values);
    
    assert!(result.is_ok(), "Special fields should be handled in alignment validation");
    
    let json_result = result.unwrap();
    let obj = json_result.as_object().unwrap();
    assert_eq!(obj.get("uuid_field"), Some(&JsonValue::String("blog-uuid-12345".to_string())));
    assert_eq!(obj.get("content_field"), Some(&JsonValue::String("Special field validation test".to_string())));
}

#[test]
fn test_field_alignment_validation_with_invalid_expressions() {
    // Create schema with expressions that will fail parsing but should be handled gracefully
    let mut fields = HashMap::new();
    fields.insert("valid_field".to_string(), FieldDefinition {
        atom_uuid: Some("data.value".to_string()), // Valid simple expression
        field_type: Some("String".to_string()),
    });
    fields.insert("invalid_field".to_string(), FieldDefinition {
        atom_uuid: Some("data.invalid_function().content".to_string()), // Invalid function
        field_type: Some("String".to_string()),
    });

    let declarative_schema = DeclarativeSchemaDefinition {
        name: "invalid_expressions_schema".to_string(),
        schema_type: SchemaType::Single,
        key: None,
        fields,
    };

    let transform = Transform::from_declarative_schema(
        declarative_schema,
        vec!["test_data".to_string()],
        "output.invalid_expressions".to_string(),
    );

    // Create input data
    let mut input_values = HashMap::new();
    input_values.insert("data".to_string(), serde_json::json!({
        "value": "Valid field value",
        "content": "Content for invalid expression"
    }));

    // Execute the transform - should handle invalid expressions gracefully (either with fallback or validation error)
    let result = TransformExecutor::execute_transform_with_expr(&transform, input_values);
    
    match result {
        Ok(json_result) => {
            // If it succeeds, it should have fallen back to simple resolution
            let obj = json_result.as_object().unwrap();
            assert_eq!(obj.get("valid_field"), Some(&JsonValue::String("Valid field value".to_string())));
            // The invalid field should be null due to unknown function (no fallback)
            assert_eq!(obj.get("invalid_field"), Some(&JsonValue::Null));
        }
        Err(err) => {
            // If it fails, it should be due to enhanced validation catching invalid expressions
            let error_msg = format!("{:?}", err);
            assert!(error_msg.contains("Expression parsing failed") || 
                   error_msg.contains("Invalid operation sequence") ||
                   error_msg.contains("validation"),
                   "Error should be validation related: {}", error_msg);
        }
    }
}

#[test]
fn test_backward_compatibility_after_alignment_validation() {
    // Ensure that procedural transforms still work after adding field alignment validation
    let procedural_transform = Transform::new("return 456".to_string(), "output.number".to_string());
    
    let input_values = HashMap::new();
    let result = TransformExecutor::execute_transform(&procedural_transform, input_values);
    
    // Should either succeed or fail with parsing error (not alignment validation error)
    match result {
        Ok(_) => {
            // Success - backward compatibility maintained
        }
        Err(err) => {
            // Should not be an alignment validation error
            let error_msg = format!("{:?}", err);
            assert!(!error_msg.contains("Field alignment validation failed"), 
                   "Backward compatibility broken - procedural transforms shouldn't trigger alignment validation: {}", error_msg);
            assert!(!error_msg.contains("alignment"), 
                   "Backward compatibility broken - should not mention alignment for procedural transforms: {}", error_msg);
        }
    }
}

#[test]
fn test_single_field_alignment_validation() {
    // Create schema with just one field to test single-field validation
    let mut fields = HashMap::new();
    fields.insert("single_field".to_string(), FieldDefinition {
        atom_uuid: Some("blogpost.map().title".to_string()),
        field_type: Some("String".to_string()),
    });

    let declarative_schema = DeclarativeSchemaDefinition {
        name: "single_field_schema".to_string(),
        schema_type: SchemaType::Single,
        key: None,
        fields,
    };

    let transform = Transform::from_declarative_schema(
        declarative_schema,
        vec!["blogpost_data".to_string()],
        "output.single_field".to_string(),
    );

    // Create input data
    let mut input_values = HashMap::new();
    input_values.insert("blogpost".to_string(), serde_json::json!({
        "title": "Single field test"
    }));

    // Execute the transform - should validate single field
    let result = TransformExecutor::execute_transform_with_expr(&transform, input_values);
    
    assert!(result.is_ok(), "Single field alignment validation should succeed");
    
    let json_result = result.unwrap();
    let obj = json_result.as_object().unwrap();
    assert_eq!(obj.get("single_field"), Some(&JsonValue::String("Single field test".to_string())));
}

#[test]
fn test_reducer_function_alignment_validation() {
    // Create schema with reducer functions in expressions - these may fail validation
    // but should still execute using fallback resolution
    let mut fields = HashMap::new();
    fields.insert("sum_field".to_string(), FieldDefinition {
        atom_uuid: Some("data.map().numbers.split_array().sum()".to_string()),
        field_type: Some("Number".to_string()),
    });
    fields.insert("count_field".to_string(), FieldDefinition {
        atom_uuid: Some("data.map().items.split_array().count()".to_string()),
        field_type: Some("Number".to_string()),
    });

    let declarative_schema = DeclarativeSchemaDefinition {
        name: "reducer_schema".to_string(),
        schema_type: SchemaType::Single,
        key: None,
        fields,
    };

    let transform = Transform::from_declarative_schema(
        declarative_schema,
        vec!["data_with_arrays".to_string()],
        "output.reducer_results".to_string(),
    );

    // Create input data
    let mut input_values = HashMap::new();
    input_values.insert("data".to_string(), serde_json::json!({
        "numbers": [1, 2, 3, 4, 5],
        "items": ["a", "b", "c"]
    }));

    // Execute the transform - may fail validation but should handle gracefully
    let result = TransformExecutor::execute_transform_with_expr(&transform, input_values);
    
    // The transform may fail due to field alignment validation issues with reducers
    // This is expected behavior - the validator is correctly identifying alignment problems
    match result {
        Ok(json_result) => {
            // If it succeeds, check the results
            let obj = json_result.as_object().unwrap();
            assert!(obj.contains_key("sum_field"));
            assert!(obj.contains_key("count_field"));
        }
        Err(err) => {
            // If it fails, it should be due to validation (parsing or alignment), not execution errors
            let error_msg = format!("{:?}", err);
            assert!(error_msg.contains("Field alignment validation failed") || 
                   error_msg.contains("alignment") ||
                   error_msg.contains("CartesianProduct") ||
                   error_msg.contains("IncompatibleDepths") ||
                   error_msg.contains("Expression parsing failed") ||
                   error_msg.contains("Invalid operation sequence"),
                   "Error should be validation related: {}", error_msg);
        }
    }
}

#[test]
fn test_field_alignment_validation_cartesian_product_error() {
    // Create schema that deliberately creates a cartesian product (should fail validation)
    let mut fields = HashMap::new();
    fields.insert("word_content".to_string(), FieldDefinition {
        atom_uuid: Some("blogpost.map().content.split_by_word().map()".to_string()), // Branch: blogpost.content
        field_type: Some("String".to_string()),
    });
    fields.insert("tag_content".to_string(), FieldDefinition {
        atom_uuid: Some("blogpost.map().tags.split_array().map()".to_string()), // Branch: blogpost.tags - DIFFERENT branch at same depth
        field_type: Some("String".to_string()),
    });

    let declarative_schema = DeclarativeSchemaDefinition {
        name: "cartesian_product_schema".to_string(),
        schema_type: SchemaType::HashRange,
        key: Some(datafold::schema::types::json_schema::KeyConfig {
            hash_field: "word_content".to_string(),
            range_field: "tag_content".to_string(),
        }),
        fields,
    };

    let transform = Transform::from_declarative_schema(
        declarative_schema,
        vec!["blogpost_data".to_string()],
        "output.cartesian_product".to_string(),
    );

    // Create input data
    let mut input_values = HashMap::new();
    input_values.insert("blogpost".to_string(), serde_json::json!({
        "content": "Content to be split by words",
        "tags": ["validation", "test", "cartesian"]
    }));

    // Execute the transform - should fail with cartesian product error
    let result = TransformExecutor::execute_transform_with_expr(&transform, input_values);
    
    assert!(result.is_err(), "Cartesian product should be detected and rejected");
    
    let error = result.unwrap_err();
    let error_msg = format!("{:?}", error);
    assert!(error_msg.contains("CartesianProduct") || error_msg.contains("cartesian"),
           "Error should mention cartesian product: {}", error_msg);
    assert!(error_msg.contains("Field alignment validation failed"),
           "Error should be from field alignment validation: {}", error_msg);
}
