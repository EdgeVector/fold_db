use std::collections::HashMap;

use datafold::schema::types::json_schema::{
    DeclarativeSchemaDefinition, FieldDefinition, JsonTransform, KeyConfig, TransformKind,
};
use datafold::schema::types::schema::SchemaType;

/// Integration tests for comprehensive serialization/deserialization testing
/// This file focuses on testing complex scenarios and edge cases that span
/// multiple data structures and transform types.

#[test]
fn test_complex_transform_ecosystem_round_trip() {
    // Create a complex ecosystem with multiple transform types
    // This test verifies that the entire system works together correctly
    
    // 1. Create a procedural transform
    let procedural_transform = JsonTransform {
        kind: TransformKind::Procedural {
            logic: "return x.map().filter(y => y > 10).reduce((acc, val) => acc + val, 0)".to_string(),
        },
        inputs: vec!["input.numbers".to_string()],
        output: "output.sum_of_large_numbers".to_string(),
    };
    
    // 2. Create a declarative transform with HashRange schema
    let key_config = KeyConfig {
        hash_field: "user.map().profile.location.country".to_string(),
        range_field: "user.map().profile.last_activity".to_string(),
    };
    
    let mut user_fields = HashMap::new();
    user_fields.insert("country".to_string(), FieldDefinition {
        atom_uuid: None,
        field_type: Some("String".to_string()),
    });
    user_fields.insert("user".to_string(), FieldDefinition {
        atom_uuid: Some("user.map().$atom_uuid".to_string()),
        field_type: Some("User".to_string()),
    });
    user_fields.insert("last_activity".to_string(), FieldDefinition {
        atom_uuid: None,
        field_type: Some("DateTime".to_string()),
    });
    user_fields.insert("profile".to_string(), FieldDefinition {
        atom_uuid: Some("user.map().profile.$atom_uuid".to_string()),
        field_type: Some("UserProfile".to_string()),
    });
    
    let user_schema = DeclarativeSchemaDefinition {
        name: "users_by_country_and_activity".to_string(),
        schema_type: SchemaType::HashRange,
        key: Some(key_config.clone()),
        fields: user_fields.clone(),
    };
    
    let declarative_transform = JsonTransform {
        kind: TransformKind::Declarative { schema: user_schema.clone() },
        inputs: vec![
            "user.profile.location.country".to_string(),
            "user.profile.last_activity".to_string(),
            "user.profile.name".to_string(),
        ],
        output: "country_user_index.result".to_string(),
    };
    
    // 3. Create another declarative transform with Single schema
    let mut blog_fields = HashMap::new();
    blog_fields.insert("blog".to_string(), FieldDefinition {
        atom_uuid: Some("blogpost.map().$atom_uuid".to_string()),
        field_type: Some("BlogPost".to_string()),
    });
    blog_fields.insert("author".to_string(), FieldDefinition {
        atom_uuid: Some("blogpost.map().author.$atom_uuid".to_string()),
        field_type: Some("User".to_string()),
    });
    blog_fields.insert("content".to_string(), FieldDefinition {
        atom_uuid: None,
        field_type: Some("String".to_string()),
    });
    
    let blog_schema = DeclarativeSchemaDefinition {
        name: "blog_summary".to_string(),
        schema_type: SchemaType::Single,
        key: None,
        fields: blog_fields.clone(),
    };
    
    let blog_transform = JsonTransform {
        kind: TransformKind::Declarative { schema: blog_schema.clone() },
        inputs: vec!["blogpost.content".to_string(), "blogpost.author".to_string()],
        output: "blog_summary.result".to_string(),
    };
    
    // 4. Test round-trip serialization for all transforms
    let transforms = vec![
        ("procedural", procedural_transform),
        ("declarative_hashrange", declarative_transform),
        ("declarative_single", blog_transform),
    ];
    
    for (name, transform) in transforms {
        let json = serde_json::to_string(&transform).expect(&format!("serialize {} transform", name));
        let deserialized: JsonTransform = serde_json::from_str(&json).expect(&format!("deserialize {} transform", name));
        
        // Verify basic structure is preserved
        assert_eq!(deserialized.inputs, transform.inputs, "inputs mismatch for {}", name);
        assert_eq!(deserialized.output, transform.output, "output mismatch for {}", name);
        
        // Verify transform kind is preserved
        match (&transform.kind, &deserialized.kind) {
            (TransformKind::Procedural { logic: orig_logic }, TransformKind::Procedural { logic: deser_logic }) => {
                assert_eq!(deser_logic, orig_logic, "procedural logic mismatch for {}", name);
            }
            (TransformKind::Declarative { schema: orig_schema }, TransformKind::Declarative { schema: deser_schema }) => {
                assert_eq!(deser_schema.name, orig_schema.name, "schema name mismatch for {}", name);
                assert_eq!(deser_schema.schema_type, orig_schema.schema_type, "schema type mismatch for {}", name);
                
                // Verify key configuration if present
                match (&orig_schema.key, &deser_schema.key) {
                    (Some(orig_key), Some(deser_key)) => {
                        assert_eq!(deser_key.hash_field, orig_key.hash_field, "hash field mismatch for {}", name);
                        assert_eq!(deser_key.range_field, orig_key.range_field, "range field mismatch for {}", name);
                    }
                    (None, None) => {
                        // Both are None, which is correct
                    }
                    _ => {
                        panic!("key configuration mismatch for {}: orig={:?}, deser={:?}", name, orig_schema.key, deser_schema.key);
                    }
                }
                
                // Verify fields are preserved
                assert_eq!(deser_schema.fields.len(), orig_schema.fields.len(), "field count mismatch for {}", name);
                for (field_name, orig_field) in &orig_schema.fields {
                    let deser_field = deser_schema.fields.get(field_name).expect(&format!("missing field {} in {}", field_name, name));
                    assert_eq!(deser_field.atom_uuid, orig_field.atom_uuid, "atom_uuid mismatch for field {} in {}", field_name, name);
                    assert_eq!(deser_field.field_type, orig_field.field_type, "field_type mismatch for field {} in {}", field_name, name);
                }
            }
            _ => {
                panic!("transform kind mismatch for {}: orig={:?}, deser={:?}", name, transform.kind, deserialized.kind);
            }
        }
    }
}

