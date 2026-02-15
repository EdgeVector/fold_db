use fold_db::fold_node::node::FoldNode;
use fold_db::fold_node::OperationProcessor;
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
    let keypair = fold_db::security::Ed25519KeyPair::generate().unwrap();
    let user_id = keypair.public_key_base64();
    config = config.with_identity(&user_id, &keypair.secret_key_base64());
    let node = FoldNode::new(config).await.unwrap();

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

    // Perform mutation within user context
    let fields_and_values = {
        let mut map = HashMap::new();
        map.insert("id".to_string(), json!("1"));
        map.insert("content".to_string(), json!("hello world"));
        map
    };

    let processor = OperationProcessor::new(node.clone());

    let key_value = KeyValue::new(Some("1".to_string()), None);

    // Execute mutation within user context
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

    // Indexing is now rules-based and inline (happens synchronously during mutation write).
    // Field names are always indexed during write_mutations_batch_async.
    // Verify by searching for indexed field names.
    let db = node.get_fold_db().await.unwrap();

    // Field name "content" should be indexed (inline field-name indexing)
    let field_results = db
        .native_search_all_classifications("content")
        .await
        .expect("Field name search failed");
    assert!(
        !field_results.is_empty(),
        "Field name 'content' should be indexed after mutation"
    );

    println!(
        "Indexing verified: content field-name results={}",
        field_results.len()
    );
}
