//! Shared source query implementation for view resolution.
//!
//! Implements [`SourceQueryFn`] for resolving a view's input queries
//! against either schemas or other views.
//!
//! Post-cache cleanup, the source-query path is mode-free: every nested
//! view is resolved inline by running its WASM transform on the latest
//! source data (or its identity pass-through). The on-disk cache that
//! used to short-circuit nested resolutions is gone — atoms written by
//! the derived-mutation fire path are the only persistent record of a
//! view's output.

use std::collections::HashMap;
use std::sync::Arc;

use async_trait::async_trait;

use crate::db_operations::DbOperations;
use crate::schema::types::field::FieldValue;
use crate::schema::types::key_value::KeyValue;
use crate::schema::types::operations::Query;
use crate::schema::{SchemaCore, SchemaError};
use crate::view::resolver::{SourceQueryFn, ViewResolver};

use super::hash_range_query::HashRangeQueryProcessor;

/// Shared [`SourceQueryFn`] implementation used by both the user-facing
/// query executor and the background view orchestrator.
pub struct StandardSourceQuery {
    schema_manager: Arc<SchemaCore>,
    db_ops: Arc<DbOperations>,
    hash_range_processor: HashRangeQueryProcessor,
    view_resolver: ViewResolver,
}

impl StandardSourceQuery {
    /// Construct a [`StandardSourceQuery`].
    pub fn new(
        schema_manager: Arc<SchemaCore>,
        db_ops: Arc<DbOperations>,
        view_resolver: ViewResolver,
    ) -> Self {
        let hash_range_processor = HashRangeQueryProcessor::new(Arc::clone(&db_ops), None);
        Self {
            schema_manager,
            db_ops,
            hash_range_processor,
            view_resolver,
        }
    }

    async fn query_view(
        &self,
        query: &Query,
    ) -> Result<HashMap<String, HashMap<KeyValue, FieldValue>>, SchemaError> {
        let view = {
            let registry = self.schema_manager.view_registry().lock().map_err(|_| {
                SchemaError::InvalidData("Failed to acquire view_registry lock".to_string())
            })?;
            registry
                .get_view(&query.schema_name)
                .cloned()
                .ok_or_else(|| {
                    SchemaError::NotFound(format!(
                        "'{}' not found as schema or view",
                        query.schema_name
                    ))
                })?
        };

        // Recursive resolution: nested view sources are resolved by spinning
        // up another `StandardSourceQuery`. The original wrapped a fresh
        // resolver per nest; we do the same so each level owns its own
        // small resolver state.
        let nested_source = Self {
            schema_manager: Arc::clone(&self.schema_manager),
            db_ops: Arc::clone(&self.db_ops),
            hash_range_processor: HashRangeQueryProcessor::new(Arc::clone(&self.db_ops), None),
            view_resolver: ViewResolver::new(Arc::clone(self.view_resolver.wasm_engine())),
        };

        // Recursive resolution must consult overrides — a view used as a
        // source for another view must serve overridden values, not the
        // computed-from-source values it would produce otherwise.
        let overrides = self
            .db_ops
            .views()
            .scan_transform_field_overrides(&view.name)
            .await?;

        self.view_resolver
            .resolve_with_overrides(&view, &query.fields, &nested_source, &overrides)
            .await
    }
}

#[async_trait]
impl SourceQueryFn for StandardSourceQuery {
    async fn execute_query(
        &self,
        query: &Query,
    ) -> Result<HashMap<String, HashMap<KeyValue, FieldValue>>, SchemaError> {
        // Views are registered as both a view (WASM, triggers) AND a
        // synthesized schema (atom store for derived mutations —
        // `projects/view-compute-as-mutations` PR 4). When resolving a
        // view-as-source, run the WASM transform via the resolver — the
        // atoms reflect the *previous* fire's input and would lag the
        // latest source data.
        let is_view = {
            let registry = self
                .schema_manager
                .view_registry()
                .lock()
                .map_err(|_| SchemaError::InvalidData("view_registry lock".to_string()))?;
            registry.get_view(&query.schema_name).is_some()
        };

        if is_view {
            return self.query_view(query).await;
        }

        match self.schema_manager.get_schema(&query.schema_name).await? {
            Some(mut schema) => {
                self.hash_range_processor
                    .query_with_filter(
                        &mut schema,
                        &query.fields,
                        query.filter.clone(),
                        query.as_of,
                    )
                    .await
            }
            None => Err(SchemaError::InvalidData(format!(
                "'{}' not found as schema or view",
                query.schema_name
            ))),
        }
    }
}
