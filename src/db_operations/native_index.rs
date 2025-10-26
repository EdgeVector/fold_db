use crate::schema::types::key_value::KeyValue;
use crate::schema::SchemaError;
use super::native_index_classification::ClassificationType;
use log::{debug, info};
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
use sled::Tree;
use std::collections::HashSet;

const WORD_PREFIX: &str = "word:";
const RECORD_PREFIX: &str = "record:";

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
    tree: Tree,
}

impl NativeIndexManager {
    pub fn new(tree: Tree) -> Self {
        Self { tree }
    }


    pub fn search_word(&self, term: &str) -> Result<Vec<IndexResult>, SchemaError> {
        debug!("Native Index Search: Searching for word '{}'", term);
        let Some(normalized) = self.normalize_search_term(term) else {
            debug!("Native Index Search: Term '{}' normalized to empty string", term);
            return Ok(Vec::new());
        };

        let key = format!("{}{}", WORD_PREFIX, normalized);
        debug!("Native Index Search: Looking up key: '{}'", key);
        let Some(bytes) = self.tree.get(key.as_bytes())? else {
            debug!("Native Index Search: No results found for key: '{}'", key);
            return Ok(Vec::new());
        };

        let results: Vec<IndexResult> = serde_json::from_slice(&bytes).map_err(|e| {
            SchemaError::InvalidData(format!("Failed to deserialize index results: {}", e))
        })?;

        info!("Native Index Search: Found {} results for word '{}'", results.len(), term);
        Ok(results)
    }

    /// Search with optional classification filter
    pub fn search_with_classification(
        &self,
        term: &str,
        classification: Option<ClassificationType>,
    ) -> Result<Vec<IndexResult>, SchemaError> {
        log::info!("🔍 Searching for term '{}' with classification {:?}", term, classification);
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
            log::info!("⚠️ Search term '{}' normalized to empty string", term);
            return Ok(Vec::new());
        };

        let key = if let Some(ref class) = classification {
            format!("{}:{}", class.prefix(), normalized)
        } else {
            format!("{}{}", WORD_PREFIX, normalized)
        };
        log::info!("🔑 Searching with key: '{}'", key);

        use crate::logging::features::{log_feature, LogFeature};
        log_feature!(
            LogFeature::Database,
            info,
            "Searching for key: {} (classification: {:?})",
            key,
            classification.as_ref().map(|c| c.prefix())
        );

