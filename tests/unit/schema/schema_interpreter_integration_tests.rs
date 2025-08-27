use std::collections::HashMap;

use datafold::schema::types::json_schema::{
    DeclarativeSchemaDefinition, FieldDefinition, JsonSchemaDefinition, JsonSchemaField, 
    JsonTransform, KeyConfig, TransformKind, JsonPermissionPolicy, JsonFieldPaymentConfig,
};
use datafold::schema::types::schema::SchemaType;
use datafold::schema::types::Field;
use datafold::schema::schema_interpretation::interpret_schema;
use datafold::schema::validator::SchemaValidator;
use datafold::fees::payment_config::SchemaPaymentConfig;
use datafold::permissions::types::policy::TrustDistance;
use datafold::fees::types::scaling::TrustDistanceScaling;

/// Tests for schema interpreter integration with declarative transforms
/// This validates that the schema interpreter can properly handle both procedural and declarative transforms

#[test]
fn test_schema_interpreter_with_procedural_transforms() {
    // Create a JSON schema with procedural transforms
    let mut fields = HashMap::new();
    fields.insert("calculated_field".to_string(), JsonSchemaField {
        permission_policy: JsonPermissionPolicy {
            read: TrustDistance::Distance(0),
            write: TrustDistance::Distance(0),
            explicit_read: None,
            explicit_write: None,
        },
        molecule_uuid: None,
        payment_config: JsonFieldPaymentConfig {
            base_multiplier: 1.0,
            trust_distance_scaling: TrustDistanceScaling::None,
            min_payment: None,
        },
        field_mappers: HashMap::new(),
        field_type: datafold::schema::types::field::FieldType::Single,
        transform: Some(JsonTransform {
            kind: TransformKind::Procedural {
                logic: "return input1 + input2".to_string(),
            },
            inputs: vec!["schema1.field1".to_string(), "schema1.field2".to_string()],
            output: "result.sum".to_string(),
        }),
    });

    let json_schema = JsonSchemaDefinition {
        name: "test_schema".to_string(),
        schema_type: SchemaType::Single,
        fields,
        payment_config: SchemaPaymentConfig::default(),
        hash: None,
    };

    // Create a mock validator
    let core = datafold::schema::core::SchemaCore::new_for_testing("test").unwrap();
    let validator = SchemaValidator::new(&core);

    // Interpret the schema
    let schema = interpret_schema(&validator, json_schema).unwrap();

    // Verify the schema was created correctly
    assert_eq!(schema.name, "test_schema");
    assert_eq!(schema.fields.len(), 1);

    // Get the field and check its transform
    let field = schema.fields.get("calculated_field").unwrap();
    if let datafold::schema::types::FieldVariant::Single(single_field) = field {
        let transform = single_field.transform().unwrap();
        assert!(transform.is_procedural());
        assert_eq!(transform.get_procedural_logic().unwrap(), "return input1 + input2");
        assert_eq!(transform.get_inputs(), &["schema1.field1", "schema1.field2"]);
        assert_eq!(transform.get_output(), "result.sum");
    } else {
        panic!("Expected SingleField");
    }
}

