//! HashRange Query Processor
//!
//! Handles specialized query processing for HashRange schemas with proper grouping
//! by hash_key -> range_key -> fields structure.

use crate::db_operations::DbOperations;
use crate::schema::{Schema, SchemaError};
use log::info;
use serde_json::{json, Value};
use std::sync::Arc;

/// Processor for HashRange schema queries with proper grouping
pub struct HashRangeQueryProcessor {
    db_ops: Arc<DbOperations>,
}

impl HashRangeQueryProcessor {
    /// Create a new HashRange query processor
    pub fn new(db_ops: Arc<DbOperations>) -> Self {
        Self { db_ops }
    }

    /// Get the hash and range field names from the schema's universal key configuration
    fn get_key_field_names(&self, schema: &Schema) -> Result<(String, String), SchemaError> {
        // For HashRange schemas, both hash_field and range_field are required
        let key_config = schema.key.as_ref().ok_or_else(|| {
            SchemaError::InvalidData(format!(
                "HashRange schema '{}' requires key configuration",
                schema.name
            ))
        })?;

        let hash_field = if key_config.hash_field.trim().is_empty() {
            return Err(SchemaError::InvalidData(format!(
                "HashRange schema '{}' requires non-empty hash_field in key configuration",
                schema.name
            )));
        } else {
            key_config.hash_field.clone()
        };

        let range_field = if key_config.range_field.trim().is_empty() {
            return Err(SchemaError::InvalidData(format!(
                "HashRange schema '{}' requires non-empty range_field in key configuration",
                schema.name
            )));
        } else {
            key_config.range_field.clone()
        };

        info!(
            "🔑 HashRange schema '{}' key fields - hash: '{}', range: '{}'",
            schema.name, hash_field, range_field
        );

        Ok((hash_field, range_field))
    }

    /// Fetch the first 10 hash keys and their associated data for a HashRange schema
    fn fetch_first_10_hash_keys(
        &self,
        schema: &Schema,
        fields: &[String],
    ) -> Result<Value, SchemaError> {
        info!(
            "🔍 Fetching first 10 hash keys for schema '{}' with hash->range->fields format",
            schema.name
        );

        // Use the first field to find hash keys (all fields should have the same hash keys)
        let first_field = fields.first().ok_or_else(|| {
            SchemaError::InvalidData("No fields specified for HashRange query".to_string())
        })?;

        // Create prefix to search for: {schema_name}_{field_name}_
        let prefix = format!("{}_{}_", schema.name, first_field);
        info!("🔍 Searching for hash keys with prefix: '{}'", prefix);

        // Get all keys with this prefix
        let all_keys = self.db_ops.list_items_with_prefix(&prefix)?;
        info!("🔍 Found {} keys with prefix '{}'", all_keys.len(), prefix);

        // Extract unique hash keys from the keys
        let mut hash_keys = std::collections::HashSet::new();
        for key in all_keys {
            // Key format: {schema_name}_{field_name}_{hash_key}
            // Extract hash_key by removing the prefix
            if let Some(hash_key) = key.strip_prefix(&prefix) {
                hash_keys.insert(hash_key.to_string());
            }
        }

        // Convert to sorted vector and take first 10
        let mut sorted_hash_keys: Vec<String> = hash_keys.into_iter().collect();
        sorted_hash_keys.sort();
        let selected_hash_keys: Vec<String> = sorted_hash_keys.into_iter().take(10).collect();

        info!(
            "🔍 Selected {} hash keys: {:?}",
            selected_hash_keys.len(),
            selected_hash_keys
        );

        // For each selected hash key, fetch all field data and restructure to hash->range->fields
        let mut result_data = serde_json::Map::new();

        for hash_key in selected_hash_keys {
            let range_data = self.fetch_range_data_for_hash_key(schema, fields, &hash_key)?;
            result_data.insert(hash_key, json!(range_data));
        }

        info!(
            "🔍 Returning {} hash keys with their data in hash->range->fields format",
            result_data.len()
        );
        Ok(json!(result_data))
    }

    /// Fetch range data for a specific hash key and restructure to hash->range->fields format
    fn fetch_range_data_for_hash_key(
        &self,
        schema: &Schema,
        fields: &[String],
        hash_key: &str,
    ) -> Result<serde_json::Map<String, Value>, SchemaError> {
        let mut range_data = serde_json::Map::new();

        for field_name in fields {
            // Construct the key for the HashRange field: schema_name_field_name_hash_key
            let field_key = format!("{}_{}_{}", schema.name, field_name, hash_key);

            // Retrieve the BTree for this hash_key and field
            match self.db_ops.get_item::<String>(&field_key)? {
                Some(btree_json) => {
                    if let Ok(btree_data) =
                        serde_json::from_str::<serde_json::Map<String, Value>>(&btree_json)
                    {
                        // For each range_key, add the field value to the range_data
                        for (range_key, value) in &btree_data {
                            if let Some(range_obj) = range_data.get_mut(range_key) {
                                if let Some(range_map) = range_obj.as_object_mut() {
                                    range_map.insert(field_name.clone(), value.clone());
                                }
                            } else {
                                let mut range_map = serde_json::Map::new();
                                range_map.insert(field_name.clone(), value.clone());
                                range_data.insert(range_key.clone(), json!(range_map));
                            }
                        }
                    }
                }
                None => {
                    // No data for this field, continue
                }
            }
        }

        Ok(range_data)
    }

    /// Query HashRange schema with proper grouping by hash_key -> range_key -> fields
    pub fn query_hashrange_schema(
        &self,
        schema: &Schema,
        fields: &[String],
        hash_key_filter: Option<Value>,
    ) -> Result<Value, SchemaError> {
        info!(
            "🔑 Querying HashRange schema '{}' with hash->range->fields grouping",
            schema.name
        );

        // Validate schema has proper key configuration
        self.get_key_field_names(schema)?;

        if let Some(hash_filter) = &hash_key_filter {
            // Query specific hash key
            let hash_key = if let Some(key_obj) = hash_filter.as_object() {
                if let Some(key_value) = key_obj.get("Key") {
                    if let Some(key_str) = key_value.as_str() {
                        key_str.to_string()
                    } else {
                        return Err(SchemaError::InvalidData(
                            "Hash filter Key must be a string".to_string(),
                        ));
                    }
                } else {
                    return Err(SchemaError::InvalidData(
                        "Hash filter must contain 'Key' field".to_string(),
                    ));
                }
            } else {
                return Err(SchemaError::InvalidData(
                    "Hash filter must be an object".to_string(),
                ));
            };

            info!("🔍 HashRange query for hash key: '{}'", hash_key);

            // Fetch range data for the specific hash key
            let range_data = self.fetch_range_data_for_hash_key(schema, fields, &hash_key)?;

            // Create the final result structure: {hash_key: {range_key: {fields}}}
            let mut result = serde_json::Map::new();
            result.insert(hash_key, json!(range_data));

            Ok(json!(result))
        } else {
            // No hash_key_filter provided - fetch first 10 hash keys and their data
            info!("🔍 No hash_key_filter provided - fetching first 10 hash keys");
            self.fetch_first_10_hash_keys(schema, fields)
        }
    }
}
