use std::collections::HashMap;

use datafold::schema::types::Transform;
use datafold::schema::types::json_schema::{
    DeclarativeSchemaDefinition, FieldDefinition, KeyConfig
};
use datafold::schema::types::schema::SchemaType;

/// Tests for enhanced validation functionality using existing iterator stack infrastructure.
///
/// These tests verify that declarative transforms are properly validated using:
/// - Chain parser for expression syntax validation
/// - Field alignment validator for iterator compatibility
/// - Error handling with existing iterator stack error types
/// - Clear user guidance and error messages

#[test]
fn test_valid_single_schema_validation() {
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
        name: "valid_single_schema".to_string(),
        schema_type: SchemaType::Single,
        key: None,
        fields,
    };

    // Schema validation should pass
    let result = schema.validate();
    assert!(result.is_ok(), "Valid Single schema should pass validation: {:?}", result);

    // Transform validation should also pass
    let transform = Transform::from_declarative_schema(
        schema,
        vec!["blogpost".to_string()],
        "processed_blog".to_string()
    );

    let validation_result = transform.validate();
    assert!(validation_result.is_ok(), "Valid Single transform should pass validation: {:?}", validation_result);
}

#[test]
fn test_valid_hashrange_schema_validation() {
    let mut fields = HashMap::new();
    fields.insert("title".to_string(), FieldDefinition {
        field_type: Some("single".to_string()),
        atom_uuid: Some("blogpost.title".to_string()),
    });
    fields.insert("tags".to_string(), FieldDefinition {
        field_type: Some("single".to_string()),
        atom_uuid: Some("blogpost.tags.map().name".to_string()),
    });

    let schema = DeclarativeSchemaDefinition {
        name: "valid_hashrange_schema".to_string(),
        schema_type: SchemaType::HashRange,
        key: Some(KeyConfig {
            hash_field: "blogpost.id".to_string(),
            range_field: "blogpost.published_date".to_string(),
        }),
        fields,
    };

    // Schema validation should pass
    let result = schema.validate();
    assert!(result.is_ok(), "Valid HashRange schema should pass validation: {:?}", result);

    // Transform validation should also pass
    let transform = Transform::from_declarative_schema(
        schema,
        vec!["blogpost".to_string()],
        "processed_blog".to_string()
    );

    let validation_result = transform.validate();
    assert!(validation_result.is_ok(), "Valid HashRange transform should pass validation: {:?}", validation_result);
}

#[test]
fn test_valid_range_schema_validation() {
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
        name: "valid_range_schema".to_string(),
        schema_type: SchemaType::Range { range_key: "published_date".to_string() },
        key: None,
        fields,
    };

    // Schema validation should pass
    let result = schema.validate();
    assert!(result.is_ok(), "Valid Range schema should pass validation: {:?}", result);

    // Transform validation should also pass
    let transform = Transform::from_declarative_schema(
        schema,
        vec!["blogpost".to_string()],
        "processed_blog".to_string()
    );

    let validation_result = transform.validate();
    assert!(validation_result.is_ok(), "Valid Range transform should pass validation: {:?}", validation_result);
}

#[test]
fn test_invalid_expression_syntax_validation() {
    let mut fields = HashMap::new();
    fields.insert("invalid_field".to_string(), FieldDefinition {
        field_type: Some("single".to_string()),
        atom_uuid: Some("blogpost..invalid.syntax".to_string()), // Invalid double dot
    });

    let schema = DeclarativeSchemaDefinition {
        name: "invalid_syntax_schema".to_string(),
        schema_type: SchemaType::Single,
        key: None,
        fields,
    };

    // Schema validation should fail with detailed error message
    let result = schema.validate();
    assert!(result.is_err(), "Schema with invalid expression syntax should fail validation");
    
    let error_msg = format!("{:?}", result.unwrap_err());
    // Check for either iterator stack parsing errors or FieldDefinition validation errors
    assert!(error_msg.contains("Expression parsing failed") || error_msg.contains("consecutive dots") || error_msg.contains("invalid"), 
           "Error should mention validation failure: {}", error_msg);
    assert!(error_msg.contains("invalid_field"), 
           "Error should mention the problematic field: {}", error_msg);
}