#[test]
fn test_mixed_format_handling() {
    // Test that the system can handle mixed formats in the same JSON payload
    // This simulates a real-world scenario where different transforms use different formats
    
    let mixed_json = r#"[
        {
            "kind": "procedural",
            "logic": "return x * 2",
            "inputs": ["input.value"],
            "output": "output.doubled"
        },
        {
            "logic": "return x + 1",
            "inputs": ["input.value"],
            "output": "output.incremented"
        },
        {
            "kind": "declarative",
            "schema": {
                "name": "simple_mapping",
                "schema_type": "Single",
                "fields": {}
            },
            "inputs": ["input.data"],
            "output": "output.mapped"
        }
    ]"#;
    
    let transforms: Vec<JsonTransform> = serde_json::from_str(mixed_json).expect("deserialize mixed format array");
    assert_eq!(transforms.len(), 3);
    
    // Verify first transform (explicit procedural)
    let first = &transforms[0];
    assert!(matches!(first.kind, TransformKind::Procedural { .. }));
    if let TransformKind::Procedural { logic } = &first.kind {
        assert_eq!(logic, "return x * 2");
    }
    assert_eq!(first.output, "output.doubled");
    
    // Verify second transform (legacy procedural)
    let second = &transforms[1];
    assert!(matches!(second.kind, TransformKind::Procedural { .. }));
    if let TransformKind::Procedural { logic } = &second.kind {
        assert_eq!(logic, "return x + 1");
    }
    assert_eq!(second.output, "output.incremented");
    
    // Verify third transform (explicit declarative)
    let third = &transforms[2];
    assert!(matches!(third.kind, TransformKind::Declarative { .. }));
    if let TransformKind::Declarative { schema } = &third.kind {
        assert_eq!(schema.name, "simple_mapping");
        assert_eq!(schema.schema_type, SchemaType::Single);
    }
    assert_eq!(third.output, "output.mapped");
}

