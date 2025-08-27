use std::collections::HashMap;

use datafold::schema::types::json_schema::{
    DeclarativeSchemaDefinition, FieldDefinition, JsonTransform, KeyConfig, TransformKind,
};
use datafold::schema::types::schema::SchemaType;
use datafold::schema::types::SchemaError;

#[test]
fn schema_type_hashrange_serializes() {
    let ty = SchemaType::HashRange;
    let json = serde_json::to_string(&ty).expect("serialize SchemaType");
    assert_eq!(json, "\"HashRange\"");
}

#[test]
fn key_config_serializes() {
    let cfg = KeyConfig {
        hash_field: "h".to_string(),
        range_field: "r".to_string(),
    };
    let json = serde_json::to_string(&cfg).expect("serialize KeyConfig");
    assert!(json.contains("\"hash_field\":"));
    assert!(json.contains("\"range_field\":"));
}

#[test]
fn field_definition_serializes() {
    let field = FieldDefinition {
        atom_uuid: Some("a".to_string()),
        field_type: Some("String".to_string()),
    };
    let json = serde_json::to_string(&field).expect("serialize FieldDefinition");
    assert!(json.contains("\"atom_uuid\":"));
    assert!(json.contains("\"field_type\":"));
}

#[test]
fn declarative_schema_definition_round_trip() {
    let mut fields = HashMap::new();
    fields.insert("id".to_string(), FieldDefinition::default());
    let schema = DeclarativeSchemaDefinition {
        name: "test".to_string(),
        schema_type: SchemaType::Single,
        key: None,
        fields,
    };
    let json = serde_json::to_string(&schema).expect("serialize schema");
    let back: DeclarativeSchemaDefinition =
        serde_json::from_str(&json).expect("deserialize schema");
    assert_eq!(back.name, "test");
    assert_eq!(back.schema_type, SchemaType::Single);
}

#[test]
fn hashrange_requires_key_config() {
    let schema = DeclarativeSchemaDefinition {
        name: "test".to_string(),
        schema_type: SchemaType::HashRange,
        key: None,
        fields: HashMap::new(),
    };
    assert!(schema.validate().is_err());
}

#[test]
fn field_validation_checks_empty() {
    let mut fields = HashMap::new();
    fields.insert(
        "ref".to_string(),
        FieldDefinition {
            atom_uuid: Some(String::new()),
            field_type: None,
        },
    );
    let schema = DeclarativeSchemaDefinition {
        name: "test".to_string(),
        schema_type: SchemaType::Single,
        key: None,
        fields,
    };
    assert!(schema.validate().is_err());
}

// Additional comprehensive tests for DTS-1-4

#[test]
fn test_key_config_serialization_edge_cases() {
    // Test with empty strings (should serialize but may fail validation)
    let empty_key_config = KeyConfig {
        hash_field: "".to_string(),
        range_field: "".to_string(),
    };
    
    let json = serde_json::to_string(&empty_key_config).expect("serialize empty KeyConfig");
    assert!(json.contains("\"hash_field\":\"\""));
    assert!(json.contains("\"range_field\":\"\""));
    
    // Test with special characters
    let special_key_config = KeyConfig {
        hash_field: "field.with.dots.and_underscores".to_string(),
        range_field: "field.with\"quotes\"and\\backslashes".to_string(),
    };
    
    let json = serde_json::to_string(&special_key_config).expect("serialize special KeyConfig");
    let deserialized: KeyConfig = serde_json::from_str(&json).expect("deserialize special KeyConfig");
    assert_eq!(deserialized.hash_field, "field.with.dots.and_underscores");
    assert_eq!(deserialized.range_field, "field.with\"quotes\"and\\backslashes");
}

