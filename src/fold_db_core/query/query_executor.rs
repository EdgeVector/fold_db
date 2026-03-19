//! Query Executor
//!
//! Main query execution logic extracted from FoldDB core, handling all query types
//! including HashRange schemas with proper delegation to specialized processors.

use crate::db_operations::DbOperations;
use crate::schema::types::field::FieldValue;
use crate::schema::types::key_value::KeyValue;
use crate::schema::types::Query;
use crate::schema::{SchemaCore, SchemaState};
use crate::schema::SchemaError;
use crate::view::registry::ViewState;
use crate::view::resolver::{SourceQueryFn, ViewResolver};
use crate::view::types::ViewCacheState;
use async_trait::async_trait;
use std::collections::HashMap;
use std::sync::Arc;

use super::hash_range_query::HashRangeQueryProcessor;

/// Main query executor that handles all query operations
pub struct QueryExecutor {
    schema_manager: Arc<SchemaCore>,
    db_ops: Arc<DbOperations>,
    hash_range_processor: HashRangeQueryProcessor,
    view_resolver: ViewResolver,
}

/// Implements SourceQueryFn by delegating back to the query executor's query path.
/// This supports recursive resolution: views can query other views or schemas.
struct RecursiveSourceQuery {
    schema_manager: Arc<SchemaCore>,
    db_ops: Arc<DbOperations>,
    hash_range_processor: HashRangeQueryProcessor,
    view_resolver: ViewResolver,
}

#[async_trait]
impl SourceQueryFn for RecursiveSourceQuery {
    async fn execute_query(
        &self,
        query: &Query,
    ) -> Result<HashMap<String, HashMap<KeyValue, FieldValue>>, SchemaError> {
        // First try as schema
        match self.schema_manager.get_schema(&query.schema_name).await? {
            Some(mut schema) => {
                self.hash_range_processor
                    .query_with_filter(&mut schema, &query.fields, query.filter.clone(), query.as_of)
                    .await
            }
            None => {
                // Try as view (recursive)
                self.try_query_view(query).await
            }
        }
    }
}

impl RecursiveSourceQuery {
    async fn try_query_view(
        &self,
        query: &Query,
    ) -> Result<HashMap<String, HashMap<KeyValue, FieldValue>>, SchemaError> {
        let view = {
            let registry = self
                .schema_manager
                .view_registry()
                .lock()
                .map_err(|_| {
                    SchemaError::InvalidData("Failed to acquire view_registry lock".to_string())
                })?;

            registry.get_view(&query.schema_name).cloned().ok_or_else(|| {
                SchemaError::NotFound(format!(
                    "'{}' not found as schema or view",
                    query.schema_name
                ))
            })?
        };

        // Load cache state
        let cache_state = self
            .db_ops
            .get_view_cache_state(&view.name)
            .await?;

        if cache_state.is_computing() {
            return Err(SchemaError::InvalidData(format!(
                "View '{}' is currently being precomputed and is not ready for queries",
                view.name
            )));
        }

        // Create a nested source query for this view's input queries
        let nested_source = RecursiveSourceQuery {
            schema_manager: Arc::clone(&self.schema_manager),
            db_ops: Arc::clone(&self.db_ops),
            hash_range_processor: HashRangeQueryProcessor::new(Arc::clone(&self.db_ops)),
            view_resolver: ViewResolver::new(Arc::clone(self.view_resolver.wasm_engine())),
        };

        let (results, new_cache) = self
            .view_resolver
            .resolve(&view, &query.fields, &cache_state, &nested_source)
            .await?;

        // Persist cache if it changed from Empty to Cached
        if cache_state.is_empty() && matches!(new_cache, ViewCacheState::Cached { .. }) {
            self.db_ops
                .set_view_cache_state(&view.name, &new_cache)
                .await?;
        }

        Ok(results)
    }
}

