use fold_db::schema::types::{
    DeclarativeSchemaDefinition, JsonTopology, PrimitiveType, SchemaType, TopologyNode,
};
use std::collections::HashMap;

#[test]
fn test_field_topology_hash_is_computed() {
    let mut schema = DeclarativeSchemaDefinition::new(
        "TestSchema".to_string(),
        SchemaType::Single,
        None,
        Some(vec!["name".to_string()]),
        None,
        None,
    );

    // Set a topology
    schema.set_field_topology(
        "name".to_string(),
        JsonTopology::new(TopologyNode::Primitive {
            value: PrimitiveType::String,
            classifications: None,
        }),
    );

    // Verify field topology hash was computed
    assert!(schema.field_topology_hashes.is_some());
    let hashes = schema.field_topology_hashes.as_ref().unwrap();
    assert!(hashes.contains_key("name"));

    let hash = hashes.get("name").unwrap();
    assert!(!hash.is_empty());
    assert_eq!(hash.len(), 64); // SHA256 produces 64 hex chars
}

#[test]
fn test_schema_topology_hash_is_computed() {
    let mut schema = DeclarativeSchemaDefinition::new(
        "TestSchema".to_string(),
        SchemaType::Single,
        None,
        Some(vec!["name".to_string(), "age".to_string()]),
        None,
        None,
    );

    // Set topologies
    schema.set_field_topology(
        "name".to_string(),
        JsonTopology::new(TopologyNode::Primitive {
            value: PrimitiveType::String,
            classifications: None,
        }),
    );
    schema.set_field_topology(
        "age".to_string(),
        JsonTopology::new(TopologyNode::Primitive {
            value: PrimitiveType::Number,
            classifications: None,
        }),
    );

    // Verify schema-level topology hash was computed
    assert!(schema.topology_hash.is_some());
    let hash = schema.topology_hash.as_ref().unwrap();
    assert!(!hash.is_empty());
    assert_eq!(hash.len(), 64); // SHA256 produces 64 hex chars
}

#[test]
fn test_same_topologies_produce_same_hash() {
    let mut schema1 = DeclarativeSchemaDefinition::new(
        "Schema1".to_string(),
        SchemaType::Single,
        None,
        Some(vec!["name".to_string(), "age".to_string()]),
        None,
        None,
    );

    let mut schema2 = DeclarativeSchemaDefinition::new(
        "Schema2".to_string(),
        SchemaType::Single,
        None,
        Some(vec!["name".to_string(), "age".to_string()]),
        None,
        None,
    );

    // Set identical topologies
    schema1.set_field_topology(
        "name".to_string(),
        JsonTopology::new(TopologyNode::Primitive {
            value: PrimitiveType::String,
            classifications: None,
        }),
    );
    schema1.set_field_topology(
        "age".to_string(),
        JsonTopology::new(TopologyNode::Primitive {
            value: PrimitiveType::Number,
            classifications: None,
        }),
    );

    schema2.set_field_topology(
        "name".to_string(),
        JsonTopology::new(TopologyNode::Primitive {
            value: PrimitiveType::String,
            classifications: None,
        }),
    );
    schema2.set_field_topology(
        "age".to_string(),
        JsonTopology::new(TopologyNode::Primitive {
            value: PrimitiveType::Number,
            classifications: None,
        }),
    );

    // Same topologies should produce same hash
    assert_eq!(schema1.topology_hash, schema2.topology_hash);
    assert_eq!(
        schema1.get_field_topology_hash("name"),
        schema2.get_field_topology_hash("name")
    );
    assert_eq!(
        schema1.get_field_topology_hash("age"),
        schema2.get_field_topology_hash("age")
    );
}

#[test]
fn test_different_topologies_produce_different_hash() {
    let mut schema1 = DeclarativeSchemaDefinition::new(
        "Schema1".to_string(),
        SchemaType::Single,
        None,
        Some(vec!["data".to_string()]),
        None,
        None,
    );

    let mut schema2 = DeclarativeSchemaDefinition::new(
        "Schema2".to_string(),
        SchemaType::Single,
        None,
        Some(vec!["data".to_string()]),
        None,
        None,
    );

    // Set different topologies
    schema1.set_field_topology(
        "data".to_string(),
        JsonTopology::new(TopologyNode::Primitive {
            value: PrimitiveType::String,
            classifications: None,
        }),
    );

    schema2.set_field_topology(
        "data".to_string(),
        JsonTopology::new(TopologyNode::Primitive {
            value: PrimitiveType::Number,
            classifications: None,
        }),
    );

    // Different topologies should produce different hashes
    assert_ne!(schema1.topology_hash, schema2.topology_hash);
    assert_ne!(
        schema1.get_field_topology_hash("data"),
        schema2.get_field_topology_hash("data")
    );
}

