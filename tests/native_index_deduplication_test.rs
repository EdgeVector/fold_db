//! Unit tests for native index deduplication logic
//!
//! These tests verify that the append-only index properly handles entries with
//! different classifications for the same field value.

use fold_db::db_operations::NativeIndexManager;
use fold_db::schema::types::key_value::KeyValue;
use fold_db::storage::{NamespacedStore, SledNamespacedStore};
use serde_json::json;
use std::sync::Arc;
use tempfile::TempDir;

// BatchIndexOperation is (schema_name, field_name, key_value, value, classifications)
type BatchIndexOperation = (
    String,
    String,
    KeyValue,
    serde_json::Value,
    Option<Vec<String>>,
);

#[tokio::test]
async fn test_append_only_preserves_different_classifications() {
    eprintln!("\n=== Testing append-only preserves different classifications ===\n");

    let temp_dir = TempDir::new().expect("failed to create temp dir");
    let db_path = temp_dir.path().to_path_buf();
    let db = sled::open(&db_path).expect("failed to open sled db");
    let store = Arc::new(SledNamespacedStore::new(db));
    let kv_store = store.open_namespace("native_index").await.unwrap();

    let manager = NativeIndexManager::new(kv_store);

    // Create index operations for the same field with different classifications
    let operations: Vec<BatchIndexOperation> = vec![
        (
            "TestPost".to_string(),
            "content".to_string(),
            KeyValue::new(Some("record1".to_string()), None),
            json!("John Smith"),
            Some(vec!["word".to_string()]),
        ),
        (
            "TestPost".to_string(),
            "content".to_string(),
            KeyValue::new(Some("record1".to_string()), None),
            json!("John Smith"),
            Some(vec!["name:person".to_string()]),
        ),
    ];

    // Batch index using append-only method
    manager
        .batch_index(&operations)
        .await
        .expect("batch indexing should succeed");

    // Verify word entries are preserved
    let word_results = manager
        .search("john")
        .await
        .expect("search should succeed");
    eprintln!("Word search results: {} entries", word_results.len());

    // Verify we have entries for record1
    let word_entries: Vec<_> = word_results
        .iter()
        .filter(|r| r.key.hash.as_deref() == Some("record1"))
        .collect();
    assert!(
        !word_entries.is_empty(),
        "Should have word entries for record1"
    );

    eprintln!("✅ SUCCESS: Different classifications preserved correctly!\n");
}

#[tokio::test]
async fn test_append_only_handles_duplicate_same_classification() {
    eprintln!("\n=== Testing append-only handles duplicates with same classification ===\n");

    let temp_dir = TempDir::new().expect("failed to create temp dir");
    let db_path = temp_dir.path().to_path_buf();
    let db = sled::open(&db_path).expect("failed to open sled db");
    let store = Arc::new(SledNamespacedStore::new(db));
    let kv_store = store.open_namespace("native_index").await.unwrap();

    let manager = NativeIndexManager::new(kv_store);

    // Create duplicate index operations with the same classification
    let operations: Vec<BatchIndexOperation> = vec![
        (
            "TestPost".to_string(),
            "title".to_string(),
            KeyValue::new(Some("record1".to_string()), None),
            json!("Learning Rust"),
            Some(vec!["word".to_string()]),
        ),
        // Duplicate entry with same classification
        (
            "TestPost".to_string(),
            "title".to_string(),
            KeyValue::new(Some("record1".to_string()), None),
            json!("Learning Rust"),
            Some(vec!["word".to_string()]),
        ),
    ];

    // Batch index using append-only method
    manager
        .batch_index(&operations)
        .await
        .expect("batch indexing should succeed");

    // Verify entries exist (append-only may have duplicates, but that's OK for search)
    let learning_results = manager
        .search("learning")
        .await
        .expect("search should succeed");
    eprintln!(
        "Search results for 'learning': {} entries",
        learning_results.len()
    );

    let learning_entries: Vec<_> = learning_results
        .iter()
        .filter(|r| r.key.hash.as_deref() == Some("record1"))
        .collect();

    assert!(
        !learning_entries.is_empty(),
        "Should have entries for record1"
    );

    eprintln!("✅ SUCCESS: Duplicate handling works correctly!\n");
}

