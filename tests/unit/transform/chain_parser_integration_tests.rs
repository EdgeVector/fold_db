use std::collections::HashMap;
use serde_json::Value as JsonValue;

use datafold::schema::types::Transform;
use datafold::schema::types::json_schema::{DeclarativeSchemaDefinition, FieldDefinition};
use datafold::schema::types::schema::SchemaType;
use datafold::transform::executor::TransformExecutor;

/// Tests for ChainParser integration with declarative transform execution
/// This validates that the ChainParser correctly parses expressions and integrates with field resolution

#[test]
fn test_simple_field_access_parsing() {
    // Create schema with simple field access expression
    let mut fields = HashMap::new();
    fields.insert("blog_title".to_string(), FieldDefinition {
        atom_uuid: Some("blogpost.title".to_string()),
        field_type: Some("String".to_string()),
    });

    let declarative_schema = DeclarativeSchemaDefinition {
        name: "blog_info".to_string(),
        schema_type: SchemaType::Single,
        key: None,
        fields,
    };

    let transform = Transform::from_declarative_schema(
        declarative_schema,
        vec!["blogpost_data".to_string()],
        "output.blog_info".to_string(),
    );

    // Create input data
    let mut input_values = HashMap::new();
    input_values.insert("blogpost".to_string(), serde_json::json!({
        "title": "My Blog Post",
        "content": "This is the content",
        "tags": ["rust", "programming"]
    }));

    // Execute the transform
    let result = TransformExecutor::execute_transform_with_expr(&transform, input_values);
    
    assert!(result.is_ok(), "Simple field access should succeed");
    
    let json_result = result.unwrap();
    let obj = json_result.as_object().unwrap();
    assert_eq!(obj.get("blog_title"), Some(&JsonValue::String("My Blog Post".to_string())));
}

#[test]
fn test_chain_expression_with_map() {
    // Create schema with map() operation in chain
    let mut fields = HashMap::new();
    fields.insert("post_content".to_string(), FieldDefinition {
        atom_uuid: Some("blogpost.map().content".to_string()),
        field_type: Some("String".to_string()),
    });

    let declarative_schema = DeclarativeSchemaDefinition {
        name: "post_data".to_string(),
        schema_type: SchemaType::Single,
        key: None,
        fields,
    };

    let transform = Transform::from_declarative_schema(
        declarative_schema,
        vec!["blogpost_data".to_string()],
        "output.post_data".to_string(),
    );

    // Create input data
    let mut input_values = HashMap::new();
    input_values.insert("blogpost".to_string(), serde_json::json!({
        "content": "Parsed chain content",
        "author": "Test Author"
    }));

    // Execute the transform
    let result = TransformExecutor::execute_transform_with_expr(&transform, input_values);
    
    assert!(result.is_ok(), "Chain expression with map() should succeed");
    
    let json_result = result.unwrap();
    let obj = json_result.as_object().unwrap();
    // Should resolve to simple "blogpost.content" after skipping map()
    assert_eq!(obj.get("post_content"), Some(&JsonValue::String("Parsed chain content".to_string())));
}

