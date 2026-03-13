use super::*;
use super::embedding_model::MockEmbeddingModel;
use crate::schema::types::key_value::KeyValue;
use crate::storage::{NamespacedStore, SledNamespacedStore};
use std::sync::Arc;

async fn make_manager() -> NativeIndexManager {
    let db = sled::Config::new().temporary(true).open().unwrap();
    let store = Arc::new(SledNamespacedStore::new(db));
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
async fn test_per_field_indexing_creates_separate_embeddings() {
    let mgr = make_manager().await;

    let key = KeyValue::new(Some("doc1".to_string()), None);
    let fields = std::collections::HashMap::from([
        ("title".to_string(), serde_json::json!("My Blog Post")),
        ("body".to_string(), serde_json::json!("content here")),
        ("author".to_string(), serde_json::json!("bob")),
    ]);

    mgr.index_record("Post", &key, &fields).await.unwrap();

    // Per-field indexing produces one embedding entry per field
    let entries = mgr.embedding_index.entries.read().unwrap();
    assert_eq!(entries.len(), 3, "Expected 3 entries (one per field)");

    let field_names: std::collections::HashSet<_> =
        entries.iter().map(|e| e.field_name.as_str()).collect();
    assert!(field_names.contains("title"));
    assert!(field_names.contains("body"));
    assert!(field_names.contains("author"));
}

#[tokio::test]
async fn test_search_deduplicates_by_record() {
    let mgr = make_manager().await;

    let key = KeyValue::new(Some("doc1".to_string()), None);
    let fields = std::collections::HashMap::from([
        ("title".to_string(), serde_json::json!("hello world")),
        ("body".to_string(), serde_json::json!("hello there")),
        ("tags".to_string(), serde_json::json!("hello greetings")),
    ]);

    mgr.index_record("Post", &key, &fields).await.unwrap();

    // Search should return at most 1 result for this record (deduped by record key)
    let results = mgr.search_all_classifications("hello").await.unwrap();
    assert_eq!(results.len(), 1, "Expected deduplication to 1 result per record");
    assert_eq!(results[0].schema_name, "Post");
}

#[tokio::test]
async fn test_upsert_replaces_existing_entry() {
    let mgr = make_manager().await;
    let key = KeyValue::new(Some("rec1".to_string()), None);

    let v1 = std::collections::HashMap::from([
        ("name".to_string(), serde_json::json!("Alice")),
    ]);
    let v2 = std::collections::HashMap::from([
        ("name".to_string(), serde_json::json!("Alice")),
        ("role".to_string(), serde_json::json!("admin")),
    ]);

    mgr.index_record("User", &key, &v1).await.unwrap();
    mgr.index_record("User", &key, &v2).await.unwrap();

    let entries = mgr.embedding_index.entries.read().unwrap();
    // v1 had 1 field (name), v2 has 2 fields (name, role).
    // "name" is upserted (same key), "role" is new → total 2
    assert_eq!(entries.len(), 2);
}

#[tokio::test]
async fn test_results_contain_metadata_with_score() {
    let mgr = make_manager().await;

    let key = KeyValue::new(Some("rec1".to_string()), None);
    let fields = std::collections::HashMap::from([
        ("text".to_string(), serde_json::json!("some text")),
    ]);
    mgr.index_record("Schema", &key, &fields).await.unwrap();

    let results = mgr.search_all_classifications("some text").await.unwrap();
    assert!(!results.is_empty());
    let meta = results[0].metadata.as_ref().unwrap();
    assert!(meta.get("score").is_some());
    assert_eq!(meta.get("match_type").unwrap(), "semantic");
}

#[tokio::test]
async fn test_restore_from_store_loads_existing_embeddings() {
    let db = sled::Config::new().temporary(true).open().unwrap();
    let store = Arc::new(SledNamespacedStore::new(db));
    let kv = store.open_namespace("native_index").await.unwrap();

    // Index a record with manager 1
    let mgr1 = NativeIndexManager::with_model(kv.clone(), Arc::new(MockEmbeddingModel));
    let key = KeyValue::new(Some("rec1".to_string()), None);
    let fields = std::collections::HashMap::from([
        ("field".to_string(), serde_json::json!("value")),
    ]);
    mgr1.index_record("S", &key, &fields).await.unwrap();

    // Create manager 2 with same store, restore from store
    let mgr2 = NativeIndexManager::with_model(kv, Arc::new(MockEmbeddingModel));
    mgr2.restore_from_store().await;

    let entries = mgr2.embedding_index.entries.read().unwrap();
    assert_eq!(entries.len(), 1);
    assert_eq!(entries[0].schema, "S");
    assert_eq!(entries[0].field_name, "field");
}

#[tokio::test]
async fn test_multiple_records_return_separate_results() {
    let mgr = make_manager().await;

    let key1 = KeyValue::new(Some("rec1".to_string()), None);
    let key2 = KeyValue::new(Some("rec2".to_string()), None);

    let fields1 = std::collections::HashMap::from([
        ("content".to_string(), serde_json::json!("chocolate cake recipe")),
    ]);
    let fields2 = std::collections::HashMap::from([
        ("content".to_string(), serde_json::json!("vanilla cake recipe")),
    ]);

    mgr.index_record("Recipe", &key1, &fields1).await.unwrap();
    mgr.index_record("Recipe", &key2, &fields2).await.unwrap();

    let results = mgr.search_all_classifications("cake recipe").await.unwrap();
    // Should return 2 results (one per record, deduped)
    assert_eq!(results.len(), 2);
}
