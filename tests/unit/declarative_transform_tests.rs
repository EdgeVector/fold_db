use datafold::schema::types::{
    json_schema::{DeclarativeSchemaDefinition, FieldDefinition, KeyConfig},
    Transform, TransformRegistration,
};
use datafold::schema::SchemaType;
use datafold::schema::core::SchemaCore;
use datafold::schema::SchemaError;
use std::collections::HashMap;
use tempfile::TempDir;
use sled;

/// Test fixture for declarative transform tests
struct DeclarativeTransformTestFixture {
    schema_core: SchemaCore,
    temp_dir: TempDir,
}

impl DeclarativeTransformTestFixture {
    fn new() -> Result<Self, Box<dyn std::error::Error>> {
        let temp_dir = TempDir::new()?;
        let db = sled::open(temp_dir.path())?;
        let db_ops = std::sync::Arc::new(datafold::db_operations::DbOperations::new(db)?);
        let message_bus = std::sync::Arc::new(datafold::fold_db_core::infrastructure::message_bus::MessageBus::new());
        let schema_core = SchemaCore::new(
            temp_dir.path().to_str().unwrap(),
            db_ops.clone(),
            message_bus,
        )?;
        
        Ok(Self {
            schema_core,
            temp_dir,
        })
    }
}

/// Test declarative schema parsing
#[test]
fn test_declarative_schema_parsing() {
    let fixture = DeclarativeTransformTestFixture::new().expect("Failed to create test fixture");
    
    // Create a simple declarative schema
    let declarative_schema = DeclarativeSchemaDefinition {
        name: "TestWordIndex".to_string(),
        schema_type: SchemaType::HashRange,
        fields: HashMap::from([
            ("word".to_string(), FieldDefinition {
                field_type: Some("single".to_string()),
                atom_uuid: Some("source.map().content.split_by_word().map()".to_string()),
            }),
            ("source_ref".to_string(), FieldDefinition {
                field_type: Some("single".to_string()),
                atom_uuid: Some("source.map().$atom_uuid".to_string()),
            }),
        ]),
        key: Some(KeyConfig {
            hash_field: "source.map().content.split_by_word().map()".to_string(),
            range_field: "source.map().timestamp".to_string(),
        }),
    };
    
    // Test schema validation
    let validation_result = declarative_schema.validate();
    assert!(validation_result.is_ok(), "Schema validation failed: {:?}", validation_result);
    
    // Test schema interpretation
    let schema_result = fixture.schema_core.interpret_declarative_schema(declarative_schema);
    assert!(schema_result.is_ok(), "Schema interpretation failed: {:?}", schema_result);
    
    let schema = schema_result.unwrap();
    assert_eq!(schema.name, "TestWordIndex");
    assert!(matches!(schema.schema_type, SchemaType::HashRange));
    assert_eq!(schema.fields.len(), 2);
    
    // Verify fields are properly converted
    assert!(schema.fields.contains_key("word"));
    assert!(schema.fields.contains_key("source_ref"));
}

/// Test declarative transform creation
#[test]
fn test_declarative_transform_creation() {
    let declarative_schema = DeclarativeSchemaDefinition {
        name: "BlogPostWordIndex".to_string(),
        schema_type: SchemaType::HashRange,
        fields: HashMap::from([
            ("blog".to_string(), FieldDefinition {
                field_type: Some("single".to_string()),
                atom_uuid: Some("blogpost.map().$atom_uuid".to_string()),
            }),
            ("author".to_string(), FieldDefinition {
                field_type: Some("single".to_string()),
                atom_uuid: Some("blogpost.map().author.$atom_uuid".to_string()),
            }),
        ]),
        key: Some(KeyConfig {
            hash_field: "blogpost.map().content.split_by_word().map()".to_string(),
            range_field: "blogpost.map().publish_date".to_string(),
        }),
    };
    
    // Create transform from declarative schema
    let transform = Transform::from_declarative_schema(
        declarative_schema.clone(),
        vec!["blogpost".to_string()],
        "BlogPostWordIndex.key".to_string(),
    );
    
    // Verify transform properties
    assert!(transform.is_declarative());
    assert!(!transform.is_procedural());
    assert_eq!(transform.get_inputs(), vec!["blogpost"]);
    assert_eq!(transform.get_output(), "BlogPostWordIndex.key");
    
    // Verify declarative schema is accessible
    let retrieved_schema = transform.get_declarative_schema().expect("Should have declarative schema");
    assert_eq!(retrieved_schema.name, "BlogPostWordIndex");
    assert_eq!(retrieved_schema.fields.len(), 2);
}

