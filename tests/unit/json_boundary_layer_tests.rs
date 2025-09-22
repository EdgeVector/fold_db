use datafold::api::{JsonBoundaryError, JsonBoundaryLayer, JsonBoundarySchema};
use datafold::transform::{FieldValue, NativeFieldDefinition, NativeFieldType};
use serde_json::json;
use std::collections::HashMap;

const SCHEMA_NAME: &str = "UserProfile";
const USERNAME_FIELD: &str = "username";
const AGE_FIELD: &str = "age";
const EXTRA_FIELD: &str = "notes";
const DEFAULT_AGE: i64 = 42;

fn create_base_schema() -> JsonBoundarySchema {
    let username = NativeFieldDefinition::new(USERNAME_FIELD, NativeFieldType::String);
    let age = NativeFieldDefinition::new(AGE_FIELD, NativeFieldType::Integer)
        .with_required(false)
        .with_default(FieldValue::Integer(DEFAULT_AGE));

    JsonBoundarySchema::from_definitions(SCHEMA_NAME, vec![username, age])
}

#[test]
fn json_to_native_validates_and_applies_defaults() {
    let mut layer = JsonBoundaryLayer::new();
    layer.register_schema(create_base_schema()).unwrap();

    let payload = json!({
        USERNAME_FIELD: "ada",
        AGE_FIELD: 31,
    });

    let native = layer
        .json_to_native(SCHEMA_NAME, &payload)
        .expect("json_to_native should succeed for valid payload");

    assert_eq!(
        native.get(USERNAME_FIELD),
        Some(&FieldValue::String("ada".to_string()))
    );
    assert_eq!(native.get(AGE_FIELD), Some(&FieldValue::Integer(31)));

    let payload_missing_age = json!({ USERNAME_FIELD: "grace" });
    let native_missing = layer
        .json_to_native(SCHEMA_NAME, &payload_missing_age)
        .expect("json_to_native should apply defaults when optional field missing");

    assert_eq!(
        native_missing.get(AGE_FIELD),
        Some(&FieldValue::Integer(DEFAULT_AGE))
    );
}

#[test]
fn json_to_native_rejects_unknown_fields() {
    let mut layer = JsonBoundaryLayer::new();
    layer.register_schema(create_base_schema()).unwrap();

    let payload = json!({
        USERNAME_FIELD: "ada",
        EXTRA_FIELD: "unexpected",
    });

    let error = layer
        .json_to_native(SCHEMA_NAME, &payload)
        .expect_err("unknown field should be rejected");

    match error {
        JsonBoundaryError::UnknownField { field, .. } => {
            assert_eq!(field, EXTRA_FIELD.to_string());
        }
        other => panic!("expected UnknownField error, got {other:?}"),
    }
}

#[test]
fn native_to_json_rejects_type_mismatches() {
    let mut layer = JsonBoundaryLayer::new();
    layer.register_schema(create_base_schema()).unwrap();

    let mut native = HashMap::new();
    native.insert(USERNAME_FIELD.to_string(), FieldValue::Integer(7));

    let error = layer
        .native_to_json(SCHEMA_NAME, &native)
        .expect_err("type mismatch should be rejected");

    match error {
        JsonBoundaryError::TypeMismatch { field, .. } => {
            assert_eq!(field, USERNAME_FIELD.to_string());
        }
        other => panic!("expected TypeMismatch error, got {other:?}"),
    }
}

#[test]
fn register_schema_reports_invalid_definitions() {
    let invalid_default = NativeFieldDefinition::new(USERNAME_FIELD, NativeFieldType::String)
        .with_default(FieldValue::Integer(5));
    let schema = JsonBoundarySchema::from_definitions(SCHEMA_NAME, vec![invalid_default]);

    let mut layer = JsonBoundaryLayer::new();
    let error = layer
        .register_schema(schema)
        .expect_err("schema registration should fail when defaults mismatch types");

    match error {
        JsonBoundaryError::InvalidFieldDefinition { field, .. } => {
            assert_eq!(field, USERNAME_FIELD.to_string());
        }
        other => panic!("expected InvalidFieldDefinition error, got {other:?}"),
    }
}

#[test]
fn additional_fields_flow_when_allowed() {
    let schema = create_base_schema().allow_additional_fields(true);
    let mut layer = JsonBoundaryLayer::new();
    layer.register_schema(schema).unwrap();

    let payload = json!({
        USERNAME_FIELD: "ada",
        EXTRA_FIELD: "notes",
    });

    let native = layer
        .json_to_native(SCHEMA_NAME, &payload)
        .expect("json_to_native should pass through additional fields when allowed");

    assert_eq!(
        native.get(EXTRA_FIELD),
        Some(&FieldValue::String("notes".to_string()))
    );

    let json_value = layer
        .native_to_json(SCHEMA_NAME, &native)
        .expect("native_to_json should re-emit additional fields");

    let object = json_value
        .as_object()
        .expect("native_to_json should always return an object");

    assert_eq!(
        object.get(EXTRA_FIELD).and_then(|v| v.as_str()),
        Some("notes")
    );
}
