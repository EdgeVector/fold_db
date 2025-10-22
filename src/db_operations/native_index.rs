use crate::schema::types::key_value::KeyValue;
use crate::schema::SchemaError;
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

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct IndexResult {
    pub schema_name: String,
    pub field: String,
    pub key_value: KeyValue,
    pub value: Value,
    pub metadata: Option<Value>,
}

#[derive(Debug, Clone)]
pub struct NativeIndexConfig {
    pub enabled: bool,
    pub min_word_length: usize,
    pub max_word_length: usize,
    pub excluded_fields: Vec<String>,
    pub filter_stopwords: bool,
}

impl Default for NativeIndexConfig {
    fn default() -> Self {
        Self {
            enabled: true,
            min_word_length: 2,
            max_word_length: 100,
            excluded_fields: vec![
                "uuid".to_string(),
                "id".to_string(),
                "password".to_string(),
                "token".to_string(),
            ],
            filter_stopwords: true,
        }
    }
}

#[derive(Clone)]
pub struct NativeIndexManager {
    tree: Tree,
    config: NativeIndexConfig,
}

impl NativeIndexManager {
    pub fn new(tree: Tree, config: NativeIndexConfig) -> Self {
        Self { tree, config }
    }

    pub fn search_word(&self, term: &str) -> Result<Vec<IndexResult>, SchemaError> {
        let Some(normalized) = self.normalize_search_term(term) else {
            return Ok(Vec::new());
        };

        let key = format!("{}{}", WORD_PREFIX, normalized);
        let Some(bytes) = self.tree.get(key.as_bytes())? else {
            return Ok(Vec::new());
        };

        let results: Vec<IndexResult> = serde_json::from_slice(&bytes).map_err(|e| {
            SchemaError::InvalidData(format!("Failed to deserialize index results: {}", e))
        })?;

        Ok(results)
    }

    pub fn index_field_value(
        &self,
        schema_name: &str,
        field_name: &str,
        key_value: &KeyValue,
        value: &Value,
    ) -> Result<(), SchemaError> {
        if !self.config.enabled || !self.should_index_field(field_name) {
            return Ok(());
        }

        let record_key = self.build_record_key(schema_name, field_name, key_value)?;
        self.remove_record_entries(&record_key, schema_name, field_name, key_value)?;

        let words = self.collect_words(value);

        if words.is_empty() {
            self.tree.remove(record_key.as_bytes())?;
            self.tree.flush()?;
            return Ok(());
        }

        for word in &words {
            let index_key = format!("{}{}", WORD_PREFIX, word);
            let mut entries = self.read_entries(&index_key)?;
            entries.retain(|entry| {
                !(entry.schema_name == schema_name
                    && entry.field == field_name
                    && entry.key_value == *key_value)
            });

            let index_entry = IndexResult {
                schema_name: schema_name.to_string(),
                field: field_name.to_string(),
                key_value: key_value.clone(),
                value: value.clone(),
                metadata: Some(json!({ "word": word })),
            };

            entries.push(index_entry);
            self.write_entries(&index_key, &entries)?;
        }

        self.store_record_words(&record_key, &words)?;
        self.tree.flush()?;
        Ok(())
    }

    fn should_index_field(&self, field_name: &str) -> bool {
        !self
            .config
            .excluded_fields
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
            _ => {}
        }
    }

    fn normalize_word(&self, raw: &str) -> Option<String> {
        let trimmed = raw.trim();
        if trimmed.is_empty() {
            return None;
        }

        let normalized = trimmed.to_ascii_lowercase();

        if normalized.len() < self.config.min_word_length
            || normalized.len() > self.config.max_word_length
        {
            return None;
        }

        if self.config.filter_stopwords && STOPWORDS.contains(&normalized.as_str()) {
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
            return Ok(Vec::new());
        };

        let entries = serde_json::from_slice(&bytes).map_err(|e| {
            SchemaError::InvalidData(format!("Failed to deserialize index entries: {}", e))
        })?;
        Ok(entries)
    }

    fn write_entries(&self, key: &str, entries: &[IndexResult]) -> Result<(), SchemaError> {
        if entries.is_empty() {
            self.tree.remove(key.as_bytes())?;
            return Ok(());
        }

        let bytes = serde_json::to_vec(entries).map_err(|e| {
            SchemaError::InvalidData(format!("Failed to serialize index entries: {}", e))
        })?;
        self.tree.insert(key.as_bytes(), bytes)?;
        Ok(())
    }

    fn store_record_words(&self, record_key: &str, words: &[String]) -> Result<(), SchemaError> {
        let bytes = serde_json::to_vec(words).map_err(|e| {
            SchemaError::InvalidData(format!("Failed to serialize record index words: {}", e))
        })?;
        self.tree.insert(record_key.as_bytes(), bytes)?;
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
}
