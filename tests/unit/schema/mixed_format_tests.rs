use std::collections::HashMap;

use datafold::schema::types::json_schema::{DeclarativeSchemaDefinition, FieldDefinition};
use datafold::schema::types::schema::SchemaType;

/// Tests for mixed format support in DeclarativeSchemaDefinition
/// This validates that schemas can contain both string expressions and FieldDefinition objects

#[test]
fn test_mixed_format_schema_with_strings_and_objects() {
    let json = r#"
    {
      "name": "BlogPostWordIndex",
      "schema_type": "HashRange",
      "key": {
        "hash_field": "BlogPost.map().content.split_by_word().map()",
        "range_field": "BlogPost.map().publish_date"
      },
      "fields": {
        "word": "BlogPost.map().content.split_by_word().map()",
        "blogpost_id": {
          "atom_uuid": "BlogPost.map().id",
          "field_type": "Single"
        },
        "publish_date": "BlogPost.map().publish_date",
        "author": {
          "atom_uuid": "BlogPost.map().author"
        }
      }
    }
    "#;

    let schema: DeclarativeSchemaDefinition = serde_json::from_str(json).unwrap();

    // Verify schema structure
    assert_eq!(schema.name, "BlogPostWordIndex");
    assert_eq!(schema.fields.len(), 4);

    // Verify string expression fields are converted to FieldDefinition with atom_uuid
    let word_field = schema.fields.get("word").unwrap();
    assert_eq!(
        word_field.atom_uuid,
        Some("BlogPost.map().content.split_by_word().map()".to_string())
    );
    assert_eq!(word_field.field_type, None);

    let publish_date_field = schema.fields.get("publish_date").unwrap();
    assert_eq!(
        publish_date_field.atom_uuid,
        Some("BlogPost.map().publish_date".to_string())
    );
    assert_eq!(publish_date_field.field_type, None);

    // Verify FieldDefinition object fields are preserved
    let blogpost_id_field = schema.fields.get("blogpost_id").unwrap();
    assert_eq!(
        blogpost_id_field.atom_uuid,
        Some("BlogPost.map().id".to_string())
    );
    assert_eq!(blogpost_id_field.field_type, Some("Single".to_string()));

    let author_field = schema.fields.get("author").unwrap();
    assert_eq!(
        author_field.atom_uuid,
        Some("BlogPost.map().author".to_string())
    );
    assert_eq!(author_field.field_type, None);

    // Verify schema_type
    match schema.schema_type {
        SchemaType::HashRange => {
            // HashRange is a unit variant, key configuration is in the key field
        }
        _ => panic!("Expected HashRange schema type"),
    }

    // Verify key configuration
    let key_config = schema.key.unwrap();
    assert_eq!(
        key_config.hash_field,
        "BlogPost.map().content.split_by_word().map()"
    );
    assert_eq!(key_config.range_field, "BlogPost.map().publish_date");
}

#[test]
fn test_all_string_format_schema() {
    let json = r#"
    {
      "name": "SimpleIndex",
      "schema_type": "Single",
      "fields": {
        "id": "Source.map().id",
        "name": "Source.map().name",
        "value": "Source.map().value"
      }
    }
    "#;

    let schema: DeclarativeSchemaDefinition = serde_json::from_str(json).unwrap();

    // Verify schema structure
    assert_eq!(schema.name, "SimpleIndex");
    assert_eq!(schema.fields.len(), 3);

    // Verify all fields are converted from strings to FieldDefinition
    for (field_name, expected_expression) in [
        ("id", "Source.map().id"),
        ("name", "Source.map().name"),
        ("value", "Source.map().value"),
    ] {
        let field = schema.fields.get(field_name).unwrap();
        assert_eq!(field.atom_uuid, Some(expected_expression.to_string()));
        assert_eq!(field.field_type, None);
    }

    // Verify schema_type
    match schema.schema_type {
        SchemaType::Single => {
            // Single schema type is correct
        }
        _ => panic!("Expected Single schema type"),
    }
}

#[test]
fn test_all_object_format_schema() {
    let json = r#"
    {
      "name": "ComplexIndex",
      "schema_type": {
        "Range": {
          "range_key": "timestamp"
        }
      },
      "fields": {
        "id": {
          "atom_uuid": "Source.map().id",
          "field_type": "Single"
        },
        "timestamp": {
          "atom_uuid": "Source.map().timestamp"
        },
        "metadata": {
          "atom_uuid": "Source.map().metadata",
          "field_type": "Single"
        }
      }
    }
    "#;

    let schema: DeclarativeSchemaDefinition = serde_json::from_str(json).unwrap();

    // Verify schema structure
    assert_eq!(schema.name, "ComplexIndex");
    assert_eq!(schema.fields.len(), 3);

    // Verify all fields are preserved as FieldDefinition objects
    let id_field = schema.fields.get("id").unwrap();
    assert_eq!(id_field.atom_uuid, Some("Source.map().id".to_string()));
    assert_eq!(id_field.field_type, Some("Single".to_string()));

    let timestamp_field = schema.fields.get("timestamp").unwrap();
    assert_eq!(
        timestamp_field.atom_uuid,
        Some("Source.map().timestamp".to_string())
    );
    assert_eq!(timestamp_field.field_type, None);

    let metadata_field = schema.fields.get("metadata").unwrap();
    assert_eq!(
        metadata_field.atom_uuid,
        Some("Source.map().metadata".to_string())
    );
    assert_eq!(metadata_field.field_type, Some("Single".to_string()));

    // Verify schema_type
    match schema.schema_type {
        SchemaType::Range { range_key } => {
            assert_eq!(range_key, "timestamp");
        }
        _ => panic!("Expected Range schema type"),
    }
}

