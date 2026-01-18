use crate::schema::types::field::HashRangeFilter;
use clap::ValueEnum;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Query {
    pub schema_name: String,
    pub fields: Vec<String>,
    pub filter: Option<HashRangeFilter>,
}

impl Query {
    #[must_use]
    pub fn new(schema_name: String, fields: Vec<String>) -> Self {
        Self {
            schema_name,
            fields,
            filter: None,
        }
    }

    #[must_use]
    pub fn new_with_filter(
        schema_name: String,
        fields: Vec<String>,
        filter: Option<HashRangeFilter>,
    ) -> Self {
        Self {
            schema_name,
            fields,
            filter,
        }
    }
}

#[derive(Debug, Clone, Serialize, ValueEnum, PartialEq)]
pub enum MutationType {
    Create,
    Update,
    Delete,
}

impl<'de> Deserialize<'de> for MutationType {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        let s = String::deserialize(deserializer)?;
        match s.to_lowercase().as_str() {
            "create" => Ok(MutationType::Create),
            "update" => Ok(MutationType::Update),
            "delete" => Ok(MutationType::Delete),
            _ => Err(serde::de::Error::custom("unknown mutation type")),
        }
    }
}

// Re-export Mutation from the dedicated mutation module
pub use super::mutation::Mutation;

use crate::schema::types::key_value::KeyValue;
use serde_json::Value;
use std::collections::HashMap;

/// Represents an operation that can be performed on the database
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type", deny_unknown_fields)]
pub enum Operation {
    #[serde(rename = "mutation")]
    Mutation {
        schema: String,
        fields_and_values: HashMap<String, Value>,
        key_value: KeyValue,
        mutation_type: MutationType,
        #[serde(skip_serializing_if = "Option::is_none")]
        source_file_name: Option<String>,
    },
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn test_operation_mutation_preserves_source_file_name() {
        let operation = Operation::Mutation {
            schema: "TestSchema".to_string(),
            fields_and_values: HashMap::new(),
            key_value: KeyValue::new(None, None),
            mutation_type: MutationType::Create,
            source_file_name: Some("test_file.json".to_string()),
        };

        // Serialize to JSON
        let json = serde_json::to_value(&operation).unwrap();

        // Verify source_file_name is in the JSON
        assert_eq!(json["source_file_name"], json!("test_file.json"));

        // Deserialize back
        let deserialized: Operation = serde_json::from_value(json).unwrap();

        // Verify source_file_name is preserved
        match deserialized {
            Operation::Mutation {
                source_file_name, ..
            } => {
                assert_eq!(source_file_name, Some("test_file.json".to_string()));
            }
        }
    }

    #[test]
    fn test_operation_mutation_without_source_file_name() {
        let operation = Operation::Mutation {
            schema: "TestSchema".to_string(),
            fields_and_values: HashMap::new(),
            key_value: KeyValue::new(None, None),
            mutation_type: MutationType::Create,
            source_file_name: None,
        };

        // Serialize to JSON
        let json = serde_json::to_value(&operation).unwrap();

        // Verify source_file_name is NOT in the JSON (due to skip_serializing_if)
        assert!(json.get("source_file_name").is_none());

        // Deserialize back
        let deserialized: Operation = serde_json::from_value(json).unwrap();

        // Verify source_file_name is None
        match deserialized {
            Operation::Mutation {
                source_file_name, ..
            } => {
                assert_eq!(source_file_name, None);
            }
        }
    }
}
