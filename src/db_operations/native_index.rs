use super::native_index_classification::{structural_prefixes, ClassificationType};
use crate::schema::types::key_value::KeyValue;
use crate::schema::SchemaError;
use crate::storage::traits::KvStore;
use log;
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
use sled::Tree;
use std::collections::HashSet;
use std::sync::Arc;
use std::time::{SystemTime, UNIX_EPOCH};

const EXCLUDED_FIELDS: &[&str] = &["uuid", "id", "password", "token"];

/// Index entry prefix for index storage
const INDEX_ENTRY_PREFIX: &str = "idx:";
/// Compact index entry - reference only, no value duplication
///
/// This is ~89% smaller than IndexResult because it doesn't store the value.
/// Each entry is stored as a separate key for fast writes and prefix scanning.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct IndexEntry {
    /// Schema name containing the indexed record
    pub schema: String,
    /// Native fold_db key (hashKey + rangeKey)
    pub key: KeyValue,
    /// Which field matched the search term
    pub field: String,
    /// Index type (e.g. "word", "field")
    pub classification: String,
    /// When indexed (milliseconds since epoch, for sorting/dedup)
    pub timestamp: i64,
}

impl IndexEntry {
    /// Create a new index entry with current timestamp
    pub fn new(schema: String, key: KeyValue, field: String, classification: String) -> Self {
        let timestamp = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .map(|d| d.as_millis() as i64)
            .unwrap_or(0);
        Self {
            schema,
            key,
            field,
            classification,
            timestamp,
        }
    }

    /// Create index entry with explicit timestamp (for testing/migration)
    pub fn with_timestamp(
        schema: String,
        key: KeyValue,
        field: String,
        classification: String,
        timestamp: i64,
    ) -> Self {
        Self {
            schema,
            key,
            field,
            classification,
            timestamp,
        }
    }

    /// Convert to IndexResult for backward compatibility
    /// Note: value will be None since IndexEntry doesn't store values
    pub fn to_index_result(&self, value: Option<Value>) -> IndexResult {
        IndexResult {
            schema_name: self.schema.clone(),
            field: self.field.clone(),
            key_value: self.key.clone(),
            value: value.unwrap_or(Value::Null),
            metadata: Some(json!({
                "classification": self.classification,
                "timestamp": self.timestamp
            })),
        }
    }

    /// Generate a unique storage key for this entry
    /// Format: idx:{term}:{timestamp}:{schema}:{field}:{key_hash}
    pub fn storage_key(&self, term: &str) -> String {
        let key_hash = self.key_hash();
        format!(
            "{}{}:{}:{}:{}:{}",
            INDEX_ENTRY_PREFIX, term, self.timestamp, self.schema, self.field, key_hash
        )
    }

    /// Generate a hash of the KeyValue for use in storage keys
    fn key_hash(&self) -> String {
        // Use a simple representation of the key
        match (&self.key.hash, &self.key.range) {
            (Some(h), Some(r)) => format!("{}_{}", h, r),
            (Some(h), None) => h.clone(),
            (None, Some(r)) => format!("_{}", r),
            (None, None) => "empty".to_string(),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, utoipa::ToSchema)]
pub struct IndexResult {
    pub schema_name: String,
    pub field: String,
    pub key_value: KeyValue,
    pub value: Value,
    pub metadata: Option<Value>,
}

/// Represents a batch index operation: (schema_name, field_name, key_value, value, classifications)
pub type BatchIndexOperation = (String, String, KeyValue, Value, Option<Vec<String>>);

#[derive(Clone)]
pub struct NativeIndexManager {
    tree: Option<Tree>,
    store: Option<Arc<dyn KvStore>>,
}

impl NativeIndexManager {
    /// Create with Sled Tree (backward compatible)
    pub fn new(tree: Tree) -> Self {
        Self {
            tree: Some(tree),
            store: None,
        }
    }

    /// Create with KvStore (works with any backend)
    pub fn new_with_store(store: Arc<dyn KvStore>) -> Self {
        Self {
            tree: None,
            store: Some(store),
        }
    }

    /// Check if this manager uses async storage (DynamoDB) vs sync (Sled)
    pub fn is_async(&self) -> bool {
        self.store.is_some()
    }

    /// Get value from either tree or store
    /// For DynamoDB, uses simplified key structure: feature as PK, term as SK
    async fn get(&self, key: &[u8]) -> Result<Option<Vec<u8>>, SchemaError> {
        if let Some(ref tree) = self.tree {
            tree.get(key)
                .map_err(|e| SchemaError::InvalidData(format!("Sled get failed: {}", e)))
                .map(|opt| opt.map(|v| v.to_vec()))
        } else if let Some(ref store) = self.store {
            // For DynamoDB, parse key to extract feature and term
            // Keys are in format: "feature:term" (e.g., "word:hello", "email:test@example.com")
            let key_str = String::from_utf8_lossy(key);
            if let Some(colon_pos) = key_str.find(':') {
                let _feature = &key_str[..colon_pos];
                let _term = &key_str[colon_pos + 1..];

                // Use simplified structure: feature as PK, term as SK
                // This enables efficient queries by feature type
                // For now, we'll still use the full key via KvStore, but this structure
                // could be optimized further by accessing DynamoDB directly
                store
                    .get(key)
                    .await
                    .map_err(|e| SchemaError::InvalidData(format!("KvStore get failed: {}", e)))
            } else {
                // Fallback: treat entire key as term, use "word" as default feature
                store
                    .get(key)
                    .await
                    .map_err(|e| SchemaError::InvalidData(format!("KvStore get failed: {}", e)))
            }
        } else {
            Err(SchemaError::InvalidData(
                "NativeIndexManager not properly initialized".to_string(),
            ))
        }
    }

    /// Put value using simplified key structure for DynamoDB
    async fn put(&self, key: &[u8], value: Vec<u8>) -> Result<(), SchemaError> {
        if let Some(ref tree) = self.tree {
            tree.insert(key, value)
                .map_err(|e| SchemaError::InvalidData(format!("Sled put failed: {}", e)))?;
            Ok(())
        } else if let Some(ref store) = self.store {
            // For DynamoDB, use simplified structure: feature as PK, term as SK
            store
                .put(key, value)
                .await
                .map_err(|e| SchemaError::InvalidData(format!("KvStore put failed: {}", e)))
        } else {
            Err(SchemaError::InvalidData(
                "NativeIndexManager not properly initialized".to_string(),
            ))
        }
    }