#[test]
fn test_field_definition_serialization_edge_cases() {
    // Test with None values
    let empty_field = FieldDefinition {
        atom_uuid: None,
        field_type: None,
    };
    
    let json = serde_json::to_string(&empty_field).expect("serialize empty FieldDefinition");
    // Should not contain the None fields
    assert!(!json.contains("atom_uuid"));
    assert!(!json.contains("field_type"));
    
    // Test with empty strings
    let empty_string_field = FieldDefinition {
        atom_uuid: Some("".to_string()),
        field_type: Some("".to_string()),
    };
    
    let json = serde_json::to_string(&empty_string_field).expect("serialize empty string FieldDefinition");
    assert!(json.contains("\"atom_uuid\":\"\""));
    assert!(json.contains("\"field_type\":\"\""));
    
    // Test with complex expressions
    let complex_field = FieldDefinition {
        atom_uuid: Some("user.map().profile.location.$atom_uuid".to_string()),
        field_type: Some("Location<City, Country>".to_string()),
    };
    
    let json = serde_json::to_string(&complex_field).expect("serialize complex FieldDefinition");
    let deserialized: FieldDefinition = serde_json::from_str(&json).expect("deserialize complex FieldDefinition");
    assert_eq!(deserialized.atom_uuid, Some("user.map().profile.location.$atom_uuid".to_string()));
    assert_eq!(deserialized.field_type, Some("Location<City, Country>".to_string()));
}

#[test]
fn test_declarative_schema_definition_complex_round_trip() {
    let key_config = KeyConfig {
        hash_field: "user.map().profile.location.city".to_string(),
        range_field: "user.map().profile.last_login".to_string(),
    };
    
    let mut fields = HashMap::new();
    fields.insert("city".to_string(), FieldDefinition {
        atom_uuid: None,
        field_type: Some("String".to_string()),
    });
    fields.insert("user".to_string(), FieldDefinition {
        atom_uuid: Some("user.map().$atom_uuid".to_string()),
        field_type: Some("User".to_string()),
    });
    fields.insert("last_login".to_string(), FieldDefinition {
        atom_uuid: None,
        field_type: Some("DateTime".to_string()),
    });
    fields.insert("profile".to_string(), FieldDefinition {
        atom_uuid: Some("user.map().profile.$atom_uuid".to_string()),
        field_type: Some("UserProfile".to_string()),
    });
    
    let original_schema = DeclarativeSchemaDefinition {
        name: "complex_user_schema".to_string(),
        schema_type: SchemaType::HashRange,
        key: Some(key_config.clone()),
        fields: fields.clone(),
    };
    
    let json = serde_json::to_string(&original_schema).expect("serialize complex schema");
    let deserialized: DeclarativeSchemaDefinition = serde_json::from_str(&json).expect("deserialize complex schema");
    
    // Verify complete round-trip preservation
    assert_eq!(deserialized.name, "complex_user_schema");
    assert_eq!(deserialized.schema_type, SchemaType::HashRange);
    assert!(deserialized.key.is_some());
    
    if let Some(key) = &deserialized.key {
        assert_eq!(key.hash_field, "user.map().profile.location.city");
        assert_eq!(key.range_field, "user.map().profile.last_login");
    }
    
    assert_eq!(deserialized.fields.len(), 4);
    assert!(deserialized.fields.contains_key("city"));
    assert!(deserialized.fields.contains_key("user"));
    assert!(deserialized.fields.contains_key("last_login"));
    assert!(deserialized.fields.contains_key("profile"));
    
    // Verify field definitions are preserved exactly
    if let Some(city_field) = deserialized.fields.get("city") {
        assert_eq!(city_field.atom_uuid, None);
        assert_eq!(city_field.field_type, Some("String".to_string()));
    }
    
    if let Some(user_field) = deserialized.fields.get("user") {
        assert_eq!(user_field.atom_uuid, Some("user.map().$atom_uuid".to_string()));
        assert_eq!(user_field.field_type, Some("User".to_string()));
    }
    
    if let Some(profile_field) = deserialized.fields.get("profile") {
        assert_eq!(profile_field.atom_uuid, Some("user.map().profile.$atom_uuid".to_string()));
        assert_eq!(profile_field.field_type, Some("UserProfile".to_string()));
    }
}

