use std::collections::HashMap;

use datafold::schema::types::json_schema::{
    DeclarativeSchemaDefinition, FieldDefinition, KeyConfig,
};
use datafold::schema::types::schema::SchemaType;

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