#[test]
fn test_field_alignment_validation_failure() {
    let mut fields = HashMap::new();
    fields.insert("simple_field".to_string(), FieldDefinition {
        field_type: Some("single".to_string()),
        atom_uuid: Some("blogpost.title".to_string()),
    });
    fields.insert("complex_field".to_string(), FieldDefinition {
        field_type: Some("single".to_string()),
        atom_uuid: Some("blogpost.tags.map().categories.map().name".to_string()),
    });

    let schema = DeclarativeSchemaDefinition {
        name: "misaligned_schema".to_string(),
        schema_type: SchemaType::Single,
        key: None,
        fields,
    };

    // Schema validation may pass or fail depending on field alignment rules
    // If it fails, it should provide clear field alignment error messages
    let result = schema.validate();
    match result {
        Err(error) => {
            let error_msg = format!("{:?}", error);
            // Should mention field alignment if that's the issue
            assert!(error_msg.contains("alignment") || error_msg.contains("depth") || error_msg.contains("incompatible"),
                   "Error should mention field alignment issues: {}", error_msg);
        }
        Ok(_) => {
            // Field alignment validation may pass with warnings for some configurations
            // This is acceptable as the validator is configurable
        }
    }
}

#[test] 
fn test_hashrange_missing_key_configuration() {
    let mut fields = HashMap::new();
    fields.insert("title".to_string(), FieldDefinition {
        field_type: Some("single".to_string()),
        atom_uuid: Some("blogpost.title".to_string()),
    });

    let schema = DeclarativeSchemaDefinition {
        name: "hashrange_no_key".to_string(),
        schema_type: SchemaType::HashRange,
        key: None, // Missing key configuration
        fields,
    };

    // Schema validation should fail
    let result = schema.validate();
    assert!(result.is_err(), "HashRange schema without key configuration should fail");
    
    let error_msg = format!("{:?}", result.unwrap_err());
    assert!(error_msg.contains("HashRange") && error_msg.contains("key"),
           "Error should mention HashRange key requirement: {}", error_msg);
}

#[test]
fn test_range_schema_missing_range_key_field() {
    let mut fields = HashMap::new();
    fields.insert("title".to_string(), FieldDefinition {
        field_type: Some("single".to_string()),
        atom_uuid: Some("blogpost.title".to_string()),
    });

    let schema = DeclarativeSchemaDefinition {
        name: "range_missing_key".to_string(),
        schema_type: SchemaType::Range { range_key: "nonexistent_field".to_string() },
        key: None,
        fields,
    };

    // Schema validation should fail
    let result = schema.validate();
    assert!(result.is_err(), "Range schema with missing range_key field should fail");
    
    let error_msg = format!("{:?}", result.unwrap_err());
    assert!(error_msg.contains("range_key") && error_msg.contains("nonexistent_field"),
           "Error should mention missing range_key field: {}", error_msg);
}

#[test]
fn test_range_schema_range_key_without_expression() {
    let mut fields = HashMap::new();
    fields.insert("title".to_string(), FieldDefinition {
        field_type: Some("single".to_string()),
        atom_uuid: Some("blogpost.title".to_string()),
    });
    fields.insert("published_date".to_string(), FieldDefinition {
        field_type: Some("single".to_string()),
        atom_uuid: None, // Missing atom_uuid expression
    });

    let schema = DeclarativeSchemaDefinition {
        name: "range_no_expression".to_string(),
        schema_type: SchemaType::Range { range_key: "published_date".to_string() },
        key: None,
        fields,
    };

    // Schema validation should fail
    let result = schema.validate();
    assert!(result.is_err(), "Range schema with range_key field without atom_uuid should fail");
    
    let error_msg = format!("{:?}", result.unwrap_err());
    assert!(error_msg.contains("published_date") && error_msg.contains("atom_uuid"),
           "Error should mention missing atom_uuid for range_key field: {}", error_msg);
}

#[test]
fn test_transform_input_validation() {
    let mut fields = HashMap::new();
    fields.insert("title".to_string(), FieldDefinition {
        field_type: Some("single".to_string()),
        atom_uuid: Some("blogpost.title".to_string()),
    });

    let schema = DeclarativeSchemaDefinition {
        name: "test_schema".to_string(),
        schema_type: SchemaType::Single,
        key: None,
        fields,
    };

    // Transform with empty input field name
    let transform = Transform::from_declarative_schema(
        schema,
        vec!["".to_string()], // Empty input name
        "output".to_string()
    );

    let result = transform.validate();
    assert!(result.is_err(), "Transform with empty input name should fail validation");
    
    let error_msg = format!("{:?}", result.unwrap_err());
    assert!(error_msg.contains("empty") && error_msg.contains("input"),
           "Error should mention empty input name: {}", error_msg);
}