#[test]
fn test_declarative_schema_validation_edge_cases() {
    // Test HashRange schema without key (should fail validation)
    let invalid_hashrange = DeclarativeSchemaDefinition {
        name: "invalid".to_string(),
        schema_type: SchemaType::HashRange,
        key: None,
        fields: HashMap::new(),
    };
    
    let validation_result = invalid_hashrange.validate();
    assert!(validation_result.is_err());
    // The error should be about empty fields since we check that first
    if let Err(SchemaError::InvalidField(msg)) = validation_result {
        assert!(msg.contains("Schema must have at least one field defined"));
    }
    
    // Test HashRange schema with empty key fields
    let empty_key_config = KeyConfig {
        hash_field: "".to_string(),
        range_field: "".to_string(),
    };
    
    let invalid_hashrange_empty_keys = DeclarativeSchemaDefinition {
        name: "invalid_empty_keys".to_string(),
        schema_type: SchemaType::HashRange,
        key: Some(empty_key_config),
        fields: HashMap::new(),
    };
    
    let validation_result = invalid_hashrange_empty_keys.validate();
    assert!(validation_result.is_err());
    if let Err(SchemaError::InvalidField(msg)) = validation_result {
        assert!(msg.contains("Schema must have at least one field defined"));
    }
    
    // Test HashRange schema with whitespace-only key fields
    let whitespace_key_config = KeyConfig {
        hash_field: "   ".to_string(),
        range_field: "\t\n".to_string(),
    };
    
    let invalid_hashrange_whitespace = DeclarativeSchemaDefinition {
        name: "invalid_whitespace".to_string(),
        schema_type: SchemaType::HashRange,
        key: Some(whitespace_key_config),
        fields: HashMap::new(),
    };
    
    let validation_result = invalid_hashrange_whitespace.validate();
    assert!(validation_result.is_err());
    if let Err(SchemaError::InvalidField(msg)) = validation_result {
        assert!(msg.contains("Schema must have at least one field defined"));
    }
}

#[test]
fn test_field_validation_edge_cases() {
    // Test field with empty atom_uuid string
    let mut fields = HashMap::new();
    fields.insert("empty_ref".to_string(), FieldDefinition {
        atom_uuid: Some("".to_string()),
        field_type: None,
    });
    
    let schema_empty_atom = DeclarativeSchemaDefinition {
        name: "test_empty_atom".to_string(),
        schema_type: SchemaType::Single,
        key: None,
        fields,
    };
    
    let validation_result = schema_empty_atom.validate();
    assert!(validation_result.is_err());
    if let Err(SchemaError::InvalidField(msg)) = validation_result {
        assert!(msg.contains("Field 'empty_ref' atom_uuid cannot be empty"));
    }
    
    // Test field with whitespace-only atom_uuid
    let mut fields = HashMap::new();
    fields.insert("whitespace_ref".to_string(), FieldDefinition {
        atom_uuid: Some("   \t\n".to_string()),
        field_type: None,
    });
    
    let schema_whitespace_atom = DeclarativeSchemaDefinition {
        name: "test_whitespace_atom".to_string(),
        schema_type: SchemaType::Single,
        key: None,
        fields,
    };
    
    let validation_result = schema_whitespace_atom.validate();
    assert!(validation_result.is_err());
    if let Err(SchemaError::InvalidField(msg)) = validation_result {
        assert!(msg.contains("Field 'whitespace_ref' atom_uuid cannot be empty"));
    }
    
    // Test field with empty field_type string
    let mut fields = HashMap::new();
    fields.insert("empty_type".to_string(), FieldDefinition {
        atom_uuid: None,
        field_type: Some("".to_string()),
    });
    
    let schema_empty_type = DeclarativeSchemaDefinition {
        name: "test_empty_type".to_string(),
        schema_type: SchemaType::Single,
        key: None,
        fields,
    };
    
    let validation_result = schema_empty_type.validate();
    assert!(validation_result.is_err());
    if let Err(SchemaError::InvalidField(msg)) = validation_result {
        assert!(msg.contains("Field 'empty_type' field_type cannot be empty"));
    }
}

