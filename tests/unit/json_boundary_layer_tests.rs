use datafold::api::{JsonBoundaryError, JsonBoundaryLayer, JsonBoundarySchema, SchemaInfo};
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

// Tests for new conversion utilities

#[test]
fn convert_json_value_validates_individual_fields() {
    let mut layer = JsonBoundaryLayer::new();
    layer.register_schema(create_base_schema()).unwrap();

    let result = layer
        .convert_json_value(SCHEMA_NAME, USERNAME_FIELD, json!("ada"))
        .expect("convert_json_value should succeed for valid field");

    assert_eq!(result, FieldValue::String("ada".to_string()));

    let error = layer
        .convert_json_value(SCHEMA_NAME, USERNAME_FIELD, json!(123))
        .expect_err("convert_json_value should reject type mismatch");

    match error {
        JsonBoundaryError::TypeMismatch { field, .. } => {
            assert_eq!(field, USERNAME_FIELD.to_string());
        }
        other => panic!("expected TypeMismatch error, got {other:?}"),
    }
}

#[test]
fn convert_json_value_rejects_unknown_fields() {
    let mut layer = JsonBoundaryLayer::new();
    layer.register_schema(create_base_schema()).unwrap();

    let error = layer
        .convert_json_value(SCHEMA_NAME, "unknown_field", json!("value"))
        .expect_err("convert_json_value should reject unknown fields");

    match error {
        JsonBoundaryError::UnknownField { field, .. } => {
            assert_eq!(field, "unknown_field");
        }
        other => panic!("expected UnknownField error, got {other:?}"),
    }
}

#[test]
fn convert_native_value_validates_individual_fields() {
    let mut layer = JsonBoundaryLayer::new();
    layer.register_schema(create_base_schema()).unwrap();

    let native_value = FieldValue::String("ada".to_string());
    let result = layer
        .convert_native_value(SCHEMA_NAME, USERNAME_FIELD, &native_value)
        .expect("convert_native_value should succeed for valid field");

    assert_eq!(result, json!("ada"));

    let invalid_native = FieldValue::Integer(123);
    let error = layer
        .convert_native_value(SCHEMA_NAME, USERNAME_FIELD, &invalid_native)
        .expect_err("convert_native_value should reject type mismatch");

    match error {
        JsonBoundaryError::TypeMismatch { field, .. } => {
            assert_eq!(field, USERNAME_FIELD.to_string());
        }
        other => panic!("expected TypeMismatch error, got {other:?}"),
    }
}

#[test]
fn get_field_default_returns_correct_defaults() {
    let mut layer = JsonBoundaryLayer::new();
    layer.register_schema(create_base_schema()).unwrap();

    // Required field should return None
    let result = layer
        .get_field_default(SCHEMA_NAME, USERNAME_FIELD)
        .expect("get_field_default should succeed");
    assert_eq!(result, None);

    // Optional field should return the default
    let result = layer
        .get_field_default(SCHEMA_NAME, AGE_FIELD)
        .expect("get_field_default should succeed");
    assert_eq!(result, Some(FieldValue::Integer(DEFAULT_AGE)));
}

#[test]
fn get_field_default_rejects_unknown_fields() {
    let mut layer = JsonBoundaryLayer::new();
    layer.register_schema(create_base_schema()).unwrap();

    let error = layer
        .get_field_default(SCHEMA_NAME, "unknown_field")
        .expect_err("get_field_default should reject unknown fields");

    match error {
        JsonBoundaryError::UnknownField { field, .. } => {
            assert_eq!(field, "unknown_field");
        }
        other => panic!("expected UnknownField error, got {other:?}"),
    }
}

#[test]
fn validate_json_payload_succeeds_for_valid_payload() {
    let mut layer = JsonBoundaryLayer::new();
    layer.register_schema(create_base_schema()).unwrap();

    let payload = json!({
        USERNAME_FIELD: "ada",
        AGE_FIELD: 31,
    });

    layer
        .validate_json_payload(SCHEMA_NAME, &payload)
        .expect("validate_json_payload should succeed for valid payload");
}

#[test]
fn validate_json_payload_rejects_invalid_structure() {
    let mut layer = JsonBoundaryLayer::new();
    layer.register_schema(create_base_schema()).unwrap();

    let payload = json!("not an object");

    let error = layer
        .validate_json_payload(SCHEMA_NAME, &payload)
        .expect_err("validate_json_payload should reject invalid structure");

    match error {
        JsonBoundaryError::InvalidPayloadStructure { .. } => {}
        other => panic!("expected InvalidPayloadStructure error, got {other:?}"),
    }
}