impl QueryExecutor {
    /// Create a new query executor with storage abstraction
    pub fn new(db_ops: Arc<DbOperations>, schema_manager: Arc<SchemaCore>) -> Self {
        let hash_range_processor = HashRangeQueryProcessor::new(Arc::clone(&db_ops));

        // Get the WASM engine from the view registry
        let wasm_engine = {
            let registry = schema_manager
                .view_registry()
                .lock()
                .expect("view_registry lock");
            Arc::clone(registry.wasm_engine())
        };
        let view_resolver = ViewResolver::new(wasm_engine);

        Self {
            schema_manager,
            db_ops,
            hash_range_processor,
            view_resolver,
        }
    }

    /// Query multiple fields from a schema or view
    pub async fn query(
        &self,
        query: Query,
    ) -> Result<HashMap<String, HashMap<KeyValue, FieldValue>>, SchemaError> {
        // First: try to resolve as a schema (existing path)
        match self.schema_manager.get_schema(&query.schema_name).await? {
            Some(mut schema) => {
                // Enforce Blocked state
                let resolved_state = self
                    .schema_manager
                    .get_schema_states()?
                    .get(&schema.name)
                    .copied()
                    .unwrap_or_default();
                if resolved_state == SchemaState::Blocked {
                    return Err(SchemaError::InvalidData(format!(
                        "Schema '{}' is blocked and cannot be queried",
                        schema.name
                    )));
                }

                self.hash_range_processor
                    .query_with_filter(&mut schema, &query.fields, query.filter, query.as_of)
                    .await
            }
            None => {
                // Second: try to resolve as a view
                self.try_query_view(&query).await
            }
        }
    }

    /// Attempt to resolve a query against the view registry.
    async fn try_query_view(
        &self,
        query: &Query,
    ) -> Result<HashMap<String, HashMap<KeyValue, FieldValue>>, SchemaError> {
        let view = {
            let registry = self
                .schema_manager
                .view_registry()
                .lock()
                .map_err(|_| {
                    SchemaError::InvalidData("Failed to acquire view_registry lock".to_string())
                })?;

            let view = registry.get_view(&query.schema_name).ok_or_else(|| {
                let available = self.schema_manager.get_schemas().unwrap_or_default();
                let schema_names: Vec<String> = available.keys().cloned().collect();
                let view_names: Vec<String> = registry
                    .list_views()
                    .iter()
                    .map(|v| v.name.clone())
                    .collect();
                log::error!(
                    "'{}' not found as schema or view. Schemas: {:?}, Views: {:?}",
                    query.schema_name,
                    schema_names,
                    view_names
                );
                SchemaError::InvalidData(format!(
                    "'{}' not found as schema or view",
                    query.schema_name
                ))
            })?;

            // Check view state
            let state = registry
                .get_view_state(&query.schema_name)
                .unwrap_or(ViewState::Available);
            if state == ViewState::Blocked {
                return Err(SchemaError::InvalidData(format!(
                    "View '{}' is blocked and cannot be queried",
                    query.schema_name
                )));
            }

            view.clone()
        };

        // Load cache state
        let cache_state = self
            .db_ops
            .get_view_cache_state(&view.name)
            .await?;

        // Reject queries on views that are being precomputed in the background
        if cache_state.is_computing() {
            return Err(SchemaError::InvalidData(format!(
                "View '{}' is currently being precomputed and is not ready for queries",
                query.schema_name
            )));
        }

        // Create source query implementation for recursive resolution
        let source_query = RecursiveSourceQuery {
            schema_manager: Arc::clone(&self.schema_manager),
            db_ops: Arc::clone(&self.db_ops),
            hash_range_processor: HashRangeQueryProcessor::new(Arc::clone(&self.db_ops)),
            view_resolver: ViewResolver::new(Arc::clone(self.view_resolver.wasm_engine())),
        };

        let (results, new_cache) = self
            .view_resolver
            .resolve(&view, &query.fields, &cache_state, &source_query)
            .await?;

        // Persist cache if it changed from Empty to Cached
        if cache_state.is_empty() && matches!(new_cache, ViewCacheState::Cached { .. }) {
            self.db_ops
                .set_view_cache_state(&view.name, &new_cache)
                .await?;
        }

        Ok(results)
    }
}
