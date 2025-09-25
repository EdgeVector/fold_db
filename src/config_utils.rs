//! Configuration utilities for eliminating duplicate initialization patterns

use serde_json::{json, Value as JsonValue};
use std::collections::HashMap;

/// Factory for creating standardized configuration objects
pub struct ConfigFactory;

impl ConfigFactory {
    /// Create empty metadata HashMap
    pub fn empty_metadata() -> HashMap<String, String> {
        HashMap::new()
    }

    /// Create empty string to JsonValue HashMap - common in mutations/queries
    pub fn empty_json_map() -> HashMap<String, JsonValue> {
        HashMap::new()
    }

    /// Create metadata with single entry
    pub fn single_metadata_entry(key: &str, value: &str) -> HashMap<String, String> {
        let mut metadata = HashMap::new();
        metadata.insert(key.to_string(), value.to_string());
        metadata
    }

    /// Create standard test metadata
    pub fn test_metadata() -> HashMap<String, String> {
        let mut metadata = HashMap::new();
        metadata.insert("test".to_string(), "true".to_string());
        metadata.insert("source".to_string(), "automated_test".to_string());
        metadata
    }

    /// Create standard mutation fields
    pub fn standard_mutation_fields() -> HashMap<String, JsonValue> {
        let mut fields = HashMap::new();
        fields.insert("name".to_string(), json!("Test User"));
        fields.insert("email".to_string(), json!("test@example.com"));
        fields.insert(
            "created_at".to_string(),
            json!(chrono::Utc::now().to_rfc3339()),
        );
        fields
    }
}

/// Builder for complex configuration scenarios
pub struct ConfigBuilder<T> {
    map: HashMap<String, T>,
}

impl<T> ConfigBuilder<T> {
    /// Create new config builder
    pub fn new() -> Self {
        Self {
            map: HashMap::new(),
        }
    }

    /// Add entry to configuration
    pub fn with_entry(mut self, key: String, value: T) -> Self {
        self.map.insert(key, value);
        self
    }

    /// Add entry with string key
    pub fn with_str_key(mut self, key: &str, value: T) -> Self {
        self.map.insert(key.to_string(), value);
        self
    }

    /// Build the final HashMap
    pub fn build(self) -> HashMap<String, T> {
        self.map
    }
}

impl<T> Default for ConfigBuilder<T> {
    fn default() -> Self {
        Self::new()
    }
}

/// Macro for creating field maps with default configurations
#[macro_export]
macro_rules! field_map {
    ($($field_name:expr => $field_value:expr),* $(,)?) => {
        {
            let mut fields = $crate::config_utils::ConfigFactory::empty_json_map();
            $(
                fields.insert($field_name.to_string(), $field_value);
            )*
            fields
        }
    };
}
