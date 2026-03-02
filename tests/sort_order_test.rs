use fold_db::fold_node::config::NodeConfig;
use fold_db::fold_node::FoldNode;
use fold_db::fold_node::OperationProcessor;
use fold_db::schema::types::key_value::KeyValue;
use fold_db::schema::types::operations::{Query, SortOrder};
use fold_db::MutationType;
use serde_json::json;
use std::collections::HashMap;
use tempfile::TempDir;

/// Helper: create a FoldNode backed by a temp directory.
async fn setup_node() -> (FoldNode, TempDir) {
    let temp_dir = TempDir::new().expect("Failed to create temp dir");
    let temp_db_path = temp_dir.path().to_str().unwrap();

    let keypair = fold_db::security::Ed25519KeyPair::generate().unwrap();
    let config = NodeConfig::new(temp_db_path.into())
        .with_schema_service_url("test://mock")
        .with_identity(
            &keypair.public_key_base64(),
            &keypair.secret_key_base64(),
        );
    let node = FoldNode::new(config)
        .await
        .expect("Failed to create FoldNode");

    (node, temp_dir)
}

/// Helper: load a schema from tests/schemas_for_testing/<filename>.
async fn load_schema(node: &FoldNode, schema_filename: &str) {
    let schema_path = std::env::current_dir()
        .expect("Failed to get current directory")
        .join("tests/schemas_for_testing")
        .join(schema_filename);

    let mut fold_db = node.get_fold_db().await.expect("Failed to get FoldDB");
    fold_db
        .load_schema_from_file(&schema_path)
        .await
        .expect("Failed to load schema");
}

/// Extract the range key string from a JSON result record.
fn range_of(record: &serde_json::Value) -> String {
    record
        .get("key")
        .and_then(|k| k.get("range"))
        .and_then(|v| v.as_str())
        .unwrap_or("")
        .to_string()
}

// ---------------------------------------------------------------------------
// Tests using BlogPost (Range-only schema, range_field = publish_date)
// ---------------------------------------------------------------------------

#[tokio::test(flavor = "multi_thread")]
async fn test_sort_order_desc_range_schema() {
    let (node, _tmp) = setup_node().await;
    load_schema(&node, "BlogPost.json").await;
    let processor = OperationProcessor::new(node);

    // Insert posts in non-sorted order
    let dates = ["2024-06-15", "2024-01-01", "2024-12-25", "2024-03-10"];
    for date in &dates {
        let mut fields = HashMap::new();
        fields.insert("title".to_string(), json!(format!("Post on {}", date)));
        fields.insert("author".to_string(), json!("Alice"));
        fields.insert("publish_date".to_string(), json!(date));
        fields.insert("content".to_string(), json!("content"));
        fields.insert("tags".to_string(), json!(["test"]));

        processor
            .execute_mutation(
                "BlogPost".to_string(),
                fields,
                KeyValue::new(None, Some(date.to_string())),
                MutationType::Create,
            )
            .await
            .expect("Failed to execute mutation");
    }

    // Query with sort_order = desc
    let mut query = Query::new(
        "BlogPost".to_string(),
        vec!["title".to_string(), "publish_date".to_string()],
    );
    query.sort_order = Some(SortOrder::Desc);

    let results = processor
        .execute_query_json(query)
        .await
        .expect("Failed to execute query");

    assert_eq!(results.len(), 4, "Should return all 4 posts");

    let ranges: Vec<String> = results.iter().map(range_of).collect();
    assert_eq!(
        ranges,
        vec!["2024-12-25", "2024-06-15", "2024-03-10", "2024-01-01"],
        "Results should be sorted descending by range key"
    );
}