#[tokio::test]
async fn test_append_only_across_different_fields_same_record() {
    eprintln!("\n=== Testing append-only across different fields same record ===\n");

    let temp_dir = TempDir::new().expect("failed to create temp dir");
    let db_path = temp_dir.path().to_path_buf();
    let db = sled::open(&db_path).expect("failed to open sled db");
    let store = Arc::new(SledNamespacedStore::new(db));
    let kv_store = store.open_namespace("native_index").await.unwrap();

    let manager = NativeIndexManager::new(kv_store);

    // Create operations for different fields with same word
    let operations: Vec<BatchIndexOperation> = vec![
        (
            "TestPost".to_string(),
            "title".to_string(),
            KeyValue::new(Some("record1".to_string()), None),
            json!("Rust Programming"),
            Some(vec!["word".to_string()]),
        ),
        (
            "TestPost".to_string(),
            "content".to_string(),
            KeyValue::new(Some("record1".to_string()), None),
            json!("Learn Rust today"),
            Some(vec!["word".to_string()]),
        ),
    ];

    // Batch index using append-only method
    manager
        .batch_index(&operations)
        .await
        .expect("batch indexing should succeed");

    // Verify both fields are indexed separately for the word "rust"
    let rust_results = manager
        .search("rust")
        .await
        .expect("search should succeed");
    eprintln!("Search results for 'rust': {} entries", rust_results.len());

    let title_entries: Vec<_> = rust_results
        .iter()
        .filter(|r| r.key.hash.as_deref() == Some("record1") && r.field == "title")
        .collect();

    let content_entries: Vec<_> = rust_results
        .iter()
        .filter(|r| r.key.hash.as_deref() == Some("record1") && r.field == "content")
        .collect();

    assert!(!title_entries.is_empty(), "Should have title entry");
    assert!(!content_entries.is_empty(), "Should have content entry");

    eprintln!("✅ SUCCESS: Different fields preserved correctly!\n");
}

#[tokio::test]
async fn test_append_only_handles_same_field_different_records() {
    eprintln!("\n=== Testing append-only with same field different records ===\n");

    let temp_dir = TempDir::new().expect("failed to create temp dir");
    let db_path = temp_dir.path().to_path_buf();
    let db = sled::open(&db_path).expect("failed to open sled db");
    let store = Arc::new(SledNamespacedStore::new(db));
    let kv_store = store.open_namespace("native_index").await.unwrap();

    let manager = NativeIndexManager::new(kv_store);

    // Create operations for different records with same content
    let operations: Vec<BatchIndexOperation> = vec![
        (
            "TestPost".to_string(),
            "title".to_string(),
            KeyValue::new(Some("record1".to_string()), None),
            json!("Shared Title"),
            Some(vec!["word".to_string()]),
        ),
        (
            "TestPost".to_string(),
            "title".to_string(),
            KeyValue::new(Some("record2".to_string()), None),
            json!("Shared Title"),
            Some(vec!["word".to_string()]),
        ),
        (
            "TestPost".to_string(),
            "title".to_string(),
            KeyValue::new(Some("record3".to_string()), None),
            json!("Shared Title"),
            Some(vec!["word".to_string()]),
        ),
    ];

    // Batch index using append-only method
    manager
        .batch_index(&operations)
        .await
        .expect("batch indexing should succeed");

    // Verify all records are preserved
    let shared_results = manager
        .search("shared")
        .await
        .expect("search should succeed");
    eprintln!(
        "Search results for 'shared': {} entries",
        shared_results.len()
    );

    assert_eq!(
        shared_results.len(),
        3,
        "Should have 3 entries (one per record)"
    );

    let record_count_1 = shared_results
        .iter()
        .filter(|r| r.key.hash.as_deref() == Some("record1"))
        .count();
    let record_count_2 = shared_results
        .iter()
        .filter(|r| r.key.hash.as_deref() == Some("record2"))
        .count();
    let record_count_3 = shared_results
        .iter()
        .filter(|r| r.key.hash.as_deref() == Some("record3"))
        .count();

    assert_eq!(record_count_1, 1, "record1 should be present");
    assert_eq!(record_count_2, 1, "record2 should be present");
    assert_eq!(record_count_3, 1, "record3 should be present");

    eprintln!("✅ SUCCESS: Different records preserved correctly!\n");
}
