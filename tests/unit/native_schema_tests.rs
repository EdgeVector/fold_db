use datafold::schema::native::{
    KeyConfig, NativeSchema, NativeSchemaBuilder, NativeSchemaError, SchemaValidationError,
};
use datafold::transform::{
    FieldValue, NativeFieldDefinition, NativeFieldDefinitionError, NativeFieldType,
};
use std::collections::HashMap;

fn make_schema_builder(name: &str) -> NativeSchemaBuilder {
    NativeSchema::builder(
        name.to_string(),
        KeyConfig::Single {
            key_field: "id".to_string(),
        },
    )
}

fn make_string_field(name: &str) -> NativeFieldDefinition {
    NativeFieldDefinition::new(name, NativeFieldType::String)
}

#[test]
fn builder_rejects_duplicate_field_names() {
    let mut builder = make_schema_builder("BlogPost");
    builder
        .add_field(make_string_field("id"))
        .expect("first field should be accepted");

    let error = builder
        .add_field(make_string_field("id"))
        .expect_err("duplicate field must fail");

    assert_eq!(
        error,
        NativeSchemaError::DuplicateField {
            schema: "BlogPost".to_string(),
            field: "id".to_string(),
        }
    );
}

#[test]
fn builder_rejects_invalid_field_definition() {
    let mut builder = make_schema_builder("Articles");
    let invalid_definition = NativeFieldDefinition::new("", NativeFieldType::String);

    let error = builder
        .add_field(invalid_definition)
        .expect_err("invalid field should be rejected");

    assert!(matches!(
        error,
        NativeSchemaError::InvalidFieldDefinition {
            schema,
            field,
            source: NativeFieldDefinitionError::EmptyName,
        } if schema == "Articles" && field.is_empty()
    ));
}

#[test]
fn builder_rejects_missing_key_field() {
    let mut builder = make_schema_builder("Comment");
    builder
        .add_field(make_string_field("title"))
        .expect("non-key field should be accepted");

    let error = builder
        .build()
        .expect_err("schema should fail when key field is missing");

    assert_eq!(
        error,
        NativeSchemaError::MissingKeyField {
            schema: "Comment".to_string(),
            field: "id".to_string(),
        }
    );
}

#[test]
fn builder_rejects_optional_key_field() {
    let mut builder = make_schema_builder("User");
    let optional_id = make_string_field("id").with_required(false);
    builder
        .add_field(optional_id)
        .expect("field registration should succeed before build");

    let error = builder
        .build()
        .expect_err("key field must be marked required");

    assert_eq!(
        error,
        NativeSchemaError::KeyFieldNotRequired {
            schema: "User".to_string(),
            field: "id".to_string(),
        }
    );
}

#[test]
fn builder_rejects_null_key_field_type() {
    let mut builder = make_schema_builder("NullKey");
    let null_field = NativeFieldDefinition::new("id", NativeFieldType::Null);
    builder
        .add_field(null_field)
        .expect("field registration should succeed before build");

    let error = builder
        .build()
        .expect_err("null key field should be rejected");

    assert_eq!(
        error,
        NativeSchemaError::InvalidKeyFieldType {
            schema: "NullKey".to_string(),
            field: "id".to_string(),
            actual: NativeFieldType::Null,
        }
    );
}

#[test]
fn builder_produces_valid_schema() {
    let mut builder = make_schema_builder("Article");
    builder
        .add_field(make_string_field("id"))
        .expect("key field should be accepted");
    builder
        .add_field(make_string_field("title"))
        .expect("secondary field should be accepted");

    let schema = builder.build().expect("schema build should succeed");

    assert_eq!(schema.name(), "Article");
    assert_eq!(schema.len(), 2);
    assert!(schema.get_field("title").is_some());
}