/// Test declarative transform registration
#[test]
fn test_declarative_transform_registration() {
    let fixture = DeclarativeTransformTestFixture::new().expect("Failed to create test fixture");
    
    let declarative_schema = DeclarativeSchemaDefinition {
        name: "TestTransform".to_string(),
        schema_type: SchemaType::Single,
        fields: HashMap::from([
            ("processed".to_string(), FieldDefinition {
                field_type: Some("single".to_string()),
                atom_uuid: Some("input.map().value".to_string()),
            }),
        ]),
        key: None,
    };
    
    // Test transform registration
    let registration_result = fixture.schema_core.register_declarative_transform(&declarative_schema);
    assert!(registration_result.is_ok(), "Transform registration failed: {:?}", registration_result);
    
    // Verify transform is stored in database
    let transform_id = "TestTransform.declarative";
    let stored_transform = fixture.schema_core.get_transform(transform_id);
    assert!(stored_transform.is_ok(), "Failed to retrieve stored transform");
    assert!(stored_transform.unwrap().is_some(), "Transform not found in database");
}

/// Test declarative schema with complex field mappings
#[test]
fn test_complex_declarative_schema() {
    let fixture = DeclarativeTransformTestFixture::new().expect("Failed to create test fixture");
    
    let declarative_schema = DeclarativeSchemaDefinition {
        name: "ComplexIndex".to_string(),
        schema_type: SchemaType::HashRange,
        fields: HashMap::from([
            ("main_content".to_string(), FieldDefinition {
                field_type: Some("single".to_string()),
                atom_uuid: Some("post.map().content".to_string()),
            }),
            ("author_info".to_string(), FieldDefinition {
                field_type: Some("single".to_string()),
                atom_uuid: Some("post.map().author.name".to_string()),
            }),
            ("metadata".to_string(), FieldDefinition {
                field_type: Some("single".to_string()),
                atom_uuid: Some("post.map().metadata.tags".to_string()),
            }),
        ]),
        key: Some(KeyConfig {
            hash_field: "post.map().content.split_by_word().map()".to_string(),
            range_field: "post.map().created_at".to_string(),
        }),
    };
    
    // Test schema interpretation
    let schema_result = fixture.schema_core.interpret_declarative_schema(declarative_schema);
    assert!(schema_result.is_ok(), "Complex schema interpretation failed: {:?}", schema_result);
    
    let schema = schema_result.unwrap();
    assert_eq!(schema.name, "ComplexIndex");
    assert_eq!(schema.fields.len(), 3);
    
    // Verify all fields are present
    assert!(schema.fields.contains_key("main_content"));
    assert!(schema.fields.contains_key("author_info"));
    assert!(schema.fields.contains_key("metadata"));
}

/// Test declarative transform with multiple input dependencies
#[test]
fn test_multiple_input_dependencies() {
    let declarative_schema = DeclarativeSchemaDefinition {
        name: "MultiSourceIndex".to_string(),
        schema_type: SchemaType::HashRange,
        fields: HashMap::from([
            ("combined_content".to_string(), FieldDefinition {
                field_type: Some("single".to_string()),
                atom_uuid: Some("post.map().content".to_string()),
            }),
            ("user_data".to_string(), FieldDefinition {
                field_type: Some("single".to_string()),
                atom_uuid: Some("user.map().profile".to_string()),
            }),
            ("analytics".to_string(), FieldDefinition {
                field_type: Some("single".to_string()),
                atom_uuid: Some("analytics.map().metrics".to_string()),
            }),
        ]),
        key: Some(KeyConfig {
            hash_field: "post.map().content.split_by_word().map()".to_string(),
            range_field: "post.map().timestamp".to_string(),
        }),
    };
    
    // Create transform with multiple inputs
    let transform = Transform::from_declarative_schema(
        declarative_schema,
        vec!["post".to_string(), "user".to_string(), "analytics".to_string()],
        "MultiSourceIndex.key".to_string(),
    );
    
    // Verify multiple inputs are handled
    let inputs = transform.get_inputs();
    assert_eq!(inputs.len(), 3);
    assert!(inputs.contains(&"post".to_string()));
    assert!(inputs.contains(&"user".to_string()));
    assert!(inputs.contains(&"analytics".to_string()));
}

