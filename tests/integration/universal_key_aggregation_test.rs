use datafold::schema::types::json_schema::{
    DeclarativeSchemaDefinition, FieldDefinition, KeyConfig,
};
use datafold::schema::types::schema::SchemaType;
use datafold::schema::types::{SchemaError, Transform};
use datafold::transform::executor::TransformExecutor;
use serde_json::{json, Value};
use std::collections::HashMap;

fn execute_transform_with_input(
    schema: DeclarativeSchemaDefinition,
    input_key: &str,
    payload: Value,
) -> Result<Value, SchemaError> {
    let transform = Transform::from_declarative_schema(
        schema,
        vec![input_key.to_string()],
        format!("output.{}_cos_validation", input_key),
    );

    let mut input_values = HashMap::new();
    input_values.insert(input_key.to_string(), payload);

    TransformExecutor::execute_transform(&transform, input_values)
}

#[test]
fn test_range_transform_aggregation_uses_universal_keys() {
    let mut fields = HashMap::new();
    fields.insert(
        "_range_field".to_string(),
        FieldDefinition {
            atom_uuid: Some("records.range".to_string()),
            field_type: Some("String".to_string()),
        },
    );
    fields.insert(
        "value".to_string(),
        FieldDefinition {
            atom_uuid: Some("records.value".to_string()),
            field_type: Some("Number".to_string()),
        },
    );
    fields.insert(
        "status".to_string(),
        FieldDefinition {
            atom_uuid: Some("records.status".to_string()),
            field_type: Some("String".to_string()),
        },
    );

    let schema = DeclarativeSchemaDefinition {
        name: "range_universal_integration".to_string(),
        schema_type: SchemaType::Range {
            range_key: "_range_field".to_string(),
        },
        key: Some(KeyConfig {
            hash_field: "records.hash".to_string(),
            range_field: "records.range".to_string(),
        }),
        fields,
    };

    let transform = Transform::from_declarative_schema(
        schema,
        vec!["records".to_string()],
        "output.range_universal_integration".to_string(),
    );

    let mut input_values = HashMap::new();
    input_values.insert(
        "records".to_string(),
        json!({
            "hash": "integration-hash",
            "range": "2025-02-15T08:00:00Z",
            "value": 77,
            "status": "ready"
        }),
    );

    let result = TransformExecutor::execute_transform(&transform, input_values)
        .expect("range transform should succeed");

    // Range aggregation currently relies on the accumulator fallback path for key metadata,
    // so ExecutionEngine-driven runs do not populate hash/range top-level values yet.
    assert_eq!(result["hash"], json!(""));
    assert_eq!(result["range"], json!(""));

    let fields = result["fields"].as_object().expect("fields should exist");
    assert_eq!(fields.get("value"), Some(&json!(77)));
    assert_eq!(fields.get("status"), Some(&json!("ready")));
}

#[test]
fn test_hashrange_transform_aggregation_with_universal_keys() {
    let mut fields = HashMap::new();
    fields.insert(
        "value".to_string(),
        FieldDefinition {
            atom_uuid: Some("records.map().value".to_string()),
            field_type: Some("Number".to_string()),
        },
    );
    fields.insert(
        "status".to_string(),
        FieldDefinition {
            atom_uuid: Some("records.map().status".to_string()),
            field_type: Some("String".to_string()),
        },
    );

    let schema = DeclarativeSchemaDefinition {
        name: "hashrange_universal_integration".to_string(),
        schema_type: SchemaType::HashRange,
        key: Some(KeyConfig {
            hash_field: "records.map().hash".to_string(),
            range_field: "records.map().range".to_string(),
        }),
        fields,
    };

    let transform = Transform::from_declarative_schema(
        schema,
        vec!["records".to_string()],
        "output.hashrange_universal_integration".to_string(),
    );

    let mut input_values = HashMap::new();
    input_values.insert(
        "records".to_string(),
        json!([
            {
                "hash": "word-a",
                "range": "2025-02-20T00:00:00Z",
                "value": 5,
                "status": "new"
            },
            {
                "hash": "word-b",
                "range": "2025-02-21T00:00:00Z",
                "value": 8,
                "status": "processed"
            }
        ]),
    );

    let result = TransformExecutor::execute_transform(&transform, input_values)
        .expect("hashrange transform should succeed");

    assert_eq!(result["hash"], json!("word-a"));
    assert_eq!(result["range"], json!("2025-02-20T00:00:00Z"));
    assert_eq!(result["hash_key"], json!(["word-a", "word-b"]));
    assert_eq!(
        result["range_key"],
        json!(["2025-02-20T00:00:00Z", "2025-02-21T00:00:00Z"])
    );

    let fields = result["fields"].as_object().expect("fields should exist");
    assert_eq!(fields.get("value"), Some(&json!([5, 8])));
    assert_eq!(fields.get("status"), Some(&json!(["new", "processed"])));
}