#[test]
fn test_schema_interpreter_with_declarative_transforms() {
    // Create a declarative schema definition
    let mut declarative_fields = HashMap::new();
    declarative_fields.insert("user_ref".to_string(), FieldDefinition {
        atom_uuid: Some("user.map().$atom_uuid".to_string()),
        field_type: Some("User".to_string()),
    });

    let declarative_schema = DeclarativeSchemaDefinition {
        name: "user_transform".to_string(),
        schema_type: SchemaType::Single,
        key: None,
        fields: declarative_fields,
    };

    // Create a JSON schema with declarative transforms
    let mut fields = HashMap::new();
    fields.insert("user_field".to_string(), JsonSchemaField {
        permission_policy: JsonPermissionPolicy {
            read: TrustDistance::Distance(0),
            write: TrustDistance::Distance(0),
            explicit_read: None,
            explicit_write: None,
        },
        molecule_uuid: None,
        payment_config: JsonFieldPaymentConfig {
            base_multiplier: 1.0,
            trust_distance_scaling: TrustDistanceScaling::None,
            min_payment: None,
        },
        field_mappers: HashMap::new(),
        field_type: datafold::schema::types::field::FieldType::Single,
        transform: Some(JsonTransform {
            kind: TransformKind::Declarative {
                schema: declarative_schema,
            },
            inputs: vec!["input.user".to_string()],
            output: "result.user_ref".to_string(),
        }),
    });

    let json_schema = JsonSchemaDefinition {
        name: "test_schema_declarative".to_string(),
        schema_type: SchemaType::Single,
        fields,
        payment_config: SchemaPaymentConfig::default(),
        hash: None,
    };

    // Create a mock validator
    let core = datafold::schema::core::SchemaCore::new_for_testing("test").unwrap();
    let validator = SchemaValidator::new(&core);

    // Interpret the schema
    let schema = interpret_schema(&validator, json_schema).unwrap();

    // Verify the schema was created correctly
    assert_eq!(schema.name, "test_schema_declarative");
    assert_eq!(schema.fields.len(), 1);

    // Get the field and check its transform
    let field = schema.fields.get("user_field").unwrap();
    if let datafold::schema::types::FieldVariant::Single(single_field) = field {
        let transform = single_field.transform().unwrap();
        assert!(transform.is_declarative());
        
        let declarative_schema = transform.get_declarative_schema().unwrap();
        assert_eq!(declarative_schema.name, "user_transform");
        assert_eq!(declarative_schema.fields.len(), 1);
        
        let user_ref_field = declarative_schema.fields.get("user_ref").unwrap();
        assert_eq!(user_ref_field.atom_uuid.as_ref().unwrap(), "user.map().$atom_uuid");
        assert_eq!(user_ref_field.field_type.as_ref().unwrap(), "User");
        
        assert_eq!(transform.get_inputs(), &["input.user"]);
        assert_eq!(transform.get_output(), "result.user_ref");
    } else {
        panic!("Expected SingleField");
    }
}

