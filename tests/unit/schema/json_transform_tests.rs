use datafold::schema::types::json_schema::{JsonTransform, TransformKind, DeclarativeSchemaDefinition, KeyConfig, FieldDefinition};
use datafold::schema::types::schema::SchemaType;
use datafold::schema::types::Transform;
use std::collections::HashMap;

#[test]
fn test_procedural_transform_serialization() {
    let transform = JsonTransform {
        kind: TransformKind::Procedural {
            logic: "return x + y".to_string(),
        },
        inputs: vec!["schema1.field1".to_string(), "schema2.field2".to_string()],
        output: "output.result".to_string(),
    };

    let json = serde_json::to_string(&transform).unwrap();
    let deserialized: JsonTransform = serde_json::from_str(&json).unwrap();

    assert!(matches!(deserialized.kind, TransformKind::Procedural { .. }));
    if let TransformKind::Procedural { logic } = &deserialized.kind {
        assert_eq!(logic, "return x + y");
    }
    assert_eq!(deserialized.inputs, vec!["schema1.field1", "schema2.field2"]);
    assert_eq!(deserialized.output, "output.result");
}

#[test]
fn test_declarative_transform_serialization() {
    let schema = DeclarativeSchemaDefinition {
        name: "test_schema".to_string(),
        schema_type: SchemaType::Single,
        key: None,
        fields: HashMap::new(),
    };

    let transform = JsonTransform {
        kind: TransformKind::Declarative { schema: schema.clone() },
        inputs: vec!["input.field".to_string()],
        output: "output.field".to_string(),
    };

    let json = serde_json::to_string(&transform).unwrap();
    let deserialized: JsonTransform = serde_json::from_str(&json).unwrap();

    assert!(matches!(deserialized.kind, TransformKind::Declarative { .. }));
    if let TransformKind::Declarative { schema: deserialized_schema } = &deserialized.kind {
        assert_eq!(deserialized_schema.name, "test_schema");
        assert_eq!(deserialized_schema.schema_type, SchemaType::Single);
    }
    assert_eq!(deserialized.inputs, vec!["input.field"]);
    assert_eq!(deserialized.output, "output.field");
}

#[test]
fn test_backward_compatibility_legacy_format() {
    // Test legacy format with logic field
    let legacy_json = r#"{
        "logic": "return x * 2",
        "inputs": ["input.field"],
        "output": "output.field"
    }"#;

    let deserialized: JsonTransform = serde_json::from_str(legacy_json).unwrap();

    assert!(matches!(deserialized.kind, TransformKind::Procedural { .. }));
    if let TransformKind::Procedural { logic } = &deserialized.kind {
        assert_eq!(logic, "return x * 2");
    }
    assert_eq!(deserialized.inputs, vec!["input.field"]);
    assert_eq!(deserialized.output, "output.field");
}

#[test]
fn test_new_format_with_explicit_kind() {
    // Test new format with explicit kind
    let new_json = r#"{
        "kind": "procedural",
        "logic": "return x + y",
        "inputs": ["input.field"],
        "output": "output.field"
    }"#;

    let deserialized: JsonTransform = serde_json::from_str(new_json).unwrap();

    assert!(matches!(deserialized.kind, TransformKind::Procedural { .. }));
    if let TransformKind::Procedural { logic } = &deserialized.kind {
        assert_eq!(logic, "return x + y");
    }
    assert_eq!(deserialized.inputs, vec!["input.field"]);
    assert_eq!(deserialized.output, "output.field");
}

#[test]
fn test_declarative_transform_with_hashrange_schema() {
    let key_config = KeyConfig {
        hash_field: "hash_field".to_string(),
        range_field: "range_field".to_string(),
    };

    let mut fields = HashMap::new();
    fields.insert("field1".to_string(), FieldDefinition {
        atom_uuid: Some("uuid1".to_string()),
        field_type: Some("String".to_string()),
    });

    let schema = DeclarativeSchemaDefinition {
        name: "hashrange_schema".to_string(),
        schema_type: SchemaType::HashRange,
        key: Some(key_config.clone()),
        fields,
    };

    let transform = JsonTransform {
        kind: TransformKind::Declarative { schema: schema.clone() },
        inputs: vec!["input.hash".to_string(), "input.range".to_string()],
        output: "output.result".to_string(),
    };

    let json = serde_json::to_string(&transform).unwrap();
    let deserialized: JsonTransform = serde_json::from_str(&json).unwrap();

    assert!(matches!(deserialized.kind, TransformKind::Declarative { .. }));
    if let TransformKind::Declarative { schema: deserialized_schema } = &deserialized.kind {
        assert_eq!(deserialized_schema.name, "hashrange_schema");
        assert_eq!(deserialized_schema.schema_type, SchemaType::HashRange);
        assert!(deserialized_schema.key.is_some());
        if let Some(key) = &deserialized_schema.key {
            assert_eq!(key.hash_field, "hash_field");
            assert_eq!(key.range_field, "range_field");
        }
    }
}