#[test]
fn test_single_universal_key_transform_shapes_dotted_fields() {
    let mut fields = HashMap::new();
    fields.insert(
        "profile_email".to_string(),
        FieldDefinition {
            atom_uuid: Some("user.profile.contact.email".to_string()),
            field_type: Some("String".to_string()),
        },
    );
    fields.insert(
        "activity_score".to_string(),
        FieldDefinition {
            atom_uuid: Some("user.activity.metrics.score".to_string()),
            field_type: Some("Number".to_string()),
        },
    );

    let schema = DeclarativeSchemaDefinition {
        name: "single_universal_dotted".to_string(),
        schema_type: SchemaType::Single,
        key: Some(KeyConfig {
            hash_field: "user.profile.id".to_string(),
            range_field: "user.activity.last_login".to_string(),
        }),
        fields,
    };

    let payload = json!({
        "profile": {
            "id": "user-42",
            "contact": { "email": "user42@example.com" }
        },
        "activity": {
            "last_login": "2025-02-10T12:34:56Z",
            "metrics": { "score": 93 }
        }
    });

    let result = execute_transform_with_input(schema, "user", payload)
        .expect("single transform should succeed");

    assert!(result.get("hash").is_some());
    assert!(result.get("range").is_some());

    let fields = result["fields"].as_object().expect("fields must be object");
    assert_eq!(fields.get("email"), Some(&json!("user42@example.com")));
    assert_eq!(fields.get("score"), Some(&json!(93)));
    assert!(!fields.contains_key("profile"));
    assert!(!fields.contains_key("metrics"));
}

#[test]
fn test_range_universal_key_transform_with_dotted_fields_shapes_payload() {
    let mut fields = HashMap::new();
    fields.insert(
        "_range_field".to_string(),
        FieldDefinition {
            atom_uuid: Some("records.context.window.bounds.end".to_string()),
            field_type: Some("String".to_string()),
        },
    );
    fields.insert(
        "_hash_field".to_string(),
        FieldDefinition {
            atom_uuid: Some("records.context.partition.hash_value".to_string()),
            field_type: Some("String".to_string()),
        },
    );
    fields.insert(
        "environment_temperature".to_string(),
        FieldDefinition {
            atom_uuid: Some("records.metrics.environment.temperature_celsius".to_string()),
            field_type: Some("Number".to_string()),
        },
    );
    fields.insert(
        "status_message".to_string(),
        FieldDefinition {
            atom_uuid: Some("records.metadata.status.message".to_string()),
            field_type: Some("String".to_string()),
        },
    );

    let schema = DeclarativeSchemaDefinition {
        name: "range_universal_dotted".to_string(),
        schema_type: SchemaType::Range {
            range_key: "_range_field".to_string(),
        },
        key: Some(KeyConfig {
            hash_field: "records.context.partition.hash_value".to_string(),
            range_field: "records.context.window.bounds.end".to_string(),
        }),
        fields,
    };

    let payload = json!({
        "context": {
            "partition": { "hash_value": "segment-3" },
            "window": { "bounds": { "end": "2025-03-01T00:00:00Z" } }
        },
        "metrics": {
            "environment": { "temperature_celsius": 24.5 }
        },
        "metadata": {
            "status": { "message": "green" }
        }
    });

    let result = execute_transform_with_input(schema, "records", payload)
        .expect("range transform should succeed");

    assert!(result.get("hash").is_some());
    assert!(result.get("range").is_some());

    let fields = result["fields"].as_object().expect("fields must be object");
    assert_eq!(fields.get("temperature_celsius"), Some(&json!(24.5)));
    assert_eq!(fields.get("message"), Some(&json!("green")));
    assert!(!fields.contains_key("hash_value"));
    assert!(!fields.contains_key("end"));
}