#[test]
fn test_schema_interpreter_with_hashrange_declarative_transforms() {
    // Create a HashRange declarative schema definition
    let key_config = KeyConfig {
        hash_field: "user.map().location".to_string(),
        range_field: "user.map().timestamp".to_string(),
    };

    let mut declarative_fields = HashMap::new();
    declarative_fields.insert("user_ref".to_string(), FieldDefinition {
        atom_uuid: Some("user.map().$atom_uuid".to_string()),
        field_type: Some("User".to_string()),
    });
    declarative_fields.insert("location".to_string(), FieldDefinition {
        atom_uuid: Some("user.map().location".to_string()),
        field_type: Some("String".to_string()),
    });

    let declarative_schema = DeclarativeSchemaDefinition {
        name: "user_location_transform".to_string(),
        schema_type: SchemaType::HashRange,
        key: Some(key_config),
        fields: declarative_fields,
    };

    // Create a JSON schema with HashRange declarative transforms
    let mut fields = HashMap::new();
    fields.insert("user_location_field".to_string(), JsonSchemaField {
        permission_policy: JsonPermissionPolicy {
            read: TrustDistance::Distance(0),
            write: TrustDistance::Distance(0),
            explicit_read: None,
            explicit_write: None,
        },
        molecule_uuid: None,
        payment_config: JsonFieldPaymentConfig {
            base_multiplier: 1.0,
            trust_distance_scaling: TrustDistanceScaling::None,
            min_payment: None,
        },
        field_mappers: HashMap::new(),
        field_type: datafold::schema::types::field::FieldType::Single,
        transform: Some(JsonTransform {
            kind: TransformKind::Declarative {
                schema: declarative_schema,
            },
            inputs: vec!["input.user".to_string()],
            output: "result.user_location".to_string(),
        }),
    });

    let json_schema = JsonSchemaDefinition {
        name: "test_schema_hashrange".to_string(),
        schema_type: SchemaType::Single,
        fields,
        payment_config: SchemaPaymentConfig::default(),
        hash: None,
    };

    // Create a mock validator
    let core = datafold::schema::core::SchemaCore::new_for_testing("test").unwrap();
    let validator = SchemaValidator::new(&core);

    // Interpret the schema
    let schema = interpret_schema(&validator, json_schema).unwrap();

    // Verify the schema was created correctly
    assert_eq!(schema.name, "test_schema_hashrange");
    assert_eq!(schema.fields.len(), 1);

    // Get the field and check its transform
    let field = schema.fields.get("user_location_field").unwrap();
    if let datafold::schema::types::FieldVariant::Single(single_field) = field {
        let transform = single_field.transform().unwrap();
        assert!(transform.is_declarative());
        
        let declarative_schema = transform.get_declarative_schema().unwrap();
        assert_eq!(declarative_schema.name, "user_location_transform");
        assert_eq!(declarative_schema.schema_type, SchemaType::HashRange);
        assert!(declarative_schema.key.is_some());
        
        let key = declarative_schema.key.as_ref().unwrap();
        assert_eq!(key.hash_field, "user.map().location");
        assert_eq!(key.range_field, "user.map().timestamp");
        
        assert_eq!(declarative_schema.fields.len(), 2);
        assert!(declarative_schema.fields.contains_key("user_ref"));
        assert!(declarative_schema.fields.contains_key("location"));
        
        assert_eq!(transform.get_inputs(), &["input.user"]);
        assert_eq!(transform.get_output(), "result.user_location");
    } else {
        panic!("Expected SingleField");
    }
}

#[test]
fn test_mixed_transform_types_in_same_schema() {
    // Create a schema with both procedural and declarative transforms
    let mut declarative_fields = HashMap::new();
    declarative_fields.insert("user_ref".to_string(), FieldDefinition {
        atom_uuid: Some("user.map().$atom_uuid".to_string()),
        field_type: Some("User".to_string()),
    });

    let declarative_schema = DeclarativeSchemaDefinition {
        name: "user_transform".to_string(),
        schema_type: SchemaType::Single,
        key: None,
        fields: declarative_fields,
    };

    let mut fields = HashMap::new();
    
    // Add a procedural transform
    fields.insert("calculated_field".to_string(), JsonSchemaField {
        permission_policy: JsonPermissionPolicy {
            read: TrustDistance::Distance(0),
            write: TrustDistance::Distance(0),
            explicit_read: None,
            explicit_write: None,
        },
        molecule_uuid: None,
        payment_config: JsonFieldPaymentConfig {
            base_multiplier: 1.0,
            trust_distance_scaling: TrustDistanceScaling::None,
            min_payment: None,
        },
        field_mappers: HashMap::new(),
        field_type: datafold::schema::types::field::FieldType::Single,
        transform: Some(JsonTransform {
            kind: TransformKind::Procedural {
                logic: "return input1 * 2".to_string(),
            },
            inputs: vec!["input.value".to_string()],
            output: "result.doubled".to_string(),
        }),
    });

    // Add a declarative transform
    fields.insert("user_field".to_string(), JsonSchemaField {
        permission_policy: JsonPermissionPolicy {
            read: TrustDistance::Distance(0),
            write: TrustDistance::Distance(0),
            explicit_read: None,
            explicit_write: None,
        },
        molecule_uuid: None,
        payment_config: JsonFieldPaymentConfig {
            base_multiplier: 1.0,
            trust_distance_scaling: TrustDistanceScaling::None,
            min_payment: None,
        },
        field_mappers: HashMap::new(),
        field_type: datafold::schema::types::field::FieldType::Single,
        transform: Some(JsonTransform {
            kind: TransformKind::Declarative {
                schema: declarative_schema,
            },
            inputs: vec!["input.user".to_string()],
            output: "result.user_ref".to_string(),
        }),
    });

    let json_schema = JsonSchemaDefinition {
        name: "mixed_transform_schema".to_string(),
        schema_type: SchemaType::Single,
        fields,
        payment_config: SchemaPaymentConfig::default(),
        hash: None,
    };

    // Create a mock validator
    let core = datafold::schema::core::SchemaCore::new_for_testing("test").unwrap();
    let validator = SchemaValidator::new(&core);

    // Interpret the schema
    let schema = interpret_schema(&validator, json_schema).unwrap();

    // Verify the schema was created correctly
    assert_eq!(schema.name, "mixed_transform_schema");
    assert_eq!(schema.fields.len(), 2);

    // Check the procedural transform
    let calc_field = schema.fields.get("calculated_field").unwrap();
    if let datafold::schema::types::FieldVariant::Single(single_field) = calc_field {
        let transform = single_field.transform().unwrap();
        assert!(transform.is_procedural());
        assert_eq!(transform.get_procedural_logic().unwrap(), "return input1 * 2");
    } else {
        panic!("Expected SingleField for calculated_field");
    }

    // Check the declarative transform
    let user_field = schema.fields.get("user_field").unwrap();
    if let datafold::schema::types::FieldVariant::Single(single_field) = user_field {
        let transform = single_field.transform().unwrap();
        assert!(transform.is_declarative());
        assert_eq!(transform.get_declarative_schema().unwrap().name, "user_transform");
    } else {
        panic!("Expected SingleField for user_field");
    }
}