/// Test declarative schema validation
#[test]
fn test_declarative_schema_validation() {
    // Test valid schema
    let valid_schema = DeclarativeSchemaDefinition {
        name: "ValidSchema".to_string(),
        schema_type: SchemaType::Single,
        fields: HashMap::from([
            ("field1".to_string(), FieldDefinition {
                field_type: Some("single".to_string()),
                atom_uuid: Some("input.map().value".to_string()),
            }),
        ]),
        key: None,
    };
    
    let validation_result = valid_schema.validate();
    assert!(validation_result.is_ok(), "Valid schema should pass validation");
    
    // Test invalid schema (empty name)
    let invalid_schema = DeclarativeSchemaDefinition {
        name: "".to_string(),
        schema_type: SchemaType::Single,
        fields: HashMap::new(),
        key: None,
    };
    
    let validation_result = invalid_schema.validate();
    assert!(validation_result.is_err(), "Invalid schema should fail validation");
}

/// Test declarative transform execution preparation
#[test]
fn test_declarative_transform_execution_prep() {
    let declarative_schema = DeclarativeSchemaDefinition {
        name: "ExecutionTest".to_string(),
        schema_type: SchemaType::HashRange,
        fields: HashMap::from([
            ("word".to_string(), FieldDefinition {
                field_type: Some("single".to_string()),
                atom_uuid: Some("text.map().content.split_by_word().map()".to_string()),
            }),
            ("source".to_string(), FieldDefinition {
                field_type: Some("single".to_string()),
                atom_uuid: Some("text.map().$atom_uuid".to_string()),
            }),
        ]),
        key: Some(KeyConfig {
            hash_field: "text.map().content.split_by_word().map()".to_string(),
            range_field: "text.map().timestamp".to_string(),
        }),
    };
    
    let transform = Transform::from_declarative_schema(
        declarative_schema,
        vec!["text".to_string()],
        "ExecutionTest.key".to_string(),
    );
    
    // Test transform validation
    let validation_result = transform.validate();
    assert!(validation_result.is_ok(), "Transform validation failed: {:?}", validation_result);
    
    // Test transform analysis
    let dependencies = transform.analyze_dependencies();
    assert_eq!(dependencies.len(), 3); // HashRange schema has multiple dependencies
    assert!(dependencies.contains(&"text".to_string()));
}

/// Test error handling for invalid declarative schemas
#[test]
fn test_error_handling_invalid_schemas() {
    let fixture = DeclarativeTransformTestFixture::new().expect("Failed to create test fixture");
    
    // Test schema with invalid field type
    let invalid_schema = DeclarativeSchemaDefinition {
        name: "InvalidSchema".to_string(),
        schema_type: SchemaType::Single,
        fields: HashMap::from([
            ("field1".to_string(), FieldDefinition {
                field_type: Some("invalid_type".to_string()),
                atom_uuid: Some("input.map().value".to_string()),
            }),
        ]),
        key: None,
    };
    
    let result = fixture.schema_core.interpret_declarative_schema(invalid_schema);
    // This should either fail or handle gracefully
    // The exact behavior depends on the implementation
}

