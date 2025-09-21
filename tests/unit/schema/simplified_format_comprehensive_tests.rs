use std::collections::HashMap;

use datafold::fees::types::config::TrustDistanceScaling;
use datafold::permissions::types::policy::TrustDistance;
use datafold::schema::types::field::FieldType;
use datafold::schema::types::json_schema::{
    DeclarativeSchemaDefinition, FieldDefinition, JsonSchemaDefinition,
};
use datafold::schema::types::schema::SchemaType;

/// Comprehensive tests for simplified schema formats
/// This file covers edge cases and scenarios not fully covered in other test files

#[test]
fn test_ultra_minimal_schema_with_all_schema_types() {
    // Test Single schema with ultra-minimal fields
    let single_json = r#"
    {
      "name": "SingleSchema",
      "schema_type": "Single",
      "fields": {
        "id": {},
        "name": {},
        "value": {}
      },
      "payment_config": {
        "base_multiplier": 1.0,
        "min_payment_threshold": 0
      }
    }
    "#;

    let single_schema: JsonSchemaDefinition = serde_json::from_str(single_json).unwrap();
    assert_eq!(single_schema.name, "SingleSchema");
    assert_eq!(single_schema.fields.len(), 3);

    // Verify all fields get default values
    for field_name in ["id", "name", "value"] {
        let field = single_schema.fields.get(field_name).unwrap();
        assert!(matches!(field.field_type, FieldType::Single));
        assert!(matches!(
            field.permission_policy.read,
            TrustDistance::Distance(0)
        ));
        assert!(matches!(
            field.permission_policy.write,
            TrustDistance::Distance(0)
        ));
        assert_eq!(field.payment_config.base_multiplier, 1.0);
        assert!(matches!(
            field.payment_config.trust_distance_scaling,
            TrustDistanceScaling::None
        ));
    }

    // Test Range schema with ultra-minimal fields
    let range_json = r#"
    {
      "name": "RangeSchema",
      "schema_type": {
        "Range": {
          "range_key": "timestamp"
        }
      },
      "fields": {
        "timestamp": {},
        "data": {}
      },
      "payment_config": {
        "base_multiplier": 1.0,
        "min_payment_threshold": 0
      }
    }
    "#;

    let range_schema: JsonSchemaDefinition = serde_json::from_str(range_json).unwrap();
    assert_eq!(range_schema.name, "RangeSchema");
    assert_eq!(range_schema.fields.len(), 2);

    // Test HashRange schema with ultra-minimal fields
    let hashrange_json = r#"
    {
      "name": "HashRangeSchema",
      "schema_type": "HashRange",
      "key": {
        "hash_field": "Source.map().category",
        "range_field": "Source.map().timestamp"
      },
      "fields": {
        "category": {},
        "timestamp": {},
        "value": {}
      },
      "payment_config": {
        "base_multiplier": 1.0,
        "min_payment_threshold": 0
      }
    }
    "#;

    let hashrange_schema: DeclarativeSchemaDefinition =
        serde_json::from_str(hashrange_json).unwrap();
    assert_eq!(hashrange_schema.name, "HashRangeSchema");
    assert_eq!(hashrange_schema.fields.len(), 3);
}