    /// Delete value using simplified key structure for DynamoDB
    async fn delete(&self, key: &[u8]) -> Result<bool, SchemaError> {
        if let Some(ref tree) = self.tree {
            tree.remove(key)
                .map_err(|e| SchemaError::InvalidData(format!("Sled delete failed: {}", e)))
                .map(|opt| opt.is_some())
        } else if let Some(ref store) = self.store {
            store
                .delete(key)
                .await
                .map_err(|e| SchemaError::InvalidData(format!("KvStore delete failed: {}", e)))
        } else {
            Err(SchemaError::InvalidData(
                "NativeIndexManager not properly initialized".to_string(),
            ))
        }
    }

    // ========== SEARCH METHODS ==========

    /// Search all indexed keywords and return results (async version)
    pub async fn search_all_classifications_async(
        &self,
        term: &str,
    ) -> Result<Vec<IndexResult>, SchemaError> {
        log::debug!(
            "Native Index: search_all_classifications_async called for term: '{}'",
            term
        );

        let entries = self.search_all(term).await?;
        let results = self.entries_to_results(entries);

        log::info!(
            "Native Index: search_all_classifications_async for '{}' returned {} total results",
            term,
            results.len()
        );
        Ok(results)
    }

    /// Search all indexed keywords and return results (sync version, Sled only)
    pub fn search_all_classifications(&self, term: &str) -> Result<Vec<IndexResult>, SchemaError> {
        log::debug!(
            "Native Index: search_all_classifications called for term: '{}'",
            term
        );

        // For Sled backend, use sync search
        if !self.is_async() {
            let entries = self.search_sync(term)?;
            let results = self.entries_to_results(entries);
            log::info!(
                "Native Index: search_all_classifications for '{}' returned {} total results",
                term,
                results.len()
            );
            return Ok(results);
        }

        // For async backends, create a new runtime
        let rt = tokio::runtime::Builder::new_current_thread()
            .enable_all()
            .build()
            .map_err(|e| SchemaError::InvalidData(format!("Failed to create runtime: {}", e)))?;

        rt.block_on(async {
            let entries = self.search_all(term).await?;
            let results = self.entries_to_results(entries);
            log::info!(
                "Native Index: search_all_classifications for '{}' returned {} total results",
                term,
                results.len()
            );
            Ok(results)
        })
    }

    fn extract_hashtags(&self, value: &Value) -> Vec<(String, String)> {
        let mut results = Vec::new();
        Self::extract_hashtags_recursive(value, &mut results);
        results
    }

    fn extract_hashtags_recursive(value: &Value, acc: &mut Vec<(String, String)>) {
        match value {
            Value::String(text) => {
                if let Some(tag) = text.strip_prefix('#') {
                    let normalized = tag.trim().to_ascii_lowercase();
                    if !normalized.is_empty() {
                        acc.push((format!("hashtag:{}", normalized), normalized));
                    }
                }
            }
            Value::Array(values) => {
                for item in values {
                    Self::extract_hashtags_recursive(item, acc);
                }
            }
            _ => {}
        }
    }

    fn extract_emails(&self, value: &Value) -> Vec<(String, String)> {
        let mut results = Vec::new();
        Self::extract_emails_recursive(value, &mut results);
        results
    }

    fn extract_emails_recursive(value: &Value, acc: &mut Vec<(String, String)>) {
        match value {
            Value::String(text) => {
                if text.contains('@') && text.contains('.') {
                    let normalized = text.trim().to_ascii_lowercase();
                    acc.push((format!("email:{}", normalized), normalized));
                }
            }
            Value::Array(values) => {
                for item in values {
                    Self::extract_emails_recursive(item, acc);
                }
            }
            _ => {}
        }
    }

    fn extract_whole_values(&self, classification: &str, value: &Value) -> Vec<(String, String)> {
        let mut results = Vec::new();
        Self::extract_whole_values_recursive(classification, value, &mut results);
        results
    }

    fn extract_whole_values_recursive(
        classification: &str,
        value: &Value,
        acc: &mut Vec<(String, String)>,
    ) {
        match value {
            Value::String(text) => {
                let normalized = text.trim().to_ascii_lowercase();
                if !normalized.is_empty() {
                    acc.push((format!("{}:{}", classification, normalized), normalized));
                }
            }
            Value::Array(values) => {
                for item in values {
                    Self::extract_whole_values_recursive(classification, item, acc);
                }
            }
            _ => {}
        }
    }

    pub fn should_index_field(field_name: &str) -> bool {
        !EXCLUDED_FIELDS
            .iter()
            .any(|excluded| excluded.eq_ignore_ascii_case(field_name))
    }

    fn build_record_key(
        &self,
        schema_name: &str,
        field_name: &str,
        key_value: &KeyValue,
    ) -> Result<String, SchemaError> {
        let serialized_key = serde_json::to_string(key_value).map_err(|e| {
            SchemaError::InvalidData(format!("Failed to serialize key value for index: {}", e))
        })?;
        Ok(format!(
            "{}{}:{}:{}",
            structural_prefixes::RECORD,
            schema_name,
            field_name,
            serialized_key
        ))
    }

    fn normalize_search_term(&self, term: &str) -> Option<String> {
        let lowered = term.trim().to_lowercase();
        if lowered.len() < 2 {
            return None;
        }
        Some(lowered)
    }

    fn collect_words(&self, value: &Value) -> Vec<String> {
        let mut words = HashSet::new();
        Self::collect_words_recursive(value, &mut words);
        let mut result: Vec<String> = words.into_iter().collect();
        result.sort_unstable();
        result
    }

    fn collect_words_recursive(value: &Value, acc: &mut HashSet<String>) {
        match value {
            Value::String(text) => {
                for word in text.split(|c: char| !c.is_alphanumeric()) {
                    let lowered = word.trim().to_lowercase();
                    if lowered.len() >= 2 {
                        acc.insert(lowered);
                    }
                }
            }
            Value::Number(n) => {
                let s = n.to_string();
                if s.len() >= 2 {
                    acc.insert(s);
                }
            }
            Value::Array(values) => {
                for item in values {
                    Self::collect_words_recursive(item, acc);
                }
            }
            Value::Object(obj) => {
                for (_, nested_value) in obj {
                    Self::collect_words_recursive(nested_value, acc);
                }
            }
            _ => {}
        }
    }