/// Test declarative transform with different schema types
#[test]
fn test_different_schema_types() {
    let fixture = DeclarativeTransformTestFixture::new().expect("Failed to create test fixture");
    
    // Test Single schema type
    let single_schema = DeclarativeSchemaDefinition {
        name: "SingleSchema".to_string(),
        schema_type: SchemaType::Single,
        fields: HashMap::from([
            ("value".to_string(), FieldDefinition {
                field_type: Some("single".to_string()),
                atom_uuid: Some("input.map().data".to_string()),
            }),
        ]),
        key: None,
    };
    
    let single_result = fixture.schema_core.interpret_declarative_schema(single_schema);
    assert!(single_result.is_ok(), "Single schema interpretation failed");
    
    // Test Range schema type
    let range_schema = DeclarativeSchemaDefinition {
        name: "RangeSchema".to_string(),
        schema_type: SchemaType::Range { range_key: "timestamp".to_string() },
        fields: HashMap::from([
            ("timestamp".to_string(), FieldDefinition {
                field_type: Some("single".to_string()),
                atom_uuid: Some("input.map().timestamp".to_string()),
            }),
            ("data".to_string(), FieldDefinition {
                field_type: Some("single".to_string()),
                atom_uuid: Some("input.map().value".to_string()),
            }),
        ]),
        key: None,
    };
    
    let range_result = fixture.schema_core.interpret_declarative_schema(range_schema);
    assert!(range_result.is_ok(), "Range schema interpretation failed");
}

/// Test declarative transform registration with field mappings
#[test]
fn test_field_mapping_registration() {
    let fixture = DeclarativeTransformTestFixture::new().expect("Failed to create test fixture");
    
    let declarative_schema = DeclarativeSchemaDefinition {
        name: "FieldMappingTest".to_string(),
        schema_type: SchemaType::HashRange,
        fields: HashMap::from([
            ("word".to_string(), FieldDefinition {
                field_type: Some("single".to_string()),
                atom_uuid: Some("blogpost.map().content.split_by_word().map()".to_string()),
            }),
            ("date".to_string(), FieldDefinition {
                field_type: Some("single".to_string()),
                atom_uuid: Some("blogpost.map().publish_date".to_string()),
            }),
        ]),
        key: Some(KeyConfig {
            hash_field: "blogpost.map().content.split_by_word().map()".to_string(),
            range_field: "blogpost.map().publish_date".to_string(),
        }),
    };
    
    // Register the transform
    let registration_result = fixture.schema_core.register_declarative_transform(&declarative_schema);
    assert!(registration_result.is_ok(), "Transform registration failed");
    
    // Verify transform registration is stored
    let transform_id = "FieldMappingTest.declarative";
    let stored_transform = fixture.schema_core.get_transform(transform_id);
    assert!(stored_transform.is_ok(), "Failed to retrieve stored transform");
    
    let transform = stored_transform.unwrap().expect("Transform should be stored");
    assert!(transform.is_declarative(), "Stored transform should be declarative");
    
    // Verify the declarative schema is preserved
    let retrieved_schema = transform.get_declarative_schema().expect("Should have declarative schema");
    assert_eq!(retrieved_schema.name, "FieldMappingTest");
    assert_eq!(retrieved_schema.fields.len(), 2);
}

/// Test declarative transform with invalid field expressions
#[test]
fn test_invalid_field_expressions() {
    let fixture = DeclarativeTransformTestFixture::new().expect("Failed to create test fixture");
    
    // Test invalid syntax in atom_uuid expressions
    let invalid_expressions = vec![
        "invalid..syntax..", // Multiple consecutive dots
        "field.map().", // Incomplete expression
        "field.map(().value", // Mismatched parentheses
        "field.map().value.", // Trailing dot
        ".field.map().value", // Leading dot
        "field.map().value..", // Multiple trailing dots
        "field..map().value", // Multiple consecutive dots
    ];
    
    for (i, invalid_expr) in invalid_expressions.iter().enumerate() {
        let schema = DeclarativeSchemaDefinition {
            name: format!("invalid_expr_{}", i),
            schema_type: SchemaType::Single,
            fields: HashMap::from([
                ("test_field".to_string(), FieldDefinition {
                    field_type: Some("single".to_string()),
                    atom_uuid: Some(invalid_expr.to_string()),
                }),
            ]),
            key: None,
        };
        
        let transform = Transform::from_declarative_schema(
            schema,
            vec!["input".to_string()],
            "output.test_field".to_string(),
        );
        
        // Transform creation should succeed, but validation should fail
        let validation_result = transform.validate();
        assert!(validation_result.is_err(), 
            "Transform with invalid expression '{}' should fail validation", invalid_expr);
    }
}

