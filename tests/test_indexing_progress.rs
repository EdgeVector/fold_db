use fold_db::datafold_node::node::DataFoldNode;
use fold_db::datafold_node::OperationProcessor;
use fold_db::logging::core::run_with_user;
use fold_db::schema::types::operations::MutationType;
use fold_db::schema::types::KeyValue;
use fold_db::testing_utils::TestDatabaseFactory;
use serde_json::json;
use std::collections::HashMap;

#[tokio::test]
async fn test_indexing_progress_tracking() {
    // Setup
    let mut config = TestDatabaseFactory::create_test_node_config();
    let keypair = datafold::security::Ed25519KeyPair::generate().unwrap();
    let user_id = keypair.public_key_base64();
    config = config.with_identity(&user_id, &keypair.secret_key_base64());
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
        db.schema_manager().approve("test_schema").await.unwrap();
    }

    // Check initial status - must be within user context
    let status = run_with_user(&user_id, async { node.get_indexing_status().await }).await;
    assert_eq!(status.total_operations_processed, 0);

    // Perform mutation within user context
    let fields_and_values = {
        let mut map = HashMap::new();
        map.insert("id".to_string(), json!("1"));
        map.insert("content".to_string(), json!("hello world"));
        map
    };

    let processor = OperationProcessor::new(node.clone());

    let key_value = KeyValue::new(Some("1".to_string()), None);

    // Execute mutation within user context so IndexStatusTracker can save status
    run_with_user(&user_id, async {
        processor
            .execute_mutation(
                "test_schema".to_string(),
                fields_and_values,
                key_value,
                MutationType::Create,
            )
            .await
            .unwrap();
    })
    .await;

    // Poll for status update - also within user context
    let start = std::time::Instant::now();
    // Increased timeout for CI reliability
    let timeout = std::time::Duration::from_secs(15);

    let mut processed = 0;
    while start.elapsed() < timeout {
        let status = run_with_user(&user_id, async { node.get_indexing_status().await }).await;
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