#[test]
fn test_declarative_schema_serialization_with_optional_fields() {
    // Test Single schema without key (should be valid)
    let single_schema = DeclarativeSchemaDefinition {
        name: "single_schema".to_string(),
        schema_type: SchemaType::Single,
        key: None,
        fields: HashMap::new(),
    };
    
    let json = serde_json::to_string(&single_schema).expect("serialize single schema");
    let deserialized: DeclarativeSchemaDefinition = serde_json::from_str(&json).expect("deserialize single schema");
    
    assert_eq!(deserialized.name, "single_schema");
    assert_eq!(deserialized.schema_type, SchemaType::Single);
    assert!(deserialized.key.is_none());
    assert!(deserialized.fields.is_empty());
    
    // Test HashRange schema with key (should be valid)
    let key_config = KeyConfig {
        hash_field: "hash.field".to_string(),
        range_field: "range.field".to_string(),
    };
    
    let hashrange_schema = DeclarativeSchemaDefinition {
        name: "hashrange_schema".to_string(),
        schema_type: SchemaType::HashRange,
        key: Some(key_config.clone()),
        fields: HashMap::new(),
    };
    
    let json = serde_json::to_string(&hashrange_schema).expect("serialize hashrange schema");
    let deserialized: DeclarativeSchemaDefinition = serde_json::from_str(&json).expect("deserialize hashrange schema");
    
    assert_eq!(deserialized.name, "hashrange_schema");
    assert_eq!(deserialized.schema_type, SchemaType::HashRange);
    assert!(deserialized.key.is_some());
    
    if let Some(key) = &deserialized.key {
        assert_eq!(key.hash_field, "hash.field");
        assert_eq!(key.range_field, "range.field");
    }
}

#[test]
fn test_declarative_schema_deserialization_edge_cases() {
    // Test deserialization with missing required fields
    let missing_name_json = r#"{
        "schema_type": "Single",
        "fields": {}
    }"#;
    
    let result: Result<DeclarativeSchemaDefinition, _> = serde_json::from_str(missing_name_json);
    assert!(result.is_err());
    
    // Test deserialization with missing schema_type
    let missing_type_json = r#"{
        "name": "test",
        "fields": {}
    }"#;
    
    let result: Result<DeclarativeSchemaDefinition, _> = serde_json::from_str(missing_type_json);
    assert!(result.is_err());
    
    // Test deserialization with missing fields
    let missing_fields_json = r#"{
        "name": "test",
        "schema_type": "Single"
    }"#;
    
    let result: Result<DeclarativeSchemaDefinition, _> = serde_json::from_str(missing_fields_json);
    assert!(result.is_err());
    
    // Test deserialization with invalid schema_type
    let invalid_type_json = r#"{
        "name": "test",
        "schema_type": "InvalidType",
        "fields": {}
    }"#;
    
    let result: Result<DeclarativeSchemaDefinition, _> = serde_json::from_str(invalid_type_json);
    assert!(result.is_err());
}

#[test]
fn test_key_config_deserialization_edge_cases() {
    // Test deserialization with missing hash_field
    let missing_hash_json = r#"{
        "range_field": "range.field"
    }"#;
    
    let result: Result<KeyConfig, _> = serde_json::from_str(missing_hash_json);
    assert!(result.is_err());
    
    // Test deserialization with missing range_field
    let missing_range_json = r#"{
        "hash_field": "hash.field"
    }"#;
    
    let result: Result<KeyConfig, _> = serde_json::from_str(missing_range_json);
    assert!(result.is_err());
    
    // Test deserialization with extra fields (should be ignored)
    let extra_fields_json = r#"{
        "hash_field": "hash.field",
        "range_field": "range.field",
        "unknown_field": "should_be_ignored"
    }"#;
    
    let deserialized: KeyConfig = serde_json::from_str(extra_fields_json).expect("deserialize with extra fields");
    assert_eq!(deserialized.hash_field, "hash.field");
    assert_eq!(deserialized.range_field, "range.field");
}

#[test]
fn test_field_definition_deserialization_edge_cases() {
    // Test deserialization with all fields present
    let complete_json = r#"{
        "atom_uuid": "user.$atom_uuid",
        "field_type": "User"
    }"#;
    
    let deserialized: FieldDefinition = serde_json::from_str(complete_json).expect("deserialize complete FieldDefinition");
    assert_eq!(deserialized.atom_uuid, Some("user.$atom_uuid".to_string()));
    assert_eq!(deserialized.field_type, Some("User".to_string()));
    
    // Test deserialization with no fields (should use defaults)
    let empty_json = r#"{}"#;
    
    let deserialized: FieldDefinition = serde_json::from_str(empty_json).expect("deserialize empty FieldDefinition");
    assert_eq!(deserialized.atom_uuid, None);
    assert_eq!(deserialized.field_type, None);
    
    // Test deserialization with extra fields (should be ignored)
    let extra_fields_json = r#"{
        "atom_uuid": "user.$atom_uuid",
        "field_type": "User",
        "unknown_field": "should_be_ignored"
    }"#;
    
    let deserialized: FieldDefinition = serde_json::from_str(extra_fields_json).expect("deserialize with extra fields");
    assert_eq!(deserialized.atom_uuid, Some("user.$atom_uuid".to_string()));
    assert_eq!(deserialized.field_type, Some("User".to_string()));
}

