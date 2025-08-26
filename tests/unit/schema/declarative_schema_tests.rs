use datafold::schema::types::{
    DeclarativeSchemaDefinition, FieldDefinition, KeyConfig, SchemaType,
};
use std::collections::HashMap;

#[test]
fn schema_type_hashrange_serializes() {
    let ty = SchemaType::HashRange {
        hash_key: "h".into(),
        range_key: "r".into(),
    };
    let json = serde_json::to_string(&ty).unwrap();
    assert!(json.contains("HashRange"));
}

#[test]
fn key_config_serializes() {
    let key = KeyConfig {
        hash_field: "user_id".into(),
        range_field: "ts".into(),
    };
    let json = serde_json::to_string(&key).unwrap();
    assert_eq!(json, "{\"hash_field\":\"user_id\",\"range_field\":\"ts\"}");
}

#[test]
fn declarative_schema_validate() {
    let mut fields = HashMap::new();
    fields.insert(
        "id".into(),
        FieldDefinition {
            atom_uuid: Some("atom".into()),
            field_type: None,
        },
    );
    let schema = DeclarativeSchemaDefinition {
        name: "test".into(),
        schema_type: SchemaType::HashRange {
            hash_key: "id".into(),
            range_key: "ts".into(),
        },
        key: Some(KeyConfig {
            hash_field: "id".into(),
            range_field: "ts".into(),
        }),
        fields,
    };
    assert!(schema.validate().is_ok());
}

#[test]
fn declarative_schema_validate_errors_without_key() {
    let schema = DeclarativeSchemaDefinition {
        name: "test".into(),
        schema_type: SchemaType::HashRange {
            hash_key: "id".into(),
            range_key: "ts".into(),
        },
        key: None,
        fields: HashMap::new(),
    };
    assert!(schema.validate().is_err());
}

#[test]
fn field_definition_requires_atom_or_type() {
    let mut fields = HashMap::new();
    fields.insert(
        "id".into(),
        FieldDefinition {
            atom_uuid: None,
            field_type: None,
        },
    );
    let schema = DeclarativeSchemaDefinition {
        name: "test".into(),
        schema_type: SchemaType::Single,
        key: None,
        fields,
    };
    assert!(schema.validate().is_err());
}