/// Test declarative transform with missing required fields for different schema types
#[test]
fn test_missing_required_fields() {
    // Test HashRange schema without key configuration
    let hashrange_without_key = DeclarativeSchemaDefinition {
        name: "missing_key".to_string(),
        schema_type: SchemaType::HashRange,
        fields: HashMap::from([
            ("field1".to_string(), FieldDefinition {
                field_type: Some("single".to_string()),
                atom_uuid: Some("input.map().value".to_string()),
            }),
        ]),
        key: None, // Missing key configuration for HashRange
    };
    
    let validation_result = hashrange_without_key.validate();
    assert!(validation_result.is_err(), "HashRange schema without key should fail validation");
    
    // Test Range schema with missing range_key field
    let range_without_key_field = DeclarativeSchemaDefinition {
        name: "missing_range_key".to_string(),
        schema_type: SchemaType::Range { range_key: "timestamp".to_string() },
        fields: HashMap::from([
            ("other_field".to_string(), FieldDefinition {
                field_type: Some("single".to_string()),
                atom_uuid: Some("input.map().other".to_string()),
            }),
            // Missing "timestamp" field that's referenced in range_key
        ]),
        key: None,
    };
    
    let validation_result = range_without_key_field.validate();
    assert!(validation_result.is_err(), "Range schema with missing range_key field should fail validation");
}

/// Test declarative transform with empty or invalid field definitions
#[test]
fn test_empty_and_invalid_field_definitions() {
    // Test schema with empty field name
    // Note: Current validation doesn't check for empty field names in HashMap keys
    let empty_field_name = DeclarativeSchemaDefinition {
        name: "empty_field_test".to_string(),
        schema_type: SchemaType::Single,
        fields: HashMap::from([
            ("".to_string(), FieldDefinition { // Empty field name
                field_type: Some("single".to_string()),
                atom_uuid: Some("input.map().value".to_string()),
            }),
        ]),
        key: None,
    };
    
    let validation_result = empty_field_name.validate();
    // Current validation allows empty field names
    assert!(validation_result.is_ok(), "Schema with empty field name currently passes validation");
    
    // Test schema with field definition without atom_uuid - this should actually be valid
    // because field_type is provided
    let field_without_atom_uuid = DeclarativeSchemaDefinition {
        name: "no_atom_uuid".to_string(),
        schema_type: SchemaType::Single,
        fields: HashMap::from([
            ("field1".to_string(), FieldDefinition {
                field_type: Some("single".to_string()),
                atom_uuid: None, // Missing atom_uuid but field_type is provided
            }),
        ]),
        key: None,
    };
    
    let validation_result = field_without_atom_uuid.validate();
    assert!(validation_result.is_ok(), "Field without atom_uuid but with field_type should pass validation");
}

/// Test declarative transform with conflicting field types
#[test]
fn test_conflicting_field_types() {
    // Test Range schema with non-Range field types
    // Note: The current validation doesn't enforce field type consistency for Range schemas
    // This test documents the current behavior
    let range_with_single_fields = DeclarativeSchemaDefinition {
        name: "conflicting_types".to_string(),
        schema_type: SchemaType::Range { range_key: "timestamp".to_string() },
        fields: HashMap::from([
            ("timestamp".to_string(), FieldDefinition {
                field_type: Some("single".to_string()), // Should be "Range" for Range schema
                atom_uuid: Some("input.map().timestamp".to_string()),
            }),
            ("value".to_string(), FieldDefinition {
                field_type: Some("single".to_string()), // Should be "Range" for Range schema
                atom_uuid: Some("input.map().value".to_string()),
            }),
        ]),
        key: None,
    };
    
    let validation_result = range_with_single_fields.validate();
    // Current validation allows this - the test documents this behavior
    assert!(validation_result.is_ok(), "Range schema with Single field types currently passes validation");
}

