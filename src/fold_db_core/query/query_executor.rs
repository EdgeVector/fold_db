//! Query Executor
//!
//! Main query execution logic extracted from FoldDB core, handling all query types
//! including HashRange schemas with proper delegation to specialized processors.

use crate::db_operations::DbOperations;
use crate::schema::types::field::FieldValue;
use crate::schema::types::key_value::KeyValue;
use crate::schema::types::Query;
use crate::schema::SchemaCore;
use crate::schema::SchemaError;
use std::collections::HashMap;
use std::sync::Arc;

use super::hash_range_query::HashRangeQueryProcessor;

/// Main query executor that handles all query operations
pub struct QueryExecutor {
    schema_manager: Arc<SchemaCore>,
    hash_range_processor: HashRangeQueryProcessor,
}

impl QueryExecutor {
    /// Create a new query executor with storage abstraction
    pub fn new(db_ops: Arc<DbOperations>, schema_manager: Arc<SchemaCore>) -> Self {
        let hash_range_processor = HashRangeQueryProcessor::new(Arc::clone(&db_ops));

        Self {
            schema_manager,
            hash_range_processor,
        }
    }

    /// Query multiple fields from a schema
    pub async fn query(
        &self,
        query: Query,
    ) -> Result<HashMap<String, HashMap<KeyValue, FieldValue>>, SchemaError> {
        // query is async, so we can await fetch_schema
        let mut schema = match self.schema_manager.fetch_schema(&query.schema_name).await? {
            Some(s) => s,
            None => {
                let available = self.schema_manager.get_schemas()?;
                let names: Vec<String> = available.keys().cloned().collect();
                log::error!(
                    "❌ Schema '{}' not found. Available schemas: {:?}",
                    query.schema_name,
                    names
                );
                return Err(SchemaError::InvalidData(format!(
                    "Schema '{}' not found",
                    query.schema_name
                )));
            }
        };
        self.hash_range_processor
            .query_with_filter(&mut schema, &query.fields, query.filter)
            .await
    }
}
