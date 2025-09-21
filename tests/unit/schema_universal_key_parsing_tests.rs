use std::collections::HashMap;

use datafold::schema::types::json_schema::{
    DeclarativeSchemaDefinition, FieldDefinition, KeyConfig,
};
use datafold::schema::types::schema::SchemaType;

fn minimal_fields() -> HashMap<String, FieldDefinition> {
    let mut fields = HashMap::new();
    fields.insert(
        "key".to_string(),
        FieldDefinition {
            atom_uuid: Some("input.map().$atom_uuid".to_string()),
            field_type: Some("String".to_string()),
        },
    );
    fields
}

#[test]
fn single_schema_parsing_without_key() {
    let json = r#"
    {
        "name": "SingleWithoutKey",
        "schema_type": "Single",
        "fields": {
            "key": "input.map().$atom_uuid"
        }
    }
    "#;

    let result: Result<DeclarativeSchemaDefinition, _> = serde_json::from_str(json);
    assert!(result.is_ok());

    let schema = result.unwrap();
    assert_eq!(schema.name, "SingleWithoutKey");
    assert_eq!(schema.schema_type, SchemaType::Single);
    assert!(schema.key.is_none());
    assert_eq!(schema.fields.len(), 1);
}

#[test]
fn single_schema_parsing_with_key() {
    let json = r#"
    {
        "name": "SingleWithKey",
        "schema_type": "Single",
        "key": {
            "hash_field": "input.map().user_id",
            "range_field": "input.map().timestamp"
        },
        "fields": {
            "key": "input.map().$atom_uuid"
        }
    }
    "#;

    let result: Result<DeclarativeSchemaDefinition, _> = serde_json::from_str(json);
    assert!(result.is_ok());

    let schema = result.unwrap();
    assert_eq!(schema.name, "SingleWithKey");
    assert_eq!(schema.schema_type, SchemaType::Single);
    assert!(schema.key.is_some());

    let key = schema.key.unwrap();
    assert_eq!(key.hash_field, "input.map().user_id");
    assert_eq!(key.range_field, "input.map().timestamp");
}

#[test]
fn range_schema_parsing_without_key() {
    let json = r#"
    {
        "name": "RangeWithoutKey",
        "schema_type": {
            "Range": {
                "range_key": "timestamp"
            }
        },
        "fields": {
            "timestamp": "input.map().timestamp",
            "data": "input.map().data"
        }
    }
    "#;

    let result: Result<DeclarativeSchemaDefinition, _> = serde_json::from_str(json);
    assert!(result.is_ok());

    let schema = result.unwrap();
    assert_eq!(schema.name, "RangeWithoutKey");
    assert!(
        matches!(schema.schema_type, SchemaType::Range { range_key } if range_key == "timestamp")
    );
    assert!(schema.key.is_none());
    assert_eq!(schema.fields.len(), 2);
}

#[test]
fn range_schema_parsing_with_key() {
    let json = r#"
    {
        "name": "RangeWithKey",
        "schema_type": {
            "Range": {
                "range_key": "timestamp"
            }
        },
        "key": {
            "hash_field": "input.map().user_id",
            "range_field": "input.map().timestamp"
        },
        "fields": {
            "timestamp": "input.map().timestamp",
            "data": "input.map().data"
        }
    }
    "#;

    let result: Result<DeclarativeSchemaDefinition, _> = serde_json::from_str(json);
    assert!(result.is_ok());

    let schema = result.unwrap();
    assert_eq!(schema.name, "RangeWithKey");
    assert!(
        matches!(schema.schema_type, SchemaType::Range { range_key } if range_key == "timestamp")
    );
    assert!(schema.key.is_some());

    let key = schema.key.unwrap();
    assert_eq!(key.hash_field, "input.map().user_id");
    assert_eq!(key.range_field, "input.map().timestamp");
}

#[test]
fn hashrange_schema_parsing_without_key() {
    let json = r#"
    {
        "name": "HashRangeWithoutKey",
        "schema_type": "HashRange",
        "fields": {
            "data": "input.map().data"
        }
    }
    "#;

    let result: Result<DeclarativeSchemaDefinition, _> = serde_json::from_str(json);
    assert!(result.is_ok());

    let schema = result.unwrap();
    assert_eq!(schema.name, "HashRangeWithoutKey");
    assert_eq!(schema.schema_type, SchemaType::HashRange);
    assert!(schema.key.is_none());
    assert_eq!(schema.fields.len(), 1);
}