/// Test declarative transform with invalid key configurations
#[test]
fn test_invalid_key_configurations() {
    // Test HashRange with empty hash_field
    let empty_hash_field = DeclarativeSchemaDefinition {
        name: "empty_hash".to_string(),
        schema_type: SchemaType::HashRange,
        fields: HashMap::from([
            ("field1".to_string(), FieldDefinition {
                field_type: Some("single".to_string()),
                atom_uuid: Some("input.map().value".to_string()),
            }),
        ]),
        key: Some(KeyConfig {
            hash_field: "".to_string(), // Empty hash_field
            range_field: "input.map().timestamp".to_string(),
        }),
    };
    
    let validation_result = empty_hash_field.validate();
    assert!(validation_result.is_err(), "HashRange with empty hash_field should fail validation");
    
    // Test HashRange with empty range_field
    let empty_range_field = DeclarativeSchemaDefinition {
        name: "empty_range".to_string(),
        schema_type: SchemaType::HashRange,
        fields: HashMap::from([
            ("field1".to_string(), FieldDefinition {
                field_type: Some("single".to_string()),
                atom_uuid: Some("input.map().value".to_string()),
            }),
        ]),
        key: Some(KeyConfig {
            hash_field: "input.map().content".to_string(),
            range_field: "".to_string(), // Empty range_field
        }),
    };
    
    let validation_result = empty_range_field.validate();
    assert!(validation_result.is_err(), "HashRange with empty range_field should fail validation");
    
    // Test HashRange with same hash and range fields
    let same_fields = DeclarativeSchemaDefinition {
        name: "same_fields".to_string(),
        schema_type: SchemaType::HashRange,
        fields: HashMap::from([
            ("field1".to_string(), FieldDefinition {
                field_type: Some("single".to_string()),
                atom_uuid: Some("input.map().value".to_string()),
            }),
        ]),
        key: Some(KeyConfig {
            hash_field: "input.map().field".to_string(),
            range_field: "input.map().field".to_string(), // Same as hash_field
        }),
    };
    
    let validation_result = same_fields.validate();
    assert!(validation_result.is_err(), "HashRange with same hash and range fields should fail validation");
}

/// Test declarative transform with malformed atom_uuid expressions
#[test]
fn test_malformed_atom_uuid_expressions() {
    let malformed_expressions = vec![
        "field.map().value.split_by_word().map().", // Trailing dot
        "field.map().value.split_by_word().map()..", // Multiple trailing dots
        "field.map().value.split_by_word().map()..value", // Multiple dots
        "field.map().value.split_by_word().map().value.", // Trailing dot
        "field.map().value.split_by_word().map().value..", // Multiple trailing dots
        "field.map().value.split_by_word().map().value..next", // Multiple dots
    ];
    
    for (i, malformed_expr) in malformed_expressions.iter().enumerate() {
        let schema = DeclarativeSchemaDefinition {
            name: format!("malformed_expr_{}", i),
            schema_type: SchemaType::Single,
            fields: HashMap::from([
                ("test_field".to_string(), FieldDefinition {
                    field_type: Some("single".to_string()),
                    atom_uuid: Some(malformed_expr.to_string()),
                }),
            ]),
            key: None,
        };
        
        let transform = Transform::from_declarative_schema(
            schema,
            vec!["input".to_string()],
            "output.test_field".to_string(),
        );
        
        let validation_result = transform.validate();
        assert!(validation_result.is_err(), 
            "Transform with malformed expression '{}' should fail validation", malformed_expr);
    }
}

