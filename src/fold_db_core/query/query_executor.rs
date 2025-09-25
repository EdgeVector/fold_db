//! Query Executor
//!
//! Main query execution logic extracted from FoldDB core, handling all query types
//! including HashRange schemas with proper delegation to specialized processors.

use crate::db_operations::DbOperations;
use crate::permissions::PermissionWrapper;
use crate::schema::types::Query;
use crate::schema::SchemaCore;
use crate::schema::{Schema, SchemaError};
use log::info;
use serde_json::Value;
use std::sync::Arc;

use super::hash_range_query::HashRangeQueryProcessor;
use crate::schema::types::field::HashRangeFilter;
use crate::transform::manager::utils::TransformUtils;
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

        // Get schema first
        let mut schema = match self.schema_manager.get_schema(&query.schema_name)? {
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

        // Extract and combine all filters into a unified HashRangeFilter for all schema types
        let unified_filter: Option<HashRangeFilter> = if let Some(filter) = &query.filter {
            let hash_filter_value = filter.get("hash_filter").cloned();
            let range_filter_value = filter.get("range_filter").cloned();
            
            let combined_filter = HashRangeFilter::from_json_values(hash_filter_value, range_filter_value);
            
            if combined_filter.is_some() {
                info!("🔑 Schema with unified filter created");
            } else {
                info!("🔑 Schema with no filters");
            }
            
            combined_filter
        } else {
            None
        };

        // Handle HashRange schema grouping
        if matches!(
            schema.schema_type,
            crate::schema::types::SchemaType::HashRange { .. }
        ) {
            return self.hash_range_processor.query_hashrange_schema(
                &schema,
                &query.fields,
                unified_filter,
            );
        }

        // Retrieve actual field values by accessing database directly
        let mut field_values = serde_json::Map::new();

        for field_name in &query.fields {
            println!(
                "🔍 DEBUG: Retrieving field '{}' for schema '{}'",
                field_name, schema.name
            );
            match self.get_field_value_from_db(
                &mut schema,
                field_name,
                unified_filter.clone(),
            ) {
                Ok(value) => {
                    println!(
                        "✅ DEBUG: Retrieved field '{}' value: {}",
                        field_name, value
                    );
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
        println!(
            "🔍 DEBUG: query_schema called for schema: {}",
            query.schema_name
        );
        // Delegate to the main query method and wrap in Vec
        vec![self.query(query)]
    }

    /// Get field value directly from database using unified resolver
    fn get_field_value_from_db(
        &self,
        schema: &mut Schema,
        field_name: &str,
        unified_filter: Option<HashRangeFilter>,
    ) -> Result<Value, SchemaError> {
        // Use the unified FieldValueResolver to eliminate duplicate code
        TransformUtils::resolve_field_value(
            &self.db_ops,
            schema,
            field_name,
            unified_filter,
        )
    }

}
