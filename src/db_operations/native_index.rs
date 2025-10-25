use crate::schema::types::key_value::KeyValue;
use crate::schema::SchemaError;
use super::native_index_classification::{
    ClassificationCacheKey, ClassificationType, FieldClassification,
    SplitStrategy,
};
use super::native_index_ai_classifier::NativeIndexAIClassifier;
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
use sled::Tree;
use std::collections::{HashMap, HashSet};
use std::sync::Arc;
use tokio::sync::RwLock;

const WORD_PREFIX: &str = "word:";
const RECORD_PREFIX: &str = "record:";

const STOPWORDS: &[&str] = &[
    "a", "an", "and", "are", "as", "at", "be", "but", "by", "for", "from", "in", "is", "it", "of",
    "on", "or", "the", "to", "with",
];

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, utoipa::ToSchema)]
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
    /// Optional AI classifier for intelligent field classification
    ai_classifier: Option<Arc<NativeIndexAIClassifier>>,
    /// Cache of field classifications to avoid repeated AI calls
    classification_cache: Arc<RwLock<HashMap<ClassificationCacheKey, FieldClassification>>>,
}

impl NativeIndexManager {
    pub fn new(tree: Tree, config: NativeIndexConfig) -> Self {
        Self {
            tree,
            config,
            ai_classifier: None,
            classification_cache: Arc::new(RwLock::new(HashMap::new())),
        }
    }

    pub fn with_ai_classifier(
        tree: Tree,
        config: NativeIndexConfig,
        ai_classifier: NativeIndexAIClassifier,
    ) -> Self {
        Self {
            tree,
            config,
            ai_classifier: Some(Arc::new(ai_classifier)),
            classification_cache: Arc::new(RwLock::new(HashMap::new())),
        }
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

    /// Search with optional classification filter
    pub fn search_with_classification(
        &self,
        term: &str,
        classification: Option<ClassificationType>,
    ) -> Result<Vec<IndexResult>, SchemaError> {
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
            return Ok(Vec::new());
        };

        let key = if let Some(ref class) = classification {
            format!("{}:{}", class.prefix(), normalized)
        } else {
            format!("{}{}", WORD_PREFIX, normalized)
        };

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

    /// Index a field value with AI-driven classification (async version)
    pub async fn index_field_value_with_ai(
        &self,
        schema_name: &str,
        field_name: &str,
        key_value: &KeyValue,
        value: &Value,
    ) -> Result<(), SchemaError> {
        if !self.config.enabled || !self.should_index_field(field_name) {
            return Ok(());
        }

        let classification = self.get_or_classify_field(schema_name, field_name).await?;

        // Remove old entries
        let record_key = self.build_record_key(schema_name, field_name, key_value)?;
        self.remove_record_entries(&record_key, schema_name, field_name, key_value)?;

        // Index based on each classification
        let mut all_index_keys = Vec::new();
        
        for class_type in &classification.classifications {
            let strategy = classification
                .get_strategy(class_type)
                .unwrap_or(&SplitStrategy::SplitWords);

            let index_entries = match strategy {
                SplitStrategy::KeepWhole => {
                    Self::process_keep_whole(class_type, value)?
                }
                SplitStrategy::SplitWords => {
                    self.process_split_words(class_type, value)?
                }
                SplitStrategy::ExtractEntities => {
                    self.process_extract_entities(class_type, value, &classification).await?
                }
            };

            for (index_key, normalized_value) in index_entries {
                self.add_to_index(
                    &index_key,
                    schema_name,
                    field_name,
                    key_value,
                    value,
                    Some(json!({
                        "classification": class_type.prefix(),
                        "normalized": normalized_value
                    })),
                )?;
                all_index_keys.push(index_key);
            }
        }

        // Store the index keys for this record
        if !all_index_keys.is_empty() {
            self.store_record_words(&record_key, &all_index_keys)?;
        }

        // Note: flush is now handled by the caller to avoid flushing on every field operation
        Ok(())
    }

    /// Get cached classification or classify using AI
    async fn get_or_classify_field(
        &self,
        schema_name: &str,
        field_name: &str,
    ) -> Result<FieldClassification, SchemaError> {
        let cache_key = ClassificationCacheKey::new(
            schema_name.to_string(),
            field_name.to_string(),
        );

        // Check cache first
        {
            let cache = self.classification_cache.read().await;
            if let Some(classification) = cache.get(&cache_key) {
                return Ok(classification.clone());
            }
        }

        // If no AI classifier, use word-only fallback
        let Some(ai_classifier) = &self.ai_classifier else {
            return Ok(FieldClassification::word_only(field_name.to_string()));
        };

        // TODO: Collect sample values from database
        // For now, we'll use an empty sample set
        let request = super::native_index_classification::ClassificationRequest {
            schema_name: schema_name.to_string(),
            field_name: field_name.to_string(),
            sample_values: Vec::new(),
        };

        let classification = ai_classifier.classify_field(request).await?;

        // Cache the result
        {
            let mut cache = self.classification_cache.write().await;
            cache.insert(cache_key, classification.clone());
        }

        Ok(classification)
    }

    fn process_keep_whole(
        classification: &ClassificationType,
        value: &Value,
    ) -> Result<Vec<(String, String)>, SchemaError> {
        let mut results = Vec::new();
        
        match value {
            Value::String(text) => {
                let normalized = text.trim().to_ascii_lowercase();
                if !normalized.is_empty() {
                    let key = format!("{}:{}", classification.prefix(), normalized);
                    results.push((key, normalized));
                }
            }
            Value::Array(values) => {
                for item in values {
                    results.extend(Self::process_keep_whole(classification, item)?);
                }
            }
            Value::Object(obj) => {
                // For objects, recursively process all string values
                for (_, nested_value) in obj {
                    results.extend(Self::process_keep_whole(classification, nested_value)?);
                }
            }
            _ => {}
        }

        Ok(results)
    }

    fn process_split_words(
        &self,
        classification: &ClassificationType,
        value: &Value,
    ) -> Result<Vec<(String, String)>, SchemaError> {
        let words = self.collect_words(value);
        let results = words
            .into_iter()
            .map(|word| {
                let key = format!("{}:{}", classification.prefix(), word);
                (key, word.clone())
            })
            .collect();
        Ok(results)
    }

    async fn process_extract_entities(
        &self,
        classification: &ClassificationType,
        value: &Value,
        field_classification: &FieldClassification,
    ) -> Result<Vec<(String, String)>, SchemaError> {
        let mut results = Vec::new();

        // Use pre-extracted entities if available
        for entity in &field_classification.entities {
            if &entity.classification == classification {
                let normalized = entity.value.trim().to_ascii_lowercase();
                let key = format!("{}:{}", classification.prefix(), normalized);
                results.push((key, normalized));
            }
        }

        // If no pre-extracted entities, try to extract from value
        if results.is_empty() && self.ai_classifier.is_some() {
            if let Value::String(text) = value {
                let ai_classifier = self.ai_classifier.as_ref().unwrap();
                let entities = ai_classifier
                    .extract_entities_from_value(text, classification)
                    .await?;

                for entity in entities {
                    let normalized = entity.value.trim().to_ascii_lowercase();
                    let key = format!("{}:{}", classification.prefix(), normalized);
                    results.push((key, normalized));
                }
            }
        }

        Ok(results)
    }

    fn add_to_index(
        &self,
        index_key: &str,
        schema_name: &str,
        field_name: &str,
        key_value: &KeyValue,
        value: &Value,
        metadata: Option<Value>,
    ) -> Result<(), SchemaError> {
        let mut entries = self.read_entries(index_key)?;
        
        // Remove duplicates
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
            metadata,
        };

        entries.push(index_entry);
        self.write_entries(index_key, &entries)?;
        Ok(())
    }

