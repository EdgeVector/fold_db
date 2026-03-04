use fold_db::fold_node::config::NodeConfig;
use fold_db::fold_node::FoldNode;
use fold_db::fold_node::OperationProcessor;
use fold_db::schema::types::key_value::KeyValue;
use fold_db::MutationType;
use serde_json::json;
use std::collections::HashMap;
use tempfile::TempDir;

async fn setup_node() -> (FoldNode, TempDir) {
    let temp_dir = TempDir::new().expect("Failed to create temp dir");
    let keypair = fold_db::security::Ed25519KeyPair::generate().unwrap();
    let config = NodeConfig::new(temp_dir.path().to_str().unwrap().into())
        .with_schema_service_url("test://mock")
        .with_identity(&keypair.public_key_base64(), &keypair.secret_key_base64());
    let node = FoldNode::new(config).await.expect("Failed to create FoldNode");
    (node, temp_dir)
}

async fn load_schema(node: &FoldNode, filename: &str) {
    let path = std::env::current_dir()
        .unwrap()
        .join("tests/schemas_for_testing")
        .join(filename);
    node.get_fold_db()
        .await
        .unwrap()
        .load_schema_from_file(&path)
        .await
        .expect("Failed to load schema");
}

async fn insert_post(
    processor: &OperationProcessor,
    date: &str,
    title: &str,
    content: &str,
    author: &str,
) {
    let mut fields = HashMap::new();
    fields.insert("title".to_string(), json!(title));
    fields.insert("content".to_string(), json!(content));
    fields.insert("author".to_string(), json!(author));
    fields.insert("publish_date".to_string(), json!(date));
    fields.insert("tags".to_string(), json!([]));

    processor
        .execute_mutation(
            "BlogPost".to_string(),
            fields,
            KeyValue::new(None, Some(date.to_string())),
            MutationType::Create,
        )
        .await
        .expect("Failed to insert BlogPost");
}

/// Indexed records should be findable by semantic search.
#[tokio::test(flavor = "multi_thread")]
async fn test_indexed_records_appear_in_search() {
    let (node, _tmp) = setup_node().await;
    load_schema(&node, "BlogPost.json").await;
    let processor = OperationProcessor::new(node);

    insert_post(
        &processor,
        "2024-01-01",
        "Introduction to Rust",
        "Rust is a systems programming language focused on safety and performance.",
        "Alice",
    )
    .await;

    insert_post(
        &processor,
        "2024-02-01",
        "Cooking Pasta",
        "How to make the perfect carbonara with eggs and pancetta.",
        "Bob",
    )
    .await;

    let results = processor
        .native_index_search("systems programming language")
        .await
        .expect("Search failed");

    assert!(!results.is_empty(), "Should find at least one result");

    let schemas: Vec<&str> = results.iter().map(|r| r.schema_name.as_str()).collect();
    assert!(
        schemas.iter().all(|s| *s == "BlogPost"),
        "All results should be from BlogPost schema"
    );
}

/// The most semantically relevant result should rank highest.
#[tokio::test(flavor = "multi_thread")]
async fn test_semantic_ranking_returns_most_relevant_first() {
    let (node, _tmp) = setup_node().await;
    load_schema(&node, "BlogPost.json").await;
    let processor = OperationProcessor::new(node);

    insert_post(
        &processor,
        "2024-01-01",
        "Deep Learning and Neural Networks",
        "Exploring convolutional neural networks for image classification tasks.",
        "Alice",
    )
    .await;

    insert_post(
        &processor,
        "2024-02-01",
        "Gardening Tips",
        "How to grow tomatoes and peppers in your backyard garden.",
        "Bob",
    )
    .await;

    insert_post(
        &processor,
        "2024-03-01",
        "Machine Learning Fundamentals",
        "An introduction to supervised and unsupervised learning algorithms.",
        "Carol",
    )
    .await;

    let results = processor
        .native_index_search("neural network machine learning")
        .await
        .expect("Search failed");

    assert!(!results.is_empty(), "Should return results");

    // Gardening post should NOT be the top result — AI/ML posts should rank higher
    let top_key = results[0].key_value.range.as_deref().unwrap_or("");
    assert_ne!(
        top_key, "2024-02-01",
        "Gardening post should not be the top result for an AI/ML query"
    );
}

