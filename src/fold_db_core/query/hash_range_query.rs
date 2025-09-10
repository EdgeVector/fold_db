//! HashRange Query Processor
//! 
//! Handles specialized query processing for HashRange schemas with proper grouping
//! by hash_key -> range_key -> fields structure.

use crate::schema::{Schema, SchemaError};
use crate::db_operations::DbOperations;
use serde_json::{Value, json};
use log::info;
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

    /// Query HashRange schema with proper grouping by hash_key -> range_key -> fields
    pub fn query_hashrange_schema(&self, schema: &Schema, fields: &[String], hash_key_filter: Option<Value>) -> Result<Value, SchemaError> {
        info!("🔑 Querying HashRange schema '{}' with grouping", schema.name);
        
        if let Some(hash_filter) = &hash_key_filter {
            // Query specific hash key
            let hash_key = if let Some(key_obj) = hash_filter.as_object() {
                if let Some(key_value) = key_obj.get("Key") {
                    if let Some(key_str) = key_value.as_str() {
                        key_str.to_string()
                    } else {
                        return Err(SchemaError::InvalidData("Hash filter Key must be a string".to_string()));
                    }
                } else {
                    return Err(SchemaError::InvalidData("Hash filter must contain 'Key' field".to_string()));
                }
            } else {
                return Err(SchemaError::InvalidData("Hash filter must be an object".to_string()));
            };
            
            info!("🔍 HashRange query for hash key: '{}'", hash_key);
            
            // Query HashRange data using the new format: {schema_name}_{field_name}_{hash_key}
            let mut field_values = serde_json::Map::new();
            
            for field_name in fields {
                // Construct the key for the HashRange field: schema_name_field_name_hash_key
                let field_key = format!("{}_{}_{}", schema.name, field_name, hash_key);
                println!("🔍 DEBUG: Querying HashRange field key: '{}'", field_key);
                
                // Retrieve the BTree for this hash_key and field
                match self.db_ops.get_item::<String>(&field_key)? {
                    Some(btree_json) => {
                        println!("🔍 DEBUG: Found data for key '{}': {}", field_key, btree_json);
                        if let Ok(btree_data) = serde_json::from_str::<serde_json::Map<String, Value>>(&btree_json) {
                            // Convert the BTree to an array of range_key -> value pairs
                            let mut range_entries = Vec::new();
                            for (range_key, value) in btree_data {
                                range_entries.push(json!({
                                    "range_key": range_key,
                                    "value": value
                                }));
                            }
                            println!("🔍 DEBUG: Parsed {} range entries for field '{}'", range_entries.len(), field_name);
                            field_values.insert(field_name.clone(), json!(range_entries));
                        } else {
                            println!("🔍 DEBUG: Failed to parse BTree data for field '{}'", field_name);
                            field_values.insert(field_name.clone(), json!([]));
                        }
                    }
                    None => {
                        println!("🔍 DEBUG: No data found for key '{}'", field_key);
                        field_values.insert(field_name.clone(), json!([]));
                    }
                }
            }
            
            Ok(json!(field_values))
        } else {
            // No hash_key_filter provided - return simplified result for now
            info!("🔍 No hash_key_filter provided - returning empty result");
            Ok(json!({}))
        }
    }
}
