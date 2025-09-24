//! Utility functions for the mutation service.
//!
//! This module contains helper functions used throughout the mutation service
//! for context summarization, field sorting, and other common operations.

use serde_json::{Map, Value};
use crate::fold_db_core::services::mutation::types::NormalizedFieldContext;

/// Summarizes a normalized context for logging purposes
pub fn summarize_normalized_context(context: &NormalizedFieldContext) -> String {
    let hash_state = context
        .hash
        .as_ref()
        .filter(|value| !value.trim().is_empty())
        .map(|_| "present")
        .unwrap_or("missing");
    let range_state = context
        .range
        .as_ref()
        .filter(|value| !value.trim().is_empty())
        .map(|_| "present")
        .unwrap_or("missing");
    let mut field_names: Vec<&str> = context.fields.keys().map(|key| key.as_str()).collect();
    field_names.sort_unstable();
    let fields_summary = if field_names.is_empty() {
        "none".to_string()
    } else {
        field_names.join(", ")
    };

    format!(
        "hash:{}, range:{}, fields:[{}], count={}",
        hash_state,
        range_state,
        fields_summary,
        field_names.len()
    )
}

/// Sets a value in a target map
pub fn set_value(target: &mut Map<String, Value>, key: &str, value: &Value) {
    target.insert(key.to_string(), value.clone());
}

/// Sorts fields in a map for consistent ordering
pub fn sort_fields(fields: &Map<String, Value>) -> Map<String, Value> {
    let mut sorted_fields = Map::new();
    let mut keys: Vec<String> = fields.keys().cloned().collect();
    keys.sort_unstable();
    for key in keys {
        if let Some(value) = fields.get(&key) {
            sorted_fields.insert(key, value.clone());
        }
    }
    sorted_fields
}

/// Normalizes an optional string value, converting empty strings to None
pub fn normalize_optional_string(value: Option<String>) -> Option<String> {
    value.filter(|s| !s.trim().is_empty())
}