/// Re-indexing the same record should not create duplicate entries.
#[tokio::test(flavor = "multi_thread")]
async fn test_update_replaces_existing_index_entry() {
    let (node, _tmp) = setup_node().await;
    load_schema(&node, "BlogPost.json").await;
    let processor = OperationProcessor::new(node);

    // Insert then update the same record (same key)
    insert_post(
        &processor,
        "2024-01-01",
        "Original Title",
        "Original content about databases.",
        "Alice",
    )
    .await;

    insert_post(
        &processor,
        "2024-01-01",
        "Updated Title",
        "Updated content about databases and storage engines.",
        "Alice",
    )
    .await;

    let results = processor
        .native_index_search("databases storage")
        .await
        .expect("Search failed");

    // Only one entry per unique key — no duplicates from re-indexing
    let matching_keys: Vec<_> = results
        .iter()
        .filter(|r| r.key_value.range.as_deref() == Some("2024-01-01"))
        .collect();

    let unique_fields: std::collections::HashSet<&str> =
        matching_keys.iter().map(|r| r.field.as_str()).collect();

    // Fields from the same record come back (one per field), but not doubled
    assert_eq!(
        matching_keys.len(),
        unique_fields.len(),
        "Each field should appear exactly once — no duplicate entries from upsert"
    );
}

/// Results must include a similarity score in metadata.
#[tokio::test(flavor = "multi_thread")]
async fn test_results_include_score_metadata() {
    let (node, _tmp) = setup_node().await;
    load_schema(&node, "BlogPost.json").await;
    let processor = OperationProcessor::new(node);

    insert_post(
        &processor,
        "2024-01-01",
        "Rust Programming",
        "Memory safety without garbage collection.",
        "Alice",
    )
    .await;

    let results = processor
        .native_index_search("memory safety")
        .await
        .expect("Search failed");

    assert!(!results.is_empty());

    let meta = results[0].metadata.as_ref().expect("Should have metadata");
    assert!(meta.get("score").is_some(), "Metadata should contain score");
    assert_eq!(
        meta.get("match_type").and_then(|v| v.as_str()),
        Some("semantic"),
        "match_type should be 'semantic'"
    );
}

/// Empty query should return an error (not a panic or empty vec).
#[tokio::test(flavor = "multi_thread")]
async fn test_empty_query_returns_error() {
    let (node, _tmp) = setup_node().await;
    load_schema(&node, "BlogPost.json").await;
    let processor = OperationProcessor::new(node);

    let result = processor.native_index_search("   ").await;
    assert!(result.is_err(), "Blank query should return an error");
}

/// Records from multiple schemas are all searchable in a single query.
#[tokio::test(flavor = "multi_thread")]
async fn test_search_spans_multiple_schemas() {
    let (node, _tmp) = setup_node().await;
    load_schema(&node, "BlogPost.json").await;
    load_schema(&node, "Message.json").await;
    let processor = OperationProcessor::new(node);

    // Insert a BlogPost
    insert_post(
        &processor,
        "2024-01-01",
        "Quantum Computing Advances",
        "Breakthroughs in qubit coherence and error correction.",
        "Alice",
    )
    .await;

    // Insert a Message (different schema, same topic)
    let mut msg_fields = HashMap::new();
    msg_fields.insert("message_id".to_string(), json!("msg-001"));
    msg_fields.insert("conversation_id".to_string(), json!("conv-1"));
    msg_fields.insert("sender_id".to_string(), json!("alice"));
    msg_fields.insert("recipient_id".to_string(), json!("bob"));
    msg_fields.insert("content".to_string(), json!("Excited about quantum computing research!"));
    msg_fields.insert("sent_at".to_string(), json!("2024-01-02T10:00:00Z"));
    msg_fields.insert("read_at".to_string(), json!(""));
    msg_fields.insert("message_type".to_string(), json!("text"));
    msg_fields.insert("attachments".to_string(), json!([]));
    processor
        .execute_mutation(
            "Message".to_string(),
            msg_fields,
            KeyValue::new(Some("conv-1".to_string()), Some("2024-01-02T10:00:00Z".to_string())),
            MutationType::Create,
        )
        .await
        .unwrap();

    let results = processor
        .native_index_search("quantum computing")
        .await
        .expect("Search failed");

    assert!(!results.is_empty(), "Should return results");

    let schemas: std::collections::HashSet<&str> =
        results.iter().map(|r| r.schema_name.as_str()).collect();

    assert!(
        schemas.contains("BlogPost"),
        "Results should include BlogPost records"
    );
    assert!(
        schemas.contains("Message"),
        "Results should include Message records"
    );
}
