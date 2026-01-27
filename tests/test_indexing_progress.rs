use datafold::datafold_node::node::DataFoldNode;
use datafold::datafold_node::OperationProcessor;
use datafold::schema::types::operations::MutationType;
use datafold::schema::types::KeyValue;
use datafold::testing_utils::TestDatabaseFactory;
use serde_json::json;
use std::collections::HashMap;

#[tokio::test]
#[ignore = "Indexing progress tracking not yet implemented for this test scenario"]
async fn test_indexing_progress_tracking() {
    // Setup
    let mut config = TestDatabaseFactory::create_test_node_config();
    let keypair = datafold::security::Ed25519KeyPair::generate().unwrap();
    config = config.with_identity(&keypair.public_key_base64(), &keypair.secret_key_base64());
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

    {
        let mut db = node.get_fold_db().await.unwrap();
        db.load_schema_from_json(schema_json).await.unwrap();
        db.schema_manager()
            .approve("test_schema")
            .await
            .unwrap();
    }

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

    let processor = OperationProcessor::new(node.clone());

    let key_value = KeyValue::new(Some("1".to_string()), None);

    processor
        .execute_mutation(
            "test_schema".to_string(),
            fields_and_values,
            key_value,
            MutationType::Create,
        )
        .await
        .unwrap();

    // Poll for status update
    let start = std::time::Instant::now();
    let timeout = std::time::Duration::from_secs(5);

    let mut processed = 0;
    while start.elapsed() < timeout {
        let status = node.get_indexing_status().await;
        if status.total_operations_processed > 0 {
            processed = status.total_operations_processed;
            break;
        }
        tokio::time::sleep(std::time::Duration::from_millis(100)).await;
    }

    println!("Total operations processed: {}", processed);
    assert!(
        processed > 0,
        "Should have processed indexing operations within timeout"
    );
}