#[test]
fn test_transform_conversion_to_transform() {
    let json_transform = JsonTransform {
        kind: TransformKind::Procedural {
            logic: "return x + y".to_string(),
        },
        inputs: vec!["input.field".to_string()],
        output: "output.field".to_string(),
    };

    let transform: Transform = json_transform.into();

    assert_eq!(transform.logic, "return x + y");
    assert_eq!(transform.get_inputs(), &["input.field"]);
    assert_eq!(transform.get_output(), "output.field");
}

#[test]
fn test_declarative_transform_conversion_to_transform() {
    let schema = DeclarativeSchemaDefinition {
        name: "test_declarative".to_string(),
        schema_type: SchemaType::Single,
        key: None,
        fields: HashMap::new(),
    };

    let json_transform = JsonTransform {
        kind: TransformKind::Declarative { schema },
        inputs: vec!["input.field".to_string()],
        output: "output.field".to_string(),
    };

    let transform: Transform = json_transform.into();

    // Should contain placeholder logic for declarative transforms
    assert!(transform.logic.contains("Declarative transform: test_declarative"));
    assert_eq!(transform.get_inputs(), &["input.field"]);
    assert_eq!(transform.get_output(), "output.field");
}

#[test]
fn test_empty_inputs_default() {
    let transform = JsonTransform {
        kind: TransformKind::Procedural {
            logic: "return x".to_string(),
        },
        inputs: vec![], // Empty inputs should be allowed
        output: "output.field".to_string(),
    };

    let json = serde_json::to_string(&transform).unwrap();
    let deserialized: JsonTransform = serde_json::from_str(&json).unwrap();

    assert!(deserialized.inputs.is_empty());
}

#[test]
fn test_round_trip_serialization() {
    let original = JsonTransform {
        kind: TransformKind::Procedural {
            logic: "return x * y + z".to_string(),
        },
        inputs: vec!["a.b".to_string(), "c.d".to_string()],
        output: "result.field".to_string(),
    };

    let json = serde_json::to_string(&original).unwrap();
    let deserialized: JsonTransform = serde_json::from_str(&json).unwrap();

    // Verify all fields are preserved
    assert!(matches!(deserialized.kind, TransformKind::Procedural { .. }));
    if let TransformKind::Procedural { logic } = &deserialized.kind {
        assert_eq!(logic, "return x * y + z");
    }
    assert_eq!(deserialized.inputs, vec!["a.b", "c.d"]);
    assert_eq!(deserialized.output, "result.field");
}

// Additional comprehensive tests for DTS-1-4

#[test]
fn test_procedural_transform_with_special_characters() {
    let transform = JsonTransform {
        kind: TransformKind::Procedural {
            logic: "return \"Hello, World!\" + '\\n' + x".to_string(),
        },
        inputs: vec!["input.field".to_string()],
        output: "output.field".to_string(),
    };

    let json = serde_json::to_string(&transform).unwrap();
    let deserialized: JsonTransform = serde_json::from_str(&json).unwrap();

    assert!(matches!(deserialized.kind, TransformKind::Procedural { .. }));
    if let TransformKind::Procedural { logic } = &deserialized.kind {
        assert_eq!(logic, "return \"Hello, World!\" + '\\n' + x");
    }
}

