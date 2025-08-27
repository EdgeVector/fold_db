use std::collections::HashMap;

use datafold::schema::types::json_schema::{
    DeclarativeSchemaDefinition, FieldDefinition, KeyConfig,
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
    if let Err(SchemaError::InvalidField(msg)) = validation_result {
        assert!(msg.contains("HashRange schema requires key configuration"));
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
        assert!(msg.contains("HashRange key fields cannot be empty"));
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
        assert!(msg.contains("HashRange key fields cannot be empty"));
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
        assert!(msg.contains("Field empty_ref atom_uuid cannot be empty"));
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
        assert!(msg.contains("Field whitespace_ref atom_uuid cannot be empty"));
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
        assert!(msg.contains("Field empty_type field_type cannot be empty"));
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