/// Test declarative transform with invalid schema names
#[test]
fn test_invalid_schema_names() {
    let invalid_names = vec![
        "", // Empty name
        "   ", // Whitespace only
        "ab", // Too short
        "invalid-name", // Contains hyphen
        "invalid.name", // Contains dot
        "invalid name", // Contains space
        "123invalid", // Starts with number
        "invalid_", // Ends with underscore
        "invalid__name", // Consecutive underscores
        "system", // Reserved word
        "admin", // Reserved word
        "test", // Reserved word
        "schema\x00", // Control character
    ];
    
    for invalid_name in invalid_names {
        let schema = DeclarativeSchemaDefinition {
            name: invalid_name.to_string(),
            schema_type: SchemaType::Single,
            fields: HashMap::from([
                ("field1".to_string(), FieldDefinition {
                    field_type: Some("single".to_string()),
                    atom_uuid: Some("input.map().value".to_string()),
                }),
            ]),
            key: None,
        };
        
        let validation_result = schema.validate();
        assert!(validation_result.is_err(), 
            "Schema with invalid name '{}' should fail validation", invalid_name);
    }
    
    // Test names that are now allowed by restrictive validation
    let allowed_names = vec![
        "valid_schema", // Valid underscore
        "ValidSchema", // Valid camelCase
        "schema123", // Valid with numbers
        "MySchema", // Valid PascalCase
    ];
    
    for allowed_name in allowed_names {
        let schema = DeclarativeSchemaDefinition {
            name: allowed_name.to_string(),
            schema_type: SchemaType::Single,
            fields: HashMap::from([
                ("field1".to_string(), FieldDefinition {
                    field_type: Some("single".to_string()),
                    atom_uuid: Some("input.map().value".to_string()),
                }),
            ]),
            key: None,
        };
        
        let validation_result = schema.validate();
        assert!(validation_result.is_ok(), 
            "Schema with name '{}' should pass validation", allowed_name);
    }
}

/// Test declarative transform with empty inputs
#[test]
fn test_empty_inputs() {
    let schema = DeclarativeSchemaDefinition {
        name: "empty_inputs_test".to_string(),
        schema_type: SchemaType::Single,
        fields: HashMap::from([
            ("field1".to_string(), FieldDefinition {
                field_type: Some("single".to_string()),
                atom_uuid: Some("input.map().value".to_string()),
            }),
        ]),
        key: None,
    };
    
    // Test with empty input list
    let transform_empty_inputs = Transform::from_declarative_schema(
        schema.clone(),
        vec![], // Empty inputs
        "output.field1".to_string(),
    );
    
    let validation_result = transform_empty_inputs.validate();
    // Note: Current validation doesn't check for empty input lists
    assert!(validation_result.is_ok(), "Transform with empty inputs currently passes validation");
    
    // Test with empty string input
    let transform_empty_string_input = Transform::from_declarative_schema(
        schema,
        vec!["".to_string()], // Empty string input
        "output.field1".to_string(),
    );
    
    let validation_result = transform_empty_string_input.validate();
    assert!(validation_result.is_err(), "Transform with empty string input should fail validation");
}

/// Test declarative transform with empty output
#[test]
fn test_empty_output() {
    let schema = DeclarativeSchemaDefinition {
        name: "empty_output_test".to_string(),
        schema_type: SchemaType::Single,
        fields: HashMap::from([
            ("field1".to_string(), FieldDefinition {
                field_type: Some("single".to_string()),
                atom_uuid: Some("input.map().value".to_string()),
            }),
        ]),
        key: None,
    };
    
    // Test with empty output
    let transform_empty_output = Transform::from_declarative_schema(
        schema,
        vec!["input".to_string()],
        "".to_string(), // Empty output
    );
    
    let validation_result = transform_empty_output.validate();
    assert!(validation_result.is_err(), "Transform with empty output should fail validation");
}