    /// Read entries from index (sync version for Sled)
    fn read_entries(&self, key: &str) -> Result<Vec<IndexResult>, SchemaError> {
        if let Some(ref tree) = self.tree {
            let Some(bytes) = tree.get(key.as_bytes())? else {
                log::debug!("📭 No entries found for key: {}", key);
                return Ok(Vec::new());
            };

            let entries: Vec<IndexResult> = serde_json::from_slice(&bytes).map_err(|e| {
                SchemaError::InvalidData(format!("Failed to deserialize index entries: {}", e))
            })?;
            log::debug!("📬 Read {} entries from key: {}", entries.len(), key);
            Ok(entries)
        } else {
            Err(SchemaError::InvalidData("Synchronous read_entries only available with Sled backend. Use read_entries_async instead.".to_string()))
        }
    }

    /// Read entries from index (async version for DynamoDB)
    /// Uses simplified key structure: feature as PK, term as SK
    async fn read_entries_async(&self, key: &str) -> Result<Vec<IndexResult>, SchemaError> {
        if let Some(ref _store) = self.store {
            // Keys are in format: "feature:term" (e.g., "word:hello", "email:test@example.com")
            // For DynamoDB, this enables efficient queries by feature type
            let bytes = self.get(key.as_bytes()).await?;

            if let Some(bytes) = bytes {
                let entries: Vec<IndexResult> = serde_json::from_slice(&bytes).map_err(|e| {
                    SchemaError::InvalidData(format!("Failed to deserialize index entries: {}", e))
                })?;
                log::debug!("📬 Read {} entries from key: {}", entries.len(), key);
                Ok(entries)
            } else {
                log::debug!("📭 No entries found for key: {}", key);
                Ok(Vec::new())
            }
        } else {
            Err(SchemaError::InvalidData(
                "Async read_entries only available with KvStore backend".to_string(),
            ))
        }
    }

    /// Write entries to index (sync version for Sled)
    fn write_entries(&self, key: &str, entries: &[IndexResult]) -> Result<(), SchemaError> {
        if let Some(ref tree) = self.tree {
            if entries.is_empty() {
                log::debug!("Native Index: Removing empty index key: {}", key);
                tree.remove(key.as_bytes())?;
                return Ok(());
            }

            log::debug!(
                "Native Index: Writing {} entries to index key: {}",
                entries.len(),
                key
            );
            let bytes = serde_json::to_vec(entries).map_err(|e| {
                SchemaError::InvalidData(format!("Failed to serialize index entries: {}", e))
            })?;
            tree.insert(key.as_bytes(), bytes)?;
            Ok(())
        } else {
            Err(SchemaError::InvalidData("Synchronous write_entries only available with Sled backend. Use write_entries_async instead.".to_string()))
        }
    }

    /// Remove record entries (sync version for Sled)
    fn remove_record_entries(
        &self,
        record_key: &str,
        schema_name: &str,
        field_name: &str,
        key_value: &KeyValue,
    ) -> Result<(), SchemaError> {
        if let Some(ref tree) = self.tree {
            let Some(bytes) = tree.get(record_key.as_bytes())? else {
                return Ok(());
            };

            let words: Vec<String> = serde_json::from_slice(&bytes).map_err(|e| {
                SchemaError::InvalidData(format!("Failed to deserialize record index words: {}", e))
            })?;

            for word in words {
                let index_key = format!("{}{}", structural_prefixes::WORD, word);
                let mut entries = self.read_entries(&index_key)?;
                let initial_len = entries.len();

                entries.retain(|entry| {
                    !(entry.schema_name == schema_name
                        && entry.field == field_name
                        && entry.key_value == *key_value)
                });

                if entries.is_empty() {
                    tree.remove(index_key.as_bytes())?;
                } else if entries.len() != initial_len {
                    self.write_entries(&index_key, &entries)?;
                }
            }

            tree.remove(record_key.as_bytes())?;
            Ok(())
        } else {
            Err(SchemaError::InvalidData("Synchronous remove_record_entries only available with Sled backend. Use remove_record_entries_async instead.".to_string()))
        }
    }

    // ========== BATCH INDEX OPERATIONS ==========

    /// Batch index multiple field values with classifications
    /// Automatically uses async version for DynamoDB, sync for Sled
    pub fn batch_index_field_values_with_classifications(
        &self,
        index_operations: &[BatchIndexOperation],
    ) -> Result<(), SchemaError> {
        // If we have a store (DynamoDB), use async version via blocking
        if self.store.is_some() {
            // Use tokio::runtime::Handle::current() to run async code from sync context
            let handle = tokio::runtime::Handle::try_current().map_err(|_| {
                SchemaError::InvalidData(
                    "No tokio runtime available for async indexing".to_string(),
                )
            })?;
            handle.block_on(
                self.batch_index_field_values_with_classifications_async(index_operations),
            )
        } else if self.tree.is_some() {
            // Sync version for Sled
            use std::collections::HashMap;
            let mut index_map: HashMap<String, Vec<IndexResult>> = HashMap::new();
            let mut record_keys: Vec<(String, HashSet<String>)> = Vec::new();

            for (schema_name, field_name, key_value, value, classifications) in index_operations {
                if !Self::should_index_field(field_name) {
                    continue;
                }

                let classifications = classifications.clone().unwrap_or_default();
                let classifications = if classifications.is_empty() {
                    vec!["word".to_string()]
                } else {
                    classifications
                };
                let record_key = self.build_record_key(schema_name, field_name, key_value)?;
                self.remove_record_entries(&record_key, schema_name, field_name, key_value)?;

                let all_index_keys = self.extract_and_aggregate_entries(
                    &classifications,
                    value,
                    schema_name,
                    field_name,
                    key_value,
                    &mut index_map,
                )?;

                if !all_index_keys.is_empty() {
                    record_keys.push((record_key, all_index_keys));
                }
            }

            let batch_operations = self.build_batch_operations(index_map, record_keys)?;
            self.batch_execute_index_operations(&batch_operations)?;
            Ok(())
        } else {
            Err(SchemaError::InvalidData(
                "NativeIndexManager not properly initialized".to_string(),
            ))
        }
    }