#[test]
fn test_empty_fields_schema() {
    let json = r#"
    {
      "name": "EmptyFieldsIndex",
      "schema_type": "Single",
      "fields": {}
    }
    "#;

    let result: Result<DeclarativeSchemaDefinition, _> = serde_json::from_str(json);

    // Empty fields should be allowed during deserialization
    // but validation should catch it later
    let schema = result.unwrap();
    assert_eq!(schema.name, "EmptyFieldsIndex");
    assert_eq!(schema.fields.len(), 0);
}

#[test]
fn test_invalid_field_type() {
    let json = r#"
    {
      "name": "InvalidIndex",
      "schema_type": "Single",
      "fields": {
        "valid_field": "Source.map().id",
        "invalid_field": 123
      }
    }
    "#;

    let result: Result<DeclarativeSchemaDefinition, _> = serde_json::from_str(json);

    // Should fail with clear error message
    assert!(result.is_err());
    let error = result.unwrap_err();
    let error_msg = format!("{}", error);
    assert!(error_msg.contains("must be either a string expression or a FieldDefinition object"));
    assert!(error_msg.contains("invalid_field"));
}

#[test]
fn test_field_definition_with_unknown_fields() {
    let json = r#"
    {
      "name": "UnknownFieldsIndex",
      "schema_type": "Single",
      "fields": {
        "valid_field": "Source.map().id",
        "field_with_unknown_property": {
          "atom_uuid": "Source.map().id",
          "unknown_property": "value"
        }
      }
    }
    "#;

    let result: Result<DeclarativeSchemaDefinition, _> = serde_json::from_str(json);

    // Should succeed - unknown fields are ignored for backward compatibility
    let schema = result.unwrap();
    assert_eq!(schema.name, "UnknownFieldsIndex");
    assert_eq!(schema.fields.len(), 2);

    let field_with_unknown = schema.fields.get("field_with_unknown_property").unwrap();
    assert_eq!(
        field_with_unknown.atom_uuid,
        Some("Source.map().id".to_string())
    );
    assert_eq!(field_with_unknown.field_type, None);
}

#[test]
fn test_serialization_preserves_structure() {
    // Create a schema programmatically
    let mut fields = HashMap::new();
    fields.insert(
        "word".to_string(),
        FieldDefinition {
            atom_uuid: Some("BlogPost.map().content.split_by_word().map()".to_string()),
            field_type: None,
        },
    );
    fields.insert(
        "blogpost_id".to_string(),
        FieldDefinition {
            atom_uuid: Some("BlogPost.map().id".to_string()),
            field_type: Some("Single".to_string()),
        },
    );

    let schema = DeclarativeSchemaDefinition {
        name: "TestIndex".to_string(),
        schema_type: SchemaType::Single,
        key: None,
        fields,
    };

    // Serialize and deserialize
    let serialized = serde_json::to_string(&schema).unwrap();
    let deserialized: DeclarativeSchemaDefinition = serde_json::from_str(&serialized).unwrap();

    // Verify structure is preserved
    assert_eq!(deserialized.name, "TestIndex");
    assert_eq!(deserialized.fields.len(), 2);

    let word_field = deserialized.fields.get("word").unwrap();
    assert_eq!(
        word_field.atom_uuid,
        Some("BlogPost.map().content.split_by_word().map()".to_string())
    );
    assert_eq!(word_field.field_type, None);

    let blogpost_id_field = deserialized.fields.get("blogpost_id").unwrap();
    assert_eq!(
        blogpost_id_field.atom_uuid,
        Some("BlogPost.map().id".to_string())
    );
    assert_eq!(blogpost_id_field.field_type, Some("Single".to_string()));
}

#[test]
fn test_complex_mixed_format_schema() {
    let json = r#"
    {
      "name": "ComplexMixedIndex",
      "schema_type": "HashRange",
      "key": {
        "hash_field": "Source.map().category",
        "range_field": "Source.map().timestamp"
      },
      "fields": {
        "category": "Source.map().category",
        "timestamp": "Source.map().timestamp",
        "id": {
          "atom_uuid": "Source.map().id",
          "field_type": "Single"
        },
        "metadata": {
          "atom_uuid": "Source.map().metadata"
        },
        "tags": "Source.map().tags",
        "status": {
          "atom_uuid": "Source.map().status",
          "field_type": "Single"
        }
      }
    }
    "#;

    let schema: DeclarativeSchemaDefinition = serde_json::from_str(json).unwrap();

    // Verify schema structure
    assert_eq!(schema.name, "ComplexMixedIndex");
    assert_eq!(schema.fields.len(), 6);

    // Verify string expression fields
    let string_fields = ["category", "timestamp", "tags"];
    for field_name in string_fields {
        let field = schema.fields.get(field_name).unwrap();
        assert!(field.atom_uuid.is_some());
        assert_eq!(field.field_type, None);
    }

    // Verify FieldDefinition object fields
    let object_fields = ["id", "metadata", "status"];
    for field_name in object_fields {
        let field = schema.fields.get(field_name).unwrap();
        assert!(field.atom_uuid.is_some());
    }

    // Verify specific field types
    let id_field = schema.fields.get("id").unwrap();
    assert_eq!(id_field.field_type, Some("Single".to_string()));

    let status_field = schema.fields.get("status").unwrap();
    assert_eq!(status_field.field_type, Some("Single".to_string()));

    let metadata_field = schema.fields.get("metadata").unwrap();
    assert_eq!(metadata_field.field_type, None);

    // Verify schema_type
    match schema.schema_type {
        SchemaType::HashRange => {
            // HashRange is a unit variant, key configuration is in the key field
        }
        _ => panic!("Expected HashRange schema type"),
    }
}
