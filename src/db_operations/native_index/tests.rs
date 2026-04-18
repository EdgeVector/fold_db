use super::embedding_model::MockEmbeddingModel;
use super::*;
use crate::schema::types::key_value::KeyValue;
use crate::storage::{NamespacedStore, SledNamespacedStore, SledPool};
use std::sync::Arc;

#[allow(deprecated)]
async fn make_manager() -> NativeIndexManager {
    let tmp = tempfile::TempDir::new().unwrap();
    let pool = Arc::new(SledPool::new(tmp.into_path()));
    let store = Arc::new(SledNamespacedStore::new(pool));
    let kv = store.open_namespace("native_index").await.unwrap();
    NativeIndexManager::with_model(kv, Arc::new(MockEmbeddingModel))
}

#[tokio::test]
async fn test_empty_index_returns_empty() {
    let mgr = make_manager().await;
    let results = mgr.search_all_classifications("anything").await.unwrap();
    assert!(results.is_empty());
}

#[tokio::test]
async fn test_blank_query_returns_empty() {
    let mgr = make_manager().await;
    let results = mgr.search_all_classifications("   ").await.unwrap();
    assert!(results.is_empty());
}

#[tokio::test]
async fn test_index_record_then_search() {
    let mgr = make_manager().await;

    let key = KeyValue::new(Some("rec1".to_string()), None);
    let fields = std::collections::HashMap::from([
        ("content".to_string(), serde_json::json!("hello world")),
        ("author".to_string(), serde_json::json!("alice")),
    ]);

    mgr.index_record("Tweet", &key, &fields).await.unwrap();

    let results = mgr.search_all_classifications("hello").await.unwrap();
    assert!(!results.is_empty());

    let schemas: Vec<_> = results.iter().map(|r| r.schema_name.as_str()).collect();
    assert!(schemas.iter().all(|s| *s == "Tweet"));

    let keys: Vec<_> = results.iter().map(|r| r.key_value.clone()).collect();
    assert!(keys.iter().all(|k| k == &key));
}

#[tokio::test]
async fn test_per_field_fragment_indexing() {
    let mgr = make_manager().await;

    let key = KeyValue::new(Some("doc1".to_string()), None);
    let fields = std::collections::HashMap::from([
        ("title".to_string(), serde_json::json!("My Blog Post")),
        ("body".to_string(), serde_json::json!("content here")),
        ("author".to_string(), serde_json::json!("bob")),
    ]);

    mgr.index_record("Post", &key, &fields).await.unwrap();

    // Per-fragment indexing: one embedding per field (each is short text, single fragment)
    let entries = mgr.embedding_index.entries.read().unwrap();
    assert_eq!(entries.len(), 3, "Expected 3 fragments (one per field)");

    let field_names: std::collections::HashSet<_> =
        entries.iter().map(|e| e.field_name.as_str()).collect();
    assert!(field_names.contains("title"));
    assert!(field_names.contains("body"));
    assert!(field_names.contains("author"));
}

#[tokio::test]
async fn test_search_dedup_by_record() {
    let mgr = make_manager().await;

    let key = KeyValue::new(Some("doc1".to_string()), None);
    let fields = std::collections::HashMap::from([
        ("title".to_string(), serde_json::json!("hello world")),
        (
            "body".to_string(),
            serde_json::json!("hello world expanded"),
        ),
    ]);

    mgr.index_record("Post", &key, &fields).await.unwrap();

    // Both fields will have some similarity to "hello world"
    // but search should deduplicate by record key
    let results = mgr.search_all_classifications("hello world").await.unwrap();

    // Should get exactly 1 result (the best-matching fragment for this record)
    assert_eq!(results.len(), 1, "Expected dedup to 1 record");
    assert_eq!(results[0].schema_name, "Post");
    assert_eq!(results[0].key_value, key);
}

#[tokio::test]
async fn test_upsert_replaces_existing_fragment() {
    let mgr = make_manager().await;
    let key = KeyValue::new(Some("rec1".to_string()), None);

    let v1 = std::collections::HashMap::from([("name".to_string(), serde_json::json!("Alice"))]);
    let v2 = std::collections::HashMap::from([("name".to_string(), serde_json::json!("Bob"))]);

    mgr.index_record("User", &key, &v1).await.unwrap();
    mgr.index_record("User", &key, &v2).await.unwrap();

    // Same field + fragment_idx should be upserted, not appended
    let entries = mgr.embedding_index.entries.read().unwrap();
    assert_eq!(entries.len(), 1);
    assert_eq!(entries[0].field_name, "name");
    assert_eq!(entries[0].fragment_text.as_deref(), Some("Bob"));
}

