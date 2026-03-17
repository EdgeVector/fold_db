//! Query Executor
//!
//! Main query execution logic extracted from FoldDB core, handling all query types
//! including HashRange schemas with proper delegation to specialized processors.

use crate::db_operations::DbOperations;
use crate::schema::types::field::{FieldValue, HashRangeFilter};
use crate::schema::types::key_value::KeyValue;
use crate::schema::types::Query;
use crate::schema::{SchemaCore, SchemaState};
use crate::schema::SchemaError;
use crate::view::registry::ViewState;
use crate::view::resolver::{SourceQueryFn, ViewFieldResolver};
use crate::view::types::TransformFieldState;
use async_trait::async_trait;
use chrono::{DateTime, Utc};
use std::collections::HashMap;
use std::sync::Arc;

use super::hash_range_query::HashRangeQueryProcessor;

/// Main query executor that handles all query operations
pub struct QueryExecutor {
    schema_manager: Arc<SchemaCore>,
    db_ops: Arc<DbOperations>,
    hash_range_processor: HashRangeQueryProcessor,
    view_resolver: ViewFieldResolver,
}

/// Implements SourceQueryFn by delegating back to the query executor's schema query path.
/// This breaks the circular dependency: QueryExecutor -> ViewFieldResolver -> SourceQueryFn -> schema query.
struct SchemaSourceQuery {
    schema_manager: Arc<SchemaCore>,
    hash_range_processor: HashRangeQueryProcessor,
}

#[async_trait]
impl SourceQueryFn for SchemaSourceQuery {
    async fn query_field(
        &self,
        schema_name: &str,
        field_name: &str,
        filter: Option<HashRangeFilter>,
        as_of: Option<DateTime<Utc>>,
    ) -> Result<HashMap<KeyValue, FieldValue>, SchemaError> {
        let mut schema = self
            .schema_manager
            .get_schema(schema_name)
            .await?
            .ok_or_else(|| {
                SchemaError::NotFound(format!(
                    "Source schema '{}' not found",
                    schema_name
                ))
            })?;

        let field_results = self
            .hash_range_processor
            .query_with_filter(&mut schema, &[field_name.to_string()], filter, as_of)
            .await?;

        // Return the keyed results for this field directly
        Ok(field_results
            .get(field_name)
            .cloned()
            .unwrap_or_default())
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
        let view_resolver = ViewFieldResolver::new(wasm_engine);

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
                // Enforce Blocked state — blocked schemas with a successor are already redirected by get_schema()
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
                    "❌ '{}' not found as schema or view. Schemas: {:?}, Views: {:?}",
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

        // Determine which fields to resolve
        let fields_to_resolve: Vec<String> = if query.fields.is_empty() {
            view.fields.keys().cloned().collect()
        } else {
            query.fields.clone()
        };

        // Create source query implementation
        let source_query = SchemaSourceQuery {
            schema_manager: Arc::clone(&self.schema_manager),
            hash_range_processor: HashRangeQueryProcessor::new(Arc::clone(&self.db_ops)),
        };

        let mut result: HashMap<String, HashMap<KeyValue, FieldValue>> = HashMap::new();

        for field_name in &fields_to_resolve {
            // Load current field state from storage
            let field_state = self
                .db_ops
                .get_transform_field_state(&view.name, field_name)
                .await?;

            let (field_results, new_state) = self
                .view_resolver
                .resolve_field(
                    &view,
                    field_name,
                    &field_state,
                    &source_query,
                    query.filter.clone(),
                    query.as_of,
                )
                .await?;

            // Persist updated state if it changed from Empty to Cached
            if field_state.is_empty() && matches!(new_state, TransformFieldState::Cached { .. }) {
                self.db_ops
                    .set_transform_field_state(&view.name, field_name, &new_state)
                    .await?;
            }

            result.insert(field_name.clone(), field_results);
        }

        Ok(result)
    }
}
