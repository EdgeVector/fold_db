use crate::schema::types::key_value::KeyValue;
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
use std::time::{SystemTime, UNIX_EPOCH};

pub(super) const EXCLUDED_FIELDS: &[&str] = &["uuid", "id", "password", "token"];

/// Index entry prefix for index storage
pub(super) const INDEX_ENTRY_PREFIX: &str = "idx:";

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