#[test]
fn test_declarative_transform_with_complex_fields() {
    let mut fields = HashMap::new();
    fields.insert("blog".to_string(), FieldDefinition {
        atom_uuid: Some("blogpost.map().$atom_uuid".to_string()),
        field_type: Some("BlogPost".to_string()),
    });
    fields.insert("author".to_string(), FieldDefinition {
        atom_uuid: Some("blogpost.map().author.$atom_uuid".to_string()),
        field_type: Some("User".to_string()),
    });
    fields.insert("content".to_string(), FieldDefinition {
        atom_uuid: None,
        field_type: Some("String".to_string()),
    });

    let schema = DeclarativeSchemaDefinition {
        name: "complex_blog_schema".to_string(),
        schema_type: SchemaType::Single,
        key: None,
        fields,
    };

    let transform = JsonTransform {
        kind: TransformKind::Declarative { schema: schema.clone() },
        inputs: vec!["blogpost.content".to_string(), "blogpost.author".to_string()],
        output: "blog_summary.result".to_string(),
    };

    let json = serde_json::to_string(&transform).unwrap();
    let deserialized: JsonTransform = serde_json::from_str(&json).unwrap();

    assert!(matches!(deserialized.kind, TransformKind::Declarative { .. }));
    if let TransformKind::Declarative { schema: deserialized_schema } = &deserialized.kind {
        assert_eq!(deserialized_schema.name, "complex_blog_schema");
        assert_eq!(deserialized_schema.fields.len(), 3);
        assert!(deserialized_schema.fields.contains_key("blog"));
        assert!(deserialized_schema.fields.contains_key("author"));
        assert!(deserialized_schema.fields.contains_key("content"));
    }
}

#[test]
fn test_declarative_transform_hashrange_with_complex_expressions() {
    let key_config = KeyConfig {
        hash_field: "blogpost.map().content.split_by_word().map()".to_string(),
        range_field: "blogpost.map().publish_date".to_string(),
    };

    let mut fields = HashMap::new();
    fields.insert("word".to_string(), FieldDefinition {
        atom_uuid: None,
        field_type: Some("String".to_string()),
    });
    fields.insert("blog".to_string(), FieldDefinition {
        atom_uuid: Some("blogpost.map().$atom_uuid".to_string()),
        field_type: Some("BlogPost".to_string()),
    });

    let schema = DeclarativeSchemaDefinition {
        name: "blogs_by_word".to_string(),
        schema_type: SchemaType::HashRange,
        key: Some(key_config.clone()),
        fields,
    };

    let transform = JsonTransform {
        kind: TransformKind::Declarative { schema: schema.clone() },
        inputs: vec!["blogpost.content".to_string(), "blogpost.publish_date".to_string()],
        output: "word_index.result".to_string(),
    };

    let json = serde_json::to_string(&transform).unwrap();
    let deserialized: JsonTransform = serde_json::from_str(&json).unwrap();

    assert!(matches!(deserialized.kind, TransformKind::Declarative { .. }));
    if let TransformKind::Declarative { schema: deserialized_schema } = &deserialized.kind {
        assert_eq!(deserialized_schema.name, "blogs_by_word");
        assert_eq!(deserialized_schema.schema_type, SchemaType::HashRange);
        assert!(deserialized_schema.key.is_some());
        if let Some(key) = &deserialized_schema.key {
            assert_eq!(key.hash_field, "blogpost.map().content.split_by_word().map()");
            assert_eq!(key.range_field, "blogpost.map().publish_date");
        }
    }
}

#[test]
fn test_deserialization_with_missing_required_fields() {
    // Test missing output field
    let invalid_json = r#"{
        "kind": "procedural",
        "logic": "return x"
    }"#;

    let result: Result<JsonTransform, _> = serde_json::from_str(invalid_json);
    assert!(result.is_err());

    // Test missing logic for procedural
    let invalid_procedural_json = r#"{
        "kind": "procedural",
        "output": "output.field"
    }"#;

    let result: Result<JsonTransform, _> = serde_json::from_str(invalid_procedural_json);
    assert!(result.is_err());

    // Test missing schema for declarative
    let invalid_declarative_json = r#"{
        "kind": "declarative",
        "output": "output.field"
    }"#;

    let result: Result<JsonTransform, _> = serde_json::from_str(invalid_declarative_json);
    assert!(result.is_err());
}

#[test]
fn test_deserialization_with_extra_unknown_fields() {
    // Test procedural with extra fields (should be ignored)
    let json_with_extra = r#"{
        "kind": "procedural",
        "logic": "return x + y",
        "inputs": ["input.field"],
        "output": "output.field",
        "unknown_field": "should_be_ignored",
        "another_unknown": 123
    }"#;

    let deserialized: JsonTransform = serde_json::from_str(json_with_extra).unwrap();
    assert!(matches!(deserialized.kind, TransformKind::Procedural { .. }));
    if let TransformKind::Procedural { logic } = &deserialized.kind {
        assert_eq!(logic, "return x + y");
    }
    assert_eq!(deserialized.inputs, vec!["input.field"]);
    assert_eq!(deserialized.output, "output.field");
}

