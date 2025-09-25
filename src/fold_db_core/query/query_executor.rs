//! Query Executor
//!
//! Main query execution logic extracted from FoldDB core, handling all query types
//! including HashRange schemas with proper delegation to specialized processors.

use crate::db_operations::DbOperations;
use crate::schema::types::Query;
use crate::schema::SchemaCore;
use crate::schema::SchemaError;
use serde_json::Value;
use std::sync::Arc;

use super::hash_range_query::HashRangeQueryProcessor;

/// Main query executor that handles all query operations
pub struct QueryExecutor {
    schema_manager: Arc<SchemaCore>,
    hash_range_processor: HashRangeQueryProcessor,
}

impl QueryExecutor {
    /// Create a new query executor
    pub fn new(
        db_ops: Arc<DbOperations>,
        schema_manager: Arc<SchemaCore>,
    ) -> Self {
        let hash_range_processor = HashRangeQueryProcessor::new(Arc::clone(&db_ops));

        Self {
            schema_manager,
            hash_range_processor,
        }
    }

    /// Query multiple fields from a schema
    pub fn query(&self, query: Query) -> Result<Value, SchemaError> {
        let mut schema = self.schema_manager.get_schema(&query.schema_name)?.ok_or_else(|| SchemaError::InvalidData(format!("Schema '{}' not found", query.schema_name)))?;
        let result = self.hash_range_processor.query_with_filter(
            &mut schema,
            &query.fields,
            query.filter,
        )?;
        Ok(Value::Object(result.into_iter().collect()))
    }
}