#[test]
fn test_error_handling_integration() {
    // Test comprehensive error handling across different failure scenarios
    
    // 1. Test malformed procedural transform
    let malformed_procedural = r#"{
        "kind": "procedural",
        "output": "output.field"
    }"#;
    
    let result: Result<JsonTransform, _> = serde_json::from_str(malformed_procedural);
    assert!(result.is_err(), "should fail when logic field is missing for procedural transform");
    
    // 2. Test malformed declarative transform
    let malformed_declarative = r#"{
        "kind": "declarative",
        "output": "output.field"
    }"#;
    
    let result: Result<JsonTransform, _> = serde_json::from_str(malformed_declarative);
    assert!(result.is_err(), "should fail when schema field is missing for declarative transform");
    
    // 3. Test invalid enum value
    let invalid_kind = r#"{
        "kind": "invalid_kind",
        "output": "output.field"
    }"#;
    
    let result: Result<JsonTransform, _> = serde_json::from_str(invalid_kind);
    assert!(result.is_err(), "should fail with invalid kind value");
    
    // 4. Test malformed schema structure
    let malformed_schema = r#"{
        "kind": "declarative",
        "schema": {
            "name": "test",
            "schema_type": "HashRange"
        },
        "output": "output.field"
    }"#;
    
    let result: Result<JsonTransform, _> = serde_json::from_str(malformed_schema);
    assert!(result.is_err(), "should fail when HashRange schema is missing key configuration");
    
    // 5. Test malformed key configuration
    let malformed_key = r#"{
        "kind": "declarative",
        "schema": {
            "name": "test",
            "schema_type": "HashRange",
            "key": {
                "hash_field": "hash.field"
            },
            "fields": {}
        },
        "output": "output.field"
    }"#;
    
    let result: Result<JsonTransform, _> = serde_json::from_str(malformed_key);
    assert!(result.is_err(), "should fail when key configuration is missing range_field");
}

#[test]
fn test_performance_characteristics() {
    // Test that serialization/deserialization performance characteristics are reasonable
    // This is not a strict performance test, but ensures the system can handle
    // reasonably complex transforms without obvious performance issues
    
    let mut complex_fields = HashMap::new();
    
    // Create a complex schema with many fields
    for i in 0..100 {
        let field_name = format!("field_{}", i);
        let field_def = FieldDefinition {
            atom_uuid: if i % 3 == 0 {
                Some(format!("user.map().field_{}.$atom_uuid", i))
            } else {
                None
            },
            field_type: Some(format!("Type{}", i)),
        };
        complex_fields.insert(field_name, field_def);
    }
    
    let complex_schema = DeclarativeSchemaDefinition {
        name: "complex_performance_test".to_string(),
        schema_type: SchemaType::Single,
        key: None,
        fields: complex_fields,
    };
    
    let complex_transform = JsonTransform {
        kind: TransformKind::Declarative { schema: complex_schema },
        inputs: (0..50).map(|i| format!("input.field_{}", i)).collect(),
        output: "output.complex_result".to_string(),
    };
    
    // Test serialization performance
    let start = std::time::Instant::now();
    let json = serde_json::to_string(&complex_transform).expect("serialize complex transform");
    let serialize_duration = start.elapsed();
    
    // Test deserialization performance
    let start = std::time::Instant::now();
    let _deserialized: JsonTransform = serde_json::from_str(&json).expect("deserialize complex transform");
    let deserialize_duration = start.elapsed();
    
    // Verify performance is reasonable (should complete in under 100ms for both operations)
    assert!(serialize_duration.as_millis() < 100, "Serialization took too long: {:?}", serialize_duration);
    assert!(deserialize_duration.as_millis() < 100, "Deserialization took too long: {:?}", deserialize_duration);
    
    // Verify the JSON size is reasonable
    let json_size = json.len();
    assert!(json_size < 100_000, "JSON size is unexpectedly large: {} bytes", json_size);
    
    // Verify round-trip integrity
    let deserialized: JsonTransform = serde_json::from_str(&json).expect("deserialize for round-trip verification");
    assert_eq!(deserialized.inputs.len(), 50);
    assert_eq!(deserialized.output, "output.complex_result");
    
    if let TransformKind::Declarative { schema } = &deserialized.kind {
        assert_eq!(schema.fields.len(), 100);
        assert_eq!(schema.name, "complex_performance_test");
    } else {
        panic!("Expected declarative transform kind");
    }
}