#[test]
fn test_mixed_format_with_all_field_combinations() {
    let json = r#"
    {
      "name": "ComprehensiveMixedSchema",
      "schema_type": "Single",
      "fields": {
        "string_field": "Source.map().id",
        "object_field_with_atom_uuid": {
          "atom_uuid": "Source.map().metadata"
        },
        "object_field_with_field_type": {
          "field_type": "Single"
        },
        "object_field_with_both": {
          "atom_uuid": "Source.map().data",
          "field_type": "Single"
        },
        "empty_object_field": {}
      }
    }
    "#;

    let schema: DeclarativeSchemaDefinition = serde_json::from_str(json).unwrap();
    assert_eq!(schema.name, "ComprehensiveMixedSchema");
    assert_eq!(schema.fields.len(), 5);

    // Verify string field
    let string_field = schema.fields.get("string_field").unwrap();
    assert_eq!(string_field.atom_uuid, Some("Source.map().id".to_string()));
    assert_eq!(string_field.field_type, None);

    // Verify object field with atom_uuid only
    let object_atom_field = schema.fields.get("object_field_with_atom_uuid").unwrap();
    assert_eq!(
        object_atom_field.atom_uuid,
        Some("Source.map().metadata".to_string())
    );
    assert_eq!(object_atom_field.field_type, None);

    // Verify object field with field_type only
    let object_type_field = schema.fields.get("object_field_with_field_type").unwrap();
    assert_eq!(object_type_field.atom_uuid, None);
    assert_eq!(object_type_field.field_type, Some("Single".to_string()));

    // Verify object field with both
    let object_both_field = schema.fields.get("object_field_with_both").unwrap();
    assert_eq!(
        object_both_field.atom_uuid,
        Some("Source.map().data".to_string())
    );
    assert_eq!(object_both_field.field_type, Some("Single".to_string()));

    // Verify empty object field
    let empty_field = schema.fields.get("empty_object_field").unwrap();
    assert_eq!(empty_field.atom_uuid, None);
    assert_eq!(empty_field.field_type, None);
}

#[test]
fn test_declarative_schema_with_complex_expressions() {
    let json = r#"
    {
      "name": "ComplexExpressionSchema",
      "schema_type": "HashRange",
      "key": {
        "hash_field": "BlogPost.map().content.split_by_word().map()",
        "range_field": "BlogPost.map().publish_date"
      },
      "fields": {
        "simple_field": "Source.map().id",
        "complex_field": "BlogPost.map().content.split_by_word().map()",
        "nested_field": "BlogPost.map().author.profile.name",
        "array_field": "BlogPost.map().tags",
        "mixed_field": {
          "atom_uuid": "BlogPost.map().metadata.tags",
          "field_type": "Single"
        }
      }
    }
    "#;

    let schema: DeclarativeSchemaDefinition = serde_json::from_str(json).unwrap();
    assert_eq!(schema.name, "ComplexExpressionSchema");
    assert_eq!(schema.fields.len(), 5);

    // Verify complex expressions are preserved
    let complex_field = schema.fields.get("complex_field").unwrap();
    assert_eq!(
        complex_field.atom_uuid,
        Some("BlogPost.map().content.split_by_word().map()".to_string())
    );

    let nested_field = schema.fields.get("nested_field").unwrap();
    assert_eq!(
        nested_field.atom_uuid,
        Some("BlogPost.map().author.profile.name".to_string())
    );

    let array_field = schema.fields.get("array_field").unwrap();
    assert_eq!(
        array_field.atom_uuid,
        Some("BlogPost.map().tags".to_string())
    );
}

#[test]
fn test_schema_with_special_characters_in_expressions() {
    let json = r#"
    {
      "name": "SpecialCharSchema",
      "schema_type": "Single",
      "fields": {
        "field_with_dots": "Source.map().field.with.dots",
        "field_with_underscores": "Source.map().field_with_underscores",
        "field_with_dashes": "Source.map().field-with-dashes",
        "field_with_numbers": "Source.map().field123",
        "field_with_mixed": "Source.map().field_123-with.dots"
      }
    }
    "#;

    let schema: DeclarativeSchemaDefinition = serde_json::from_str(json).unwrap();
    assert_eq!(schema.name, "SpecialCharSchema");
    assert_eq!(schema.fields.len(), 5);

    // Verify special characters are preserved
    let dots_field = schema.fields.get("field_with_dots").unwrap();
    assert_eq!(
        dots_field.atom_uuid,
        Some("Source.map().field.with.dots".to_string())
    );

    let underscores_field = schema.fields.get("field_with_underscores").unwrap();
    assert_eq!(
        underscores_field.atom_uuid,
        Some("Source.map().field_with_underscores".to_string())
    );

    let dashes_field = schema.fields.get("field_with_dashes").unwrap();
    assert_eq!(
        dashes_field.atom_uuid,
        Some("Source.map().field-with-dashes".to_string())
    );

    let numbers_field = schema.fields.get("field_with_numbers").unwrap();
    assert_eq!(
        numbers_field.atom_uuid,
        Some("Source.map().field123".to_string())
    );

    let mixed_field = schema.fields.get("field_with_mixed").unwrap();
    assert_eq!(
        mixed_field.atom_uuid,
        Some("Source.map().field_123-with.dots".to_string())
    );
}