#[test]
fn hashrange_schema_parsing_with_key() {
    let json = r#"
    {
        "name": "HashRangeWithKey",
        "schema_type": "HashRange",
        "key": {
            "hash_field": "input.map().user_id",
            "range_field": "input.map().timestamp"
        },
        "fields": {
            "data": "input.map().data"
        }
    }
    "#;

    let result: Result<DeclarativeSchemaDefinition, _> = serde_json::from_str(json);
    assert!(result.is_ok());

    let schema = result.unwrap();
    assert_eq!(schema.name, "HashRangeWithKey");
    assert_eq!(schema.schema_type, SchemaType::HashRange);
    assert!(schema.key.is_some());

    let key = schema.key.unwrap();
    assert_eq!(key.hash_field, "input.map().user_id");
    assert_eq!(key.range_field, "input.map().timestamp");
}

#[test]
fn key_parsing_with_empty_fields() {
    let json = r#"
    {
        "name": "EmptyKeyFields",
        "schema_type": "Single",
        "key": {
            "hash_field": "",
            "range_field": ""
        },
        "fields": {
            "data": "input.map().data"
        }
    }
    "#;

    let result: Result<DeclarativeSchemaDefinition, _> = serde_json::from_str(json);
    assert!(result.is_ok());

    let schema = result.unwrap();
    assert!(schema.key.is_some());

    let key = schema.key.unwrap();
    assert_eq!(key.hash_field, "");
    assert_eq!(key.range_field, "");
}

#[test]
fn key_parsing_with_partial_fields() {
    let json = r#"
    {
        "name": "PartialKeyFields",
        "schema_type": "Single",
        "key": {
            "hash_field": "input.map().user_id"
        },
        "fields": {
            "data": "input.map().data"
        }
    }
    "#;

    let result: Result<DeclarativeSchemaDefinition, _> = serde_json::from_str(json);
    assert!(result.is_ok());

    let schema = result.unwrap();
    assert!(schema.key.is_some());

    let key = schema.key.unwrap();
    assert_eq!(key.hash_field, "input.map().user_id");
    assert_eq!(key.range_field, ""); // Should default to empty string
}

#[test]
fn backward_compatibility_range_without_key() {
    // This test ensures legacy Range schemas without key still parse correctly
    let json = r#"
    {
        "name": "LegacyRange",
        "schema_type": {
            "Range": {
                "range_key": "created_at"
            }
        },
        "fields": {
            "created_at": "input.map().created_at",
            "content": "input.map().content"
        }
    }
    "#;

    let result: Result<DeclarativeSchemaDefinition, _> = serde_json::from_str(json);
    assert!(result.is_ok());

    let schema = result.unwrap();
    assert_eq!(schema.name, "LegacyRange");
    assert!(
        matches!(schema.schema_type, SchemaType::Range { range_key } if range_key == "created_at")
    );
    assert!(schema.key.is_none()); // Legacy schemas don't have key
    assert_eq!(schema.fields.len(), 2);
}

#[test]
fn serialization_roundtrip_with_key() {
    let original_schema = DeclarativeSchemaDefinition {
        name: "RoundtripTest".to_string(),
        schema_type: SchemaType::Single,
        key: Some(KeyConfig {
            hash_field: "input.map().user_id".to_string(),
            range_field: "input.map().timestamp".to_string(),
        }),
        fields: minimal_fields(),
    };

    // Serialize to JSON
    let json = serde_json::to_string(&original_schema).unwrap();

    // Deserialize back
    let deserialized: DeclarativeSchemaDefinition = serde_json::from_str(&json).unwrap();

    assert_eq!(original_schema.name, deserialized.name);
    assert_eq!(original_schema.schema_type, deserialized.schema_type);
    assert_eq!(original_schema.fields.len(), deserialized.fields.len());

    assert!(original_schema.key.is_some());
    assert!(deserialized.key.is_some());

    let original_key = original_schema.key.unwrap();
    let deserialized_key = deserialized.key.unwrap();

    assert_eq!(original_key.hash_field, deserialized_key.hash_field);
    assert_eq!(original_key.range_field, deserialized_key.range_field);
}

#[test]
fn serialization_roundtrip_without_key() {
    let original_schema = DeclarativeSchemaDefinition {
        name: "RoundtripNoKey".to_string(),
        schema_type: SchemaType::Single,
        key: None,
        fields: minimal_fields(),
    };

    // Serialize to JSON
    let json = serde_json::to_string(&original_schema).unwrap();

    // Verify key is omitted from serialization (should not contain top-level "key" field)
    // The JSON should not contain a top-level "key" property for KeyConfig
    // It may contain "key" as a field name, but not as a top-level key configuration
    assert!(!json.contains(",\"key\":{") && !json.contains("\"key\":{\"hash_field\""));

    // Deserialize back
    let deserialized: DeclarativeSchemaDefinition = serde_json::from_str(&json).unwrap();

    assert_eq!(original_schema.name, deserialized.name);
    assert_eq!(original_schema.schema_type, deserialized.schema_type);
    assert_eq!(original_schema.fields.len(), deserialized.fields.len());
    assert!(deserialized.key.is_none());
}