    /// Index a field value with classifications from schema topology
    pub fn index_field_value_with_classifications(
        &self,
        schema_name: &str,
        field_name: &str,
        key_value: &KeyValue,
        value: &Value,
        classifications: Option<Vec<String>>,
    ) -> Result<(), SchemaError> {
        if !self.config.enabled || !self.should_index_field(field_name) {
            return Ok(());
        }

        // If no classifications provided, fall back to word-only indexing
        let classifications = classifications.unwrap_or_else(|| vec!["word".to_string()]);
        
        // Indexing field with classifications (logging removed for performance)

        let record_key = self.build_record_key(schema_name, field_name, key_value)?;
        self.remove_record_entries(&record_key, schema_name, field_name, key_value)?;

        let mut all_index_keys = Vec::new();

        for classification_str in &classifications {
            let index_entries = if classification_str == "word" {
                // Split into words
                let words = self.collect_words(value);
                words.into_iter().map(|w| (format!("word:{}", w), w)).collect()
            } else if classification_str.starts_with("hashtag") {
                // Keep hashtags whole (including the #)
                self.extract_hashtags(value)
            } else if classification_str.starts_with("email") {
                // Keep emails whole
                self.extract_emails(value)
            } else if classification_str.starts_with("name:") || classification_str.starts_with("username") || classification_str.starts_with("phone") || classification_str.starts_with("url") || classification_str.starts_with("date") {
                // Keep these whole (normalized)
                self.extract_whole_values(classification_str, value)
            } else {
                // Default: treat as word
                let words = self.collect_words(value);
                words.into_iter().map(|w| (format!("word:{}", w), w)).collect()
            };

            for (index_key, normalized_value) in index_entries {
                // Index creation logging removed for performance
                
                self.add_to_index(
                    &index_key,
                    schema_name,
                    field_name,
                    key_value,
                    value,
                    Some(json!({
                        "classification": classification_str,
                        "normalized": normalized_value
                    })),
                )?;
                all_index_keys.push(index_key);
            }
        }

        if !all_index_keys.is_empty() {
            self.store_record_words(&record_key, &all_index_keys)?;
        }

        // Note: flush is now handled by the caller (batch mutation manager)
        // to avoid flushing on every field operation
        Ok(())
    }