#[test]
fn validate_json_payload_rejects_unknown_fields() {
    let mut layer = JsonBoundaryLayer::new();
    layer.register_schema(create_base_schema()).unwrap();

    let payload = json!({
        USERNAME_FIELD: "ada",
        EXTRA_FIELD: "unexpected",
    });

    let error = layer
        .validate_json_payload(SCHEMA_NAME, &payload)
        .expect_err("validate_json_payload should reject unknown fields");

    match error {
        JsonBoundaryError::UnknownField { field, .. } => {
            assert_eq!(field, EXTRA_FIELD.to_string());
        }
        other => panic!("expected UnknownField error, got {other:?}"),
    }
}

#[test]
fn validate_json_payload_rejects_type_mismatches() {
    let mut layer = JsonBoundaryLayer::new();
    layer.register_schema(create_base_schema()).unwrap();

    let payload = json!({
        USERNAME_FIELD: 123, // Should be string
        AGE_FIELD: 31,
    });

    let error = layer
        .validate_json_payload(SCHEMA_NAME, &payload)
        .expect_err("validate_json_payload should reject type mismatches");

    match error {
        JsonBoundaryError::TypeMismatch { field, .. } => {
            assert_eq!(field, USERNAME_FIELD.to_string());
        }
        other => panic!("expected TypeMismatch error, got {other:?}"),
    }
}

#[test]
fn validate_json_payload_allows_additional_fields_when_permitted() {
    let schema = create_base_schema().allow_additional_fields(true);
    let mut layer = JsonBoundaryLayer::new();
    layer.register_schema(schema).unwrap();

    let payload = json!({
        USERNAME_FIELD: "ada",
        EXTRA_FIELD: "notes",
    });

    layer
        .validate_json_payload(SCHEMA_NAME, &payload)
        .expect("validate_json_payload should allow additional fields when permitted");
}

#[test]
fn json_to_native_partial_converts_only_present_fields() {
    let mut layer = JsonBoundaryLayer::new();
    layer.register_schema(create_base_schema()).unwrap();

    let payload = json!({
        USERNAME_FIELD: "ada",
    });

    let native = layer
        .json_to_native_partial(SCHEMA_NAME, &payload)
        .expect("json_to_native_partial should succeed");

    assert_eq!(
        native.get(USERNAME_FIELD),
        Some(&FieldValue::String("ada".to_string()))
    );
    // Age field should not be present since it wasn't in the JSON
    assert_eq!(native.get(AGE_FIELD), None);
}

#[test]
fn json_to_native_partial_includes_additional_fields_when_allowed() {
    let schema = create_base_schema().allow_additional_fields(true);
    let mut layer = JsonBoundaryLayer::new();
    layer.register_schema(schema).unwrap();

    let payload = json!({
        USERNAME_FIELD: "ada",
        EXTRA_FIELD: "notes",
    });

    let native = layer
        .json_to_native_partial(SCHEMA_NAME, &payload)
        .expect("json_to_native_partial should succeed");

    assert_eq!(
        native.get(EXTRA_FIELD),
        Some(&FieldValue::String("notes".to_string()))
    );
}

#[test]
fn registered_schemas_returns_all_schema_names() {
    let mut layer = JsonBoundaryLayer::new();
    layer.register_schema(create_base_schema()).unwrap();

    let schemas = layer.registered_schemas();
    assert_eq!(schemas.len(), 1);
    assert_eq!(schemas[0], SCHEMA_NAME);
}

#[test]
fn has_schema_checks_schema_existence() {
    let mut layer = JsonBoundaryLayer::new();
    
    assert!(!layer.has_schema(SCHEMA_NAME));
    
    layer.register_schema(create_base_schema()).unwrap();
    
    assert!(layer.has_schema(SCHEMA_NAME));
    assert!(!layer.has_schema("nonexistent"));
}

#[test]
fn schema_info_returns_correct_information() {
    let mut layer = JsonBoundaryLayer::new();
    layer.register_schema(create_base_schema()).unwrap();

    let info = layer
        .schema_info(SCHEMA_NAME)
        .expect("schema_info should succeed");

    assert_eq!(info.name, SCHEMA_NAME);
    assert_eq!(info.field_count, 2);
    assert_eq!(info.allows_additional_fields, false);
    assert_eq!(info.required_fields, vec![USERNAME_FIELD.to_string()]);
}

#[test]
fn schema_info_rejects_unknown_schemas() {
    let layer = JsonBoundaryLayer::new();

    let error = layer
        .schema_info("nonexistent")
        .expect_err("schema_info should reject unknown schemas");

    match error {
        JsonBoundaryError::SchemaNotRegistered { schema, .. } => {
            assert_eq!(schema, "nonexistent");
        }
        other => panic!("expected SchemaNotRegistered error, got {other:?}"),
    }
}

#[test]
fn schema_info_with_additional_fields_allowed() {
    let schema = create_base_schema().allow_additional_fields(true);
    let mut layer = JsonBoundaryLayer::new();
    layer.register_schema(schema).unwrap();

    let info = layer
        .schema_info(SCHEMA_NAME)
        .expect("schema_info should succeed");

    assert_eq!(info.allows_additional_fields, true);
}
