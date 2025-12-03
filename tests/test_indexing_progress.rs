use datafold::datafold_node::node::DataFoldNode;
use datafold::testing_utils::TestDatabaseFactory;
use serde_json::json;
use std::collections::HashMap;
use datafold::datafold_node::OperationProcessor;
use std::sync::Arc;
use tokio::sync::Mutex;
use datafold::schema::types::operations::MutationType;
use datafold::schema::types::KeyValue;

#[tokio::test]
async fn test_indexing_progress_tracking() {
    // Setup
    let config = TestDatabaseFactory::create_test_node_config();
    let node = DataFoldNode::new(config).await.unwrap();
    
    // Create a schema
    let schema_json = r#"{
        "name": "test_schema",
        "type": "Single",
        "key": {
            "fields": ["id"],
            "primary": true
        },
        "fields": ["id", "content"],
        "field_topologies": {
            "id": {
                "root": {
                    "type": "Primitive",
                    "value": "String"
                }
            },
            "content": {
                "root": {
                    "type": "Primitive",
                    "value": "String",
                    "classifications": ["word"]
                }
            }
        }
    }"#;
    
    node.get_fold_db().unwrap().load_schema_from_json(schema_json).await.unwrap();
    
    // Check initial status
    let status = node.get_indexing_status().await;
    assert_eq!(status.total_operations_processed, 0);
    
    // Perform mutation
    let fields_and_values = {
        let mut map = HashMap::new();
        map.insert("id".to_string(), json!("1"));
        map.insert("content".to_string(), json!("hello world"));
        map
    };
    
    let node_arc = Arc::new(Mutex::new(node));
    let processor = OperationProcessor::new(node_arc.clone());
    
    let key_value = KeyValue::new(Some("1".to_string()), None);
    
    processor.execute_mutation(
        "test_schema".to_string(),
        fields_and_values,
        key_value,
        MutationType::Create
    ).await.unwrap();
    
    // Check status again
    let node_guard = node_arc.lock().await;
    let status = node_guard.get_indexing_status().await;
    
    println!("Total operations processed: {}", status.total_operations_processed);
    assert!(status.total_operations_processed > 0, "Should have processed indexing operations");
}