#[test]
fn validate_payload_accepts_matching_data() {
    let schema = NativeSchema::try_from_definitions(
        "Post",
        KeyConfig::Single {
            key_field: "id".to_string(),
        },
        vec![
            make_string_field("id"),
            make_string_field("title").with_required(false),
        ],
    )
    .expect("schema creation should succeed");

    let mut payload = HashMap::new();
    payload.insert("id".to_string(), FieldValue::String("123".to_string()));

    schema
        .validate_payload(&payload)
        .expect("payload should be considered valid");
}

#[test]
fn validate_payload_rejects_missing_required_field() {
    let schema = NativeSchema::try_from_definitions(
        "Inventory",
        KeyConfig::Single {
            key_field: "id".to_string(),
        },
        vec![make_string_field("id"), make_string_field("sku")],
    )
    .expect("schema creation should succeed");

    let mut payload = HashMap::new();
    payload.insert("id".to_string(), FieldValue::String("1".to_string()));

    let error = schema
        .validate_payload(&payload)
        .expect_err("missing required field should be rejected");

    assert_eq!(
        error,
        SchemaValidationError::MissingRequiredField {
            schema: "Inventory".to_string(),
            field: "sku".to_string(),
        }
    );
}

#[test]
fn validate_payload_rejects_unknown_field() {
    let schema = NativeSchema::try_from_definitions(
        "Product",
        KeyConfig::Single {
            key_field: "id".to_string(),
        },
        vec![make_string_field("id")],
    )
    .expect("schema creation should succeed");

    let mut payload = HashMap::new();
    payload.insert("id".to_string(), FieldValue::String("1".to_string()));
    payload.insert(
        "extra".to_string(),
        FieldValue::String("surplus".to_string()),
    );

    let error = schema
        .validate_payload(&payload)
        .expect_err("unknown field should be rejected");

    assert_eq!(
        error,
        SchemaValidationError::UnknownField {
            schema: "Product".to_string(),
            field: "extra".to_string(),
        }
    );
}

#[test]
fn validate_payload_rejects_type_mismatch() {
    let schema = NativeSchema::try_from_definitions(
        "Metrics",
        KeyConfig::Single {
            key_field: "id".to_string(),
        },
        vec![make_string_field("id")],
    )
    .expect("schema creation should succeed");

    let mut payload = HashMap::new();
    payload.insert("id".to_string(), FieldValue::Integer(42));

    let error = schema
        .validate_payload(&payload)
        .expect_err("incorrect type should be rejected");

    assert_eq!(
        error,
        SchemaValidationError::TypeMismatch {
            schema: "Metrics".to_string(),
            field: "id".to_string(),
            expected: Box::new(NativeFieldType::String),
            actual: Box::new(FieldValue::Integer(42).field_type()),
        }
    );
}

#[test]
fn normalise_payload_inserts_defaults_for_optional_fields() {
    let schema = NativeSchema::try_from_definitions(
        "Profiles",
        KeyConfig::Single {
            key_field: "id".to_string(),
        },
        vec![
            make_string_field("id"),
            make_string_field("display_name").with_required(false),
        ],
    )
    .expect("schema creation should succeed");

    let mut payload = HashMap::from([("id".to_string(), FieldValue::String("user-1".to_string()))]);

    schema
        .normalise_payload(&mut payload)
        .expect("normalisation should succeed");

    assert_eq!(
        payload.get("display_name"),
        Some(&FieldValue::String(String::new()))
    );
}

#[test]
fn project_payload_returns_clone_with_defaults() {
    let schema = NativeSchema::try_from_definitions(
        "Accounts",
        KeyConfig::Single {
            key_field: "id".to_string(),
        },
        vec![
            make_string_field("id"),
            make_string_field("nickname").with_required(false),
        ],
    )
    .expect("schema creation should succeed");

    let payload = HashMap::from([("id".to_string(), FieldValue::String("acct-42".to_string()))]);

    let projected = schema
        .project_payload(&payload)
        .expect("projection should succeed");

    assert_eq!(payload.len(), 1, "original payload must stay untouched");
    assert_eq!(
        projected.get("nickname"),
        Some(&FieldValue::String(String::new()))
    );
}