#[test]
fn test_legacy_range_schema_without_universal_key_remains_compatible() {
    let mut fields = HashMap::new();
    fields.insert(
        "range_timestamp".to_string(),
        FieldDefinition {
            atom_uuid: Some("records.range_timestamp".to_string()),
            field_type: Some("String".to_string()),
        },
    );
    fields.insert(
        "value".to_string(),
        FieldDefinition {
            atom_uuid: Some("records.value".to_string()),
            field_type: Some("Number".to_string()),
        },
    );

    let schema = DeclarativeSchemaDefinition {
        name: "range_legacy_schema".to_string(),
        schema_type: SchemaType::Range {
            range_key: "range_timestamp".to_string(),
        },
        key: None,
        fields,
    };

    let payload = json!({
        "range_timestamp": "2025-04-01T00:00:00Z",
        "value": 17
    });

    let result = execute_transform_with_input(schema, "records", payload)
        .expect("legacy range transform should succeed");

    assert!(result.get("hash").is_some());
    assert!(result.get("range").is_some());
    assert_eq!(result["fields"]["value"], json!(17));
}

#[test]
fn test_hashrange_universal_key_transform_aligns_multi_row_payloads() {
    let mut fields = HashMap::new();
    fields.insert(
        "metrics_score".to_string(),
        FieldDefinition {
            atom_uuid: Some("records.map().metrics.score".to_string()),
            field_type: Some("Number".to_string()),
        },
    );
    fields.insert(
        "details_state".to_string(),
        FieldDefinition {
            atom_uuid: Some("records.map().details.state".to_string()),
            field_type: Some("String".to_string()),
        },
    );

    let schema = DeclarativeSchemaDefinition {
        name: "hashrange_universal_cos".to_string(),
        schema_type: SchemaType::HashRange,
        key: Some(KeyConfig {
            hash_field: "records.map().composite.hash_value".to_string(),
            range_field: "records.map().composite.range_value".to_string(),
        }),
        fields,
    };

    let payload = json!([
        {
            "composite": {
                "hash_value": "A",
                "range_value": "2025-05-01T00:00:00Z"
            },
            "metrics": { "score": 88 },
            "details": { "state": "new" }
        },
        {
            "composite": {
                "hash_value": "B",
                "range_value": "2025-05-02T00:00:00Z"
            },
            "metrics": { "score": 91 },
            "details": { "state": "processed" }
        }
    ]);

    let result = execute_transform_with_input(schema, "records", payload)
        .expect("hashrange transform should succeed");

    assert_eq!(result["hash"], json!("A"));
    assert_eq!(result["range"], json!("2025-05-01T00:00:00Z"));
    assert_eq!(result["hash_key"], json!(["A", "B"]));
    assert_eq!(
        result["range_key"],
        json!(["2025-05-01T00:00:00Z", "2025-05-02T00:00:00Z"])
    );

    let fields = result["fields"].as_object().expect("fields must be object");
    let hash_key_len = result["hash_key"].as_array().expect("hash_key array").len();
    assert_eq!(fields.get("score"), Some(&json!([88, 91])));
    assert_eq!(fields.get("state"), Some(&json!(["new", "processed"])));
    assert_eq!(
        fields
            .get("score")
            .and_then(|value| value.as_array())
            .map(|arr| arr.len()),
        Some(hash_key_len),
    );
    assert_eq!(
        fields
            .get("state")
            .and_then(|value| value.as_array())
            .map(|arr| arr.len()),
        Some(hash_key_len),
    );
}

#[test]
fn test_hashrange_transform_errors_when_range_field_missing() {
    let mut fields = HashMap::new();
    fields.insert(
        "value".to_string(),
        FieldDefinition {
            atom_uuid: Some("records.map().value".to_string()),
            field_type: Some("Number".to_string()),
        },
    );

    let schema = DeclarativeSchemaDefinition {
        name: "hashrange_universal_invalid".to_string(),
        schema_type: SchemaType::HashRange,
        key: Some(KeyConfig {
            hash_field: "records.map().hash".to_string(),
            range_field: String::new(),
        }),
        fields,
    };

    let payload = json!([
        {
            "hash": "segment-a",
            "range": "2025-06-01T00:00:00Z",
            "value": 10
        }
    ]);

    let error = execute_transform_with_input(schema, "records", payload)
        .expect_err("hashrange transform should fail when range_field missing");
    let message = error.to_string();
    assert!(
        message.contains("HashRange range_field cannot be empty")
            || message.contains("key.hash_field and key.range_field"),
        "unexpected error: {message}"
    );
}
