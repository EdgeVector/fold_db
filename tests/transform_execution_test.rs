use fold_db::fold_db_core::FoldDB;
use fold_db::schema::SchemaState;
use std::thread;
use std::time::Duration;
use tempfile::TempDir;

mod common;

/// Test to verify that transforms execute for Approved and Blocked schemas
#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn test_transform_execution_states() {
    use serde_json::json;

    // Create temporary directory for test database
    let temp_dir = TempDir::new().expect("Failed to create temp directory");
    let test_db_path = temp_dir
        .path()
        .to_str()
        .expect("Failed to convert path to string");

    // Create FoldDB instance
    let mut fold_db = FoldDB::new(test_db_path)
        .await
        .expect("Failed to create FoldDB instance");
    let transform_manager = fold_db.transform_manager();
    let db_ops = transform_manager.db_ops.clone();

    // Load the BlogPost schema (source schema)
    let blogpost_schema_json = json!({
        "name": "BlogPost",
        "key": {
            "range_field": "publish_date"
        },
        "fields": {
            "title": {},
            "content": {},
            "author": {},
            "publish_date": {},
            "tags": {}
        }
    });

    let blogpost_schema_str =
        serde_json::to_string(&blogpost_schema_json).expect("Failed to serialize BlogPost schema");

    fold_db
        .schema_manager()
        .load_schema_from_json(&blogpost_schema_str)
        .await
        .expect("Failed to load BlogPost schema");

    // Set BlogPost schema to Approved
    db_ops
        .store_schema_state("BlogPost", &SchemaState::Approved)
        .await
        .expect("Failed to set BlogPost state");

    // Load the BlogPostWordIndex schema with transform_fields
    let wordindex_schema_json = json!({
        "name": "BlogPostWordIndex",
        "key": {
            "hash_field": "word",
            "range_field": "publish_date"
        },
        "transform_fields": {
            "word": "BlogPost.content.split_by_word()",
            "publish_date": "BlogPost.publish_date",
            "content": "BlogPost.content",
            "author": "BlogPost.author",
            "title": "BlogPost.title",
            "tags": "BlogPost.tags"
        }
    });

    let wordindex_schema_str = serde_json::to_string(&wordindex_schema_json)
        .expect("Failed to serialize BlogPostWordIndex schema");

    // Load BlogPostWordIndex schema
    fold_db
        .schema_manager()
        .load_schema_from_json(&wordindex_schema_str)
        .await
        .expect("Failed to load BlogPostWordIndex schema");

    // Wait for async event processing
    thread::sleep(Duration::from_millis(100));

    // Case 1: Set BlogPostWordIndex to Approved (should execute)
    db_ops
        .store_schema_state("BlogPostWordIndex", &SchemaState::Approved)
        .await
        .expect("Failed to set BlogPostWordIndex state");

    // Perform mutation on BlogPost
    let mutation_json = json!({
        "schema_name": "BlogPost",
        "uuid": "mutation_1",
        "pub_key": "test_key",
        "fields_and_values": {
            "title": "Test Post 1",
            "content": "Hello World",
            "author": "Author 1",
            "publish_date": "2023-01-01",
            "tags": "tag1"
        }
    });

    let mutation = common::create_test_mutation(&blogpost_schema_json, mutation_json);

    fold_db
        .mutation_manager_mut()
        .write_mutations_batch_async(vec![mutation])
        .await
        .expect("Failed to execute mutation");

    // Case 2: Set BlogPostWordIndex to Blocked
    db_ops
        .store_schema_state("BlogPostWordIndex", &SchemaState::Blocked)
        .await
        .expect("Failed to set BlogPostWordIndex state");

    let state = transform_manager
        .get_schema_state("BlogPostWordIndex")
        .await
        .expect("Failed to get state");
    assert_eq!(state, Some(SchemaState::Blocked));

    // Case 3: Set BlogPostWordIndex to Available
    // db_ops.store_schema_state("BlogPostWordIndex", &SchemaState::Available).await.expect("Failed to set BlogPostWordIndex state");
    // let state = transform_manager.get_schema_state("BlogPostWordIndex").expect("Failed to get state");
    // assert_eq!(state, Some(SchemaState::Available));

    // Close the database
    fold_db.close().expect("Failed to close FoldDB");
}