#[test]
fn test_field_order_independent_schema_hash() {
    let mut schema1 = DeclarativeSchemaDefinition::new(
        "Schema1".to_string(),
        SchemaType::Single,
        None,
        Some(vec!["name".to_string(), "age".to_string()]),
        None,
        None,
    );

    let mut schema2 = DeclarativeSchemaDefinition::new(
        "Schema2".to_string(),
        SchemaType::Single,
        None,
        Some(vec!["age".to_string(), "name".to_string()]), // Different order
        None,
        None,
    );

    // Set same topologies but in different order
    schema1.set_field_topology(
        "name".to_string(),
        JsonTopology::new(TopologyNode::Primitive {
            value: PrimitiveType::String,
            classifications: None,
        }),
    );
    schema1.set_field_topology(
        "age".to_string(),
        JsonTopology::new(TopologyNode::Primitive {
            value: PrimitiveType::Number,
            classifications: None,
        }),
    );

    schema2.set_field_topology(
        "age".to_string(),
        JsonTopology::new(TopologyNode::Primitive {
            value: PrimitiveType::Number,
            classifications: None,
        }),
    );
    schema2.set_field_topology(
        "name".to_string(),
        JsonTopology::new(TopologyNode::Primitive {
            value: PrimitiveType::String,
            classifications: None,
        }),
    );

    // Schema hash should be the same regardless of field order (we sort internally)
    assert_eq!(schema1.topology_hash, schema2.topology_hash);
}

#[test]
fn test_nested_topology_hash() {
    let mut schema = DeclarativeSchemaDefinition::new(
        "TestSchema".to_string(),
        SchemaType::Single,
        None,
        Some(vec!["user".to_string()]),
        None,
        None,
    );

    let mut user_fields = HashMap::new();
    user_fields.insert(
        "id".to_string(),
        TopologyNode::Primitive {
            value: PrimitiveType::Number,
            classifications: None,
        },
    );
    user_fields.insert(
        "name".to_string(),
        TopologyNode::Primitive {
            value: PrimitiveType::String,
            classifications: None,
        },
    );

    schema.set_field_topology(
        "user".to_string(),
        JsonTopology::new(TopologyNode::Object { value: user_fields }),
    );

    // Verify hash was computed for nested structure
    assert!(schema.get_field_topology_hash("user").is_some());
    assert!(schema.get_topology_hash().is_some());
}

#[test]
fn test_array_topology_hash() {
    let mut schema = DeclarativeSchemaDefinition::new(
        "TestSchema".to_string(),
        SchemaType::Single,
        None,
        Some(vec!["tags".to_string()]),
        None,
        None,
    );

    schema.set_field_topology(
        "tags".to_string(),
        JsonTopology::new(TopologyNode::Array {
            value: Box::new(TopologyNode::Primitive {
                value: PrimitiveType::String,
                classifications: None,
            }),
        }),
    );

    // Verify hash was computed for array structure
    assert!(schema.get_field_topology_hash("tags").is_some());
    assert!(schema.get_topology_hash().is_some());
}

#[test]
fn test_topology_hash_persists_through_serialization() {
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
        JsonTopology::new(TopologyNode::Primitive {
            value: PrimitiveType::String,
            classifications: None,
        }),
    );
    schema.set_field_topology(
        "age".to_string(),
        JsonTopology::new(TopologyNode::Primitive {
            value: PrimitiveType::Number,
            classifications: None,
        }),
    );

    let original_hash = schema.topology_hash.clone();
    let original_field_hashes = schema.field_topology_hashes.clone();

    // Serialize and deserialize
    let serialized = serde_json::to_string(&schema).expect("Failed to serialize");
    let deserialized: DeclarativeSchemaDefinition =
        serde_json::from_str(&serialized).expect("Failed to deserialize");

    // Hashes should be preserved
    assert_eq!(deserialized.topology_hash, original_hash);
    assert_eq!(deserialized.field_topology_hashes, original_field_hashes);
}

#[test]
fn test_topology_hash_changes_when_topology_changes() {
    let mut schema = DeclarativeSchemaDefinition::new(
        "TestSchema".to_string(),
        SchemaType::Single,
        None,
        Some(vec!["data".to_string()]),
        None,
        None,
    );

    // Set initial topology
    schema.set_field_topology(
        "data".to_string(),
        JsonTopology::new(TopologyNode::Primitive {
            value: PrimitiveType::String,
            classifications: None,
        }),
    );

    let initial_hash = schema.topology_hash.clone();
    let initial_field_hash = schema.get_field_topology_hash("data").cloned();

    // Change topology
    schema.set_field_topology(
        "data".to_string(),
        JsonTopology::new(TopologyNode::Primitive {
            value: PrimitiveType::Number,
            classifications: None,
        }),
    );

    let new_hash = schema.topology_hash.clone();
    let new_field_hash = schema.get_field_topology_hash("data").cloned();

    // Hashes should change
    assert_ne!(initial_hash, new_hash);
    assert_ne!(initial_field_hash, new_field_hash);
}
