//! Key value extraction from JSON data based on schema key configuration.
//!
//! Extracts hash and range key values from ingested data, including
//! support for nested field paths and date normalization.

use crate::ingestion::date_handling::try_normalize_date;
use crate::ingestion::IngestionResult;
use crate::log_feature;
use crate::logging::features::LogFeature;
use crate::schema::SchemaCore;
use serde_json::Value;
use std::collections::HashMap;
use std::sync::Arc;

/// Extract key values from JSON data based on schema key fields.
/// Looks up the schema in the node's schema manager to find key configuration,
/// then extracts the corresponding values from the data.
pub(crate) async fn extract_key_values_from_data(
    fields_and_values: &HashMap<String, Value>,
    schema_name: &str,
    schema_manager: &Arc<SchemaCore>,
) -> IngestionResult<HashMap<String, String>> {
    let mut keys_and_values = HashMap::new();

    match schema_manager.get_schema(schema_name) {
        Ok(Some(schema)) => {
            if let Some(key_def) = &schema.key {
                // Extract hash field value if present
                if let Some(hash_field) = &key_def.hash_field {
                    if let Some(hash_value) = fields_and_values.get(hash_field) {
                        if let Some(hash_str) = hash_value.as_str() {
                            keys_and_values.insert("hash_field".to_string(), hash_str.to_string());
                        } else if let Some(hash_num) = hash_value.as_f64() {
                            keys_and_values.insert("hash_field".to_string(), hash_num.to_string());
                        } else {
                            log_feature!(
                                LogFeature::Ingestion,
                                warn,
                                "Hash field '{}' in schema '{}' has unsupported type (not string or number): {:?}",
                                hash_field, schema_name, hash_value
                            );
                        }
                    } else {
                        log_feature!(
                            LogFeature::Ingestion,
                            warn,
                            "Hash field '{}' not found in data for schema '{}'",
                            hash_field, schema_name
                        );
                    }
                }

                // Extract range field value if present, normalizing dates to
                // YYYY-MM-DD HH:MM:SS so records sort chronologically.
                if let Some(range_field) = &key_def.range_field {
                    if let Some(range_value) =
                        extract_nested_field_value(fields_and_values, range_field)
                    {
                        if let Some(range_str) = range_value.as_str() {
                            keys_and_values.insert("range_field".to_string(), try_normalize_date(range_str));
                        } else if let Some(range_num) = range_value.as_f64() {
                            keys_and_values.insert("range_field".to_string(), range_num.to_string());
                        } else {
                            log_feature!(
                                LogFeature::Ingestion,
                                warn,
                                "Range field '{}' in schema '{}' has unsupported type (not string or number): {:?}",
                                range_field, schema_name, range_value
                            );
                        }
                    } else {
                        log_feature!(
                            LogFeature::Ingestion,
                            warn,
                            "Range field '{}' not found in data for schema '{}'",
                            range_field, schema_name
                        );
                    }
                }
            }
        }
        Ok(None) => {
            log_feature!(
                LogFeature::Ingestion,
                warn,
                "Schema '{}' not found — cannot extract key values",
                schema_name
            );
        }
        Err(e) => {
            log_feature!(
                LogFeature::Ingestion,
                error,
                "Failed to get schema '{}' for key extraction: {}",
                schema_name, e
            );
        }
    }

    log_feature!(
        LogFeature::Ingestion,
        info,
        "Extracted key values for schema '{}': {:?}",
        schema_name,
        keys_and_values
    );

    Ok(keys_and_values)
}

/// Extract nested field value from JSON data using dot notation.
pub(crate) fn extract_nested_field_value<'a>(
    fields_and_values: &'a HashMap<String, Value>,
    field_path: &str,
) -> Option<&'a Value> {
    // First try direct field access
    if let Some(value) = fields_and_values.get(field_path) {
        return Some(value);
    }

    // Then try nested field access (e.g., "like.tweetId")
    if field_path.contains('.') {
        let parts: Vec<&str> = field_path.split('.').collect();
        if parts.len() == 2 {
            if let Some(parent_value) = fields_and_values.get(parts[0]) {
                if let Some(parent_obj) = parent_value.as_object() {
                    if let Some(result) = parent_obj.get(parts[1]) {
                        return Some(result);
                    }
                }
            }
        }
    }

    // Try to find the field in nested objects
    for value in fields_and_values.values() {
        if let Some(obj) = value.as_object() {
            if let Some(nested_value) = obj.get(field_path) {
                return Some(nested_value);
            }
        }
    }

    None
}
