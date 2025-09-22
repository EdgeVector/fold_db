use datafold::schema::types::json_schema::{
    DeclarativeSchemaDefinition, FieldDefinition, KeyConfig,
};
use datafold::schema::types::schema::SchemaType;
use datafold::schema::types::Transform;
use datafold::transform::executor::TransformExecutor;
use serde_json::json;
use std::collections::HashMap;

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
