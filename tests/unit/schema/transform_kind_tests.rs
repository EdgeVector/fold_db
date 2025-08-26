use datafold::schema::types::json_schema::{DeclarativeSchemaDefinition, TransformKind};

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
    let schema = DeclarativeSchemaDefinition::default();
    let kind = TransformKind::Declarative { schema };
    let json = serde_json::to_string(&kind).unwrap();
    assert_eq!(json, r#"{"kind":"declarative","schema":{}}"#);
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