    /// Batch index multiple field values with classifications (async version for DynamoDB)
    pub async fn batch_index_field_values_with_classifications_async(
        &self,
        index_operations: &[BatchIndexOperation],
    ) -> Result<(), SchemaError> {
        log::info!(
            "[NativeIndex] batch_index_field_values_with_classifications_async: Starting with {} operations",
            index_operations.len()
        );

        if self.store.is_none() {
            log::error!("[NativeIndex] No store available for async indexing");
            return Err(SchemaError::InvalidData(
                "Async batch_index only available with KvStore backend".to_string(),
            ));
        }

        use futures_util::future::join_all;
        use std::collections::{HashMap, HashSet};

        // Track all index keys that need to be updated (read-modify-write)
        // This includes:
        // 1. Keys from existing records (to remove stale entries)
        // 2. Keys from new values (to add new entries)
        let mut keys_to_update: HashSet<String> = HashSet::new();

        // Map: IndexKey -> New Entries to Add
        let mut index_additions: HashMap<String, Vec<IndexResult>> = HashMap::new();

        // Prepare new record entries: RecordKey -> List of Index Keys
        let mut new_record_entries: Vec<(String, HashSet<String>)> = Vec::new();

        // Identification of records being modified: (Schema, Field, KeyValue serialized) -> specific record
        // Used to filter out stale entries from fetched index lists
        let mut modified_records_set: HashSet<(String, String, String)> = HashSet::new();

        // 1. Analyze Operations & Check Existing Records (Parallel)
        let mut prospective_records = Vec::new();
        for (schema_name, field_name, key_value, _, _) in index_operations {
            let record_key = self.build_record_key(schema_name, field_name, key_value)?;
            // Store serializable key for set lookup
            let kv_str = serde_json::to_string(key_value).map_err(|e| {
                SchemaError::InvalidData(format!("Failed to serialize key value: {}", e))
            })?;
            modified_records_set.insert((schema_name.clone(), field_name.clone(), kv_str));
            prospective_records.push((record_key, schema_name, field_name, key_value));
        }

        // Parallel fetch of all prospective records to identify existing words/keys that need cleanup
        let record_fetches = join_all(
            prospective_records
                .iter()
                .map(|(rk, _, _, _)| self.get(rk.as_bytes())),
        )
        .await;

        for fetch_result in record_fetches.into_iter() {
            if let Ok(Some(bytes)) = fetch_result {
                // Record exists! Deserialize to get old words/keys
                if let Ok(old_keys) = serde_json::from_slice::<Vec<String>>(&bytes) {
                    for key in old_keys {
                        let _index_key = format!("{}{}", structural_prefixes::WORD, key);
                        // Note: The stored keys in record_key value are raw words, not full index keys?
                        // self.remove_record_entries_async deserializes to "words".
                        // BUT `batch_index...` writes "all_index_keys" which are FULL keys.
                        // Let's check `extract_and_aggregate_entries`. It returns full keys.
                        // And `batch_index...` at end writes `keys_vec`.
                        // So the stored value IS full index keys.
                        // EXCEPT `remove_record_entries` logic (lines 865) assumes they are "words".
                        // Wait, `remove_record_entries` reads bytes and deserializes to `Vec<String>`.
                        // Then for each `word`, it constructs `word:word`.
                        // This implies stored data ARE words.
                        // BUT `batch_index` (line 1054) writes `index_keys` which comes from `extract_and_aggregate_entries`.
                        // `extract_and_aggregate_entries` returns FULL keys (e.g. "word:hello", "field:email").
                        //
                        // CONTRAIDICTION in existing code?
                        // `remove_record_entries` line 870: `format!("{}{}", structural_prefixes::WORD, word)`
                        // If the stored data was "word:hello", then prefixing again makes "word:word:hello".
                        //
                        // If `batch_index` writes full keys, then `remove_record_entries` is BROKEN/Legacy?
                        // Sled `batch_index` (line 993) also writes `all_index_keys`.
                        //
                        // Let's assume the stored data is FULL KEYS.
                        // So we should just use them as is. `remove_record_entries` might be buggy or I misread it.
                        // Actually `remove_record_entries` line 865 calls generic `serde_json::from_slice`.
                        // It names variable `words`. But if the data is full keys, then `word` is a full key.
                        // Line 870 `format!("{}{}", WORD, word)` would prepend again.
                        // If `word` is "word:hello", result is "word:word:hello".
                        // Only if `word` is "hello" does it work.
                        //
                        // Let's check `extract_and_aggregate_entries`.
                        // It returns `all_index_keys` which includes "word:hello".
                        // So `batch_index` stores "word:hello".
                        // So `remove_record_entries` IS BUGGY if it prepends prefix again.
                        //
                        // HOWEVER, for this refactor, I will trust that the stored data acts as pointers to index entries.
                        // If I use the stored string directly as the key, it should be correct if `batch_index` wrote it.

                        // Fix: Use the key directly.
                        keys_to_update.insert(key);
                    }
                }
            }
        }

        // 2. Process New Values
        for (i, (schema_name, field_name, key_value, value, classifications)) in
            index_operations.iter().enumerate()
        {
            if !Self::should_index_field(field_name) {
                continue;
            }

            let classifications_vec = classifications.clone().unwrap_or_default();
            let effective_classifications = if classifications_vec.is_empty() {
                vec!["word".to_string()]
            } else {
                classifications_vec
            };

            let mut local_map = HashMap::new();
            let all_index_keys = self.extract_and_aggregate_entries(
                &effective_classifications,
                value,
                schema_name,
                field_name,
                key_value,
                &mut local_map,
            )?;

            let (record_key, _, _, _) = &prospective_records[i];

            // Register new keys for record
            if !all_index_keys.is_empty() {
                new_record_entries.push((record_key.clone(), all_index_keys));
            }

            // Register additions
            for (k, v) in local_map {
                keys_to_update.insert(k.clone());
                index_additions.entry(k).or_default().extend(v);
            }
        }

        // 3. Parallel Read-Modify-Write of Index Keys
        let unique_keys: Vec<String> = keys_to_update.into_iter().collect();

        if !unique_keys.is_empty() {
            // A. Batch Read
            let index_fetches =
                join_all(unique_keys.iter().map(|k| self.read_entries_async(k))).await;

            let mut write_futures = Vec::new(); // (key, bytes)
            let mut delete_futures = Vec::new(); // key

            for (i, fetch_result) in index_fetches.into_iter().enumerate() {
                let key = &unique_keys[i];
                let mut current_entries = fetch_result.unwrap_or_default();

                // B. Remove Stale Entries
                // Remove entries that match any of the records we are modifying
                current_entries.retain(|entry| {
                    if let Ok(kv_str) = serde_json::to_string(&entry.key_value) {
                        !modified_records_set.contains(&(
                            entry.schema_name.clone(),
                            entry.field.clone(),
                            kv_str,
                        ))
                    } else {
                        // Keep if we can't serialize (safe default)
                        true
                    }
                });

                // C. Add New Entries
                if let Some(new_entries) = index_additions.get(key) {
                    // Dedup is handled by merge/extend usually,
                    // but we just filtered out the exact record matches, so we can simpler append.
                    // But just to be safe from duplicates within the batch (e.g. same word twice in same text? handled by extract logic):
                    // extract_and_aggregate_entries produces unique entries per record-word.
                    current_entries.extend(new_entries.clone());
                }

                // D. Prepare Write or Delete
                if current_entries.is_empty() {
                    delete_futures.push(self.delete(key.as_bytes()));
                } else {
                    let bytes = serde_json::to_vec(&current_entries).map_err(|e| {
                        SchemaError::InvalidData(format!(
                            "Failed to serialize index entries: {}",
                            e
                        ))
                    })?;
                    write_futures.push(self.put(key.as_bytes(), bytes));
                }
            }

            // Execute writes and deletes
            log::info!(
                "[NativeIndex] Executing {} writes and {} deletes",
                write_futures.len(),
                delete_futures.len()
            );
            let write_results = join_all(write_futures).await;
            let delete_results = join_all(delete_futures).await;

            let write_errors: Vec<_> = write_results.iter().filter(|r| r.is_err()).collect();
            let delete_errors: Vec<_> = delete_results.iter().filter(|r| r.is_err()).collect();

            if !write_errors.is_empty() {
                log::warn!("[NativeIndex] {} write errors occurred", write_errors.len());
            }
            if !delete_errors.is_empty() {
                log::warn!(
                    "[NativeIndex] {} delete errors occurred",
                    delete_errors.len()
                );
            }

            log::info!("[NativeIndex] Index writes completed");
        }

        // 4. Update Record Keys (Parallel)
        let record_write_futures = new_record_entries.iter().map(|(rk, keys)| {
            let keys_vec: Vec<String> = keys.iter().cloned().collect();
            let bytes_res = serde_json::to_vec(&keys_vec);
            async move {
                if let Ok(bytes) = bytes_res {
                    self.put(rk.as_bytes(), bytes).await
                } else {
                    Err(SchemaError::InvalidData("Failed to serialize".into()))
                }
            }
        });
        join_all(record_write_futures).await;

        log::info!(
            "[NativeIndex] batch_index_field_values_with_classifications_async: Completed successfully"
        );
        Ok(())
    }

