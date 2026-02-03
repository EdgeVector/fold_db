use tempfile::TempDir;

#[tokio::test]
async fn test_schema_service_rejects_schema_without_topologies() {
    let temp_dir = TempDir::new().expect("Failed to create temp dir");
    let db_path = temp_dir
        .path()
        .join("test_schema_db")
        .to_string_lossy()
        .to_string();

    let state = datafold::schema_service::server::SchemaServiceState::new(db_path)
        .expect("Failed to create schema service state");

    // Create schema WITHOUT topologies
    let schema = datafold::schema::types::Schema::new(
        "TestSchema".to_string(),
        datafold::schema::types::SchemaType::Single,
        None,
        Some(vec!["id".to_string(), "name".to_string()]),
        None,
        None,
    );

    // Attempt to add schema - should fail
    let result = state
        .add_schema(schema, std::collections::HashMap::new())
        .await;

    assert!(
        result.is_err(),
        "Schema without topologies should be rejected"
    );
    let err = result.unwrap_err();
    let err_msg = err.to_string();
    assert!(
        err_msg.contains("missing a topology definition"),
        "Error should mention missing topology: {}",
        err_msg
    );
}

#[test]
fn test_topology_inference_from_sample_data() {
    use fold_db::schema::types::{PrimitiveType, Schema, SchemaType, TopologyNode};
    use serde_json::json;

    // Create a schema
    let mut schema = Schema::new(
        "TestSchema".to_string(),
        SchemaType::Single,
        None,
        Some(vec![
            "id".to_string(),
            "name".to_string(),
            "age".to_string(),
            "tags".to_string(),
        ]),
        None,
        None,
    );

    // Sample data
    let sample_data = json!({
        "id": "123",
        "name": "Alice",
        "age": 30,
        "tags": ["rust", "database"]
    });

    // Infer topologies from sample data
    let sample_map: std::collections::HashMap<String, serde_json::Value> = sample_data
        .as_object()
        .unwrap()
        .iter()
        .map(|(k, v)| (k.clone(), v.clone()))
        .collect();

    schema.infer_topologies_from_data(&sample_map);

    // Verify topologies were inferred
    assert!(schema.field_topologies.contains_key("id"));
    assert!(schema.field_topologies.contains_key("name"));
    assert!(schema.field_topologies.contains_key("age"));
    assert!(schema.field_topologies.contains_key("tags"));

    // Verify correct types
    let id_topology = schema.field_topologies.get("id").unwrap();
    assert_eq!(
        id_topology.root,
        TopologyNode::Primitive {
            value: PrimitiveType::String,
            classifications: None
        }
    );

    let name_topology = schema.field_topologies.get("name").unwrap();
    assert_eq!(
        name_topology.root,
        TopologyNode::Primitive {
            value: PrimitiveType::String,
            classifications: None
        }
    );

    let age_topology = schema.field_topologies.get("age").unwrap();
    assert_eq!(
        age_topology.root,
        TopologyNode::Primitive {
            value: PrimitiveType::Number,
            classifications: None
        }
    );

    let tags_topology = schema.field_topologies.get("tags").unwrap();
    match &tags_topology.root {
        TopologyNode::Array { .. } => {}
        other => panic!("Expected Array topology for tags, got {:?}", other),
    }
}