#[tokio::test(flavor = "multi_thread")]
async fn test_sort_order_asc_range_schema() {
    let (node, _tmp) = setup_node().await;
    load_schema(&node, "BlogPost.json").await;
    let processor = OperationProcessor::new(node);

    let dates = ["2024-06-15", "2024-01-01", "2024-12-25", "2024-03-10"];
    for date in &dates {
        let mut fields = HashMap::new();
        fields.insert("title".to_string(), json!(format!("Post on {}", date)));
        fields.insert("author".to_string(), json!("Alice"));
        fields.insert("publish_date".to_string(), json!(date));
        fields.insert("content".to_string(), json!("content"));
        fields.insert("tags".to_string(), json!(["test"]));

        processor
            .execute_mutation(
                "BlogPost".to_string(),
                fields,
                KeyValue::new(None, Some(date.to_string())),
                MutationType::Create,
            )
            .await
            .expect("Failed to execute mutation");
    }

    // Query with sort_order = asc
    let mut query = Query::new(
        "BlogPost".to_string(),
        vec!["title".to_string(), "publish_date".to_string()],
    );
    query.sort_order = Some(SortOrder::Asc);

    let results = processor
        .execute_query_json(query)
        .await
        .expect("Failed to execute query");

    assert_eq!(results.len(), 4, "Should return all 4 posts");

    let ranges: Vec<String> = results.iter().map(range_of).collect();
    assert_eq!(
        ranges,
        vec!["2024-01-01", "2024-03-10", "2024-06-15", "2024-12-25"],
        "Results should be sorted ascending by range key"
    );
}

#[tokio::test(flavor = "multi_thread")]
async fn test_sort_order_none_returns_results_without_crash() {
    let (node, _tmp) = setup_node().await;
    load_schema(&node, "BlogPost.json").await;
    let processor = OperationProcessor::new(node);

    let dates = ["2024-06-15", "2024-01-01"];
    for date in &dates {
        let mut fields = HashMap::new();
        fields.insert("title".to_string(), json!(format!("Post on {}", date)));
        fields.insert("author".to_string(), json!("Alice"));
        fields.insert("publish_date".to_string(), json!(date));
        fields.insert("content".to_string(), json!("content"));
        fields.insert("tags".to_string(), json!(["test"]));

        processor
            .execute_mutation(
                "BlogPost".to_string(),
                fields,
                KeyValue::new(None, Some(date.to_string())),
                MutationType::Create,
            )
            .await
            .expect("Failed to execute mutation");
    }

    // Query with no sort_order (None) — should still return results
    let query = Query::new(
        "BlogPost".to_string(),
        vec!["title".to_string(), "publish_date".to_string()],
    );

    let results = processor
        .execute_query_json(query)
        .await
        .expect("Failed to execute query");

    assert_eq!(results.len(), 2, "Should return both posts");
}

// ---------------------------------------------------------------------------
// Tests using Message (HashRange schema, hash=conversation_id, range=sent_at)
// ---------------------------------------------------------------------------

#[tokio::test(flavor = "multi_thread")]
async fn test_sort_order_desc_hashrange_schema() {
    let (node, _tmp) = setup_node().await;
    load_schema(&node, "Message.json").await;
    let processor = OperationProcessor::new(node);

    // Insert messages in non-sorted order
    let messages = [
        ("conv1", "2024-01-15T10:00:00Z", "Hello"),
        ("conv1", "2024-01-15T08:00:00Z", "Good morning"),
        ("conv1", "2024-01-15T14:00:00Z", "See you later"),
        ("conv1", "2024-01-15T12:00:00Z", "Lunch time"),
    ];

    for (conv_id, sent_at, content) in &messages {
        let mut fields = HashMap::new();
        fields.insert("message_id".to_string(), json!(format!("msg-{}", sent_at)));
        fields.insert("conversation_id".to_string(), json!(conv_id));
        fields.insert("sender_id".to_string(), json!("user1"));
        fields.insert("recipient_id".to_string(), json!("user2"));
        fields.insert("content".to_string(), json!(content));
        fields.insert("sent_at".to_string(), json!(sent_at));
        fields.insert("read_at".to_string(), json!(""));
        fields.insert("message_type".to_string(), json!("text"));
        fields.insert("attachments".to_string(), json!([]));

        processor
            .execute_mutation(
                "Message".to_string(),
                fields,
                KeyValue::new(Some(conv_id.to_string()), Some(sent_at.to_string())),
                MutationType::Create,
            )
            .await
            .expect("Failed to execute mutation");
    }

    // Query with sort_order = desc
    let mut query = Query::new(
        "Message".to_string(),
        vec!["content".to_string(), "sent_at".to_string()],
    );
    query.sort_order = Some(SortOrder::Desc);

    let results = processor
        .execute_query_json(query)
        .await
        .expect("Failed to execute query");

    assert_eq!(results.len(), 4, "Should return all 4 messages");

    let ranges: Vec<String> = results.iter().map(range_of).collect();
    assert_eq!(
        ranges,
        vec![
            "2024-01-15T14:00:00Z",
            "2024-01-15T12:00:00Z",
            "2024-01-15T10:00:00Z",
            "2024-01-15T08:00:00Z",
        ],
        "Messages should be sorted newest-first by sent_at range key"
    );
}