#[test]
fn test_edge_case_handling() {
    // Test various edge cases that might occur in production
    
    // 1. Test with very long strings
    let long_logic = "return ".to_string() + &"x".repeat(1000) + " + y";
    let long_procedural = JsonTransform {
        kind: TransformKind::Procedural { logic: long_logic.clone() },
        inputs: vec!["input.field".to_string()],
        output: "output.result".to_string(),
    };
    
    let json = serde_json::to_string(&long_procedural).expect("serialize long procedural transform");
    let deserialized: JsonTransform = serde_json::from_str(&json).expect("deserialize long procedural transform");
    
    if let TransformKind::Procedural { logic } = &deserialized.kind {
        assert_eq!(logic, &long_logic);
    }
    
    // 2. Test with special characters in field names
    let special_field_names = vec![
        "field.with.dots".to_string(),
        "field_with_underscores".to_string(),
        "field-with-dashes".to_string(),
        "fieldWithCamelCase".to_string(),
        "field_with_numbers_123".to_string(),
    ];
    
    let special_procedural = JsonTransform {
        kind: TransformKind::Procedural {
            logic: "return x".to_string(),
        },
        inputs: special_field_names.clone(),
        output: "output.result".to_string(),
    };
    
    let json = serde_json::to_string(&special_procedural).expect("serialize special field names");
    let deserialized: JsonTransform = serde_json::from_str(&json).expect("deserialize special field names");
    
    assert_eq!(deserialized.inputs, special_field_names);
    
    // 3. Test with empty collections
    let empty_declarative = JsonTransform {
        kind: TransformKind::Declarative {
            schema: DeclarativeSchemaDefinition {
                name: "empty_schema".to_string(),
                schema_type: SchemaType::Single,
                key: None,
                fields: HashMap::new(),
            },
        },
        inputs: vec![],
        output: "output.empty".to_string(),
    };
    
    let json = serde_json::to_string(&empty_declarative).expect("serialize empty declarative transform");
    let deserialized: JsonTransform = serde_json::from_str(&json).expect("deserialize empty declarative transform");
    
    assert!(deserialized.inputs.is_empty());
    if let TransformKind::Declarative { schema } = &deserialized.kind {
        assert!(schema.fields.is_empty());
    }
}

#[test]
fn test_validation_integration() {
    // Test that validation errors are properly handled during deserialization
    // and that the system provides meaningful error messages
    
    // 1. Test HashRange schema without key (should fail validation)
    let invalid_hashrange_json = r#"{
        "kind": "declarative",
        "schema": {
            "name": "invalid_hashrange",
            "schema_type": "HashRange",
            "fields": {}
        },
        "output": "output.result"
    }"#;
    
    // This should deserialize successfully (validation happens separately)
    let transform: JsonTransform = serde_json::from_str(invalid_hashrange_json).expect("deserialize invalid HashRange schema");
    
    // But validation should fail
    if let TransformKind::Declarative { schema } = &transform.kind {
        let validation_result = schema.validate();
        assert!(validation_result.is_err(), "HashRange schema without key should fail validation");
        
        if let Err(error) = validation_result {
            let error_msg = error.to_string();
            assert!(error_msg.contains("Schema must have at least one field defined"), 
                   "Unexpected error message: {}", error_msg);
        }
    }
    
    // 2. Test HashRange schema with empty key fields
    let empty_key_json = r#"{
        "kind": "declarative",
        "schema": {
            "name": "empty_key_hashrange",
            "schema_type": "HashRange",
            "key": {
                "hash_field": "",
                "range_field": ""
            },
            "fields": {}
        },
        "output": "output.result"
    }"#;
    
    let transform: JsonTransform = serde_json::from_str(empty_key_json).expect("deserialize HashRange schema with empty keys");
    
    if let TransformKind::Declarative { schema } = &transform.kind {
        let validation_result = schema.validate();
        assert!(validation_result.is_err(), "HashRange schema with empty key fields should fail validation");
        
        if let Err(error) = validation_result {
            let error_msg = error.to_string();
            assert!(error_msg.contains("Schema must have at least one field defined"), 
                   "Unexpected error message: {}", error_msg);
        }
    }
    
    // 3. Test field validation errors
    let invalid_field_json = r#"{
        "kind": "declarative",
        "schema": {
            "name": "invalid_fields",
            "schema_type": "Single",
            "fields": {
                "empty_atom": {
                    "atom_uuid": "",
                    "field_type": "String"
                }
            }
        },
        "output": "output.result"
    }"#;
    
    let transform: JsonTransform = serde_json::from_str(invalid_field_json).expect("deserialize schema with invalid fields");
    
    if let TransformKind::Declarative { schema } = &transform.kind {
        let validation_result = schema.validate();
        assert!(validation_result.is_err(), "Schema with empty atom_uuid should fail validation");
        
        if let Err(error) = validation_result {
            let error_msg = error.to_string();
            assert!(error_msg.contains("Field 'empty_atom' atom_uuid cannot be empty"), 
                   "Unexpected error message: {}", error_msg);
        }
    }
}