#[test]
fn test_deserialization_with_invalid_enum_values() {
    // Test invalid kind value without logic field (should fail)
    let invalid_kind_json = r#"{
        "kind": "invalid_kind",
        "output": "output.field"
    }"#;

    let result: Result<JsonTransform, _> = serde_json::from_str(invalid_kind_json);
    assert!(result.is_err(), "Should fail when kind is invalid and no logic field is present");

    // Test malformed kind structure
    let malformed_kind_json = r#"{
        "kind": {"invalid": "structure"},
        "output": "output.field"
    }"#;

    let result: Result<JsonTransform, _> = serde_json::from_str(malformed_kind_json);
    assert!(result.is_err(), "Should fail when kind is malformed");

    // Test valid kind with invalid schema structure for declarative
    let invalid_declarative_json = r#"{
        "kind": "declarative",
        "schema": "not_an_object",
        "output": "output.field"
    }"#;

    let result: Result<JsonTransform, _> = serde_json::from_str(invalid_declarative_json);
    assert!(result.is_err(), "Should fail when declarative schema is malformed");
}

#[test]
fn test_deserialization_with_empty_strings() {
    // Test empty logic string
    let empty_logic_json = r#"{
        "kind": "procedural",
        "logic": "",
        "output": "output.field"
    }"#;

    let deserialized: JsonTransform = serde_json::from_str(empty_logic_json).unwrap();
    assert!(matches!(deserialized.kind, TransformKind::Procedural { .. }));
    if let TransformKind::Procedural { logic } = &deserialized.kind {
        assert_eq!(logic, "");
    }

    // Test empty output string
    let empty_output_json = r#"{
        "kind": "procedural",
        "logic": "return x",
        "output": ""
    }"#;

    let deserialized: JsonTransform = serde_json::from_str(empty_output_json).unwrap();
    assert_eq!(deserialized.output, "");
}

#[test]
fn test_complex_round_trip_declarative_transform() {
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

    let original_schema = DeclarativeSchemaDefinition {
        name: "users_by_city_and_time".to_string(),
        schema_type: SchemaType::HashRange,
        key: Some(key_config.clone()),
        fields: fields.clone(),
    };

    let original_transform = JsonTransform {
        kind: TransformKind::Declarative { schema: original_schema },
        inputs: vec![
            "user.profile.location.city".to_string(),
            "user.profile.last_login".to_string(),
            "user.profile.name".to_string(),
        ],
        output: "city_user_index.result".to_string(),
    };

    let json = serde_json::to_string(&original_transform).unwrap();
    let deserialized: JsonTransform = serde_json::from_str(&json).unwrap();

    // Verify complete round-trip preservation
    assert!(matches!(deserialized.kind, TransformKind::Declarative { .. }));
    if let TransformKind::Declarative { schema: deserialized_schema } = &deserialized.kind {
        assert_eq!(deserialized_schema.name, "users_by_city_and_time");
        assert_eq!(deserialized_schema.schema_type, SchemaType::HashRange);
        assert!(deserialized_schema.key.is_some());
        
        if let Some(key) = &deserialized_schema.key {
            assert_eq!(key.hash_field, "user.map().profile.location.city");
            assert_eq!(key.range_field, "user.map().profile.last_login");
        }

        assert_eq!(deserialized_schema.fields.len(), 3);
        assert!(deserialized_schema.fields.contains_key("city"));
        assert!(deserialized_schema.fields.contains_key("user"));
        assert!(deserialized_schema.fields.contains_key("last_login"));

        // Verify field definitions are preserved exactly
        if let Some(city_field) = deserialized_schema.fields.get("city") {
            assert_eq!(city_field.atom_uuid, None);
            assert_eq!(city_field.field_type, Some("String".to_string()));
        }

        if let Some(user_field) = deserialized_schema.fields.get("user") {
            assert_eq!(user_field.atom_uuid, Some("user.map().$atom_uuid".to_string()));
            assert_eq!(user_field.field_type, Some("User".to_string()));
        }
    }

    assert_eq!(deserialized.inputs, vec![
        "user.profile.location.city",
        "user.profile.last_login",
        "user.profile.name",
    ]);
    assert_eq!(deserialized.output, "city_user_index.result");
}

