//! Query Executor
//! 
//! Main query execution logic extracted from FoldDB core, handling all query types
//! including HashRange schemas with proper delegation to specialized processors.

use crate::schema::{Schema, SchemaError};
use crate::schema::types::Query;
use crate::db_operations::DbOperations;
use crate::permissions::PermissionWrapper;
use crate::schema::SchemaCore;
use serde_json::Value;
use log::info;
use std::sync::Arc;

use super::hash_range_query::HashRangeQueryProcessor;

/// Main query executor that handles all query operations
pub struct QueryExecutor {
    db_ops: Arc<DbOperations>,
    schema_manager: Arc<SchemaCore>,
    permission_wrapper: PermissionWrapper,
    hash_range_processor: HashRangeQueryProcessor,
}

impl QueryExecutor {
    /// Create a new query executor
    pub fn new(
        db_ops: Arc<DbOperations>,
        schema_manager: Arc<SchemaCore>,
        permission_wrapper: PermissionWrapper,
    ) -> Self {
        let hash_range_processor = HashRangeQueryProcessor::new(Arc::clone(&db_ops));
        
        Self {
            db_ops,
            schema_manager,
            permission_wrapper,
            hash_range_processor,
        }
    }

    /// Query multiple fields from a schema
    pub fn query(&self, query: Query) -> Result<Value, SchemaError> {
        info!("🔍 EVENT-DRIVEN query for schema: {}", query.schema_name);
        println!("🔍 DEBUG: Query called for schema: {}", query.schema_name);
        
        // Get schema first
        let schema = match self.schema_manager.get_schema(&query.schema_name)? {
            Some(schema) => schema,
            None => {
                return Err(SchemaError::NotFound(format!(
                    "Schema '{}' not found",
                    query.schema_name
                )));
            }
        };
        
        // Check field-level permissions for each field in the query
        for field_name in &query.fields {
            let permission_result = self.permission_wrapper.check_query_field_permission(
                &query,
                field_name,
                &self.schema_manager,
            );
            
            if !permission_result.allowed {
                return Err(permission_result.error.unwrap_or_else(|| {
                    SchemaError::InvalidData(format!(
                        "Permission denied for field '{}' in schema '{}' with trust distance {}",
                        field_name, query.schema_name, query.trust_distance
                    ))
                }));
            }
        }
        
        // Extract range key filter if this is a range schema with a filter
        let range_key_filter: Option<Value> = if let (Some(range_key), Some(filter)) = (schema.range_key(), &query.filter) {
            if let Some(range_filter_obj) = filter.get("range_filter") {
                if let Some(range_filter_map) = range_filter_obj.as_object() {
                    range_filter_map.get(range_key).cloned()
                } else {
                    None
                }
            } else {
                None
            }
        } else {
            None
        };

        // Extract hash key filter if this is a HashRange schema with a filter
        let hash_key_filter: Option<Value> = if matches!(schema.schema_type, crate::schema::types::SchemaType::HashRange) {
            println!("🔍 DEBUG: Schema '{}' is HashRange type", schema.name);
            if let Some(filter) = &query.filter {
                println!("🔍 DEBUG: Query has filter: {:?}", filter);
                // Check for both hash_filter and hash_key formats
                if let Some(hash_filter_obj) = filter.get("hash_filter") {
                    println!("🔑 HashRange schema detected with hash_filter: {:?}", hash_filter_obj);
                    Some(hash_filter_obj.clone())
                } else if let Some(hash_key_value) = filter.get("hash_key") {
                    println!("🔑 HashRange schema detected with hash_key: {:?}", hash_key_value);
                    // Convert hash_key format to hash_filter format for compatibility
                    Some(serde_json::json!({"Key": hash_key_value}))
                } else {
                    println!("🔍 DEBUG: No hash_filter or hash_key found in query filter");
                    None
                }
            } else {
                println!("🔍 DEBUG: Query has no filter");
                None
            }
        } else {
            println!("🔍 DEBUG: Schema '{}' is not HashRange type: {:?}", schema.name, schema.schema_type);
            None
        };

        // Handle HashRange schema grouping
        if matches!(schema.schema_type, crate::schema::types::SchemaType::HashRange) {
            return self.hash_range_processor.query_hashrange_schema(&schema, &query.fields, hash_key_filter);
        }
        
        // Retrieve actual field values by accessing database directly
        let mut field_values = serde_json::Map::new();
        
        for field_name in &query.fields {
            println!("🔍 DEBUG: Retrieving field '{}' for schema '{}'", field_name, schema.name);
            match self.get_field_value_from_db(&schema, field_name, range_key_filter.clone(), hash_key_filter.clone()) {
                Ok(value) => {
                    println!("✅ DEBUG: Retrieved field '{}' value: {}", field_name, value);
                    field_values.insert(field_name.clone(), value);
                }
                Err(e) => {
                    println!("❌ DEBUG: Failed to retrieve field '{}': {}", field_name, e);
                    field_values.insert(field_name.clone(), serde_json::Value::Null);
                }
            }
        }
        
        // Return actual field values
        Ok(serde_json::Value::Object(field_values))
    }

    /// Query schema (compatibility method)
    pub fn query_schema(&self, query: Query) -> Vec<Result<Value, SchemaError>> {
        println!("🔍 DEBUG: query_schema called for schema: {}", query.schema_name);
        // Delegate to the main query method and wrap in Vec
        vec![self.query(query)]
    }


    /// Get field value directly from database using unified resolver
    fn get_field_value_from_db(&self, schema: &Schema, field_name: &str, range_key_filter: Option<Value>, hash_key_filter: Option<Value>) -> Result<Value, SchemaError> {
        // Use the unified FieldValueResolver to eliminate duplicate code
        crate::fold_db_core::transform_manager::utils::TransformUtils::resolve_field_value(&self.db_ops, schema, field_name, range_key_filter, hash_key_filter)
    }
}