// Comprehensive validation tests for DTS-1-5

#[test]
fn test_declarative_schema_comprehensive_validation() {
    // Valid HashRange schema should pass validation
    let key_config = KeyConfig {
        hash_field: "user.map().location".to_string(),
        range_field: "user.map().timestamp".to_string(),
    };

    let mut fields = HashMap::new();
    fields.insert("user".to_string(), FieldDefinition {
        atom_uuid: Some("user.map().$atom_uuid".to_string()),
        field_type: Some("User".to_string()),
    });

    let valid_schema = DeclarativeSchemaDefinition {
        name: "valid_user_schema".to_string(),
        schema_type: SchemaType::HashRange,
        key: Some(key_config),
        fields,
    };

    assert!(valid_schema.validate().is_ok(), "Valid HashRange schema should pass validation");

    // Valid Single schema should pass validation
    let mut single_fields = HashMap::new();
    single_fields.insert("name".to_string(), FieldDefinition {
        atom_uuid: None,
        field_type: Some("String".to_string()),
    });

    let valid_single_schema = DeclarativeSchemaDefinition {
        name: "valid_single_schema".to_string(),
        schema_type: SchemaType::Single,
        key: None,
        fields: single_fields,
    };

    assert!(valid_single_schema.validate().is_ok(), "Valid Single schema should pass validation");
}

#[test]
fn test_declarative_schema_validation_failures() {
    // Schema with empty name should fail
    let empty_name_schema = DeclarativeSchemaDefinition {
        name: "".to_string(),
        schema_type: SchemaType::Single,
        key: None,
        fields: HashMap::new(),
    };

    assert!(empty_name_schema.validate().is_err(), "Schema with empty name should fail validation");

    // Schema with no fields should fail
    let no_fields_schema = DeclarativeSchemaDefinition {
        name: "test_schema".to_string(),
        schema_type: SchemaType::Single,
        key: None,
        fields: HashMap::new(),
    };

    assert!(no_fields_schema.validate().is_err(), "Schema with no fields should fail validation");

    // Single schema with key configuration should fail
    let single_with_key = DeclarativeSchemaDefinition {
        name: "invalid_single".to_string(),
        schema_type: SchemaType::Single,
        key: Some(KeyConfig {
            hash_field: "field1".to_string(),
            range_field: "field2".to_string(),
        }),
        fields: {
            let mut fields = HashMap::new();
            fields.insert("test".to_string(), FieldDefinition::default());
            fields
        },
    };

    assert!(single_with_key.validate().is_err(), "Single schema with key should fail validation");
}

#[test]
fn test_key_config_comprehensive_validation() {
    // Valid key config should pass
    let valid_key = KeyConfig {
        hash_field: "user.map().location".to_string(),
        range_field: "user.map().timestamp".to_string(),
    };

    assert!(valid_key.validate().is_ok(), "Valid key config should pass validation");

    // Empty hash field should fail
    let empty_hash = KeyConfig {
        hash_field: "".to_string(),
        range_field: "user.map().timestamp".to_string(),
    };

    assert!(empty_hash.validate().is_err(), "Key config with empty hash field should fail");

    // Empty range field should fail
    let empty_range = KeyConfig {
        hash_field: "user.map().location".to_string(),
        range_field: "".to_string(),
    };

    assert!(empty_range.validate().is_err(), "Key config with empty range field should fail");

    // Whitespace-only fields should fail
    let whitespace_hash = KeyConfig {
        hash_field: "   ".to_string(),
        range_field: "user.map().timestamp".to_string(),
    };

    assert!(whitespace_hash.validate().is_err(), "Key config with whitespace-only hash field should fail");

    // Same hash and range fields should fail
    let same_fields = KeyConfig {
        hash_field: "user.map().field".to_string(),
        range_field: "user.map().field".to_string(),
    };

    assert!(same_fields.validate().is_err(), "Key config with same hash and range fields should fail");

    // Invalid field expressions should fail
    let invalid_expression = KeyConfig {
        hash_field: ".invalid.start".to_string(),
        range_field: "user.map().timestamp".to_string(),
    };

    assert!(invalid_expression.validate().is_err(), "Key config with invalid field expression should fail");

    let consecutive_dots = KeyConfig {
        hash_field: "user..invalid".to_string(),
        range_field: "user.map().timestamp".to_string(),
    };

    assert!(consecutive_dots.validate().is_err(), "Key config with consecutive dots should fail");
}