    /// Legacy method: Index a field value (word-only, for backward compatibility)
    pub fn index_field_value(
        &self,
        schema_name: &str,
        field_name: &str,
        key_value: &KeyValue,
        value: &Value,
    ) -> Result<(), SchemaError> {
        self.index_field_value_with_classifications(
            schema_name,
            field_name,
            key_value,
            value,
            None, // No classifications = word-only
        )
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

    // ========== BATCH INDEX OPERATIONS ==========

    /// Batch index multiple field values with classifications
    pub fn batch_index_field_values_with_classifications(
        &self,
        index_operations: &[(String, String, KeyValue, Value, Option<Vec<String>>)], // (schema_name, field_name, key_value, value, classifications)
    ) -> Result<(), SchemaError> {
        if !self.config.enabled {
            return Ok(());
        }

        let mut batch_operations = Vec::new();

        for (schema_name, field_name, key_value, value, classifications) in index_operations {
            if !self.should_index_field(field_name) {
                continue;
            }

            let classifications = classifications.clone().unwrap_or_else(|| vec!["word".to_string()]);
            let record_key = self.build_record_key(schema_name, field_name, key_value)?;

            // Remove old entries
            self.remove_record_entries(&record_key, schema_name, field_name, key_value)?;

            let mut all_index_keys = Vec::new();

            for classification_str in &classifications {
                let index_entries = if classification_str == "word" {
                    // Split into words
                    let words = self.collect_words(value);
                    words.into_iter().map(|w| (format!("word:{}", w), w)).collect()
                } else if classification_str.starts_with("hashtag") {
                    // Keep hashtags whole (including the #)
                    self.extract_hashtags(value)
                } else if classification_str.starts_with("email") {
                    // Keep emails whole
                    self.extract_emails(value)
                } else if classification_str.starts_with("name:") || classification_str.starts_with("username") || classification_str.starts_with("phone") || classification_str.starts_with("url") || classification_str.starts_with("date") {
                    // Keep these whole (normalized)
                    self.extract_whole_values(classification_str, value)
                } else {
                    // Default: treat as word
                    let words = self.collect_words(value);
                    words.into_iter().map(|w| (format!("word:{}", w), w)).collect()
                };

                for (index_key, normalized_value) in index_entries {
                    let index_entry = IndexResult {
                        schema_name: schema_name.clone(),
                        field: field_name.clone(),
                        key_value: key_value.clone(),
                        value: value.clone(),
                        metadata: Some(json!({
                            "classification": classification_str,
                            "normalized": normalized_value
                        })),
                    };

                    batch_operations.push((index_key.clone(), serde_json::to_value(index_entry).map_err(|e| SchemaError::InvalidData(format!("Serialization failed: {}", e)))?));
                    all_index_keys.push(index_key.clone());
                }
            }

            if !all_index_keys.is_empty() {
                batch_operations.push((record_key.clone(), serde_json::Value::Array(
                    all_index_keys.into_iter().map(|k| serde_json::Value::String(k)).collect()
                )));
            }
        }

        // Batch execute all index operations
        self.batch_execute_index_operations(&batch_operations)?;

        Ok(())
    }

    /// Batch execute index operations using sled's batch API
    fn batch_execute_index_operations(
        &self,
        operations: &[(String, serde_json::Value)],
    ) -> Result<(), SchemaError> {
        let mut batch = sled::Batch::default();

        for (key, value) in operations {
            let bytes = serde_json::to_vec(value)
                .map_err(|e| SchemaError::InvalidData(format!("Serialization failed: {}", e)))?;
            batch.insert(key.as_bytes(), bytes);
        }

        self.tree.apply_batch(batch)
            .map_err(|e| SchemaError::InvalidData(format!("Batch apply failed: {}", e)))?;

        Ok(())
    }
}