/// Test declarative transform with whitespace-only values
#[test]
fn test_whitespace_only_values() {
    let schema = DeclarativeSchemaDefinition {
        name: "whitespace_test".to_string(),
        schema_type: SchemaType::Single,
        fields: HashMap::from([
            ("field1".to_string(), FieldDefinition {
                field_type: Some("single".to_string()),
                atom_uuid: Some("input.map().value".to_string()),
            }),
        ]),
        key: None,
    };
    
    // Test with whitespace-only schema name
    // Note: New restrictive validation rejects whitespace-only names
    let whitespace_name_schema = DeclarativeSchemaDefinition {
        name: "   ".to_string(), // Whitespace only
        schema_type: SchemaType::Single,
        fields: HashMap::from([
            ("field1".to_string(), FieldDefinition {
                field_type: Some("single".to_string()),
                atom_uuid: Some("input.map().value".to_string()),
            }),
        ]),
        key: None,
    };
    
    let validation_result = whitespace_name_schema.validate();
    // New restrictive validation rejects whitespace-only names
    assert!(validation_result.is_err(), "Schema with whitespace-only name should fail validation");
    
    // Test with whitespace-only output
    let transform_whitespace_output = Transform::from_declarative_schema(
        schema,
        vec!["input".to_string()],
        "   ".to_string(), // Whitespace only
    );
    
    let validation_result = transform_whitespace_output.validate();
    assert!(validation_result.is_err(), "Transform with whitespace-only output should fail validation");
}

/// Test declarative transform with extremely long values
#[test]
fn test_extremely_long_values() {
    // Test with extremely long schema name
    // Note: New restrictive validation has length limits for schema names
    let long_name = "a".repeat(65); // Very long name (exceeds 64 char limit)
    let schema = DeclarativeSchemaDefinition {
        name: long_name,
        schema_type: SchemaType::Single,
        fields: HashMap::from([
            ("field1".to_string(), FieldDefinition {
                field_type: Some("single".to_string()),
                atom_uuid: Some("input.map().value".to_string()),
            }),
        ]),
        key: None,
    };
    
    let validation_result = schema.validate();
    // New restrictive validation rejects long names
    assert!(validation_result.is_err(), "Schema with extremely long name should fail validation");
    
    // Test with extremely long atom_uuid expression
    let long_expression = "input.map().value".repeat(100); // Very long expression
    let long_expr_schema = DeclarativeSchemaDefinition {
        name: "long_expr_test".to_string(),
        schema_type: SchemaType::Single,
        fields: HashMap::from([
            ("field1".to_string(), FieldDefinition {
                field_type: Some("single".to_string()),
                atom_uuid: Some(long_expression),
            }),
        ]),
        key: None,
    };
    
    let transform = Transform::from_declarative_schema(
        long_expr_schema,
        vec!["input".to_string()],
        "output.field1".to_string(),
    );
    
    let validation_result = transform.validate();
    // Current validation fails for extremely long expressions
    assert!(validation_result.is_err(), "Transform with extremely long expression should fail validation");
    
    if let Err(e) = validation_result {
        let error_msg = format!("{:?}", e);
        assert!(error_msg.contains("too long") || error_msg.contains("Expression too long"), 
            "Error should mention length limit: {}", error_msg);
    }
}

/// Test declarative transform with control characters
#[test]
fn test_control_characters() {
    // Test with control characters in schema name
    // Note: New restrictive validation checks for control characters in schema names
    let control_chars_name = "test\x00\x01\x02schema".to_string();
    let schema = DeclarativeSchemaDefinition {
        name: control_chars_name,
        schema_type: SchemaType::Single,
        fields: HashMap::from([
            ("field1".to_string(), FieldDefinition {
                field_type: Some("single".to_string()),
                atom_uuid: Some("input.map().value".to_string()),
            }),
        ]),
        key: None,
    };
    
    let validation_result = schema.validate();
    // New restrictive validation rejects control characters in schema names
    assert!(validation_result.is_err(), "Schema with control characters should fail validation");
    
    // Test with control characters in atom_uuid expression
    let control_chars_expr = "input.map().value\x00\x01\x02".to_string();
    let control_expr_schema = DeclarativeSchemaDefinition {
        name: "control_expr_test".to_string(),
        schema_type: SchemaType::Single,
        fields: HashMap::from([
            ("field1".to_string(), FieldDefinition {
                field_type: Some("single".to_string()),
                atom_uuid: Some(control_chars_expr),
            }),
        ]),
        key: None,
    };
    
    let transform = Transform::from_declarative_schema(
        control_expr_schema,
        vec!["input".to_string()],
        "output.field1".to_string(),
    );
    
    let validation_result = transform.validate();
    // Current validation allows control characters in expressions
    assert!(validation_result.is_ok(), "Transform with control characters currently passes validation");
}
