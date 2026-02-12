use super::*;
use crate::schema::types::key_value::KeyValue;
use crate::storage::{NamespacedStore, SledNamespacedStore};

#[tokio::test]
async fn test_basic_indexing_flow() {
    let db = sled::Config::new().temporary(true).open().unwrap();
    let store = std::sync::Arc::new(SledNamespacedStore::new(db));
    let kv_store = store.open_namespace("native_index").await.unwrap();

    let manager = NativeIndexManager::new(kv_store);

    let operations = vec![(
        "AsyncSchema".to_string(),
        "content".to_string(),
        KeyValue::new(Some("k1".to_string()), None),
        serde_json::Value::String("Jennifer wrote async code".to_string()),
        None,
    )];

    manager
        .batch_index(&operations)
        .await
        .expect("indexing failed");

    let results = manager
        .search("Jennifer")
        .await
        .expect("search failed");

    assert_eq!(results.len(), 1, "Should find 1 result for Jennifer");
    assert_eq!(results[0].key, KeyValue::new(Some("k1".to_string()), None));

    let results = manager
        .search("async")
        .await
        .expect("search failed");
    assert_eq!(results.len(), 1);

    // Verify we can find by field name
    let results = manager
        .search_all("content")
        .await
        .expect("field search");
    assert!(!results.is_empty());
}

#[tokio::test]
async fn test_indexing_with_empty_classifications() {
    let db = sled::Config::new().temporary(true).open().unwrap();
    let store = std::sync::Arc::new(SledNamespacedStore::new(db));
    let kv_store = store.open_namespace("native_index").await.unwrap();

    let manager = NativeIndexManager::new(kv_store);

    let operations = vec![(
        "TestSchema".to_string(),
        "test_field".to_string(),
        KeyValue::new(Some("key1".to_string()), None),
        serde_json::Value::String("hello world".to_string()),
        Some(vec![]),
    )];

    manager
        .batch_index(&operations)
        .await
        .expect("indexing failed");

    let results = manager
        .search("hello")
        .await
        .expect("search failed");
    assert_eq!(results.len(), 1);
    assert_eq!(
        results[0].key,
        KeyValue::new(Some("key1".to_string()), None)
    );

    assert_eq!(results[0].classification, "word");
}

#[tokio::test]
async fn test_indexing_complex_tweet() {
    let db = sled::Config::new().temporary(true).open().unwrap();
    let store = std::sync::Arc::new(SledNamespacedStore::new(db));
    let kv_store = store.open_namespace("native_index").await.unwrap();

    let manager = NativeIndexManager::new(kv_store);

    let tweet_content = "RT @TwitterDev: Hello world! ... https://t.co/123456";
    let operations = vec![(
        "TwitterSchema".to_string(),
        "content".to_string(),
        KeyValue::new(Some("tweet_1".to_string()), None),
        serde_json::Value::String(tweet_content.to_string()),
        Some(vec!["word".to_string()]),
    )];

    manager
        .batch_index(&operations)
        .await
        .expect("indexing failed");

    let results = manager
        .search("Hello")
        .await
        .expect("search failed for Hello");

    assert_eq!(results.len(), 1, "Should find 1 result for Hello");

    let results = manager
        .search("world")
        .await
        .expect("search failed for world");

    assert_eq!(results.len(), 1, "Should find 1 result for world");

    let results = manager
        .search("https")
        .await
        .expect("search failed for https");

    assert_eq!(results.len(), 1, "Should find 1 result for https");
}

// ========== INDEX TESTS ==========

#[tokio::test]
async fn test_single_record_indexing() {
    let db = sled::Config::new().temporary(true).open().unwrap();
    let store = std::sync::Arc::new(SledNamespacedStore::new(db));
    let kv_store = store.open_namespace("native_index").await.unwrap();

    let manager = NativeIndexManager::new(kv_store);

    let operations = vec![(
        "TestSchema".to_string(),
        "content".to_string(),
        KeyValue::new(Some("key1".to_string()), None),
        serde_json::Value::String("hello world from the index".to_string()),
        None,
    )];

    manager
        .batch_index(&operations)
        .await
        .expect("indexing failed");

    let results = manager
        .search("hello")
        .await
        .expect("search failed");

    assert_eq!(results.len(), 1, "Should find 1 result for hello");
    assert_eq!(results[0].schema, "TestSchema");
    assert_eq!(results[0].field, "content");
    assert_eq!(
        results[0].key,
        KeyValue::new(Some("key1".to_string()), None)
    );
}

#[tokio::test]
async fn test_multiple_record_indexing() {
    let db = sled::Config::new().temporary(true).open().unwrap();
    let store = std::sync::Arc::new(SledNamespacedStore::new(db));
    let kv_store = store.open_namespace("native_index").await.unwrap();

    let manager = NativeIndexManager::new(kv_store);

    let operations = vec![
        (
            "Tweet".to_string(),
            "content".to_string(),
            KeyValue::new(Some("tweet1".to_string()), None),
            serde_json::Value::String("hello from tweet one".to_string()),
            None,
        ),
        (
            "Tweet".to_string(),
            "content".to_string(),
            KeyValue::new(Some("tweet2".to_string()), None),
            serde_json::Value::String("hello from tweet two".to_string()),
            None,
        ),
        (
            "Tweet".to_string(),
            "content".to_string(),
            KeyValue::new(Some("tweet3".to_string()), None),
            serde_json::Value::String("goodbye from tweet three".to_string()),
            None,
        ),
    ];

    manager
        .batch_index(&operations)
        .await
        .expect("indexing failed");

    // Search for "hello" - should find 2 results
    let results = manager
        .search("hello")
        .await
        .expect("search failed");
    assert_eq!(results.len(), 2, "Should find 2 results for hello");

    // Search for "goodbye" - should find 1 result
    let results = manager
        .search("goodbye")
        .await
        .expect("search failed");
    assert_eq!(results.len(), 1, "Should find 1 result for goodbye");

    // Search for "tweet" - should find 3 results
    let results = manager
        .search("tweet")
        .await
        .expect("search failed");
    assert_eq!(results.len(), 3, "Should find 3 results for tweet");
}