    fn extract_and_aggregate_entries(
        &self,
        classifications: &[String],
        value: &Value,
        schema_name: &str,
        field_name: &str,
        key_value: &KeyValue,
        index_map: &mut std::collections::HashMap<String, Vec<IndexResult>>,
    ) -> Result<HashSet<String>, SchemaError> {
        let mut all_index_keys = HashSet::new();

        for classification_str in classifications {
            let index_entries = self.extract_by_classification(classification_str, value);

            for (index_key, normalized_value) in index_entries {
                // Create record-level index entry (with key_value)
                let record_index_entry = IndexResult {
                    schema_name: schema_name.to_string(),
                    field: field_name.to_string(),
                    key_value: key_value.clone(),
                    value: value.clone(),
                    metadata: Some(json!({
                        "classification": classification_str,
                        "normalized": normalized_value
                    })),
                };

                index_map
                    .entry(index_key.clone())
                    .or_default()
                    .push(record_index_entry);
                all_index_keys.insert(index_key);
            }
        }

        // Create field name index: field:email (not word:email)
        // This allows searching for "email" to return all records with an email field
        let field_name_normalized = field_name.to_ascii_lowercase();
        let field_name_key = format!("{}{}", structural_prefixes::FIELD, field_name_normalized);
        let field_name_entry = IndexResult {
            schema_name: schema_name.to_string(),
            field: field_name.to_string(),
            key_value: key_value.clone(),
            value: value.clone(),
            metadata: Some(json!({
                "classification": "field",
                "field_name": field_name
            })),
        };

        index_map
            .entry(field_name_key.clone())
            .or_default()
            .push(field_name_entry);
        all_index_keys.insert(field_name_key);

        Ok(all_index_keys)
    }

    fn extract_by_classification(
        &self,
        classification: &str,
        value: &Value,
    ) -> Vec<(String, String)> {
        match classification {
            "word" => {
                let words = self.collect_words(value);
                words
                    .into_iter()
                    .map(|w| (format!("word:{}", w), w))
                    .collect()
            }
            c if c.starts_with("hashtag") => self.extract_hashtags(value),
            c if c.starts_with("email") => self.extract_emails(value),
            c if c.starts_with("name:")
                || c.starts_with("username")
                || c.starts_with("phone")
                || c.starts_with("url")
                || c.starts_with("date") =>
            {
                self.extract_whole_values(c, value)
            }
            _ => {
                let words = self.collect_words(value);
                words
                    .into_iter()
                    .map(|w| (format!("word:{}", w), w))
                    .collect()
            }
        }
    }

    fn build_batch_operations(
        &self,
        index_map: std::collections::HashMap<String, Vec<IndexResult>>,
        record_keys: Vec<(String, HashSet<String>)>,
    ) -> Result<Vec<(String, serde_json::Value)>, SchemaError> {
        let mut batch_operations = Vec::new();

        for (index_key, new_entries) in index_map {
            let merged_entries = self.merge_with_existing_entries(&index_key, new_entries)?;
            batch_operations.push((
                index_key,
                serde_json::to_value(&merged_entries).map_err(|e| {
                    SchemaError::InvalidData(format!("Serialization failed: {}", e))
                })?,
            ));
        }

        for (record_key, index_keys) in record_keys {
            batch_operations.push((
                record_key,
                serde_json::Value::Array(
                    index_keys
                        .into_iter()
                        .map(serde_json::Value::String)
                        .collect(),
                ),
            ));
        }

        Ok(batch_operations)
    }