#[test]
fn test_field_definition_comprehensive_validation() {
    // Valid field definition with atom_uuid should pass
    let valid_with_atom = FieldDefinition {
        atom_uuid: Some("user.map().$atom_uuid".to_string()),
        field_type: None,
    };

    assert!(valid_with_atom.validate("test_field").is_ok(), "Valid field with atom_uuid should pass");

    // Valid field definition with field_type should pass
    let valid_with_type = FieldDefinition {
        atom_uuid: None,
        field_type: Some("String".to_string()),
    };

    assert!(valid_with_type.validate("test_field").is_ok(), "Valid field with field_type should pass");

    // Valid field definition with both should pass
    let valid_with_both = FieldDefinition {
        atom_uuid: Some("user.map().$atom_uuid".to_string()),
        field_type: Some("User".to_string()),
    };

    assert!(valid_with_both.validate("test_field").is_ok(), "Valid field with both properties should pass");

    // Field definition with neither should fail
    let empty_field = FieldDefinition {
        atom_uuid: None,
        field_type: None,
    };

    assert!(empty_field.validate("test_field").is_err(), "Field with no properties should fail validation");

    // Field with empty atom_uuid should fail
    let empty_atom_uuid = FieldDefinition {
        atom_uuid: Some("".to_string()),
        field_type: None,
    };

    assert!(empty_atom_uuid.validate("test_field").is_err(), "Field with empty atom_uuid should fail");

    // Field with empty field_type should fail
    let empty_field_type = FieldDefinition {
        atom_uuid: None,
        field_type: Some("".to_string()),
    };

    assert!(empty_field_type.validate("test_field").is_err(), "Field with empty field_type should fail");

    // Field with invalid atom_uuid expression should fail
    let invalid_atom_expr = FieldDefinition {
        atom_uuid: Some(".invalid.start".to_string()),
        field_type: None,
    };

    assert!(invalid_atom_expr.validate("test_field").is_err(), "Field with invalid atom_uuid expression should fail");

    // Field with too long field_type should fail
    let long_type = FieldDefinition {
        atom_uuid: None,
        field_type: Some("A".repeat(101)),
    };

    assert!(long_type.validate("test_field").is_err(), "Field with too long field_type should fail");

    // Field with control characters in field_type should fail
    let control_char_type = FieldDefinition {
        atom_uuid: None,
        field_type: Some("String\n".to_string()),
    };

    assert!(control_char_type.validate("test_field").is_err(), "Field with control characters should fail");
}