#[test]
fn test_mixed_transform_scenarios() {
    // Create a procedural transform
    let procedural_transform = JsonTransform {
        kind: TransformKind::Procedural {
            logic: "return x * 2".to_string(),
        },
        inputs: vec!["input.value".to_string()],
        output: "output.doubled".to_string(),
    };

    // Create a declarative transform
    let schema = DeclarativeSchemaDefinition {
        name: "simple_mapping".to_string(),
        schema_type: SchemaType::Single,
        key: None,
        fields: HashMap::new(),
    };

    let declarative_transform = JsonTransform {
        kind: TransformKind::Declarative { schema },
        inputs: vec!["input.data".to_string()],
        output: "output.mapped".to_string(),
    };

    // Test both can coexist and serialize/deserialize independently
    let procedural_json = serde_json::to_string(&procedural_transform).unwrap();
    let declarative_json = serde_json::to_string(&declarative_transform).unwrap();

    let deserialized_procedural: JsonTransform = serde_json::from_str(&procedural_json).unwrap();
    let deserialized_declarative: JsonTransform = serde_json::from_str(&declarative_json).unwrap();

    // Verify procedural transform
    assert!(matches!(deserialized_procedural.kind, TransformKind::Procedural { .. }));
    if let TransformKind::Procedural { logic } = &deserialized_procedural.kind {
        assert_eq!(logic, "return x * 2");
    }

    // Verify declarative transform
    assert!(matches!(deserialized_declarative.kind, TransformKind::Declarative { .. }));
    if let TransformKind::Declarative { schema: deserialized_schema } = &deserialized_declarative.kind {
        assert_eq!(deserialized_schema.name, "simple_mapping");
    }
}

#[test]
fn test_legacy_format_with_various_input_combinations() {
    // Test legacy format with empty inputs
    let legacy_empty_inputs = r#"{
        "logic": "return 42",
        "inputs": [],
        "output": "output.constant"
    }"#;

    let deserialized: JsonTransform = serde_json::from_str(legacy_empty_inputs).unwrap();
    assert!(matches!(deserialized.kind, TransformKind::Procedural { .. }));
    assert!(deserialized.inputs.is_empty());

    // Test legacy format with single input
    let legacy_single_input = r#"{
        "logic": "return x + 1",
        "inputs": ["input.value"],
        "output": "output.incremented"
    }"#;

    let deserialized: JsonTransform = serde_json::from_str(legacy_single_input).unwrap();
    assert!(matches!(deserialized.kind, TransformKind::Procedural { .. }));
    assert_eq!(deserialized.inputs, vec!["input.value"]);

    // Test legacy format with multiple inputs
    let legacy_multiple_inputs = r#"{
        "logic": "return x + y + z",
        "inputs": ["input.a", "input.b", "input.c"],
        "output": "output.sum"
    }"#;

    let deserialized: JsonTransform = serde_json::from_str(legacy_multiple_inputs).unwrap();
    assert!(matches!(deserialized.kind, TransformKind::Procedural { .. }));
    assert_eq!(deserialized.inputs, vec!["input.a", "input.b", "input.c"]);
}

#[test]
fn test_new_format_with_various_kind_values() {
    // Test explicit procedural kind
    let explicit_procedural = r#"{
        "kind": "procedural",
        "logic": "return x * y",
        "inputs": ["input.a", "input.b"],
        "output": "output.product"
    }"#;

    let deserialized: JsonTransform = serde_json::from_str(explicit_procedural).unwrap();
    assert!(matches!(deserialized.kind, TransformKind::Procedural { .. }));
    if let TransformKind::Procedural { logic } = &deserialized.kind {
        assert_eq!(logic, "return x * y");
    }

    // Test explicit declarative kind
    let explicit_declarative = r#"{
        "kind": "declarative",
        "schema": {
            "name": "test_schema",
            "schema_type": "Single",
            "fields": {}
        },
        "inputs": ["input.data"],
        "output": "output.result"
    }"#;

    let deserialized: JsonTransform = serde_json::from_str(explicit_declarative).unwrap();
    assert!(matches!(deserialized.kind, TransformKind::Declarative { .. }));
    if let TransformKind::Declarative { schema } = &deserialized.kind {
        assert_eq!(schema.name, "test_schema");
        assert_eq!(schema.schema_type, SchemaType::Single);
    }
}