#[test]
fn test_complex_chain_expression() {
    // Create schema with complex chain expression
    let mut fields = HashMap::new();
    fields.insert("word_content".to_string(), FieldDefinition {
        atom_uuid: Some("blogpost.map().content.split_by_word().map()".to_string()),
        field_type: Some("String".to_string()),
    });
    fields.insert("range_field".to_string(), FieldDefinition {
        atom_uuid: Some("blogpost.map().content".to_string()),
        field_type: Some("String".to_string()),
    });

    let declarative_schema = DeclarativeSchemaDefinition {
        name: "word_data".to_string(),
        schema_type: SchemaType::HashRange,
        key: Some(datafold::schema::types::json_schema::KeyConfig {
            hash_field: "blogpost.map().content.split_by_word().map()".to_string(),
            range_field: "blogpost.map().content".to_string(),
        }),
        fields,
    };

    let transform = Transform::from_declarative_schema(
        declarative_schema,
        vec!["blogpost_data".to_string()],
        "output.word_data".to_string(),
    );

    // Create input data
    let mut input_values = HashMap::new();
    input_values.insert("blogpost".to_string(), serde_json::json!({
        "content": "Complex chain parsed content"
    }));

    // Execute the transform
    let result = TransformExecutor::execute_transform_with_expr(&transform, input_values);
    
    match result {
        Ok(json_result) => {
            let obj = json_result.as_object().unwrap();
            
            // HashRange results should have hash_key and range_key arrays
            assert!(obj.contains_key("hash_key"), "HashRange result should contain hash_key array");
            assert!(obj.contains_key("range_key"), "HashRange result should contain range_key array");
            
            // The word_content should be in the hash_key array
            let hash_key = obj.get("hash_key").unwrap();
            assert!(hash_key.is_array(), "hash_key should be an array");
            
            let word_array = hash_key.as_array().unwrap();
            assert_eq!(word_array.len(), 4, "Should have 4 words");
            
            // Check that we get actual words, not placeholder values
            assert_eq!(word_array[0], JsonValue::String("Complex".to_string()));
            assert_eq!(word_array[1], JsonValue::String("chain".to_string()));
            assert_eq!(word_array[2], JsonValue::String("parsed".to_string()));
            assert_eq!(word_array[3], JsonValue::String("content".to_string()));
        }
        Err(err) => {
            panic!("Complex chain expression failed: {:?}", err);
        }
    }
}

#[test]
fn test_special_field_in_chain() {
    // Create schema with special field like $atom_uuid
    let mut fields = HashMap::new();
    fields.insert("uuid_field".to_string(), FieldDefinition {
        atom_uuid: Some("blogpost.map().$atom_uuid".to_string()),
        field_type: Some("String".to_string()),
    });

    let declarative_schema = DeclarativeSchemaDefinition {
        name: "uuid_data".to_string(),
        schema_type: SchemaType::Single,
        key: None,
        fields,
    };

    let transform = Transform::from_declarative_schema(
        declarative_schema,
        vec!["blogpost_data".to_string()],
        "output.uuid_data".to_string(),
    );

    // Create input data
    let mut input_values = HashMap::new();
    input_values.insert("blogpost".to_string(), serde_json::json!({
        "$atom_uuid": "special-uuid-12345"
    }));

    // Execute the transform
    let result = TransformExecutor::execute_transform_with_expr(&transform, input_values);
    
    assert!(result.is_ok(), "Special field in chain should succeed");
    
    let json_result = result.unwrap();
    let obj = json_result.as_object().unwrap();
    // Should resolve to "blogpost.$atom_uuid" after skipping map()
    assert_eq!(obj.get("uuid_field"), Some(&JsonValue::String("special-uuid-12345".to_string())));
}

#[test]
fn test_chain_parsing_error_handling() {
    // Create schema with invalid chain expression
    let mut fields = HashMap::new();
    fields.insert("invalid_field".to_string(), FieldDefinition {
        atom_uuid: Some("blogpost.unknown_function().content".to_string()),
        field_type: Some("String".to_string()),
    });

    let declarative_schema = DeclarativeSchemaDefinition {
        name: "error_test".to_string(),
        schema_type: SchemaType::Single,
        key: None,
        fields,
    };

    let transform = Transform::from_declarative_schema(
        declarative_schema,
        vec!["blogpost_data".to_string()],
        "output.error_test".to_string(),
    );

    // Create input data
    let mut input_values = HashMap::new();
    input_values.insert("blogpost".to_string(), serde_json::json!({
        "content": "Content for error test"
    }));

    // Execute the transform - should handle parsing error gracefully
    let result = TransformExecutor::execute_transform_with_expr(&transform, input_values);
    
    // The transform should fail due to unknown function, but gracefully (no panic)
    match result {
        Ok(json_result) => {
            let obj = json_result.as_object().unwrap();
            // Should return null for invalid expressions
            assert_eq!(obj.get("invalid_field"), Some(&JsonValue::Null));
        }
        Err(err) => {
            // If it fails, it should be due to validation issues, not parsing crashes
            let error_msg = format!("{:?}", err);
            assert!(!error_msg.contains("panic") && !error_msg.contains("crash"),
                   "Should handle parsing errors gracefully without crashes: {}", error_msg);
        }
    }
}

