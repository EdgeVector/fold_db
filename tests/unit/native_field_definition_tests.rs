use datafold::transform::{
    FieldValue, NativeFieldDefinition, NativeFieldDefinitionError, NativeFieldType,
};
use std::collections::HashMap;

#[test]
fn validate_rejects_empty_field_name() {
    let definition = NativeFieldDefinition::new("", NativeFieldType::String);

    let error = definition
        .validate()
        .expect_err("empty field name should be rejected");
    assert_eq!(error, NativeFieldDefinitionError::EmptyName);
}

#[test]
fn validate_rejects_invalid_field_name_characters() {
    let definition = NativeFieldDefinition::new("bad-name", NativeFieldType::Integer);

    let error = definition
        .validate()
        .expect_err("hyphenated field name should be rejected");
    assert_eq!(
        error,
        NativeFieldDefinitionError::InvalidNameCharacters {
            name: "bad-name".to_string(),
        },
    );
}

#[test]
fn validate_rejects_mismatched_default_value() {
    let definition = NativeFieldDefinition::new("count", NativeFieldType::Integer)
        .with_default(FieldValue::String("oops".to_string()));

    let error = definition
        .validate()
        .expect_err("default type mismatch should fail validation");
    assert_eq!(
        error,
        NativeFieldDefinitionError::DefaultTypeMismatch {
            name: "count".to_string(),
            declared: Box::new(NativeFieldType::Integer),
            actual: Box::new(NativeFieldType::String),
        },
    );
}

#[test]
fn validate_accepts_valid_definition() {
    let definition = NativeFieldDefinition::new("count", NativeFieldType::Integer)
        .with_required(false)
        .with_default(FieldValue::Integer(10));

    definition
        .validate()
        .expect("valid field definition should pass validation");
}

#[test]
fn effective_default_prefers_explicit_defaults() {
    let definition = NativeFieldDefinition::new("flag", NativeFieldType::Boolean)
        .with_required(false)
        .with_default(FieldValue::Boolean(true));

    assert_eq!(
        definition.effective_default(),
        Some(FieldValue::Boolean(true))
    );
}

#[test]
fn effective_default_generates_nested_defaults_for_optional_fields() {
    let nested_type = NativeFieldType::Object {
        fields: HashMap::from([
            ("title".to_string(), NativeFieldType::String),
            (
                "tags".to_string(),
                NativeFieldType::Array {
                    element_type: Box::new(NativeFieldType::String),
                },
            ),
        ]),
    };

    let definition = NativeFieldDefinition::new("metadata", nested_type).with_required(false);

    let default_value = definition
        .effective_default()
        .expect("optional field should provide generated default");

    assert_eq!(
        default_value,
        FieldValue::Object(HashMap::from([
            ("title".to_string(), FieldValue::String(String::new())),
            ("tags".to_string(), FieldValue::Array(Vec::new())),
        ])),
    );
}
