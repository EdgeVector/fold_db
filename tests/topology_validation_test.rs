use datafold::schema::types::{
    DeclarativeSchemaDefinition, JsonTopology, PrimitiveType, TopologyNode, KeyConfig, SchemaType,
};
use serde_json::json;
use std::collections::HashMap;

#[test]
fn test_topology_validation_rejects_invalid_type() {
    // Create schema with topology - expecting string for "name" field
    let mut schema = DeclarativeSchemaDefinition::new(
        "TestSchema".to_string(),
        SchemaType::Range,
        Some(KeyConfig {
            hash_field: None,
            range_field: Some("id".to_string()),
        }),
        Some(vec!["id".to_string(), "name".to_string()]),
        None,
        None,
    );

    // Set topology: name should be a string
    schema.set_field_topology(
        "name".to_string(),
        JsonTopology::new(TopologyNode::Primitive(PrimitiveType::String)),
    );

    // Try to validate wrong type (number instead of string)
    let result = schema.validate_field_value("name", &json!(42));
    assert!(result.is_err(), "Expected validation error");
    
    let err_msg = result.unwrap_err().to_string();
    assert!(
        err_msg.contains("Topology validation failed") || err_msg.contains("expected"),
        "Error message should mention topology validation: {}",
        err_msg
    );
}

#[test]
fn test_topology_validation_accepts_valid_type() {
    // Create schema with topology
    let mut schema = DeclarativeSchemaDefinition::new(
        "TestSchema".to_string(),
        SchemaType::Range,
        Some(KeyConfig {
            hash_field: None,
            range_field: Some("id".to_string()),
        }),
        Some(vec!["id".to_string(), "name".to_string()]),
        None,
        None,
    );

    // Set topology: name should be a string
    schema.set_field_topology(
        "name".to_string(),
        JsonTopology::new(TopologyNode::Primitive(PrimitiveType::String)),
    );

    // Validate correct type
    let result = schema.validate_field_value("name", &json!("Alice"));
    assert!(result.is_ok(), "Expected successful validation: {:?}", result);
}

#[test]
fn test_topology_nested_object_validation() {
    // Create schema with nested object topology
    let mut schema = DeclarativeSchemaDefinition::new(
        "TestSchema".to_string(),
        SchemaType::Range,
        Some(KeyConfig {
            hash_field: None,
            range_field: Some("id".to_string()),
        }),
        Some(vec!["id".to_string(), "user".to_string()]),
        None,
        None,
    );

    // Set topology: user is an object with id (number) and name (string)
    let mut user_fields = HashMap::new();
    user_fields.insert("id".to_string(), TopologyNode::Primitive(PrimitiveType::Number));
    user_fields.insert("name".to_string(), TopologyNode::Primitive(PrimitiveType::String));
    
    schema.set_field_topology(
        "user".to_string(),
        JsonTopology::new(TopologyNode::Object(user_fields)),
    );

    // Test 1: Valid nested object
    let result = schema.validate_field_value("user", &json!({"id": 1, "name": "Alice"}));
    assert!(result.is_ok(), "Expected successful validation with valid nested object");

    // Test 2: Invalid nested object (wrong type for user.id)
    let result = schema.validate_field_value("user", &json!({"id": "not_a_number", "name": "Bob"}));
    assert!(result.is_err(), "Expected validation error for invalid nested field");
}

#[test]
fn test_topology_array_validation() {
    // Create schema with array topology
    let mut schema = DeclarativeSchemaDefinition::new(
        "TestSchema".to_string(),
        SchemaType::Range,
        Some(KeyConfig {
            hash_field: None,
            range_field: Some("id".to_string()),
        }),
        Some(vec!["id".to_string(), "tags".to_string()]),
        None,
        None,
    );

    // Set topology: tags is an array of strings
    schema.set_field_topology(
        "tags".to_string(),
        JsonTopology::new(TopologyNode::Array(Box::new(
            TopologyNode::Primitive(PrimitiveType::String)
        ))),
    );

    // Test 1: Valid array
    let result = schema.validate_field_value("tags", &json!(["rust", "database"]));
    assert!(result.is_ok(), "Expected successful validation with valid array");

    // Test 2: Invalid array (contains numbers)
    let result = schema.validate_field_value("tags", &json!(["rust", 42, "database"]));
    assert!(result.is_err(), "Expected validation error for invalid array element");
}

#[test]
fn test_topology_inference_from_data() {
    let sample_data = json!({
        "name": "Alice",
        "age": 30,
        "active": true,
        "tags": ["rust", "database"]
    });

    let topology = JsonTopology::infer_from_value(&sample_data);

    // Validate that the inferred topology accepts the same structure
    assert!(topology.validate(&sample_data).is_ok());

    // Validate that it accepts similar data
    let similar_data = json!({
        "name": "Bob",
        "age": 25,
        "active": false,
        "tags": ["python"]
    });
    assert!(topology.validate(&similar_data).is_ok());

    // Validate that it rejects different structure
    let invalid_data = json!({
        "name": "Charlie",
        "age": "thirty", // Wrong type
        "active": false
    });
    assert!(topology.validate(&invalid_data).is_err());
}

#[test]
fn test_schema_serialization_includes_topology() {
    // Create schema with topology
    let mut schema = DeclarativeSchemaDefinition::new(
        "TestSchema".to_string(),
        SchemaType::Single,
        None,
        Some(vec!["name".to_string(), "age".to_string()]),
        None,
        None,
    );

    schema.set_field_topology(
        "name".to_string(),
        JsonTopology::new(TopologyNode::Primitive(PrimitiveType::String)),
    );
    schema.set_field_topology(
        "age".to_string(),
        JsonTopology::new(TopologyNode::Primitive(PrimitiveType::Number)),
    );

    // Serialize and deserialize
    let serialized = serde_json::to_string(&schema).expect("Failed to serialize");
    let deserialized: DeclarativeSchemaDefinition = serde_json::from_str(&serialized)
        .expect("Failed to deserialize");

    // Verify topology was preserved
    assert!(deserialized.field_topologies.is_some());
    let topologies = deserialized.field_topologies.as_ref().unwrap();
    assert!(topologies.contains_key("name"));
    assert!(topologies.contains_key("age"));

    // Verify the actual topology values
    let name_topology = topologies.get("name").unwrap();
    assert_eq!(
        name_topology.root,
        TopologyNode::Primitive(PrimitiveType::String)
    );

    let age_topology = topologies.get("age").unwrap();
    assert_eq!(
        age_topology.root,
        TopologyNode::Primitive(PrimitiveType::Number)
    );
}

#[test]
fn test_no_topology_allows_any_value() {
    // Create schema WITHOUT topology
    let schema = DeclarativeSchemaDefinition::new(
        "TestSchema".to_string(),
        SchemaType::Range,
        Some(KeyConfig {
            hash_field: None,
            range_field: Some("id".to_string()),
        }),
        Some(vec!["id".to_string(), "data".to_string()]),
        None,
        None,
    );

    // Should accept any type of data when no topology is defined
    let test_cases = vec![
        json!("string"),
        json!(42),
        json!(true),
        json!({"nested": "object"}),
        json!(["array", "of", "values"]),
    ];

    for (idx, data) in test_cases.into_iter().enumerate() {
        let result = schema.validate_field_value("data", &data);
        assert!(
            result.is_ok(),
            "Expected validation to succeed without topology (test case {}): {:?}",
            idx,
            result
        );
    }
}