#[tokio::test(flavor = "multi_thread")]
async fn test_sort_order_asc_hashrange_schema() {
    let (node, _tmp) = setup_node().await;
    load_schema(&node, "Message.json").await;
    let processor = OperationProcessor::new(node);

    let messages = [
        ("conv1", "2024-01-15T10:00:00Z", "Hello"),
        ("conv1", "2024-01-15T08:00:00Z", "Good morning"),
        ("conv1", "2024-01-15T14:00:00Z", "See you later"),
    ];

    for (conv_id, sent_at, content) in &messages {
        let mut fields = HashMap::new();
        fields.insert("message_id".to_string(), json!(format!("msg-{}", sent_at)));
        fields.insert("conversation_id".to_string(), json!(conv_id));
        fields.insert("sender_id".to_string(), json!("user1"));
        fields.insert("recipient_id".to_string(), json!("user2"));
        fields.insert("content".to_string(), json!(content));
        fields.insert("sent_at".to_string(), json!(sent_at));
        fields.insert("read_at".to_string(), json!(""));
        fields.insert("message_type".to_string(), json!("text"));
        fields.insert("attachments".to_string(), json!([]));

        processor
            .execute_mutation(
                "Message".to_string(),
                fields,
                KeyValue::new(Some(conv_id.to_string()), Some(sent_at.to_string())),
                MutationType::Create,
            )
            .await
            .expect("Failed to execute mutation");
    }

    let mut query = Query::new(
        "Message".to_string(),
        vec!["content".to_string(), "sent_at".to_string()],
    );
    query.sort_order = Some(SortOrder::Asc);

    let results = processor
        .execute_query_json(query)
        .await
        .expect("Failed to execute query");

    assert_eq!(results.len(), 3, "Should return all 3 messages");

    let ranges: Vec<String> = results.iter().map(range_of).collect();
    assert_eq!(
        ranges,
        vec![
            "2024-01-15T08:00:00Z",
            "2024-01-15T10:00:00Z",
            "2024-01-15T14:00:00Z",
        ],
        "Messages should be sorted oldest-first by sent_at range key"
    );
}

// ---------------------------------------------------------------------------
// Test that sort_order deserializes correctly from JSON (API path)
// ---------------------------------------------------------------------------

#[tokio::test(flavor = "multi_thread")]
async fn test_sort_order_via_json_deserialization() {
    let (node, _tmp) = setup_node().await;
    load_schema(&node, "BlogPost.json").await;
    let processor = OperationProcessor::new(node);

    let dates = ["2024-06-15", "2024-01-01", "2024-12-25"];
    for date in &dates {
        let mut fields = HashMap::new();
        fields.insert("title".to_string(), json!(format!("Post on {}", date)));
        fields.insert("author".to_string(), json!("Alice"));
        fields.insert("publish_date".to_string(), json!(date));
        fields.insert("content".to_string(), json!("content"));
        fields.insert("tags".to_string(), json!(["test"]));

        processor
            .execute_mutation(
                "BlogPost".to_string(),
                fields,
                KeyValue::new(None, Some(date.to_string())),
                MutationType::Create,
            )
            .await
            .expect("Failed to execute mutation");
    }

    // Simulate the JSON that an API client or LLM agent would send
    let query_json = json!({
        "schema_name": "BlogPost",
        "fields": ["title", "publish_date"],
        "filter": null,
        "sort_order": "DESC"
    });

    let query: Query = serde_json::from_value(query_json).expect("Failed to parse query JSON");
    assert_eq!(query.sort_order, Some(SortOrder::Desc));

    let results = processor
        .execute_query_json(query)
        .await
        .expect("Failed to execute query");

    assert_eq!(results.len(), 3, "Should return all 3 posts");

    let ranges: Vec<String> = results.iter().map(range_of).collect();
    assert_eq!(
        ranges,
        vec!["2024-12-25", "2024-06-15", "2024-01-01"],
        "Results should be sorted descending by range key even when sort_order was uppercase"
    );
}