#[tokio::test]
async fn test_results_contain_metadata_with_score() {
    let mgr = make_manager().await;

    let key = KeyValue::new(Some("rec1".to_string()), None);
    let fields =
        std::collections::HashMap::from([("text".to_string(), serde_json::json!("some text"))]);
    mgr.index_record("Schema", &key, &fields).await.unwrap();

    let results = mgr.search_all_classifications("some text").await.unwrap();
    assert!(!results.is_empty());
    let meta = results[0].metadata.as_ref().unwrap();
    assert!(meta.get("score").is_some());
    assert_eq!(meta.get("match_type").unwrap(), "semantic");
}

#[tokio::test]
async fn test_fragment_text_is_stored() {
    let mgr = make_manager().await;

    let key = KeyValue::new(Some("rec1".to_string()), None);
    let fields = std::collections::HashMap::from([(
        "content".to_string(),
        serde_json::json!("hello world"),
    )]);

    mgr.index_record("Test", &key, &fields).await.unwrap();

    let entries = mgr.embedding_index.entries.read().unwrap();
    assert_eq!(entries.len(), 1);
    assert_eq!(entries[0].fragment_text.as_deref(), Some("hello world"));
}

#[tokio::test]
#[allow(deprecated)]
async fn test_restore_from_store_loads_existing_embeddings() {
    let tmp = tempfile::TempDir::new().unwrap();
    let pool = Arc::new(SledPool::new(tmp.into_path()));
    let store = Arc::new(SledNamespacedStore::new(pool));
    let kv = store.open_namespace("native_index").await.unwrap();

    // Index a record with manager 1
    let mgr1 = NativeIndexManager::with_model(kv.clone(), Arc::new(MockEmbeddingModel));
    let key = KeyValue::new(Some("rec1".to_string()), None);
    let fields =
        std::collections::HashMap::from([("field".to_string(), serde_json::json!("value"))]);
    mgr1.index_record("S", &key, &fields).await.unwrap();

    // Create manager 2 with same store, restore from store
    let mgr2 = NativeIndexManager::with_model(kv, Arc::new(MockEmbeddingModel));
    mgr2.restore_from_store().await;

    let entries = mgr2.embedding_index.entries.read().unwrap();
    assert_eq!(entries.len(), 1);
    assert_eq!(entries[0].schema, "S");
    assert_eq!(entries[0].field_name, "field");
    assert_eq!(entries[0].fragment_text.as_deref(), Some("value"));
}

#[tokio::test]
async fn test_multi_field_record_different_values() {
    let mgr = make_manager().await;

    let key = KeyValue::new(Some("rec1".to_string()), None);
    let fields = std::collections::HashMap::from([
        ("title".to_string(), serde_json::json!("Rust Programming")),
        ("category".to_string(), serde_json::json!("technology")),
    ]);

    mgr.index_record("Article", &key, &fields).await.unwrap();

    let entries = mgr.embedding_index.entries.read().unwrap();
    assert_eq!(entries.len(), 2);

    // Each field should have its own fragment_text
    let texts: std::collections::HashSet<_> = entries
        .iter()
        .filter_map(|e| e.fragment_text.as_deref())
        .collect();
    assert!(texts.contains("Rust Programming"));
    assert!(texts.contains("technology"));
}

#[tokio::test]
async fn test_null_field_skipped() {
    let mgr = make_manager().await;

    let key = KeyValue::new(Some("rec1".to_string()), None);
    let fields = std::collections::HashMap::from([
        ("content".to_string(), serde_json::json!("hello")),
        ("optional".to_string(), serde_json::Value::Null),
    ]);

    mgr.index_record("Test", &key, &fields).await.unwrap();

    let entries = mgr.embedding_index.entries.read().unwrap();
    assert_eq!(entries.len(), 1, "Null field should be skipped");
    assert_eq!(entries[0].field_name, "content");
}

#[cfg(feature = "face-detection")]
#[tokio::test]
async fn test_detect_faces_errors_without_processor() {
    let mgr = make_manager().await;
    let result = mgr.detect_faces(b"not-an-image");
    assert!(result.is_err(), "expected error when no face processor configured");
    let msg = format!("{:?}", result.unwrap_err());
    assert!(
        msg.contains("No face processor configured"),
        "unexpected error: {}",
        msg
    );
}
