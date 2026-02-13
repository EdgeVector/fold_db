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
    // Format: idx:{term}:{schema}:{field}:{key_hash}
    assert!(key.starts_with("idx:hello:Tweet:content:"));
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

    let manager = NativeIndexManager::new(kv_store, None);

    let key = KeyValue::new(Some("rec1".to_string()), None);
    let keywords = vec![
        "machine learning".to_string(),
        "neural network".to_string(),
        "deep learning".to_string(),
    ];

    manager
        .batch_index_from_keywords("AiSchema", &key, keywords, None)
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

    let manager = NativeIndexManager::new(kv_store, None);

    // Index keywords for two "people" records using batch_index_from_keywords
    let key_p1 = KeyValue::new(Some("p1".to_string()), None);
    manager
        .batch_index_from_keywords(
            "People",
            &key_p1,
            vec!["alice".to_string(), "johnson".to_string()],
            None,
        )
        .await
        .expect("indexing p1 failed");

    let key_p2 = KeyValue::new(Some("p2".to_string()), None);
    manager
        .batch_index_from_keywords(
            "People",
            &key_p2,
            vec!["alice".to_string(), "smith".to_string()],
            None,
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

#[test]
fn test_normalize_search_term_edge_cases() {
    // Empty string
    assert_eq!(NativeIndexManager::normalize_search_term(""), None);

    // Single character (below min length of 2)
    assert_eq!(NativeIndexManager::normalize_search_term("a"), None);

    // Whitespace only
    assert_eq!(NativeIndexManager::normalize_search_term("   "), None);

    // Exactly 2 characters (minimum)
    assert_eq!(
        NativeIndexManager::normalize_search_term("ab"),
        Some("ab".to_string())
    );

    // Normal term with mixed case
    assert_eq!(
        NativeIndexManager::normalize_search_term("  Hello World  "),
        Some("hello world".to_string())
    );
}

#[tokio::test]
async fn test_batch_index_field_names() {
    let db = sled::Config::new().temporary(true).open().unwrap();
    let store = std::sync::Arc::new(SledNamespacedStore::new(db));
    let kv_store = store.open_namespace("native_index").await.unwrap();

    let manager = NativeIndexManager::new(kv_store, None);

    let key = KeyValue::new(Some("rec1".to_string()), None);
    let field_names = vec![
        "username".to_string(),
        "email".to_string(),
        "bio".to_string(),
    ];

    manager
        .batch_index_field_names("UserSchema", &key, &field_names, None)
        .await
        .expect("batch_index_field_names failed");

    // Each field name should be searchable via search_all
    for field in &["username", "email", "bio"] {
        let results = manager.search_all(field).await.expect("search_all failed");
        assert_eq!(results.len(), 1, "Should find 1 result for field '{}'", field);
        assert_eq!(results[0].classification, "field");
    }

    // Excluded fields should not be indexed
    let key2 = KeyValue::new(Some("rec2".to_string()), None);
    let with_excluded = vec!["password".to_string(), "display_name".to_string()];
    manager
        .batch_index_field_names("UserSchema", &key2, &with_excluded, None)
        .await
        .expect("batch_index_field_names failed");

    let results = manager.search_all("password").await.expect("search failed");
    assert!(results.is_empty(), "Excluded field 'password' should not be indexed");

    let results = manager.search_all("display_name").await.expect("search failed");
    assert_eq!(results.len(), 1, "Non-excluded field should be indexed");
}

#[tokio::test]
async fn test_search_all_combines_words_and_fields() {
    let db = sled::Config::new().temporary(true).open().unwrap();
    let store = std::sync::Arc::new(SledNamespacedStore::new(db));
    let kv_store = store.open_namespace("native_index").await.unwrap();

    let manager = NativeIndexManager::new(kv_store, None);

    let key = KeyValue::new(Some("rec1".to_string()), None);

    // Index "email" as both a keyword and a field name
    manager
        .batch_index_from_keywords("Schema1", &key, vec!["email".to_string()], None)
        .await
        .expect("keyword indexing failed");

    manager
        .batch_index_field_names("Schema1", &key, &["email".to_string()], None)
        .await
        .expect("field indexing failed");

    // search_all should return both, but dedup by (schema, key, field)
    let results = manager.search_all("email").await.expect("search_all failed");
    // word entry has field="llm_keyword", field entry has field="email" — both unique
    assert_eq!(results.len(), 2, "Should find both word and field entries");

    let classifications: std::collections::HashSet<String> =
        results.iter().map(|r| r.classification.clone()).collect();
    assert!(classifications.contains("word"), "Should include word classification");
    assert!(classifications.contains("field"), "Should include field classification");
}

#[tokio::test]
async fn test_matched_term_populated_in_search() {
    let db = sled::Config::new().temporary(true).open().unwrap();
    let store = std::sync::Arc::new(SledNamespacedStore::new(db));
    let kv_store = store.open_namespace("native_index").await.unwrap();

    let manager = NativeIndexManager::new(kv_store, None);

    let key = KeyValue::new(Some("rec1".to_string()), None);
    manager
        .batch_index_from_keywords("Tweet", &key, vec!["hello".to_string()], None)
        .await
        .expect("indexing failed");

    // Search returns entries with matched_term populated
    let entries = manager.search("hello").await.expect("search failed");
    assert_eq!(entries.len(), 1);
    assert_eq!(entries[0].matched_term, Some("hello".to_string()));

    // to_index_result should use matched_term as value when no explicit value given
    let result = entries[0].to_index_result(None);
    assert_eq!(result.value, serde_json::json!("hello"));

    // Explicit value takes precedence
    let result_with_value = entries[0].to_index_result(Some(serde_json::json!("override")));
    assert_eq!(result_with_value.value, serde_json::json!("override"));

    // Field-name entries also get matched_term
    manager
        .batch_index_field_names("Tweet", &key, &["content".to_string()], None)
        .await
        .expect("field indexing failed");

    let field_entries = manager.search_all("content").await.expect("search failed");
    let field_entry = field_entries.iter().find(|e| e.classification == "field").unwrap();
    assert_eq!(field_entry.matched_term, Some("content".to_string()));
}

#[tokio::test]
async fn test_molecule_versions_preserved_through_index_roundtrip() {
    let db = sled::Config::new().temporary(true).open().unwrap();
    let store = std::sync::Arc::new(SledNamespacedStore::new(db));
    let kv_store = store.open_namespace("native_index").await.unwrap();

    let manager = NativeIndexManager::new(kv_store, None);

    let key = KeyValue::new(Some("rec1".to_string()), None);
    let mol_versions = std::collections::HashMap::from([
        ("content".to_string(), 3u64),
        ("title".to_string(), 1u64),
    ]);

    // Index with molecule versions
    manager
        .batch_index_from_keywords("Tweet", &key, vec!["rust".to_string()], Some(&mol_versions))
        .await
        .expect("indexing failed");

    // Search and verify molecule_versions are preserved
    let entries = manager.search("rust").await.expect("search failed");
    assert_eq!(entries.len(), 1);
    let entry = &entries[0];
    assert!(entry.molecule_versions.is_some());
    let versions = entry.molecule_versions.as_ref().unwrap();
    assert_eq!(versions.get("content"), Some(&3u64));
    assert_eq!(versions.get("title"), Some(&1u64));
}

#[tokio::test]
async fn test_molecule_versions_none_by_default() {
    let db = sled::Config::new().temporary(true).open().unwrap();
    let store = std::sync::Arc::new(SledNamespacedStore::new(db));
    let kv_store = store.open_namespace("native_index").await.unwrap();

    let manager = NativeIndexManager::new(kv_store, None);

    let key = KeyValue::new(Some("rec1".to_string()), None);

    // Index without molecule versions
    manager
        .batch_index_from_keywords("Tweet", &key, vec!["hello".to_string()], None)
        .await
        .expect("indexing failed");

    let entries = manager.search("hello").await.expect("search failed");
    assert_eq!(entries.len(), 1);
    assert!(entries[0].molecule_versions.is_none());
}

