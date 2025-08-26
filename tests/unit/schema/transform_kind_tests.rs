use std::collections::HashMap;

use datafold::schema::types::json_schema::{DeclarativeSchemaDefinition, TransformKind};
use datafold::schema::types::schema::SchemaType;

#[test]
fn procedural_serialization() {
    let kind = TransformKind::Procedural {
        logic: "return x + 1".to_string(),
    };
    let json = serde_json::to_string(&kind).unwrap();
    assert_eq!(json, r#"{"kind":"procedural","logic":"return x + 1"}"#);
}

#[test]
fn declarative_serialization() {
    let schema = DeclarativeSchemaDefinition {
        name: "test".to_string(),
        schema_type: SchemaType::Single,
        key: None,
        fields: HashMap::new(),
    };
    let kind = TransformKind::Declarative { schema };
    let json = serde_json::to_value(&kind).unwrap();
    let expected = serde_json::json!({
        "kind": "declarative",
        "schema": {
            "name": "test",
            "schema_type": "Single",
            "fields": {}
        }
    });
    assert_eq!(json, expected);
}

#[test]
fn deserialization_roundtrip() {
    let json = r#"{"kind":"procedural","logic":"return y"}"#;
    let kind: TransformKind = serde_json::from_str(json).unwrap();
    assert_eq!(
        kind,
        TransformKind::Procedural {
            logic: "return y".to_string(),
        }
    );
}