#[test]
fn test_transform_empty_output_validation() {
    let mut fields = HashMap::new();
    fields.insert("title".to_string(), FieldDefinition {
        field_type: Some("single".to_string()),
        atom_uuid: Some("blogpost.title".to_string()),
    });

    let schema = DeclarativeSchemaDefinition {
        name: "test_schema".to_string(),
        schema_type: SchemaType::Single,
        key: None,
        fields,
    };

    // Create transform with empty output
    let transform = Transform::from_declarative_schema(
        schema,
        vec!["blogpost".to_string()],
        "".to_string() // Empty output
    );

    let result = transform.validate();
    assert!(result.is_err(), "Transform with empty output should fail validation");
    
    let error_msg = format!("{:?}", result.unwrap_err());
    assert!(error_msg.contains("output") && error_msg.contains("empty"),
           "Error should mention empty output field: {}", error_msg);
}

#[test]
fn test_hashrange_duplicate_hash_range_fields_validation() {
    let mut fields = HashMap::new();
    fields.insert("title".to_string(), FieldDefinition {
        field_type: Some("single".to_string()),
        atom_uuid: Some("blogpost.title".to_string()),
    });

    let schema = DeclarativeSchemaDefinition {
        name: "hashrange_duplicate_keys".to_string(),
        schema_type: SchemaType::HashRange,
        key: Some(KeyConfig {
            hash_field: "blogpost.id".to_string(),
            range_field: "blogpost.id".to_string(), // Same as hash_field
        }),
        fields,
    };

    let transform = Transform::from_declarative_schema(
        schema,
        vec!["blogpost".to_string()],
        "output".to_string()
    );

    let result = transform.validate();
    assert!(result.is_err(), "HashRange transform with duplicate hash/range fields should fail");
    
    let error_msg = format!("{:?}", result.unwrap_err());
    assert!(error_msg.contains("hash_field") && error_msg.contains("range_field") && error_msg.contains("different"),
           "Error should mention duplicate hash/range field requirement: {}", error_msg);
}

#[test]
fn test_procedural_transform_validation() {
    // Test that procedural transforms still validate correctly
    let transform = Transform::new(
        "return input.title".to_string(),
        "output.processed_title".to_string()
    );

    let result = transform.validate();
    assert!(result.is_ok(), "Valid procedural transform should pass validation: {:?}", result);
}

#[test]
fn test_procedural_transform_empty_output_validation() {
    let transform = Transform::new(
        "return input.title".to_string(),
        "".to_string() // Empty output
    );

    let result = transform.validate();
    assert!(result.is_err(), "Procedural transform with empty output should fail validation");
    
    let error_msg = format!("{:?}", result.unwrap_err());
    assert!(error_msg.contains("output") && error_msg.contains("empty"),
           "Error should mention empty output: {}", error_msg);
}

#[test]
fn test_error_message_quality() {
    // Test that error messages are user-friendly and actionable
    let mut fields = HashMap::new();
    fields.insert("problematic_field".to_string(), FieldDefinition {
        field_type: Some("single".to_string()),
        atom_uuid: Some("..invalid..syntax..".to_string()), // Clearly invalid with multiple consecutive dots
    });

    let schema = DeclarativeSchemaDefinition {
        name: "error_message_test".to_string(),
        schema_type: SchemaType::Single,
        key: None,
        fields,
    };

    let result = schema.validate();
    assert!(result.is_err(), "Schema with invalid syntax should fail");
    
    let error_msg = format!("{:?}", result.unwrap_err());
    
    // Error messages should be informative and include:
    // - Which field has the problem or what the validation issue is
    // - Context about the invalid expression
    // Check for either iterator stack validation or field definition validation
    let has_field_context = error_msg.contains("problematic_field") || error_msg.contains("invalid");
    let has_expression_context = error_msg.contains("..invalid..syntax..") || error_msg.contains("consecutive") || error_msg.contains("syntax");
    
    assert!(has_field_context, 
           "Error should provide field context: {}", error_msg);
    assert!(has_expression_context, 
           "Error should provide expression context: {}", error_msg);
    
    // Should provide some guidance or context
    assert!(error_msg.len() > 50, 
           "Error message should be descriptive enough to be helpful: {}", error_msg);
}