#[test]
fn test_json_transform_comprehensive_validation() {
    // Valid procedural transform should pass
    let valid_procedural = JsonTransform {
        kind: TransformKind::Procedural {
            logic: "return x + y".to_string(),
        },
        inputs: vec!["schema1.field1".to_string(), "schema2.field2".to_string()],
        output: "output.result".to_string(),
    };

    assert!(valid_procedural.validate().is_ok(), "Valid procedural transform should pass validation");

    // Valid declarative transform should pass
    let valid_declarative = JsonTransform {
        kind: TransformKind::Declarative {
            schema: DeclarativeSchemaDefinition {
                name: "test_schema".to_string(),
                schema_type: SchemaType::Single,
                key: None,
                fields: {
                    let mut fields = HashMap::new();
                    fields.insert("test".to_string(), FieldDefinition {
                        atom_uuid: None,
                        field_type: Some("String".to_string()),
                    });
                    fields
                },
            },
        },
        inputs: vec!["input.field".to_string()],
        output: "output.result".to_string(),
    };

    assert!(valid_declarative.validate().is_ok(), "Valid declarative transform should pass validation");

    // Transform with empty output should fail
    let empty_output = JsonTransform {
        kind: TransformKind::Procedural {
            logic: "return x".to_string(),
        },
        inputs: vec!["input.field".to_string()],
        output: "".to_string(),
    };

    assert!(empty_output.validate().is_err(), "Transform with empty output should fail");

    // Transform with invalid output format should fail
    let invalid_output_format = JsonTransform {
        kind: TransformKind::Procedural {
            logic: "return x".to_string(),
        },
        inputs: vec!["input.field".to_string()],
        output: "invalid_format".to_string(),
    };

    assert!(invalid_output_format.validate().is_err(), "Transform with invalid output format should fail");

    // Transform with invalid input format should fail
    let invalid_input_format = JsonTransform {
        kind: TransformKind::Procedural {
            logic: "return x".to_string(),
        },
        inputs: vec!["invalid_format".to_string()],
        output: "output.result".to_string(),
    };

    assert!(invalid_input_format.validate().is_err(), "Transform with invalid input format should fail");

    // Transform with empty input should fail
    let empty_input = JsonTransform {
        kind: TransformKind::Procedural {
            logic: "return x".to_string(),
        },
        inputs: vec!["".to_string()],
        output: "output.result".to_string(),
    };

    assert!(empty_input.validate().is_err(), "Transform with empty input should fail");
}

#[test]
fn test_transform_kind_validation() {
    // Valid procedural transform kind should pass
    let valid_procedural = TransformKind::Procedural {
        logic: "return x + y".to_string(),
    };

    assert!(valid_procedural.validate().is_ok(), "Valid procedural transform kind should pass");

    // Empty procedural logic should fail
    let empty_logic = TransformKind::Procedural {
        logic: "".to_string(),
    };

    assert!(empty_logic.validate().is_err(), "Procedural transform with empty logic should fail");

    // Too long procedural logic should fail
    let too_long_logic = TransformKind::Procedural {
        logic: "return x".repeat(2000),
    };

    assert!(too_long_logic.validate().is_err(), "Procedural transform with too long logic should fail");

    // Mismatched braces should fail
    let mismatched_braces = TransformKind::Procedural {
        logic: "if (x > 0) { return x".to_string(),
    };

    assert!(mismatched_braces.validate().is_err(), "Procedural transform with mismatched braces should fail");

    // Mismatched parentheses should fail
    let mismatched_parens = TransformKind::Procedural {
        logic: "return func(x, y".to_string(),
    };

    assert!(mismatched_parens.validate().is_err(), "Procedural transform with mismatched parentheses should fail");

    // Valid declarative transform kind should pass
    let valid_declarative = TransformKind::Declarative {
        schema: DeclarativeSchemaDefinition {
            name: "test_schema".to_string(),
            schema_type: SchemaType::Single,
            key: None,
            fields: {
                let mut fields = HashMap::new();
                fields.insert("test".to_string(), FieldDefinition {
                    atom_uuid: None,
                    field_type: Some("String".to_string()),
                });
                fields
            },
        },
    };

    assert!(valid_declarative.validate().is_ok(), "Valid declarative transform kind should pass");
}

#[test]
fn test_range_schema_validation() {
    use datafold::schema::types::schema::SchemaType;

    // Range schema should not have key configuration
    let range_with_key = DeclarativeSchemaDefinition {
        name: "invalid_range".to_string(),
        schema_type: SchemaType::Range {
            range_key: "timestamp".to_string(),
        },
        key: Some(KeyConfig {
            hash_field: "field1".to_string(),
            range_field: "field2".to_string(),
        }),
        fields: {
            let mut fields = HashMap::new();
            fields.insert("test".to_string(), FieldDefinition::default());
            fields
        },
    };

    assert!(range_with_key.validate().is_err(), "Range schema with key should fail validation");

    // Range schema with empty range_key should fail
    let empty_range_key = DeclarativeSchemaDefinition {
        name: "invalid_range".to_string(),
        schema_type: SchemaType::Range {
            range_key: "".to_string(),
        },
        key: None,
        fields: {
            let mut fields = HashMap::new();
            fields.insert("test".to_string(), FieldDefinition {
                field_type: Some("String".to_string()),
                atom_uuid: None,
            });
            fields
        },
    };

    assert!(empty_range_key.validate().is_err(), "Range schema with empty range_key should fail validation");
}
