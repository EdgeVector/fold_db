use super::*;
use crate::schema::types::key_value::KeyValue;
use crate::storage::{NamespacedStore, SledNamespacedStore};

#[tokio::test]
async fn test_async_indexing_flow() {
    // Setup async store using Sled backend wrapped in NamespacedStore
    let db = sled::Config::new().temporary(true).open().unwrap();
    let store = std::sync::Arc::new(SledNamespacedStore::new(db));
    let kv_store = store.open_namespace("native_index").await.unwrap();

    let manager = NativeIndexManager::new_with_store(kv_store);
    assert!(manager.is_async());

    let operations = vec![(
        "AsyncSchema".to_string(),
        "content".to_string(),
        KeyValue::new(Some("k1".to_string()), None),
        serde_json::Value::String("Jennifer wrote async code".to_string()),
        None, // Default to word classification
    )];

    // Index using append-only method
    manager
        .batch_index(&operations)
        .await
        .expect("indexing failed");

    // Search using append-only method
    let results = manager
        .search("Jennifer")
        .await
        .expect("search failed");

    assert_eq!(results.len(), 1, "Should find 1 result for Jennifer");
    assert_eq!(results[0].key, KeyValue::new(Some("k1".to_string()), None));

    // Search parts
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

    let manager = NativeIndexManager::new_with_store(kv_store);

    let operations = vec![(
        "TestSchema".to_string(),
        "test_field".to_string(),
        KeyValue::new(Some("key1".to_string()), None),
        serde_json::Value::String("hello world".to_string()),
        Some(vec![]), // Empty classifications - should default to "word"
    )];

    // Index using append-only method (should default to "word" indexing)
    manager
        .batch_index(&operations)
        .await
        .expect("indexing failed");

    // Verify "word" search works
    let results = manager
        .search("hello")
        .await
        .expect("search failed");
    assert_eq!(results.len(), 1);
    assert_eq!(
        results[0].key,
        KeyValue::new(Some("key1".to_string()), None)
    );

    // Verify classification is "word"
    assert_eq!(results[0].classification, "word");
}

#[tokio::test]
async fn test_async_indexing_complex_tweet() {
    // Setup async store using Sled backend wrapped in NamespacedStore
    let db = sled::Config::new().temporary(true).open().unwrap();
    let store = std::sync::Arc::new(SledNamespacedStore::new(db));
    let kv_store = store.open_namespace("native_index").await.unwrap();

    let manager = NativeIndexManager::new_with_store(kv_store);

    let tweet_content = "RT @TwitterDev: Hello world! ... https://t.co/123456";
    let operations = vec![(
        "TwitterSchema".to_string(),
        "content".to_string(),
        KeyValue::new(Some("tweet_1".to_string()), None),
        serde_json::Value::String(tweet_content.to_string()),
        Some(vec!["word".to_string()]),
    )];

    // Index using append-only method
    manager
        .batch_index(&operations)
        .await
        .expect("indexing failed");

    // Search "Hello"
    let results = manager
        .search("Hello")
        .await
        .expect("search failed for Hello");

    assert_eq!(results.len(), 1, "Should find 1 result for Hello");

    // Search "world"
    let results = manager
        .search("world")
        .await
        .expect("search failed for world");

    assert_eq!(results.len(), 1, "Should find 1 result for world");

    // Search "https" (part of URL, should be extracted as word "https")
    let results = manager
        .search("https")
        .await
        .expect("search failed for https");

    assert_eq!(results.len(), 1, "Should find 1 result for https");
}

// ========== INDEX TESTS ==========

#[tokio::test]
async fn test_append_only_basic_indexing() {
    let db = sled::Config::new().temporary(true).open().unwrap();
    let store = std::sync::Arc::new(SledNamespacedStore::new(db));
    let kv_store = store.open_namespace("native_index").await.unwrap();

    let manager = NativeIndexManager::new_with_store(kv_store);

    let operations = vec![(
        "TestSchema".to_string(),
        "content".to_string(),
        KeyValue::new(Some("key1".to_string()), None),
        serde_json::Value::String("hello world from append-only index".to_string()),
        None,
    )];

    // Index using append-only method
    manager
        .batch_index(&operations)
        .await
        .expect("append-only indexing failed");

    // Search using append-only method
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
async fn test_append_only_multiple_records() {
    let db = sled::Config::new().temporary(true).open().unwrap();
    let store = std::sync::Arc::new(SledNamespacedStore::new(db));
    let kv_store = store.open_namespace("native_index").await.unwrap();

    let manager = NativeIndexManager::new_with_store(kv_store);

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
        .expect("append-only indexing failed");

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
async fn test_append_only_field_name_search() {
    let db = sled::Config::new().temporary(true).open().unwrap();
    let store = std::sync::Arc::new(SledNamespacedStore::new(db));
    let kv_store = store.open_namespace("native_index").await.unwrap();

    let manager = NativeIndexManager::new_with_store(kv_store);

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
        .expect("append-only indexing failed");

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
