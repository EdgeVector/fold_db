use crate::schema::types::field::HashRangeFilter;
use chrono::{DateTime, Utc};
#[cfg(feature = "cli")]
use clap::ValueEnum;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, PartialEq)]
pub enum SortOrder {
    Asc,
    Desc,
}

impl Serialize for SortOrder {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        match self {
            SortOrder::Asc => serializer.serialize_str("asc"),
            SortOrder::Desc => serializer.serialize_str("desc"),
        }
    }
}

impl<'de> Deserialize<'de> for SortOrder {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        let s = String::deserialize(deserializer)?;
        match s.to_lowercase().as_str() {
            "asc" => Ok(SortOrder::Asc),
            "desc" => Ok(SortOrder::Desc),
            _ => Err(serde::de::Error::custom(format!(
                "unknown sort order '{}', expected 'asc' or 'desc'",
                s
            ))),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Query {
    pub schema_name: String,
    pub fields: Vec<String>,
    pub filter: Option<HashRangeFilter>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub as_of: Option<DateTime<Utc>>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub rehydrate_depth: Option<u32>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub sort_order: Option<SortOrder>,
}

impl Query {
    #[must_use]
    pub fn new(schema_name: String, fields: Vec<String>) -> Self {
        Self {
            schema_name,
            fields,
            filter: None,
            as_of: None,
            rehydrate_depth: None,
            sort_order: None,
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
            as_of: None,
            rehydrate_depth: None,
            sort_order: None,
        }
    }
}

#[derive(Debug, Clone, Serialize, PartialEq)]
#[cfg_attr(feature = "cli", derive(ValueEnum))]
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

    #[test]
    fn test_sort_order_round_trip() {
        let query = Query {
            schema_name: "Tweet".to_string(),
            fields: vec!["text".to_string()],
            filter: None,
            as_of: None,
            rehydrate_depth: None,
            sort_order: Some(SortOrder::Desc),
        };

        let json = serde_json::to_value(&query).unwrap();
        assert_eq!(json["sort_order"], json!("desc"));

        let deserialized: Query = serde_json::from_value(json).unwrap();
        assert_eq!(deserialized.sort_order, Some(SortOrder::Desc));
    }

    #[test]
    fn test_sort_order_case_insensitive() {
        let json = json!({
            "schema_name": "Tweet",
            "fields": ["text"],
            "filter": null,
            "sort_order": "ASC"
        });
        let query: Query = serde_json::from_value(json).unwrap();
        assert_eq!(query.sort_order, Some(SortOrder::Asc));
    }

    #[test]
    fn test_sort_order_none_by_default() {
        let json = json!({
            "schema_name": "Tweet",
            "fields": ["text"],
            "filter": null
        });
        let query: Query = serde_json::from_value(json).unwrap();
        assert_eq!(query.sort_order, None);
    }

    #[test]
    fn test_sort_order_skipped_when_none() {
        let query = Query::new("Tweet".to_string(), vec!["text".to_string()]);
        let json = serde_json::to_value(&query).unwrap();
        assert!(json.get("sort_order").is_none());
    }
}
