use super::*;
use super::embedding_model::MockEmbeddingModel;
use crate::schema::types::key_value::KeyValue;
use crate::storage::{NamespacedStore, SledNamespacedStore};
use std::sync::Arc;

async fn make_manager() -> NativeIndexManager {
    let db = sled::Config::new().temporary(true).open().unwrap();
    let store = Arc::new(SledNamespacedStore::new(db));
    let kv = store.open_namespace("native_index").await.unwrap();
    NativeIndexManager::new(kv, Arc::new(MockEmbeddingModel))
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
async fn test_index_record_produces_one_result_per_field() {
    let mgr = make_manager().await;

    let key = KeyValue::new(Some("doc1".to_string()), None);
    let fields = std::collections::HashMap::from([
        ("title".to_string(), serde_json::json!("My Blog Post")),
        ("body".to_string(), serde_json::json!("content here")),
        ("author".to_string(), serde_json::json!("bob")),
    ]);

    mgr.index_record("Post", &key, &fields).await.unwrap();

    let results = mgr.search_all_classifications("blog").await.unwrap();
    // One IndexResult per field in the matched document
    assert_eq!(results.len(), 3);
    let field_names: std::collections::HashSet<_> = results.iter().map(|r| r.field.as_str()).collect();
    assert!(field_names.contains("title"));
    assert!(field_names.contains("body"));
    assert!(field_names.contains("author"));
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

    // Only one document in the index (upserted, not appended)
    let entries = mgr.embedding_index.entries.read().unwrap();
    assert_eq!(entries.len(), 1);
    assert_eq!(entries[0].field_names.len(), 2);
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
    let mgr1 = NativeIndexManager::new(kv.clone(), Arc::new(MockEmbeddingModel));
    let key = KeyValue::new(Some("rec1".to_string()), None);
    let fields = std::collections::HashMap::from([
        ("field".to_string(), serde_json::json!("value")),
    ]);
    mgr1.index_record("S", &key, &fields).await.unwrap();

    // Create manager 2 with same store, restore from store
    let mgr2 = NativeIndexManager::new(kv, Arc::new(MockEmbeddingModel));
    mgr2.restore_from_store().await;

    let entries = mgr2.embedding_index.entries.read().unwrap();
    assert_eq!(entries.len(), 1);
    assert_eq!(entries[0].schema, "S");
}