#[tokio::test]
async fn test_field_name_search() {
    let db = sled::Config::new().temporary(true).open().unwrap();
    let store = std::sync::Arc::new(SledNamespacedStore::new(db));
    let kv_store = store.open_namespace("native_index").await.unwrap();

    let manager = NativeIndexManager::new(kv_store);

    let operations = vec![(
        "User".to_string(),
        "email".to_string(),
        KeyValue::new(Some("user1".to_string()), None),
        serde_json::Value::String("test@example.com".to_string()),
        None,
    )];

    manager
        .batch_index(&operations)
        .await
        .expect("indexing failed");

    // Search for field name "email"
    let results = manager
        .search_all("email")
        .await
        .expect("search failed");

    assert!(
        !results.is_empty(),
        "Should find results for field name email"
    );
}

#[test]
fn test_index_entry_storage_key() {
    let entry = IndexEntry::with_timestamp(
        "Tweet".to_string(),
        KeyValue::new(Some("abc123".to_string()), None),
        "content".to_string(),
        "word".to_string(),
        1705312200000,
    );

    let key = entry.storage_key("hello");
    // Format: idx:{term}:{timestamp}:{schema}:{field}:{key_hash}
    assert!(key.starts_with("idx:hello:1705312200000:Tweet:content:"));
    assert!(key.contains("abc123"));
}

#[test]
fn test_index_entry_to_result_conversion() {
    let entry = IndexEntry::new(
        "Tweet".to_string(),
        KeyValue::new(Some("abc123".to_string()), None),
        "content".to_string(),
        "word".to_string(),
    );

    let result = entry.to_index_result(Some(serde_json::json!("test value")));

    assert_eq!(result.schema_name, "Tweet");
    assert_eq!(result.field, "content");
    assert_eq!(
        result.key_value,
        KeyValue::new(Some("abc123".to_string()), None)
    );
    assert_eq!(result.value, serde_json::json!("test value"));
    assert!(result.metadata.is_some());
}

#[tokio::test]
async fn test_batch_index_from_keywords() {
    let db = sled::Config::new().temporary(true).open().unwrap();
    let store = std::sync::Arc::new(SledNamespacedStore::new(db));
    let kv_store = store.open_namespace("native_index").await.unwrap();

    let manager = NativeIndexManager::new(kv_store);

    let key = KeyValue::new(Some("rec1".to_string()), None);
    let keywords = vec![
        "machine learning".to_string(),
        "neural network".to_string(),
        "deep learning".to_string(),
    ];

    manager
        .batch_index_from_keywords("AiSchema", &key, keywords)
        .await
        .expect("batch_index_from_keywords failed");

    // Each keyword should be searchable
    for term in &["machine learning", "neural network", "deep learning"] {
        let results = manager.search(term).await.expect("search failed");
        assert_eq!(results.len(), 1, "Should find 1 result for '{}'", term);
        assert_eq!(results[0].field, "llm_keyword");
    }

    // Single-char term should return nothing (below min length)
    let results = manager.search("a").await.expect("search failed");
    assert!(results.is_empty(), "Single-char term should return nothing");
}

#[tokio::test]
async fn test_multi_word_search_intersection() {
    let db = sled::Config::new().temporary(true).open().unwrap();
    let store = std::sync::Arc::new(SledNamespacedStore::new(db));
    let kv_store = store.open_namespace("native_index").await.unwrap();

    let manager = NativeIndexManager::new(kv_store);

    let operations = vec![
        (
            "People".to_string(),
            "name".to_string(),
            KeyValue::new(Some("p1".to_string()), None),
            serde_json::Value::String("alice johnson".to_string()),
            None,
        ),
        (
            "People".to_string(),
            "name".to_string(),
            KeyValue::new(Some("p2".to_string()), None),
            serde_json::Value::String("alice smith".to_string()),
            None,
        ),
    ];

    manager
        .batch_index(&operations)
        .await
        .expect("indexing failed");

    // Single-word "alice" should find both records
    let results = manager.search("alice").await.expect("search failed");
    assert_eq!(results.len(), 2, "Should find 2 results for 'alice'");

    // Multi-word "alice johnson" should find only p1
    let results = manager
        .search("alice johnson")
        .await
        .expect("search failed");
    assert_eq!(
        results.len(),
        1,
        "Should find 1 result for 'alice johnson'"
    );
    assert_eq!(
        results[0].key,
        KeyValue::new(Some("p1".to_string()), None)
    );

    // "johnson smith" should find none (no single record has both)
    let results = manager
        .search("johnson smith")
        .await
        .expect("search failed");
    assert_eq!(
        results.len(),
        0,
        "Should find 0 results for 'johnson smith'"
    );
}