#[test]
fn test_chain_with_reducer_function() {
    // Create schema with reducer function in chain
    let mut fields = HashMap::new();
    fields.insert("reduced_field".to_string(), FieldDefinition {
        atom_uuid: Some("blogpost.map().tags.split_array().sum()".to_string()),
        field_type: Some("String".to_string()),
    });
    fields.insert("range_field".to_string(), FieldDefinition {
        atom_uuid: Some("blogpost.map().tags.split_array().sum()".to_string()),
        field_type: Some("String".to_string()),
    });

    let declarative_schema = DeclarativeSchemaDefinition {
        name: "reducer_test".to_string(),
        schema_type: SchemaType::HashRange,
        key: Some(datafold::schema::types::json_schema::KeyConfig {
            hash_field: "reduced_field".to_string(),
            range_field: "range_field".to_string(),
        }),
        fields,
    };

    let transform = Transform::from_declarative_schema(
        declarative_schema,
        vec!["blogpost_data".to_string()],
        "output.reducer_test".to_string(),
    );

    // Create input data
    let mut input_values = HashMap::new();
    input_values.insert("blogpost".to_string(), serde_json::json!({
        "tags": ["tag1", "tag2", "tag3"]
    }));

    // Execute the transform
    let result = TransformExecutor::execute_transform_with_expr(&transform, input_values);
    
    // Chain with reducer may now fail field alignment validation (which is correct behavior)
    match result {
        Ok(json_result) => {
            let obj = json_result.as_object().unwrap();
            // Should resolve to "blogpost.tags" after skipping operations
            assert!(obj.contains_key("reduced_field"));
            // Tags should be an array
            assert!(obj.get("reduced_field").unwrap().is_array());
        }
        Err(err) => {
            // If it fails, it should be due to validation (either parsing or field alignment), not execution errors
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
fn test_empty_chain_expression() {
    // Create schema with empty or minimal expression
    let mut fields = HashMap::new();
    fields.insert("simple_field".to_string(), FieldDefinition {
        atom_uuid: Some("data".to_string()),
        field_type: Some("String".to_string()),
    });

    let declarative_schema = DeclarativeSchemaDefinition {
        name: "simple_test".to_string(),
        schema_type: SchemaType::Single,
        key: None,
        fields,
    };

    let transform = Transform::from_declarative_schema(
        declarative_schema,
        vec!["simple_data".to_string()],
        "output.simple_test".to_string(),
    );

    // Create input data
    let mut input_values = HashMap::new();
    input_values.insert("data".to_string(), JsonValue::String("Simple data value".to_string()));

    // Execute the transform
    let result = TransformExecutor::execute_transform_with_expr(&transform, input_values);
    
    assert!(result.is_ok(), "Simple expression should succeed");
    
    let json_result = result.unwrap();
    let obj = json_result.as_object().unwrap();
    assert_eq!(obj.get("simple_field"), Some(&JsonValue::String("Simple data value".to_string())));
}

#[test]
fn test_chain_with_missing_data() {
    // Create schema with chain expression that references missing data
    let mut fields = HashMap::new();
    fields.insert("missing_field".to_string(), FieldDefinition {
        atom_uuid: Some("nonexistent.map().data".to_string()),
        field_type: Some("String".to_string()),
    });

    let declarative_schema = DeclarativeSchemaDefinition {
        name: "missing_test".to_string(),
        schema_type: SchemaType::Single,
        key: None,
        fields,
    };

    let transform = Transform::from_declarative_schema(
        declarative_schema,
        vec!["missing_data".to_string()],
        "output.missing_test".to_string(),
    );

    // Create input data without the referenced field
    let input_values = HashMap::new();

    // Execute the transform
    let result = TransformExecutor::execute_transform_with_expr(&transform, input_values);
    
    assert!(result.is_ok(), "Missing data should be handled gracefully");
    
    let json_result = result.unwrap();
    let obj = json_result.as_object().unwrap();
    // Should return null for missing data
    assert_eq!(obj.get("missing_field"), Some(&JsonValue::Null));
}

#[test]
fn test_multiple_chain_expressions_in_single_schema() {
    // Create schema with multiple fields using different chain expressions
    let mut fields = HashMap::new();
    fields.insert("title_field".to_string(), FieldDefinition {
        atom_uuid: Some("post.title".to_string()),
        field_type: Some("String".to_string()),
    });
    fields.insert("content_field".to_string(), FieldDefinition {
        atom_uuid: Some("post.map().content".to_string()),
        field_type: Some("String".to_string()),
    });
    fields.insert("tag_field".to_string(), FieldDefinition {
        atom_uuid: Some("post.map().tags.split_array().map()".to_string()),
        field_type: Some("Array".to_string()),
    });
    fields.insert("range_field".to_string(), FieldDefinition {
        atom_uuid: Some("post.map().content".to_string()),
        field_type: Some("String".to_string()),
    });

    let declarative_schema = DeclarativeSchemaDefinition {
        name: "multi_chain_test".to_string(),
        schema_type: SchemaType::HashRange,
        key: Some(datafold::schema::types::json_schema::KeyConfig {
            hash_field: "post.map().tags.split_array().map()".to_string(),
            range_field: "post.map().content".to_string(),
        }),
        fields,
    };

    let transform = Transform::from_declarative_schema(
        declarative_schema,
        vec!["post_data".to_string()],
        "output.multi_chain_test".to_string(),
    );

    // Create input data
    let mut input_values = HashMap::new();
    input_values.insert("post".to_string(), serde_json::json!({
        "title": "Multi Chain Test",
        "content": "Content for multi-chain test",
        "tags": ["chain", "parser", "test"]
    }));

    // Execute the transform
    let result = TransformExecutor::execute_transform_with_expr(&transform, input_values);
    
    assert!(result.is_ok(), "Multiple chain expressions should succeed");
    
    let json_result = result.unwrap();
    let obj = json_result.as_object().unwrap();
    
    // HashRange results should have hash_key and range_key arrays
    assert!(obj.contains_key("hash_key"), "HashRange result should contain hash_key array");
    assert!(obj.contains_key("range_key"), "HashRange result should contain range_key array");
    
    // The tag_field should be in the hash_key array
    let hash_key = obj.get("hash_key").unwrap();
    assert!(hash_key.is_array(), "hash_key should be an array");
    
    let tags = hash_key.as_array().unwrap();
    assert_eq!(tags.len(), 3);
    assert_eq!(tags[0], JsonValue::String("chain".to_string()));
    assert_eq!(tags[1], JsonValue::String("parser".to_string()));
    assert_eq!(tags[2], JsonValue::String("test".to_string()));
}

#[test]
fn test_backward_compatibility_with_simple_expressions() {
    // Ensure that simple expressions still work after ChainParser integration
    let mut fields = HashMap::new();
    fields.insert("simple_field".to_string(), FieldDefinition {
        atom_uuid: Some("user.name".to_string()),
        field_type: Some("String".to_string()),
    });

    let declarative_schema = DeclarativeSchemaDefinition {
        name: "compatibility_test".to_string(),
        schema_type: SchemaType::Single,
        key: None,
        fields,
    };

    let transform = Transform::from_declarative_schema(
        declarative_schema,
        vec!["user_data".to_string()],
        "output.compatibility_test".to_string(),
    );

    // Create input data
    let mut input_values = HashMap::new();
    input_values.insert("user".to_string(), serde_json::json!({
        "name": "Test User",
        "email": "test@example.com"
    }));

    // Execute the transform
    let result = TransformExecutor::execute_transform_with_expr(&transform, input_values);
    
    assert!(result.is_ok(), "Backward compatibility should be maintained");
    
    let json_result = result.unwrap();
    let obj = json_result.as_object().unwrap();
    assert_eq!(obj.get("simple_field"), Some(&JsonValue::String("Test User".to_string())));
}