#[test]
fn test_validation_of_declarative_transforms_during_interpretation() {
    // Create an invalid declarative schema (missing required fields)
    let declarative_schema = DeclarativeSchemaDefinition {
        name: "".to_string(), // Invalid: empty name
        schema_type: SchemaType::Single,
        key: None,
        fields: HashMap::new(), // Invalid: no fields
    };

    let mut fields = HashMap::new();
    fields.insert("invalid_field".to_string(), JsonSchemaField {
        permission_policy: JsonPermissionPolicy {
            read: TrustDistance::Distance(0),
            write: TrustDistance::Distance(0),
            explicit_read: None,
            explicit_write: None,
        },
        molecule_uuid: None,
        payment_config: JsonFieldPaymentConfig {
            base_multiplier: 1.0,
            trust_distance_scaling: TrustDistanceScaling::None,
            min_payment: None,
        },
        field_mappers: HashMap::new(),
        field_type: datafold::schema::types::field::FieldType::Single,
        transform: Some(JsonTransform {
            kind: TransformKind::Declarative {
                schema: declarative_schema,
            },
            inputs: vec!["input.user".to_string()],
            output: "result.invalid".to_string(),
        }),
    });

    let json_schema = JsonSchemaDefinition {
        name: "invalid_schema".to_string(),
        schema_type: SchemaType::Single,
        fields,
        payment_config: SchemaPaymentConfig::default(),
        hash: None,
    };

    // Create a mock validator
    let core = datafold::schema::core::SchemaCore::new_for_testing("test").unwrap();
    let validator = SchemaValidator::new(&core);

    // This should succeed at the schema level but the transforms can be validated separately
    let schema = interpret_schema(&validator, json_schema).unwrap();
    
    // Extract the transform and validate it separately
    let field = schema.fields.get("invalid_field").unwrap();
    if let datafold::schema::types::FieldVariant::Single(single_field) = field {
        let transform = single_field.transform().unwrap();
        
        // Validation should fail for the declarative transform
        let validation_result = datafold::transform::executor::TransformExecutor::validate_transform(transform);
        assert!(validation_result.is_err(), "Invalid declarative transform should fail validation");
    }
}
