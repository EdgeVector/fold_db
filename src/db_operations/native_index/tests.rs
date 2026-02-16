use super::*;
use crate::db_operations::native_index::types::IndexClassification;
use crate::schema::types::key_value::KeyValue;
use crate::storage::{NamespacedStore, SledNamespacedStore};

#[test]
fn test_index_entry_storage_key() {
    let entry = IndexEntry::with_timestamp(
        "Tweet".to_string(),
        KeyValue::new(Some("abc123".to_string()), None),
        "content".to_string(),
        IndexClassification::Word,
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
        IndexClassification::Word,
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
        .batch_index_from_keywords("AiSchema", &key, "content", keywords, None)
        .await
        .expect("batch_index_from_keywords failed");

    // Each keyword should be searchable
    for term in &["machine learning", "neural network", "deep learning"] {
        let results = manager.search(term).await.expect("search failed");
        assert_eq!(results.len(), 1, "Should find 1 result for '{}'", term);
        assert_eq!(results[0].field, "content");
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
            "name",
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
            "name",
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
        assert_eq!(results[0].classification, IndexClassification::Field);
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

    // Index "email" as both a keyword (on "bio" field) and a field name
    manager
        .batch_index_from_keywords("Schema1", &key, "bio", vec!["email".to_string()], None)
        .await
        .expect("keyword indexing failed");

    manager
        .batch_index_field_names("Schema1", &key, &["email".to_string()], None)
        .await
        .expect("field indexing failed");

    // search_all should return both: word entry (field="bio") and field entry (field="email")
    let results = manager.search_all("email").await.expect("search_all failed");
    assert_eq!(results.len(), 2, "Should find both word and field entries");

    let classifications: std::collections::HashSet<IndexClassification> =
        results.iter().map(|r| r.classification).collect();
    assert!(classifications.contains(&IndexClassification::Word), "Should include word classification");
    assert!(classifications.contains(&IndexClassification::Field), "Should include field classification");
}

#[tokio::test]
async fn test_matched_term_populated_in_search() {
    let db = sled::Config::new().temporary(true).open().unwrap();
    let store = std::sync::Arc::new(SledNamespacedStore::new(db));
    let kv_store = store.open_namespace("native_index").await.unwrap();

    let manager = NativeIndexManager::new(kv_store, None);

    let key = KeyValue::new(Some("rec1".to_string()), None);
    manager
        .batch_index_from_keywords("Tweet", &key, "content", vec!["hello".to_string()], None)
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
    let field_entry = field_entries.iter().find(|e| e.classification == IndexClassification::Field).unwrap();
    assert_eq!(field_entry.matched_term, Some("content".to_string()));
}

#[tokio::test]
async fn test_molecule_versions_preserved_through_index_roundtrip() {
    let db = sled::Config::new().temporary(true).open().unwrap();
    let store = std::sync::Arc::new(SledNamespacedStore::new(db));
    let kv_store = store.open_namespace("native_index").await.unwrap();

    let manager = NativeIndexManager::new(kv_store, None);

    let key = KeyValue::new(Some("rec1".to_string()), None);
    let mol_versions: std::collections::HashSet<u64> = [3u64, 1u64].into_iter().collect();

    // Index with molecule versions
    manager
        .batch_index_from_keywords("Tweet", &key, "content", vec!["rust".to_string()], Some(&mol_versions))
        .await
        .expect("indexing failed");

    // Search and verify molecule_versions are preserved
    let entries = manager.search("rust").await.expect("search failed");
    assert_eq!(entries.len(), 1);
    let entry = &entries[0];
    assert!(entry.molecule_versions.is_some());
    let versions = entry.molecule_versions.as_ref().unwrap();
    let expected: std::collections::HashSet<u64> = [3u64, 1u64].into_iter().collect();
    assert_eq!(versions, &expected);
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
        .batch_index_from_keywords("Tweet", &key, "content", vec!["hello".to_string()], None)
        .await
        .expect("indexing failed");

    let entries = manager.search("hello").await.expect("search failed");
    assert_eq!(entries.len(), 1);
    assert!(entries[0].molecule_versions.is_none());
}

#[tokio::test]
async fn test_email_indexed_with_email_classification() {
    let db = sled::Config::new().temporary(true).open().unwrap();
    let store = std::sync::Arc::new(SledNamespacedStore::new(db));
    let kv_store = store.open_namespace("native_index").await.unwrap();

    let manager = NativeIndexManager::new(kv_store, None);

    let key = KeyValue::new(Some("rec1".to_string()), None);
    let keywords = vec![
        "alice@example.com".to_string(),
        "hello".to_string(),
    ];

    manager
        .batch_index_from_keywords("ContactSchema", &key, "email_field", keywords, None)
        .await
        .expect("batch_index_from_keywords failed");

    // Regular word search should find "hello" but NOT "alice@example.com"
    let word_results = manager.search("hello").await.expect("search failed");
    assert_eq!(word_results.len(), 1);
    assert_eq!(word_results[0].classification, IndexClassification::Word);

    // Word search should not find the email (it's stored under email: prefix)
    let email_as_word = manager.search("alice@example.com").await.expect("search failed");
    assert!(email_as_word.is_empty(), "Email should not be found via word search");

    // search_all should find the email
    let all_results = manager.search_all("alice@example.com").await.expect("search_all failed");
    assert_eq!(all_results.len(), 1);
    assert_eq!(all_results[0].classification, IndexClassification::Email);
    assert_eq!(all_results[0].field, "email_field");
}

#[tokio::test]
async fn test_search_all_finds_emails() {
    let db = sled::Config::new().temporary(true).open().unwrap();
    let store = std::sync::Arc::new(SledNamespacedStore::new(db));
    let kv_store = store.open_namespace("native_index").await.unwrap();

    let manager = NativeIndexManager::new(kv_store, None);

    let key = KeyValue::new(Some("rec1".to_string()), None);

    // Index an email, a word, and a field name
    manager
        .batch_index_from_keywords(
            "Schema1", &key, "contact",
            vec!["bob@test.org".to_string(), "engineer".to_string()],
            None,
        )
        .await
        .expect("keyword indexing failed");

    manager
        .batch_index_field_names("Schema1", &key, &["contact".to_string()], None)
        .await
        .expect("field indexing failed");

    // search_all for the email should return the Email-classified entry
    let email_results = manager.search_all("bob@test.org").await.expect("search_all failed");
    assert_eq!(email_results.len(), 1);
    assert_eq!(email_results[0].classification, IndexClassification::Email);

    // search_all for "engineer" should return the Word-classified entry
    let word_results = manager.search_all("engineer").await.expect("search_all failed");
    assert_eq!(word_results.len(), 1);
    assert_eq!(word_results[0].classification, IndexClassification::Word);

    // search_all for "contact" should return both word and field entries
    let contact_results = manager.search_all("contact").await.expect("search_all failed");
    let classifications: std::collections::HashSet<IndexClassification> =
        contact_results.iter().map(|r| r.classification).collect();
    assert!(classifications.contains(&IndexClassification::Field));
}

#[tokio::test]
async fn test_date_indexed_with_date_classification() {
    let db = sled::Config::new().temporary(true).open().unwrap();
    let store = std::sync::Arc::new(SledNamespacedStore::new(db));
    let kv_store = store.open_namespace("native_index").await.unwrap();

    let manager = NativeIndexManager::new(kv_store, None);

    let key = KeyValue::new(Some("rec1".to_string()), None);
    let keywords = vec![
        "2024-01-05".to_string(),
        "meeting".to_string(),
    ];

    manager
        .batch_index_from_keywords("EventSchema", &key, "description", keywords, None)
        .await
        .expect("batch_index_from_keywords failed");

    // Word search should find "meeting" but NOT the date
    let word_results = manager.search("meeting").await.expect("search failed");
    assert_eq!(word_results.len(), 1);
    assert_eq!(word_results[0].classification, IndexClassification::Word);

    let date_as_word = manager.search("2024-01-05").await.expect("search failed");
    assert!(date_as_word.is_empty(), "Date should not be found via word search");

    // search_all should find the date
    let all_results = manager.search_all("2024-01-05").await.expect("search_all failed");
    assert_eq!(all_results.len(), 1);
    assert_eq!(all_results[0].classification, IndexClassification::Date);
    assert_eq!(all_results[0].field, "description");
}

#[tokio::test]
async fn test_search_all_finds_dates_emails_and_words() {
    let db = sled::Config::new().temporary(true).open().unwrap();
    let store = std::sync::Arc::new(SledNamespacedStore::new(db));
    let kv_store = store.open_namespace("native_index").await.unwrap();

    let manager = NativeIndexManager::new(kv_store, None);

    let key = KeyValue::new(Some("rec1".to_string()), None);

    manager
        .batch_index_from_keywords(
            "Schema1", &key, "notes",
            vec![
                "alice@example.com".to_string(),
                "2024-06-15".to_string(),
                "conference".to_string(),
            ],
            None,
        )
        .await
        .expect("keyword indexing failed");

    // Each type should be searchable via search_all with correct classification
    let email_results = manager.search_all("alice@example.com").await.expect("search_all failed");
    assert_eq!(email_results.len(), 1);
    assert_eq!(email_results[0].classification, IndexClassification::Email);

    let date_results = manager.search_all("2024-06-15").await.expect("search_all failed");
    assert_eq!(date_results.len(), 1);
    assert_eq!(date_results[0].classification, IndexClassification::Date);

    let word_results = manager.search_all("conference").await.expect("search_all failed");
    assert_eq!(word_results.len(), 1);
    assert_eq!(word_results[0].classification, IndexClassification::Word);
}