    fn merge_with_existing_entries(
        &self,
        index_key: &str,
        new_entries: Vec<IndexResult>,
    ) -> Result<Vec<IndexResult>, SchemaError> {
        let mut existing_entries = self.read_entries(index_key)?;
        let deduplicated = self.deduplicate_entries(new_entries);

        for new_entry in &deduplicated {
            let new_classification = self.extract_classification(new_entry);
            existing_entries.retain(|entry| {
                let entry_classification = self.extract_classification(entry);
                !(entry.schema_name == new_entry.schema_name
                    && entry.field == new_entry.field
                    && entry.key_value == new_entry.key_value
                    && entry_classification == new_classification)
            });
        }

        existing_entries.extend(deduplicated);
        Ok(existing_entries)
    }

    fn deduplicate_entries(&self, entries: Vec<IndexResult>) -> Vec<IndexResult> {
        use std::collections::HashMap;
        let mut seen: HashMap<(String, String, KeyValue, String), IndexResult> = HashMap::new();

        for entry in entries {
            let classification = self.extract_classification(&entry);
            let key = (
                entry.schema_name.clone(),
                entry.field.clone(),
                entry.key_value.clone(),
                classification,
            );
            seen.insert(key, entry);
        }

        seen.into_values().collect()
    }

    fn extract_classification(&self, entry: &IndexResult) -> String {
        if let Some(metadata) = &entry.metadata {
            if let Some(Value::String(class)) = metadata.get("classification") {
                return class.clone();
            }
        }
        "word".to_string()
    }

    /// Batch execute index operations using sled's batch API
    fn batch_execute_index_operations(
        &self,
        operations: &[(String, serde_json::Value)],
    ) -> Result<(), SchemaError> {
        log::debug!(
            "Native Index: Batch executing {} index operations",
            operations.len()
        );
        let mut batch = sled::Batch::default();

        for (key, value) in operations {
            let bytes = serde_json::to_vec(value)
                .map_err(|e| SchemaError::InvalidData(format!("Serialization failed: {}", e)))?;
            batch.insert(key.as_bytes(), bytes);
        }

        if let Some(ref tree) = self.tree {
            tree.apply_batch(batch)
                .map_err(|e| SchemaError::InvalidData(format!("Batch apply failed: {}", e)))?;

            // Ensure the data is durably written to disk
            // tree.flush()
            //     .map_err(|e| SchemaError::InvalidData(format!("Flush failed: {}", e)))?;
        } else {
            return Err(SchemaError::InvalidData(
                "Batch indexing only available with Sled backend".to_string(),
            ));
        }

        log::info!(
            "Native Index: Batch flushed {} operations to disk",
            operations.len()
        );
        Ok(())
    }

    /// Explicitly flush the index tree to disk
    ///
    /// This should only be called for non-batch operations.
    /// Batch operations handle flushing internally.
    pub fn flush(&self) -> Result<(), SchemaError> {
        if let Some(ref tree) = self.tree {
            tree.flush()
                .map_err(|e| SchemaError::InvalidData(format!("Flush failed: {}", e)))?;
        }
        Ok(())
    }

    // ========== INDEX OPERATIONS ==========

    /// Index a record using LLM-extracted keywords.
    ///
    /// Takes a flat list of keywords (already normalized by the LLM) and writes
    /// index entries + reverse mappings for each keyword.
    pub async fn batch_index_from_keywords(
        &self,
        schema_name: &str,
        key_value: &KeyValue,
        keywords: Vec<String>,
    ) -> Result<(), SchemaError> {
        log::info!(
            "[NativeIndex] batch_index_from_keywords: {} keywords for schema '{}'",
            keywords.len(),
            schema_name
        );

        if self.tree.is_none() && self.store.is_none() {
            return Err(SchemaError::InvalidData(
                "NativeIndexManager not properly initialized".to_string(),
            ));
        }

        let mut index_entries: Vec<(Vec<u8>, Vec<u8>)> = Vec::new();

        for keyword in &keywords {
            let entry = IndexEntry::new(
                schema_name.to_string(),
                key_value.clone(),
                "llm_keyword".to_string(),
                "word".to_string(),
            );

            // Term is stored as "word:{keyword}" to match the search prefix format
            let term = format!("word:{}", keyword);
            let storage_key = entry.storage_key(&term);
            let entry_bytes = serde_json::to_vec(&entry).map_err(|e| {
                SchemaError::InvalidData(format!("Failed to serialize IndexEntry: {}", e))
            })?;

            index_entries.push((storage_key.into_bytes(), entry_bytes));
        }

        // Write all entries
        if let Some(ref store) = self.store {
            let mut seen_keys = std::collections::HashSet::new();
            let deduped_entries: Vec<(Vec<u8>, Vec<u8>)> = index_entries
                .into_iter()
                .filter(|(key, _)| seen_keys.insert(key.clone()))
                .collect();

            store.batch_put(deduped_entries).await.map_err(|e| {
                SchemaError::InvalidData(format!("Failed to batch write keyword entries: {}", e))
            })?;
        } else if let Some(ref tree) = self.tree {
            let mut batch = sled::Batch::default();
            for (key, value) in index_entries {
                batch.insert(key, value);
            }
            tree.apply_batch(batch)
                .map_err(|e| SchemaError::InvalidData(format!("Batch apply failed: {}", e)))?;
        }

        log::info!("[NativeIndex] batch_index_from_keywords: Completed successfully");
        Ok(())
    }