#[test]
fn test_schema_with_empty_and_null_values() {
    let json = r#"
    {
      "name": "EmptyNullSchema",
      "schema_type": "Single",
      "fields": {
        "empty_string_field": "",
        "null_field": null,
        "empty_object_field": {},
        "valid_field": "Source.map().id"
      }
    }
    "#;

    let result: Result<DeclarativeSchemaDefinition, _> = serde_json::from_str(json);

    // Empty string and null should fail deserialization
    assert!(result.is_err());
    let error = result.unwrap_err();
    let error_msg = format!("{}", error);
    assert!(error_msg.contains("must be either a string expression or a FieldDefinition object"));
}

#[test]
fn test_schema_with_very_long_expressions() {
    let long_expression = "Source.map().very.long.expression.with.many.dots.and.nested.fields.that.goes.on.and.on.and.on";

    let json = format!(
        r#"
    {{
      "name": "LongExpressionSchema",
      "schema_type": "Single",
      "fields": {{
        "long_field": "{}"
      }}
    }}
    "#,
        long_expression
    );

    let schema: DeclarativeSchemaDefinition = serde_json::from_str(&json).unwrap();
    assert_eq!(schema.name, "LongExpressionSchema");
    assert_eq!(schema.fields.len(), 1);

    let long_field = schema.fields.get("long_field").unwrap();
    assert_eq!(long_field.atom_uuid, Some(long_expression.to_string()));
}

#[test]
fn test_schema_with_unicode_characters() {
    let json = r#"
    {
      "name": "UnicodeSchema",
      "schema_type": "Single",
      "fields": {
        "unicode_field": "Source.map().字段.数据.信息",
        "emoji_field": "Source.map().🚀.🌟.💫",
        "mixed_unicode_field": "Source.map().field_字段.emoji_🚀"
      }
    }
    "#;

    let schema: DeclarativeSchemaDefinition = serde_json::from_str(json).unwrap();
    assert_eq!(schema.name, "UnicodeSchema");
    assert_eq!(schema.fields.len(), 3);

    // Verify unicode characters are preserved
    let unicode_field = schema.fields.get("unicode_field").unwrap();
    assert_eq!(
        unicode_field.atom_uuid,
        Some("Source.map().字段.数据.信息".to_string())
    );

    let emoji_field = schema.fields.get("emoji_field").unwrap();
    assert_eq!(
        emoji_field.atom_uuid,
        Some("Source.map().🚀.🌟.💫".to_string())
    );

    let mixed_field = schema.fields.get("mixed_unicode_field").unwrap();
    assert_eq!(
        mixed_field.atom_uuid,
        Some("Source.map().field_字段.emoji_🚀".to_string())
    );
}

#[test]
fn test_schema_performance_with_many_fields() {
    let mut fields = HashMap::new();
    for i in 0..100 {
        fields.insert(
            format!("field_{}", i),
            FieldDefinition {
                atom_uuid: Some(format!("Source.map().field_{}", i)),
                field_type: None,
            },
        );
    }

    let schema = DeclarativeSchemaDefinition {
        name: "PerformanceTestSchema".to_string(),
        schema_type: SchemaType::Single,
        key: None,
        fields,
    };

    // Test serialization performance
    let start = std::time::Instant::now();
    let serialized = serde_json::to_string(&schema).unwrap();
    let serialize_time = start.elapsed();

    // Test deserialization performance
    let start = std::time::Instant::now();
    let deserialized: DeclarativeSchemaDefinition = serde_json::from_str(&serialized).unwrap();
    let deserialize_time = start.elapsed();

    // Verify structure is preserved
    assert_eq!(deserialized.name, "PerformanceTestSchema");
    assert_eq!(deserialized.fields.len(), 100);

    // Performance should be reasonable (less than 10ms for 100 fields)
    assert!(
        serialize_time.as_millis() < 10,
        "Serialization took too long: {:?}",
        serialize_time
    );
    assert!(
        deserialize_time.as_millis() < 10,
        "Deserialization took too long: {:?}",
        deserialize_time
    );
}

