//! Shared source query implementation for view resolution.
//!
//! Implements [`SourceQueryFn`] for resolving a view's input queries against
//! either schemas or other views. Used by both [`QueryExecutor`] (user-facing
//! query path) and [`ViewOrchestrator`] (background precomputation), which
//! differ only in how they handle views that are not yet cached.
//!
//! [`QueryExecutor`]: crate::fold_db_core::query::query_executor::QueryExecutor
//! [`ViewOrchestrator`]: crate::fold_db_core::view_orchestrator::ViewOrchestrator

use std::collections::HashMap;
use std::sync::Arc;

use async_trait::async_trait;

use crate::db_operations::DbOperations;
use crate::schema::types::field::FieldValue;
use crate::schema::types::key_value::KeyValue;
use crate::schema::types::operations::Query;
use crate::schema::{SchemaCore, SchemaError};
use crate::view::resolver::{SourceQueryFn, ViewResolver};
use crate::view::types::ViewCacheState;

use super::hash_range_query::HashRangeQueryProcessor;

/// Behavior mode for [`StandardSourceQuery`].
///
/// Controls how views whose caches are not yet `Cached` are handled.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum SourceQueryMode {
    /// User-facing query path. Rejects views in the `Computing` state
    /// (a background precompute is in flight) and otherwise resolves the
    /// view via its input queries, persisting the resulting cache if the
    /// view transitioned from `Empty` to `Cached`.
    Recursive,
    /// Background precomputation path. Treats any non-`Cached` state
    /// (including `Computing`) as `Empty` and inline-computes the view.
    /// This is safe because precomputation walks the dependency graph
    /// in bottom-up order, so callers are already precomputing deeper
    /// views first.
    Precompute,
}

/// Shared [`SourceQueryFn`] implementation used by both the user-facing
/// query executor and the background view orchestrator.
pub struct StandardSourceQuery {
    schema_manager: Arc<SchemaCore>,
    db_ops: Arc<DbOperations>,
    hash_range_processor: HashRangeQueryProcessor,
    view_resolver: ViewResolver,
    mode: SourceQueryMode,
}

impl StandardSourceQuery {
    /// Construct a [`StandardSourceQuery`] in [`SourceQueryMode::Recursive`] mode.
    pub fn new_recursive(
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
            mode: SourceQueryMode::Recursive,
        }
    }

    /// Construct a [`StandardSourceQuery`] in [`SourceQueryMode::Precompute`] mode.
    pub fn new_precompute(
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
            mode: SourceQueryMode::Precompute,
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

        let cache_state = self.db_ops.get_view_cache_state(&view.name).await?;

        // Unavailable propagates as an error up the view chain. A parent
        // view that tried to read this view gets the same visibility as
        // if it had queried this view directly — no silent empty results,
        // no retry. Applies to both modes.
        if let Some(reason) = cache_state.unavailable_reason() {
            return Err(SchemaError::InvalidTransform(format!(
                "View '{}' unavailable: {}",
                view.name, reason
            )));
        }

        // Determine the effective cache state to hand to the resolver.
        let effective_cache = match self.mode {
            SourceQueryMode::Recursive => {
                // Recursive mode rejects views that are currently being
                // precomputed in the background — the user-facing query
                // path should never race with the precomputer.
                if cache_state.is_computing() {
                    return Err(SchemaError::InvalidData(format!(
                        "View '{}' is currently being precomputed and is not ready for queries",
                        view.name
                    )));
                }
                cache_state
            }
            SourceQueryMode::Precompute => {
                // Precompute mode inline-computes any non-Cached view
                // (including Computing), because the orchestrator walks
                // the view graph bottom-up and already holds the semantics
                // that deeper views must be materialized first.
                if matches!(cache_state, ViewCacheState::Cached { .. }) {
                    cache_state
                } else {
                    ViewCacheState::Empty
                }
            }
        };

        // Nested recursion uses the same mode so that the entire resolution
        // walk shares identical semantics.
        let nested_source = Self {
            schema_manager: Arc::clone(&self.schema_manager),
            db_ops: Arc::clone(&self.db_ops),
            hash_range_processor: HashRangeQueryProcessor::new(Arc::clone(&self.db_ops), None),
            view_resolver: ViewResolver::new(Arc::clone(self.view_resolver.wasm_engine())),
            mode: self.mode,
        };

        // Recursive resolution must also consult overrides — a view used as
        // a source for another view must serve overridden values, not the
        // computed-from-source values it would produce otherwise.
        let overrides = self
            .db_ops
            .views()
            .scan_transform_field_overrides(&view.name)
            .await?;

        let (results, new_cache) = self
            .view_resolver
            .resolve_with_overrides(
                &view,
                &query.fields,
                &effective_cache,
                &nested_source,
                &overrides,
            )
            .await?;

        // Persist terminal transitions from Empty. Cached makes the next
        // read a hit; Unavailable makes it fail-fast with the reason.
        match &new_cache {
            ViewCacheState::Cached { .. } if effective_cache.is_empty() => {
                self.db_ops
                    .set_view_cache_state(&view.name, &new_cache)
                    .await?;
            }
            ViewCacheState::Unavailable { reason } => {
                self.db_ops
                    .set_view_cache_state(&view.name, &new_cache)
                    .await?;
                return Err(SchemaError::InvalidTransform(format!(
                    "View '{}' unavailable: {}",
                    view.name, reason
                )));
            }
            _ => {}
        }

        Ok(results)
    }
}

#[async_trait]
impl SourceQueryFn for StandardSourceQuery {
    async fn execute_query(
        &self,
        query: &Query,
    ) -> Result<HashMap<String, HashMap<KeyValue, FieldValue>>, SchemaError> {
        // Views are now registered as both a view (WASM, triggers, cache)
        // AND a synthesized schema (atom store for derived mutations —
        // `projects/view-compute-as-mutations` PR 4). The view path still
        // owns the primary semantics: it runs the WASM transform and
        // applies overrides. Route view queries to the view path; only
        // non-view schemas go down the atom-store path.
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