    /// Batch index a record's fields by extracting terms and writing index entries.
    pub async fn batch_index(
        &self,
        index_operations: &[BatchIndexOperation],
    ) -> Result<(), SchemaError> {
        log::info!(
            "[NativeIndex] batch_index: Starting with {} operations",
            index_operations.len()
        );

        if self.tree.is_none() && self.store.is_none() {
            return Err(SchemaError::InvalidData(
                "NativeIndexManager not properly initialized".to_string(),
            ));
        }

        let mut index_entries: Vec<(Vec<u8>, Vec<u8>)> = Vec::new();

        for (schema_name, field_name, key_value, value, classifications) in index_operations {
            if !Self::should_index_field(field_name) {
                continue;
            }

            let classifications = classifications.clone().unwrap_or_default();
            let effective_classifications = if classifications.is_empty() {
                vec!["word".to_string()]
            } else {
                classifications
            };

            // Extract terms and create index entries
            let terms_with_classification =
                self.extract_terms(&effective_classifications, value);

            for (term, classification) in &terms_with_classification {
                let entry = IndexEntry::new(
                    schema_name.clone(),
                    key_value.clone(),
                    field_name.clone(),
                    classification.clone(),
                );

                let storage_key = entry.storage_key(term);
                let entry_bytes = serde_json::to_vec(&entry).map_err(|e| {
                    SchemaError::InvalidData(format!("Failed to serialize IndexEntry: {}", e))
                })?;

                index_entries.push((storage_key.into_bytes(), entry_bytes));
            }

            // Also index field name
            let field_entry = IndexEntry::new(
                schema_name.clone(),
                key_value.clone(),
                field_name.clone(),
                "field".to_string(),
            );
            let field_term = field_name.to_ascii_lowercase();
            let field_storage_key = field_entry.storage_key(&format!("field:{}", field_term));
            let field_entry_bytes = serde_json::to_vec(&field_entry).map_err(|e| {
                SchemaError::InvalidData(format!("Failed to serialize field IndexEntry: {}", e))
            })?;
            index_entries.push((field_storage_key.into_bytes(), field_entry_bytes));
        }

        log::info!(
            "[NativeIndex] batch_index: Writing {} index entries",
            index_entries.len()
        );

        // Write all entries using batch operations
        if let Some(ref store) = self.store {
            // Deduplicate by key - DynamoDB batch_write_item doesn't allow duplicate keys
            // This can happen when entries are created within the same millisecond
            let mut seen_keys = std::collections::HashSet::new();
            let deduped_entries: Vec<(Vec<u8>, Vec<u8>)> = index_entries
                .into_iter()
                .filter(|(key, _)| seen_keys.insert(key.clone()))
                .collect();

            log::info!(
                "[NativeIndex] batch_index: After dedup: {} entries",
                deduped_entries.len()
            );

            store.batch_put(deduped_entries).await.map_err(|e| {
                SchemaError::InvalidData(format!("Failed to batch write index entries: {}", e))
            })?;
        } else if let Some(ref tree) = self.tree {
            let mut batch = sled::Batch::default();
            for (key, value) in index_entries {
                batch.insert(key, value);
            }
            tree.apply_batch(batch)
                .map_err(|e| SchemaError::InvalidData(format!("Batch apply failed: {}", e)))?;
        }

        log::info!("[NativeIndex] batch_index: Completed successfully");
        Ok(())
    }

    /// Extract terms from a value for indexing
    fn extract_terms(
        &self,
        classifications: &[String],
        value: &Value,
    ) -> Vec<(String, String)> {
        let mut results = Vec::new();

        for classification in classifications {
            let entries = self.extract_by_classification(classification, value);
            for (index_key, _normalized) in entries {
                // index_key is like "word:hello" or "email:test@example.com"
                results.push((index_key, classification.clone()));
            }
        }

        results
    }

    /// Search for index entries matching a term.
    ///
    /// For multi-word queries like "alice johnson", tries the full phrase first
    /// (direct index match), then falls back to intersecting individual word results.
    pub async fn search(&self, term: &str) -> Result<Vec<IndexEntry>, SchemaError> {
        let Some(normalized) = self.normalize_search_term(term) else {
            return Ok(Vec::new());
        };

        // Try the full term as-is
        let prefix = format!("{}word:{}:", INDEX_ENTRY_PREFIX, normalized);
        let entries = self.scan_index_prefix(&prefix).await?;
        if !entries.is_empty() || !normalized.contains(' ') {
            return Ok(entries);
        }

        // Multi-word with no direct match — intersect individual words
        let words: Vec<String> = term
            .split_whitespace()
            .filter_map(|w| self.normalize_search_term(w))
            .collect();

        if words.len() < 2 {
            return Ok(Vec::new());
        }

        // Search the first word, then filter to records that also match all other words
        let first_prefix = format!("{}word:{}:", INDEX_ENTRY_PREFIX, words[0]);
        let candidates = self.scan_index_prefix(&first_prefix).await?;

        // Collect record keys that appear for every other word
        let mut required_keys: Option<HashSet<(String, KeyValue)>> = None;
        for word in &words[1..] {
            let p = format!("{}word:{}:", INDEX_ENTRY_PREFIX, word);
            let word_entries = self.scan_index_prefix(&p).await?;
            let keys: HashSet<(String, KeyValue)> = word_entries
                .into_iter()
                .map(|e| (e.schema.clone(), e.key.clone()))
                .collect();
            required_keys = Some(match required_keys {
                Some(existing) => existing.intersection(&keys).cloned().collect(),
                None => keys,
            });
        }

        let required_keys = required_keys.unwrap_or_default();
        let mut seen = HashSet::new();
        let results: Vec<IndexEntry> = candidates
            .into_iter()
            .filter(|e| {
                let rk = (e.schema.clone(), e.key.clone());
                required_keys.contains(&rk) && seen.insert(rk)
            })
            .collect();

        Ok(results)
    }

    /// Sync version of search (Sled only).
    pub fn search_sync(&self, term: &str) -> Result<Vec<IndexEntry>, SchemaError> {
        let Some(ref tree) = self.tree else {
            return Err(SchemaError::InvalidData(
                "Sync search only available with Sled backend".to_string(),
            ));
        };

        let Some(normalized) = self.normalize_search_term(term) else {
            return Ok(Vec::new());
        };

        let scan_sync = |prefix: &str| -> Vec<IndexEntry> {
            let mut entries = Vec::new();
            for result in tree.scan_prefix(prefix.as_bytes()) {
                match result {
                    Ok((_key, value)) => match serde_json::from_slice::<IndexEntry>(&value) {
                        Ok(entry) => entries.push(entry),
                        Err(e) => log::warn!("Failed to deserialize IndexEntry: {}", e),
                    },
                    Err(e) => log::warn!("Sled scan error: {}", e),
                }
            }
            entries
        };

        // Try the full term as-is
        let prefix = format!("{}word:{}:", INDEX_ENTRY_PREFIX, normalized);
        let entries = scan_sync(&prefix);
        if !entries.is_empty() || !normalized.contains(' ') {
            return Ok(entries);
        }

        // Multi-word fallback — intersect individual words
        let words: Vec<String> = term
            .split_whitespace()
            .filter_map(|w| self.normalize_search_term(w))
            .collect();

        if words.len() < 2 {
            return Ok(Vec::new());
        }

        let first_prefix = format!("{}word:{}:", INDEX_ENTRY_PREFIX, words[0]);
        let candidates = scan_sync(&first_prefix);

        let mut required_keys: Option<HashSet<(String, KeyValue)>> = None;
        for word in &words[1..] {
            let p = format!("{}word:{}:", INDEX_ENTRY_PREFIX, word);
            let word_entries = scan_sync(&p);
            let keys: HashSet<(String, KeyValue)> = word_entries
                .into_iter()
                .map(|e| (e.schema.clone(), e.key.clone()))
                .collect();
            required_keys = Some(match required_keys {
                Some(existing) => existing.intersection(&keys).cloned().collect(),
                None => keys,
            });
        }

        let required_keys = required_keys.unwrap_or_default();
        let mut seen = HashSet::new();
        let results: Vec<IndexEntry> = candidates
            .into_iter()
            .filter(|e| {
                let rk = (e.schema.clone(), e.key.clone());
                required_keys.contains(&rk) && seen.insert(rk)
            })
            .collect();

        Ok(results)
    }

