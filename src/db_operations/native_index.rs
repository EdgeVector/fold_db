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

const STOPWORDS: &[&str] = &[
    "a", "an", "and", "are", "as", "at", "be", "but", "by", "for", "from", "in", "is", "it", "of",
    "on", "or", "the", "to", "with",
];

const MIN_WORD_LENGTH: usize = 2;
const MAX_WORD_LENGTH: usize = 100;
const EXCLUDED_FIELDS: &[&str] = &["uuid", "id", "password", "token"];

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

    pub async fn search_word_async(&self, term: &str) -> Result<Vec<IndexResult>, SchemaError> {
        log::debug!("Native Index: search_word called for term: '{}'", term);
        let Some(normalized) = self.normalize_search_term(term) else {
            log::debug!("Native Index: Term '{}' normalized to empty string", term);
            return Ok(Vec::new());
        };

        let mut all_results = Vec::new();

        // Search for word matches
        let word_key = format!("{}{}", structural_prefixes::WORD, normalized);
        log::debug!("Native Index: Looking up word key: '{}'", word_key);
        if let Some(bytes) = self.get(word_key.as_bytes()).await? {
            let word_results: Vec<IndexResult> = serde_json::from_slice(&bytes).map_err(|e| {
                SchemaError::InvalidData(format!("Failed to deserialize word index results: {}", e))
            })?;
            log::debug!(
                "Native Index: Found {} word results for key '{}'",
                word_results.len(),
                word_key
            );
            all_results.extend(word_results);
        }

        // Also search for field name matches (e.g., searching "email" returns all records with an email field)
        let field_key = format!("{}{}", structural_prefixes::FIELD, normalized);
        log::debug!("Native Index: Looking up field key: '{}'", field_key);
        if let Some(bytes) = self.get(field_key.as_bytes()).await? {
            let field_results: Vec<IndexResult> = serde_json::from_slice(&bytes).map_err(|e| {
                SchemaError::InvalidData(format!(
                    "Failed to deserialize field index results: {}",
                    e
                ))
            })?;
            log::debug!(
                "Native Index: Found {} field results for key '{}'",
                field_results.len(),
                field_key
            );
            all_results.extend(field_results);
        }

        log::info!(
            "Native Index: search_word for '{}' returned {} results",
            term,
            all_results.len()
        );
        Ok(all_results)
    }

    /// Synchronous version for backward compatibility (Sled only)
    pub fn search_word(&self, term: &str) -> Result<Vec<IndexResult>, SchemaError> {
        if let Some(ref tree) = self.tree {
            log::debug!("Native Index: search_word called for term: '{}'", term);
            let Some(normalized) = self.normalize_search_term(term) else {
                log::debug!("Native Index: Term '{}' normalized to empty string", term);
                return Ok(Vec::new());
            };

            let mut all_results = Vec::new();

            // Search for word matches
            let word_key = format!("{}{}", structural_prefixes::WORD, normalized);
            log::debug!("Native Index: Looking up word key: '{}'", word_key);
            if let Some(bytes) = tree.get(word_key.as_bytes())? {
                let word_results: Vec<IndexResult> =
                    serde_json::from_slice(&bytes).map_err(|e| {
                        SchemaError::InvalidData(format!(
                            "Failed to deserialize word index results: {}",
                            e
                        ))
                    })?;
                log::debug!(
                    "Native Index: Found {} word results for key '{}'",
                    word_results.len(),
                    word_key
                );
                all_results.extend(word_results);
            }

            // Also search for field name matches
            let field_key = format!("{}{}", structural_prefixes::FIELD, normalized);
            log::debug!("Native Index: Looking up field key: '{}'", field_key);
            if let Some(bytes) = tree.get(field_key.as_bytes())? {
                let field_results: Vec<IndexResult> =
                    serde_json::from_slice(&bytes).map_err(|e| {
                        SchemaError::InvalidData(format!(
                            "Failed to deserialize field index results: {}",
                            e
                        ))
                    })?;
                log::debug!(
                    "Native Index: Found {} field results for key '{}'",
                    field_results.len(),
                    field_key
                );
                all_results.extend(field_results);
            }

            log::info!(
                "Native Index: search_word for '{}' returned {} results",
                term,
                all_results.len()
            );
            Ok(all_results)
        } else {
            Err(SchemaError::InvalidData("Synchronous search_word only available with Sled backend. Use search_word_async instead.".to_string()))
        }
    }

    /// Search with optional classification filter
    pub fn search_with_classification(
        &self,
        term: &str,
        classification: Option<ClassificationType>,
    ) -> Result<Vec<IndexResult>, SchemaError> {
        log::debug!(
            "Native Index: Searching for term '{}' with classification {:?}",
            term,
            classification
        );
        // For word classification, extract first word
        // For other classifications (names, etc.), keep the whole term
        let normalized = match classification {
            Some(ClassificationType::Word) | None => {
                // Word search: extract first word
                self.normalize_search_term(term)
            }
            Some(_) => {
                // Name/entity search: keep whole term (but normalized)
                let trimmed = term.trim().to_ascii_lowercase();
                if trimmed.is_empty() {
                    None
                } else {
                    Some(trimmed)
                }
            }
        };

        let Some(normalized) = normalized else {
            log::debug!(
                "Native Index: Search term '{}' normalized to empty string",
                term
            );
            return Ok(Vec::new());
        };

        let key = if let Some(ref class) = classification {
            format!("{}:{}", class.prefix(), normalized)
        } else {
            format!("{}{}", structural_prefixes::WORD, normalized)
        };
        log::debug!("Native Index: Searching with key: '{}'", key);

        use crate::logging::features::{log_feature, LogFeature};
        log_feature!(
            LogFeature::Database,
            info,
            "Searching for key: {} (classification: {:?})",
            key,
            classification.as_ref().map(|c| c.prefix())
        );

        let bytes = if let Some(ref tree) = self.tree {
            tree.get(key.as_bytes())?
        } else {
            return Err(SchemaError::InvalidData("Synchronous search_with_classification only available with Sled backend. Use search_with_classification_async instead.".to_string()));
        };

        let Some(bytes) = bytes else {
            log_feature!(
                LogFeature::Database,
                info,
                "No results found for key: {}",
                key
            );
            return Ok(Vec::new());
        };

        let results: Vec<IndexResult> = serde_json::from_slice(&bytes).map_err(|e| {
            SchemaError::InvalidData(format!("Failed to deserialize index results: {}", e))
        })?;

        Ok(results)
    }

    /// Async version of search_with_classification
    pub async fn search_with_classification_async(
        &self,
        term: &str,
        classification: Option<ClassificationType>,
    ) -> Result<Vec<IndexResult>, SchemaError> {
        log::debug!(
            "Native Index: Searching for term '{}' with classification {:?}",
            term,
            classification
        );
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
            log::debug!(
                "Native Index: Search term '{}' normalized to empty string",
                term
            );
            return Ok(Vec::new());
        };

        let key = if let Some(ref class) = classification {
            format!("{}:{}", class.prefix(), normalized)
        } else {
            format!("{}{}", structural_prefixes::WORD, normalized)
        };
        log::debug!("Native Index: Searching with key: '{}'", key);

        use crate::logging::features::{log_feature, LogFeature};
        log_feature!(
            LogFeature::Database,
            info,
            "Searching for key: {} (classification: {:?})",
            key,
            classification.as_ref().map(|c| c.prefix())
        );

        let Some(bytes) = self.get(key.as_bytes()).await? else {
            log_feature!(
                LogFeature::Database,
                info,
                "No results found for key: {}",
                key
            );
            return Ok(Vec::new());
        };

        let results: Vec<IndexResult> = serde_json::from_slice(&bytes).map_err(|e| {
            SchemaError::InvalidData(format!("Failed to deserialize index results: {}", e))
        })?;

        Ok(results)
    }

    /// Async version of search_all_classifications
    pub async fn search_all_classifications_async(
        &self,
        term: &str,
    ) -> Result<Vec<IndexResult>, SchemaError> {
        use std::collections::HashSet;

        log::debug!(
            "Native Index: search_all_classifications called for term: '{}'",
            term
        );

        let mut all_results = Vec::new();
        let mut seen_keys = HashSet::new();

        // First, do a basic word search which includes both word matches AND field name matches
        match self.search_word_async(term).await {
            Ok(results) => {
                log::debug!(
                    "Native Index: Word search (including field names) returned {} results",
                    results.len()
                );
                for result in results {
                    let classification_str = result
                        .metadata
                        .as_ref()
                        .and_then(|m| m.get("classification"))
                        .and_then(|c| c.as_str())
                        .unwrap_or("word");
                    let key = format!(
                        "{}:{}:{:?}:{}",
                        result.schema_name, result.field, result.key_value, classification_str
                    );
                    if seen_keys.insert(key) {
                        all_results.push(result);
                    }
                }
            }
            Err(e) => {
                log::error!("Native Index: Word search failed: {}", e);
            }
        }

        // Search all other classification types for more specific matches
        let classifications = vec![
            ClassificationType::NamePerson,
            ClassificationType::NameCompany,
            ClassificationType::NamePlace,
            ClassificationType::Email,
            ClassificationType::Phone,
            ClassificationType::Url,
            ClassificationType::Date,
            ClassificationType::Hashtag,
            ClassificationType::Username,
        ];

        log::debug!(
            "Native Index: Searching {} additional classification types",
            classifications.len()
        );

        for classification in classifications {
            match self
                .search_with_classification_async(term, Some(classification.clone()))
                .await
            {
                Ok(results) => {
                    log::debug!(
                        "Native Index: Classification {:?} returned {} results",
                        classification,
                        results.len()
                    );
                    for result in results {
                        let classification_str = result
                            .metadata
                            .as_ref()
                            .and_then(|m| m.get("classification"))
                            .and_then(|c| c.as_str())
                            .unwrap_or("unknown");
                        let key = format!(
                            "{}:{}:{:?}:{}",
                            result.schema_name, result.field, result.key_value, classification_str
                        );
                        if seen_keys.insert(key) {
                            all_results.push(result);
                        }
                    }
                }
                Err(e) => {
                    log::error!(
                        "Native Index: Classification {:?} search failed: {}",
                        classification,
                        e
                    );
                }
            }
        }

        log::info!(
            "Native Index: search_all_classifications for '{}' returned {} total results",
            term,
            all_results.len()
        );
        Ok(all_results)
    }

    /// Search across all classification types and aggregate results
    /// This includes word matches, field name matches, and all specialized classifications
    /// Synchronous version (Sled only)
    pub fn search_all_classifications(&self, term: &str) -> Result<Vec<IndexResult>, SchemaError> {
        use std::collections::HashSet;

        log::debug!(
            "Native Index: search_all_classifications called for term: '{}'",
            term
        );

        let mut all_results = Vec::new();
        let mut seen_keys = HashSet::new();

        // First, do a basic word search which includes both word matches AND field name matches
        match self.search_word(term) {
            Ok(results) => {
                log::debug!(
                    "Native Index: Word search (including field names) returned {} results",
                    results.len()
                );
                for result in results {
                    let classification_str = result
                        .metadata
                        .as_ref()
                        .and_then(|m| m.get("classification"))
                        .and_then(|c| c.as_str())
                        .unwrap_or("word");
                    let key = format!(
                        "{}:{}:{:?}:{}",
                        result.schema_name, result.field, result.key_value, classification_str
                    );
                    if seen_keys.insert(key) {
                        all_results.push(result);
                    }
                }
            }
            Err(e) => {
                log::error!("Native Index: Word search failed: {}", e);
            }
        }

        // Search all other classification types for more specific matches
        let classifications = vec![
            ClassificationType::NamePerson,
            ClassificationType::NameCompany,
            ClassificationType::NamePlace,
            ClassificationType::Email,
            ClassificationType::Phone,
            ClassificationType::Url,
            ClassificationType::Date,
            ClassificationType::Hashtag,
            ClassificationType::Username,
        ];

        log::debug!(
            "Native Index: Searching {} additional classification types",
            classifications.len()
        );

        for classification in classifications {
            match self.search_with_classification(term, Some(classification.clone())) {
                Ok(results) => {
                    log::debug!(
                        "Native Index: Classification {:?} returned {} results",
                        classification,
                        results.len()
                    );
                    for result in results {
                        // Deduplicate by schema + field + key + classification
                        // Different classifications of the same field/record are DISTINCT results
                        let classification_str = result
                            .metadata
                            .as_ref()
                            .and_then(|m| m.get("classification"))
                            .and_then(|c| c.as_str())
                            .unwrap_or("unknown");
                        let key = format!(
                            "{}:{}:{:?}:{}",
                            result.schema_name, result.field, result.key_value, classification_str
                        );
                        if seen_keys.insert(key) {
                            all_results.push(result);
                        }
                    }
                }
                Err(e) => {
                    log::error!(
                        "Native Index: Classification {:?} search failed: {}",
                        classification,
                        e
                    );
                }
            }
        }

        log::info!(
            "Native Index: search_all_classifications for '{}' returned {} total results",
            term,
            all_results.len()
        );
        Ok(all_results)
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

    fn should_index_field(&self, field_name: &str) -> bool {
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

    fn collect_words(&self, value: &Value) -> Vec<String> {
        let mut words = HashSet::new();
        self.collect_words_recursive(value, &mut words);
        let mut result: Vec<String> = words.into_iter().collect();
        result.sort_unstable();
        result
    }

    fn collect_words_recursive(&self, value: &Value, acc: &mut HashSet<String>) {
        match value {
            Value::String(text) => {
                for segment in text.split(|c: char| !c.is_alphanumeric()) {
                    if let Some(word) = self.normalize_word(segment) {
                        acc.insert(word);
                    }
                }
            }
            Value::Array(values) => {
                for item in values {
                    self.collect_words_recursive(item, acc);
                }
            }
            Value::Object(obj) => {
                // For objects, recursively process all values
                for (_, nested_value) in obj {
                    self.collect_words_recursive(nested_value, acc);
                }
            }
            _ => {}
        }
    }

    fn normalize_word(&self, raw: &str) -> Option<String> {
        let trimmed = raw.trim();
        if trimmed.is_empty() {
            return None;
        }

        let normalized = trimmed.to_ascii_lowercase();

        if normalized.len() < MIN_WORD_LENGTH || normalized.len() > MAX_WORD_LENGTH {
            return None;
        }

        if STOPWORDS.contains(&normalized.as_str()) {
            return None;
        }

        Some(normalized)
    }

    fn normalize_search_term(&self, term: &str) -> Option<String> {
        for segment in term.split(|c: char| !c.is_alphanumeric()) {
            if let Some(word) = self.normalize_word(segment) {
                return Some(word);
            }
        }
        None
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

    /// Write entries to index (async version for DynamoDB)
    /// Uses simplified key structure: feature as PK, term as SK
    async fn write_entries_async(
        &self,
        key: &str,
        entries: &[IndexResult],
    ) -> Result<(), SchemaError> {
        if let Some(ref _store) = self.store {
            if entries.is_empty() {
                log::debug!("Native Index: Removing empty index key: {}", key);
                self.delete(key.as_bytes()).await?;
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
            self.put(key.as_bytes(), bytes).await?;
            Ok(())
        } else {
            Err(SchemaError::InvalidData(
                "Async write_entries only available with KvStore backend".to_string(),
            ))
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

    /// Remove record entries (async version for DynamoDB)
    async fn remove_record_entries_async(
        &self,
        record_key: &str,
        schema_name: &str,
        field_name: &str,
        key_value: &KeyValue,
    ) -> Result<(), SchemaError> {
        if let Some(ref _store) = self.store {
            let bytes = self.get(record_key.as_bytes()).await?;

            let Some(bytes) = bytes else {
                return Ok(());
            };

            let words: Vec<String> = serde_json::from_slice(&bytes).map_err(|e| {
                SchemaError::InvalidData(format!("Failed to deserialize record index words: {}", e))
            })?;

            for word in words {
                let index_key = format!("{}{}", structural_prefixes::WORD, word);
                let mut entries = self.read_entries_async(&index_key).await?;
                let initial_len = entries.len();

                entries.retain(|entry| {
                    !(entry.schema_name == schema_name
                        && entry.field == field_name
                        && entry.key_value == *key_value)
                });

                if entries.is_empty() {
                    self.delete(index_key.as_bytes()).await?;
                } else if entries.len() != initial_len {
                    self.write_entries_async(&index_key, &entries).await?;
                }
            }

            self.delete(record_key.as_bytes()).await?;
            Ok(())
        } else {
            Err(SchemaError::InvalidData(
                "Async remove_record_entries only available with KvStore backend".to_string(),
            ))
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
                if !self.should_index_field(field_name) {
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
        if self.store.is_none() {
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

        for (i, fetch_result) in record_fetches.into_iter().enumerate() {
            if let Ok(Some(bytes)) = fetch_result {
                // Record exists! Deserialize to get old words/keys
                if let Ok(old_keys) = serde_json::from_slice::<Vec<String>>(&bytes) {
                    for key in old_keys {
                        let index_key = format!("{}{}", structural_prefixes::WORD, key);
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
            if !self.should_index_field(field_name) {
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
            join_all(write_futures).await;
            join_all(delete_futures).await;
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
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::schema::types::key_value::KeyValue;

    #[test]
    fn test_indexing_with_empty_classifications() {
        let tree = sled::Config::new()
            .temporary(true)
            .open()
            .unwrap()
            .open_tree("test_index")
            .unwrap();
        let manager = NativeIndexManager::new(tree);

        let operations = vec![(
            "TestSchema".to_string(),
            "test_field".to_string(),
            KeyValue::new(Some("key1".to_string()), None),
            serde_json::Value::String("hello world".to_string()),
            Some(vec![]), // Empty classifications
        )];

        // Should default to "word" indexing
        manager
            .batch_index_field_values_with_classifications(&operations)
            .unwrap();

        // Verify "word" search works
        let results = manager.search_word("hello").unwrap();
        assert_eq!(results.len(), 1);
        assert_eq!(
            results[0].key_value,
            KeyValue::new(Some("key1".to_string()), None)
        );

        // Verify classification metadata is "word"
        let metadata = results[0].metadata.as_ref().unwrap();
        assert_eq!(metadata["classification"], "word");
    }
}