        let Some(bytes) = self.tree.get(key.as_bytes())? else {
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

    fn extract_whole_values_recursive(classification: &str, value: &Value, acc: &mut Vec<(String, String)>) {
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
            RECORD_PREFIX, schema_name, field_name, serialized_key
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

    fn read_entries(&self, key: &str) -> Result<Vec<IndexResult>, SchemaError> {
        let Some(bytes) = self.tree.get(key.as_bytes())? else {
            log::debug!("📭 No entries found for key: {}", key);
            return Ok(Vec::new());
        };

        let entries: Vec<IndexResult> = serde_json::from_slice(&bytes).map_err(|e| {
            SchemaError::InvalidData(format!("Failed to deserialize index entries: {}", e))
        })?;
        log::debug!("📬 Read {} entries from key: {}", entries.len(), key);
        Ok(entries)
    }

    fn write_entries(&self, key: &str, entries: &[IndexResult]) -> Result<(), SchemaError> {
        if entries.is_empty() {
            log::info!("🗑️ Removing empty index key: {}", key);
            self.tree.remove(key.as_bytes())?;
            return Ok(());
        }

        log::info!("✍️ Writing {} entries to index key: {}", entries.len(), key);
        let bytes = serde_json::to_vec(entries).map_err(|e| {
            SchemaError::InvalidData(format!("Failed to serialize index entries: {}", e))
        })?;
        self.tree.insert(key.as_bytes(), bytes)?;
        Ok(())
    }


    fn remove_record_entries(
        &self,
        record_key: &str,
        schema_name: &str,
        field_name: &str,
        key_value: &KeyValue,
    ) -> Result<(), SchemaError> {
        let Some(bytes) = self.tree.get(record_key.as_bytes())? else {
            return Ok(());
        };

        let words: Vec<String> = serde_json::from_slice(&bytes).map_err(|e| {
            SchemaError::InvalidData(format!("Failed to deserialize record index words: {}", e))
        })?;

        for word in words {
            let index_key = format!("{}{}", WORD_PREFIX, word);
            let mut entries = self.read_entries(&index_key)?;
            let initial_len = entries.len();

            entries.retain(|entry| {
                !(entry.schema_name == schema_name
                    && entry.field == field_name
                    && entry.key_value == *key_value)
            });

            if entries.is_empty() {
                self.tree.remove(index_key.as_bytes())?;
            } else if entries.len() != initial_len {
                self.write_entries(&index_key, &entries)?;
            }
        }

        self.tree.remove(record_key.as_bytes())?;
        Ok(())
    }

    // ========== BATCH INDEX OPERATIONS ==========

    /// Batch index multiple field values with classifications
    pub fn batch_index_field_values_with_classifications(
        &self,
        index_operations: &[BatchIndexOperation],
    ) -> Result<(), SchemaError> {
        use std::collections::HashMap;
        let mut index_map: HashMap<String, Vec<IndexResult>> = HashMap::new();
        let mut record_keys: Vec<(String, Vec<String>)> = Vec::new();

        for (schema_name, field_name, key_value, value, classifications) in index_operations {
            if !self.should_index_field(field_name) {
                continue;
            }

            let classifications = classifications.clone().unwrap_or_else(|| vec!["word".to_string()]);
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
    }

    fn extract_and_aggregate_entries(
        &self,
        classifications: &[String],
        value: &Value,
        schema_name: &str,
        field_name: &str,
        key_value: &KeyValue,
        index_map: &mut std::collections::HashMap<String, Vec<IndexResult>>,
    ) -> Result<Vec<String>, SchemaError> {
        let mut all_index_keys = Vec::new();

        for classification_str in classifications {
            let index_entries = self.extract_by_classification(classification_str, value);

            for (index_key, normalized_value) in index_entries {
                let index_entry = IndexResult {
                    schema_name: schema_name.to_string(),
                    field: field_name.to_string(),
                    key_value: key_value.clone(),
                    value: value.clone(),
                    metadata: Some(json!({
                        "classification": classification_str,
                        "normalized": normalized_value
                    })),
                };

                index_map.entry(index_key.clone()).or_default().push(index_entry);
                all_index_keys.push(index_key);
            }
        }

        Ok(all_index_keys)
    }

    fn extract_by_classification(&self, classification: &str, value: &Value) -> Vec<(String, String)> {
        match classification {
            "word" => {
                let words = self.collect_words(value);
                words.into_iter().map(|w| (format!("word:{}", w), w)).collect()
            }
            c if c.starts_with("hashtag") => self.extract_hashtags(value),
            c if c.starts_with("email") => self.extract_emails(value),
            c if c.starts_with("name:") || c.starts_with("username") || c.starts_with("phone") 
                || c.starts_with("url") || c.starts_with("date") => {
                self.extract_whole_values(c, value)
            }
            _ => {
                let words = self.collect_words(value);
                words.into_iter().map(|w| (format!("word:{}", w), w)).collect()
            }
        }
    }

    fn build_batch_operations(
        &self,
        index_map: std::collections::HashMap<String, Vec<IndexResult>>,
        record_keys: Vec<(String, Vec<String>)>,
    ) -> Result<Vec<(String, serde_json::Value)>, SchemaError> {
        let mut batch_operations = Vec::new();

        for (index_key, new_entries) in index_map {
            let merged_entries = self.merge_with_existing_entries(&index_key, new_entries)?;
            batch_operations.push((index_key, serde_json::to_value(&merged_entries)
                .map_err(|e| SchemaError::InvalidData(format!("Serialization failed: {}", e)))?));
        }

        for (record_key, index_keys) in record_keys {
            batch_operations.push((record_key, serde_json::Value::Array(
                index_keys.into_iter().map(serde_json::Value::String).collect()
            )));
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
        log::info!("📦 Batch executing {} index operations", operations.len());
        let mut batch = sled::Batch::default();

        for (key, value) in operations {
            let bytes = serde_json::to_vec(value)
                .map_err(|e| SchemaError::InvalidData(format!("Serialization failed: {}", e)))?;
            batch.insert(key.as_bytes(), bytes);
        }

        self.tree.apply_batch(batch)
            .map_err(|e| SchemaError::InvalidData(format!("Batch apply failed: {}", e)))?;

        // Ensure the data is durably written to disk
        self.tree.flush()
            .map_err(|e| SchemaError::InvalidData(format!("Flush failed: {}", e)))?;

        log::info!("✅ Batch flushed {} operations to disk", operations.len());
        Ok(())
    }

    /// Explicitly flush the index tree to disk
    /// 
    /// This should only be called for non-batch operations.
    /// Batch operations handle flushing internally.
    pub fn flush(&self) -> Result<(), SchemaError> {
        self.tree.flush()
            .map_err(|e| SchemaError::InvalidData(format!("Flush failed: {}", e)))?;
        Ok(())
    }
}