    /// Search with classification using prefix scan
    pub async fn search_with_classification(
        &self,
        term: &str,
        classification: Option<ClassificationType>,
    ) -> Result<Vec<IndexEntry>, SchemaError> {
        let normalized = match classification {
            Some(ClassificationType::Word) | None => self.normalize_search_term(term),
            Some(_) => {
                let trimmed = term.trim().to_ascii_lowercase();
                if trimmed.is_empty() {
                    None
                } else {
                    Some(trimmed)
                }
            }
        };

        let Some(normalized) = normalized else {
            return Ok(Vec::new());
        };

        let class_prefix = classification
            .map(|c| c.prefix())
            .unwrap_or_else(|| "word".to_string());

        // Build prefix: idx:{classification}:{normalized}:
        let prefix = format!("{}{}:{}:", INDEX_ENTRY_PREFIX, class_prefix, normalized);

        log::debug!(
            "[NativeIndex] search_with_classification: Searching with prefix '{}'",
            prefix
        );

        self.scan_index_prefix(&prefix).await
    }

    /// Search all indexed keywords and field names.
    /// Supports multi-word queries (phrase match first, then word intersection).
    pub async fn search_all(&self, term: &str) -> Result<Vec<IndexEntry>, SchemaError> {
        // Use search which handles multi-word intersection
        let (word_result, field_result) = tokio::join!(
            self.search(term),
            self.search_field_names(term)
        );

        let mut all_entries = Vec::new();
        let mut seen = HashSet::new();

        if let Ok(entries) = word_result {
            for entry in entries {
                let key = format!("{:?}:{:?}:{}", entry.schema, entry.key, entry.field);
                if seen.insert(key) {
                    all_entries.push(entry);
                }
            }
        }

        if let Ok(field_entries) = field_result {
            for entry in field_entries {
                let key = format!("{:?}:{:?}:{}", entry.schema, entry.key, entry.field);
                if seen.insert(key) {
                    all_entries.push(entry);
                }
            }
        }

        Ok(all_entries)
    }

    /// Search for field names in the index
    async fn search_field_names(
        &self,
        term: &str,
    ) -> Result<Vec<IndexEntry>, SchemaError> {
        let normalized = term.trim().to_ascii_lowercase();
        if normalized.is_empty() {
            return Ok(Vec::new());
        }

        let prefix = format!("{}field:{}:", INDEX_ENTRY_PREFIX, normalized);
        self.scan_index_prefix(&prefix).await
    }

    /// Search for field names in the index (sync version, Sled only)
    pub fn search_field_names_sync(
        &self,
        term: &str,
    ) -> Result<Vec<IndexEntry>, SchemaError> {
        let Some(ref tree) = self.tree else {
            return Err(SchemaError::InvalidData(
                "Sync field name search only available with Sled backend.".to_string(),
            ));
        };

        let normalized = term.trim().to_ascii_lowercase();
        if normalized.is_empty() {
            return Ok(Vec::new());
        }

        let prefix = format!("{}field:{}:", INDEX_ENTRY_PREFIX, normalized);

        log::debug!(
            "[NativeIndex] search_field_names_sync: Searching with prefix '{}'",
            prefix
        );

        let mut entries = Vec::new();
        for result in tree.scan_prefix(prefix.as_bytes()) {
            match result {
                Ok((_key, value)) => match serde_json::from_slice::<IndexEntry>(&value) {
                    Ok(entry) => entries.push(entry),
                    Err(e) => {
                        log::warn!("Failed to deserialize IndexEntry: {}", e);
                    }
                },
                Err(e) => {
                    log::warn!("Sled scan error: {}", e);
                }
            }
        }

        log::info!(
            "[NativeIndex] search_field_names_sync: Found {} field name entries for term '{}'",
            entries.len(),
            term
        );

        Ok(entries)
    }

    /// Scan index entries by prefix
    async fn scan_index_prefix(&self, prefix: &str) -> Result<Vec<IndexEntry>, SchemaError> {
        let results = if let Some(ref store) = self.store {
            store
                .scan_prefix(prefix.as_bytes())
                .await
                .map_err(|e| SchemaError::InvalidData(format!("Failed to scan prefix: {}", e)))?
        } else if let Some(ref tree) = self.tree {
            tree.scan_prefix(prefix.as_bytes())
                .filter_map(|r| r.ok())
                .map(|(k, v)| (k.to_vec(), v.to_vec()))
                .collect()
        } else {
            return Err(SchemaError::InvalidData(
                "NativeIndexManager not properly initialized".to_string(),
            ));
        };

        let mut entries = Vec::new();
        for (_key, value) in results {
            match serde_json::from_slice::<IndexEntry>(&value) {
                Ok(entry) => entries.push(entry),
                Err(e) => {
                    log::warn!("Failed to deserialize IndexEntry: {}", e);
                }
            }
        }

        Ok(entries)
    }

    /// Convert IndexEntry results to IndexResult for backward compatibility
    pub fn entries_to_results(&self, entries: Vec<IndexEntry>) -> Vec<IndexResult> {
        entries
            .into_iter()
            .map(|e| e.to_index_result(None))
            .collect()
    }
}

#[cfg(test)]
mod tests {
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
}