#[test]
fn test_schema_with_edge_case_field_names() {
    let json = r#"
    {
      "name": "EdgeCaseFieldNames",
      "schema_type": "Single",
      "fields": {
        "a": "Source.map().a",
        "field_with_spaces": "Source.map().field with spaces",
        "field_with_quotes": "Source.map().field\"with\"quotes",
        "field_with_backslashes": "Source.map().field\\with\\backslashes",
        "field_with_newlines": "Source.map().field\nwith\nnewlines"
      }
    }
    "#;

    let schema: DeclarativeSchemaDefinition = serde_json::from_str(json).unwrap();
    assert_eq!(schema.name, "EdgeCaseFieldNames");
    assert_eq!(schema.fields.len(), 5);

    // Verify edge case field names are preserved
    let single_char_field = schema.fields.get("a").unwrap();
    assert_eq!(
        single_char_field.atom_uuid,
        Some("Source.map().a".to_string())
    );

    let spaces_field = schema.fields.get("field_with_spaces").unwrap();
    assert_eq!(
        spaces_field.atom_uuid,
        Some("Source.map().field with spaces".to_string())
    );

    let quotes_field = schema.fields.get("field_with_quotes").unwrap();
    assert_eq!(
        quotes_field.atom_uuid,
        Some("Source.map().field\"with\"quotes".to_string())
    );

    let backslashes_field = schema.fields.get("field_with_backslashes").unwrap();
    assert_eq!(
        backslashes_field.atom_uuid,
        Some("Source.map().field\\with\\backslashes".to_string())
    );

    let newlines_field = schema.fields.get("field_with_newlines").unwrap();
    assert_eq!(
        newlines_field.atom_uuid,
        Some("Source.map().field\nwith\nnewlines".to_string())
    );
}

#[test]
fn test_schema_validation_with_simplified_formats() {
    let json = r#"
    {
      "name": "ValidationTestSchema",
      "schema_type": "HashRange",
      "key": {
        "hash_field": "Source.map().category",
        "range_field": "Source.map().timestamp"
      },
      "fields": {
        "category": "Source.map().category",
        "timestamp": "Source.map().timestamp",
        "value": "Source.map().value"
      }
    }
    "#;

    let schema: DeclarativeSchemaDefinition = serde_json::from_str(json).unwrap();

    // Test that validation works with simplified formats
    let validation_result = schema.validate();
    assert!(
        validation_result.is_ok(),
        "Schema validation failed: {:?}",
        validation_result
    );
}

#[test]
fn test_schema_round_trip_with_all_formats() {
    // Test that all format combinations survive round-trip serialization
    let original_json = r#"
    {
      "name": "RoundTripTestSchema",
      "schema_type": "Single",
      "fields": {
        "string_field": "Source.map().id",
        "object_field": {
          "atom_uuid": "Source.map().data",
          "field_type": "Single"
        },
        "empty_field": {}
      }
    }
    "#;

    let original_schema: DeclarativeSchemaDefinition = serde_json::from_str(original_json).unwrap();

    // Serialize and deserialize
    let serialized = serde_json::to_string(&original_schema).unwrap();
    let deserialized: DeclarativeSchemaDefinition = serde_json::from_str(&serialized).unwrap();

    // Verify structure is preserved
    assert_eq!(deserialized.name, original_schema.name);
    assert_eq!(deserialized.fields.len(), original_schema.fields.len());

    // Verify each field type is preserved
    let string_field = deserialized.fields.get("string_field").unwrap();
    assert_eq!(string_field.atom_uuid, Some("Source.map().id".to_string()));
    assert_eq!(string_field.field_type, None);

    let object_field = deserialized.fields.get("object_field").unwrap();
    assert_eq!(
        object_field.atom_uuid,
        Some("Source.map().data".to_string())
    );
    assert_eq!(object_field.field_type, Some("Single".to_string()));

    let empty_field = deserialized.fields.get("empty_field").unwrap();
    assert_eq!(empty_field.atom_uuid, None);
    assert_eq!(empty_field.field_type, None);
}
