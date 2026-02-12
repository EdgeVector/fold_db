use super::*;
use crate::schema::types::key_value::KeyValue;
use crate::storage::{NamespacedStore, SledNamespacedStore};

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

    // Index keywords for two "people" records using batch_index_from_keywords
    let key_p1 = KeyValue::new(Some("p1".to_string()), None);
    manager
        .batch_index_from_keywords(
            "People",
            &key_p1,
            vec!["alice".to_string(), "johnson".to_string()],
        )
        .await
        .expect("indexing p1 failed");

    let key_p2 = KeyValue::new(Some("p2".to_string()), None);
    manager
        .batch_index_from_keywords(
            "People",
            &key_p2,
            vec!["alice".to_string(), "smith".to_string()],
        )
        .await
        .expect("indexing p2 failed");

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
