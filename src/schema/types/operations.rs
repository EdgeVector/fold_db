use crate::schema::types::field::HashRangeFilter;
use chrono::{DateTime, Utc};
#[cfg(feature = "cli")]
use clap::ValueEnum;
use serde::{Deserialize, Serialize};

/// Numeric comparison filters for field values.
///
/// These filters are applied post-fetch on the actual field content (atom values),
/// unlike `HashRangeFilter` which operates on key structure at the molecule level.
/// Multiple `ValueFilter`s on a query are AND'd together.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum ValueFilter {
    /// field value > threshold
    GreaterThan { field: String, value: f64 },
    /// field value < threshold
    LessThan { field: String, value: f64 },
    /// field value == target (exact float equality)
    Equals { field: String, value: f64 },
    /// min <= field value <= max
    Between { field: String, min: f64, max: f64 },
}

impl ValueFilter {
    /// Tests whether the given JSON value satisfies this filter condition.
    /// Returns `false` if the value is not numeric.
    pub fn matches(&self, field_value: &serde_json::Value) -> bool {
        let num = match field_value.as_f64() {
            Some(n) => n,
            None => return false,
        };
        match self {
            ValueFilter::GreaterThan { value, .. } => num > *value,
            ValueFilter::LessThan { value, .. } => num < *value,
            ValueFilter::Equals { value, .. } => (num - *value).abs() < f64::EPSILON,
            ValueFilter::Between { min, max, .. } => num >= *min && num <= *max,
        }
    }

    /// Returns the field name this filter targets.
    pub fn field_name(&self) -> &str {
        match self {
            ValueFilter::GreaterThan { field, .. }
            | ValueFilter::LessThan { field, .. }
            | ValueFilter::Equals { field, .. }
            | ValueFilter::Between { field, .. } => field,
        }
    }
}

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
    /// Post-fetch numeric filters on field values. Multiple filters are AND'd.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub value_filters: Option<Vec<ValueFilter>>,
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
            value_filters: None,
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
            value_filters: None,
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
            _ => Err(serde::de::Error::custom("unknown mutation type, expected one of: create, update, delete")),
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
            value_filters: None,
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

    #[test]
    fn test_value_filter_greater_than() {
        let filter = ValueFilter::GreaterThan {
            field: "price".to_string(),
            value: 500.0,
        };
        assert!(filter.matches(&json!(600.0)));
        assert!(!filter.matches(&json!(500.0)));
        assert!(!filter.matches(&json!(400.0)));
        assert!(!filter.matches(&json!("not a number")));
    }

    #[test]
    fn test_value_filter_less_than() {
        let filter = ValueFilter::LessThan {
            field: "price".to_string(),
            value: 600.0,
        };
        assert!(filter.matches(&json!(500.0)));
        assert!(!filter.matches(&json!(600.0)));
        assert!(!filter.matches(&json!(700.0)));
    }

    #[test]
    fn test_value_filter_equals() {
        let filter = ValueFilter::Equals {
            field: "score".to_string(),
            value: 100.0,
        };
        assert!(filter.matches(&json!(100.0)));
        assert!(filter.matches(&json!(100)));
        assert!(!filter.matches(&json!(99.99)));
    }

    #[test]
    fn test_value_filter_between() {
        let filter = ValueFilter::Between {
            field: "price".to_string(),
            min: 200.0,
            max: 600.0,
        };
        assert!(filter.matches(&json!(200.0)));
        assert!(filter.matches(&json!(400.0)));
        assert!(filter.matches(&json!(600.0)));
        assert!(!filter.matches(&json!(199.99)));
        assert!(!filter.matches(&json!(600.01)));
    }

    #[test]
    fn test_value_filter_field_name() {
        assert_eq!(
            ValueFilter::GreaterThan {
                field: "price".to_string(),
                value: 0.0
            }
            .field_name(),
            "price"
        );
        assert_eq!(
            ValueFilter::Between {
                field: "score".to_string(),
                min: 0.0,
                max: 100.0
            }
            .field_name(),
            "score"
        );
    }

    #[test]
    fn test_value_filter_serde_round_trip() {
        let filters = vec![
            ValueFilter::LessThan {
                field: "price".to_string(),
                value: 600.0,
            },
            ValueFilter::GreaterThan {
                field: "rating".to_string(),
                value: 3.0,
            },
        ];
        let json = serde_json::to_value(&filters).unwrap();
        let deserialized: Vec<ValueFilter> = serde_json::from_value(json).unwrap();
        assert_eq!(deserialized, filters);
    }

    #[test]
    fn test_query_with_value_filters_round_trip() {
        let json = json!({
            "schema_name": "Flight",
            "fields": ["airline", "price"],
            "filter": null,
            "value_filters": [
                {"LessThan": {"field": "price", "value": 600}},
                {"GreaterThan": {"field": "price", "value": 100}}
            ]
        });
        let query: Query = serde_json::from_value(json).unwrap();
        assert_eq!(query.value_filters.as_ref().unwrap().len(), 2);

        let serialized = serde_json::to_value(&query).unwrap();
        assert!(serialized.get("value_filters").is_some());
    }

    #[test]
    fn test_query_value_filters_none_by_default() {
        let query = Query::new("Test".to_string(), vec![]);
        assert!(query.value_filters.is_none());
        let json = serde_json::to_value(&query).unwrap();
        assert!(json.get("value_filters").is_none());
    }
}
