use super::FoldNode;

mod admin_ops;
mod mutation_ops;
mod query_ops;
mod schema_ops;

/// Centralized operation processor that handles all operation types consistently.
///
/// This eliminates code duplication across HTTP routes, TCP server, CLI, and direct API usage.
/// All operation execution goes through this single processor to ensure consistent behavior.
pub struct OperationProcessor {
    node: FoldNode,
}

impl OperationProcessor {
    /// Creates a new operation processor with a FoldNode instance.
    pub fn new(node: FoldNode) -> Self {
        Self { node }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::fold_node::NodeConfig;
    use crate::schema::types::declarative_schemas::DeclarativeSchemaDefinition;
    use crate::schema::types::field::HashRangeFilter;
    use crate::schema::types::key_config::KeyConfig;
    use crate::schema::types::operations::MutationType;
    use crate::schema::types::schema::DeclarativeSchemaType as SchemaType;
    use crate::schema::types::topology::{JsonTopology, TopologyNode, PrimitiveValueType};
    use crate::schema::types::{KeyValue, Mutation};
    use crate::schema::SchemaState;
    use crate::security::Ed25519KeyPair;
    use serde_json::json;
    use std::collections::HashMap;
    use tempfile::tempdir;

    /// Helper: create a FoldNode + OperationProcessor backed by a temp directory.
    async fn setup_processor() -> (OperationProcessor, FoldNode) {
        let temp_dir = tempdir().unwrap();
        let keypair = Ed25519KeyPair::generate().unwrap();
        let config = NodeConfig::new(temp_dir.path().to_path_buf())
            .with_schema_service_url("test://mock")
            .with_identity(&keypair.public_key_base64(), &keypair.secret_key_base64());
        let node = FoldNode::new(config).await.unwrap();
        let processor = OperationProcessor::new(node.clone());
        (processor, node)
    }

    /// Helper: create a schema, load it, and approve it so mutations work.
    async fn load_and_approve_schema(node: &FoldNode, mut schema: DeclarativeSchemaDefinition) {
        schema.populate_runtime_fields().unwrap();
        let db = node.get_fold_db().await.unwrap();
        db.schema_manager.load_schema_internal(schema).await.unwrap();
    }

    async fn approve_schema(node: &FoldNode, name: &str) {
        let db = node.get_fold_db().await.unwrap();
        db.schema_manager.set_schema_state(name, SchemaState::Approved).await.unwrap();
    }

    #[tokio::test]
    async fn test_operation_processor_creation() {
        // This test would require a mock FoldNode
        // For now, just test that the struct can be created
        // In a real test, you'd create a test FoldNode instance
    }

    #[tokio::test]
    async fn test_logging_methods_signature() {
        // This test ensures the logging methods are available on OperationProcessor
        // without needing to instantiate a full FoldNode (which is complex).
        // It relies on the fact that if this compiles, the methods exist.
        async fn check_methods(processor: &crate::fold_node::OperationProcessor) {
            let _ = processor.list_logs(None, None).await;
            let _ = processor.get_log_config().await;
            let _ = processor.get_log_features().await;
        }
        let _ = check_methods;
    }

    fn string_topology() -> JsonTopology {
        JsonTopology::new(TopologyNode::Primitive {
            value: PrimitiveValueType::String,
            classifications: None,
        })
    }

    /// Helper: create a child HashRange schema with hash+range keys and one data field.
    /// Uses `field` as hash key and `_rk` as range key, both included in `fields`.
    fn make_child_schema(name: &str, field: &str) -> DeclarativeSchemaDefinition {
        let mut schema = DeclarativeSchemaDefinition::new(
            name.to_string(),
            SchemaType::HashRange,
            Some(KeyConfig {
                hash_field: Some(field.to_string()),
                range_field: Some("_rk".to_string()),
            }),
            Some(vec![field.to_string(), "_rk".to_string()]),
            None,
            None,
        );
        schema.field_topologies.insert(field.to_string(), string_topology());
        schema.field_topologies.insert("_rk".to_string(), string_topology());
        schema
    }

    /// Helper: create a parent HashRange schema with a name field and a Reference field.
    fn make_parent_schema(name: &str, ref_field: &str, child_schema_name: &str) -> DeclarativeSchemaDefinition {
        let mut schema = DeclarativeSchemaDefinition::new(
            name.to_string(),
            SchemaType::HashRange,
            Some(KeyConfig {
                hash_field: Some("name".to_string()),
                range_field: Some("_rk".to_string()),
            }),
            Some(vec!["name".to_string(), "_rk".to_string(), ref_field.to_string()]),
            None,
            None,
        );
        schema.field_topologies.insert("name".to_string(), string_topology());
        schema.field_topologies.insert("_rk".to_string(), string_topology());
        schema.field_topologies.insert(
            ref_field.to_string(),
            JsonTopology::new(TopologyNode::Reference {
                schema_name: child_schema_name.to_string(),
            }),
        );
        schema
    }

    #[tokio::test]
    async fn test_query_without_rehydrate_depth_returns_raw_references() {
        let (processor, node) = setup_processor().await;
        let pub_key = processor.get_node_public_key();

        let child_schema = make_child_schema("PostSchema", "title");
        let parent_schema = make_parent_schema("UserSchema", "posts", "PostSchema");

        load_and_approve_schema(&node, child_schema).await;
        approve_schema(&node, "PostSchema").await;
        load_and_approve_schema(&node, parent_schema).await;
        approve_schema(&node, "UserSchema").await;

        // Create a child record with hash+range key
        let mut child_fields = HashMap::new();
        child_fields.insert("title".to_string(), json!("Hello World"));
        child_fields.insert("_rk".to_string(), json!("r1"));
        processor.execute_mutation_op(Mutation::new(
            "PostSchema".to_string(), child_fields,
            KeyValue::new(Some("Hello World".to_string()), Some("r1".to_string())),
            pub_key.clone(), 0, MutationType::Create,
        )).await.unwrap();

        // Create a parent record with reference to the child
        let mut parent_fields = HashMap::new();
        parent_fields.insert("name".to_string(), json!("Alice"));
        parent_fields.insert("_rk".to_string(), json!("r1"));
        parent_fields.insert("posts".to_string(), json!([
            {"schema": "PostSchema", "key": {"hash": "Hello World", "range": "r1"}}
        ]));
        processor.execute_mutation_op(Mutation::new(
            "UserSchema".to_string(), parent_fields,
            KeyValue::new(Some("Alice".to_string()), Some("r1".to_string())),
            pub_key.clone(), 0, MutationType::Create,
        )).await.unwrap();

        // Query WITHOUT rehydration - should return raw reference objects
        let query = crate::schema::types::Query::new(
            "UserSchema".to_string(),
            vec!["name".to_string(), "posts".to_string()],
        );
        let results = processor.execute_query_json(query).await.unwrap();

        assert_eq!(results.len(), 1);
        let record = &results[0];
        assert_eq!(record["fields"]["name"], json!("Alice"));

        // posts field should contain raw reference objects (no rehydration)
        let posts = record["fields"]["posts"].as_array().unwrap();
        assert_eq!(posts.len(), 1);
        assert_eq!(posts[0]["schema"], json!("PostSchema"));
    }

    #[tokio::test]
    async fn test_query_with_rehydrate_depth_resolves_references() {
        let (processor, node) = setup_processor().await;
        let pub_key = processor.get_node_public_key();

        let child_schema = make_child_schema("PostSchema", "title");
        let parent_schema = make_parent_schema("UserSchema", "posts", "PostSchema");

        load_and_approve_schema(&node, child_schema).await;
        approve_schema(&node, "PostSchema").await;
        load_and_approve_schema(&node, parent_schema).await;
        approve_schema(&node, "UserSchema").await;

        // Create child record with hash+range key
        let mut child_fields = HashMap::new();
        child_fields.insert("title".to_string(), json!("Hello World"));
        child_fields.insert("_rk".to_string(), json!("r1"));
        processor.execute_mutation_op(Mutation::new(
            "PostSchema".to_string(), child_fields,
            KeyValue::new(Some("Hello World".to_string()), Some("r1".to_string())),
            pub_key.clone(), 0, MutationType::Create,
        )).await.unwrap();

        // Create parent record with reference to child
        let mut parent_fields = HashMap::new();
        parent_fields.insert("name".to_string(), json!("Alice"));
        parent_fields.insert("_rk".to_string(), json!("r1"));
        parent_fields.insert("posts".to_string(), json!([
            {"schema": "PostSchema", "key": {"hash": "Hello World", "range": "r1"}}
        ]));
        processor.execute_mutation_op(Mutation::new(
            "UserSchema".to_string(), parent_fields,
            KeyValue::new(Some("Alice".to_string()), Some("r1".to_string())),
            pub_key.clone(), 0, MutationType::Create,
        )).await.unwrap();

        // Query WITH rehydration depth 1 - should resolve references
        let mut query = crate::schema::types::Query::new(
            "UserSchema".to_string(),
            vec!["name".to_string(), "posts".to_string()],
        );
        query.rehydrate_depth = Some(1);
        let results = processor.execute_query_json(query).await.unwrap();

        assert_eq!(results.len(), 1);
        let record = &results[0];
        assert_eq!(record["fields"]["name"], json!("Alice"));

        // posts field should now contain hydrated child records
        let posts = record["fields"]["posts"].as_array().unwrap();
        assert_eq!(posts.len(), 1);

        // Hydrated record should have "fields" with the child's data
        let hydrated_post = &posts[0];
        assert!(hydrated_post.get("fields").is_some(), "Hydrated post should have 'fields': {}", hydrated_post);
        assert_eq!(hydrated_post["fields"]["title"], json!("Hello World"));
        // Should also have a "key"
        assert!(hydrated_post.get("key").is_some(), "Hydrated post should have 'key'");
    }

    #[tokio::test]
    async fn test_rehydrate_depth_zero_does_not_resolve() {
        let (processor, node) = setup_processor().await;
        let pub_key = processor.get_node_public_key();

        let child_schema = make_child_schema("ItemSchema", "label");
        let parent_schema = make_parent_schema("ContainerSchema", "items", "ItemSchema");

        load_and_approve_schema(&node, child_schema).await;
        approve_schema(&node, "ItemSchema").await;
        load_and_approve_schema(&node, parent_schema).await;
        approve_schema(&node, "ContainerSchema").await;

        // Create child
        let mut child_fields = HashMap::new();
        child_fields.insert("label".to_string(), json!("Widget"));
        child_fields.insert("_rk".to_string(), json!("r1"));
        processor.execute_mutation_op(Mutation::new(
            "ItemSchema".to_string(), child_fields,
            KeyValue::new(Some("Widget".to_string()), Some("r1".to_string())),
            pub_key.clone(), 0, MutationType::Create,
        )).await.unwrap();

        // Create parent with reference
        let mut parent_fields = HashMap::new();
        parent_fields.insert("name".to_string(), json!("c1"));
        parent_fields.insert("_rk".to_string(), json!("r1"));
        parent_fields.insert("items".to_string(), json!([
            {"schema": "ItemSchema", "key": {"hash": "Widget", "range": "r1"}}
        ]));
        processor.execute_mutation_op(Mutation::new(
            "ContainerSchema".to_string(), parent_fields,
            KeyValue::new(Some("c1".to_string()), Some("r1".to_string())),
            pub_key.clone(), 0, MutationType::Create,
        )).await.unwrap();

        // Query with depth 0 - should NOT resolve references
        let mut query = crate::schema::types::Query::new(
            "ContainerSchema".to_string(),
            vec!["name".to_string(), "items".to_string()],
        );
        query.rehydrate_depth = Some(0);
        let results = processor.execute_query_json(query).await.unwrap();

        assert_eq!(results.len(), 1);
        let items = results[0]["fields"]["items"].as_array().unwrap();
        assert_eq!(items.len(), 1);
        // Should still be raw reference - has "schema" key, not "fields" key
        assert!(items[0].get("schema").is_some(), "depth=0 should leave raw references");
    }

    #[test]
    fn test_parse_ref_key_with_hash_only() {
        let ref_obj = json!({"schema": "SomeSchema", "key": {"hash": "abc"}});
        let kv = OperationProcessor::parse_ref_key(&ref_obj).unwrap();
        assert_eq!(kv.hash, Some("abc".to_string()));
        assert_eq!(kv.range, None);
    }

    #[test]
    fn test_parse_ref_key_with_hash_and_range() {
        let ref_obj = json!({"schema": "S", "key": {"hash": "h1", "range": "r1"}});
        let kv = OperationProcessor::parse_ref_key(&ref_obj).unwrap();
        assert_eq!(kv.hash, Some("h1".to_string()));
        assert_eq!(kv.range, Some("r1".to_string()));
    }

    #[test]
    fn test_parse_ref_key_missing_key_returns_none() {
        let ref_obj = json!({"schema": "S"});
        assert!(OperationProcessor::parse_ref_key(&ref_obj).is_none());
    }

    #[test]
    fn test_filter_from_key_value_hash_only() {
        let kv = KeyValue::new(Some("abc".to_string()), None);
        let filter = OperationProcessor::filter_from_key_value(&kv);
        assert!(matches!(filter, Some(HashRangeFilter::HashKey(ref h)) if h == "abc"));
    }

    #[test]
    fn test_filter_from_key_value_hash_and_range() {
        let kv = KeyValue::new(Some("h".to_string()), Some("r".to_string()));
        let filter = OperationProcessor::filter_from_key_value(&kv);
        assert!(matches!(filter, Some(HashRangeFilter::HashRangeKey { ref hash, ref range }) if hash == "h" && range == "r"));
    }

    #[test]
    fn test_filter_from_key_value_no_keys_returns_none() {
        let kv = KeyValue::new(None, None);
        assert!(OperationProcessor::filter_from_key_value(&kv).is_none());
    }
}
