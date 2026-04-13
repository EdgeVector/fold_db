//! View Orchestrator
//!
//! Extracted from MutationManager. Owns all view lifecycle logic triggered by
//! mutations: redirecting mutations targeting identity views to their source
//! schemas, invalidating dependent view caches, topologically ordering cascade
//! views, and spawning background precomputation tasks.
//!
//! This is pure graph/view orchestration — it has nothing to do with mutation
//! execution per se (atoms, molecules, idempotency). It's triggered BY
//! mutations but is not part of the mutation pipeline.

use std::collections::{HashMap, HashSet, VecDeque};
use std::sync::Arc;

use crate::db_operations::DbOperations;
use crate::schema::types::Mutation;
use crate::schema::{SchemaCore, SchemaError};
use crate::view::resolver::ViewResolver;
use crate::view::types::ViewCacheState;

use super::query::StandardSourceQuery;

/// Orchestrates view lifecycle: dependency-graph traversal, invalidation,
/// and precomputation of derived views triggered by mutations.
pub struct ViewOrchestrator {
    schema_manager: Arc<SchemaCore>,
    db_ops: Arc<DbOperations>,
}

impl ViewOrchestrator {
    /// Create a new ViewOrchestrator.
    pub fn new(schema_manager: Arc<SchemaCore>, db_ops: Arc<DbOperations>) -> Self {
        Self {
            schema_manager,
            db_ops,
        }
    }

    /// Redirect mutations targeting identity views to their source schemas.
    /// WASM views are not writable (would require inverse transforms).
    pub async fn redirect_mutation(
        &self,
        mutations: Vec<Mutation>,
    ) -> Result<Vec<Mutation>, SchemaError> {
        let mut result = Vec::with_capacity(mutations.len());

        for mutation in mutations {
            let view_info = {
                let registry = self.schema_manager.view_registry().lock().map_err(|_| {
                    SchemaError::InvalidData("Failed to acquire view_registry lock".to_string())
                })?;
                registry.get_view(&mutation.schema_name).cloned()
            };

            let Some(view) = view_info else {
                // Not a view — pass through to normal pipeline
                result.push(mutation);
                continue;
            };

            // Get source field map (only works for identity views)
            let field_map = view.source_field_map().ok_or_else(|| {
                SchemaError::InvalidData(format!(
                    "Cannot write to WASM view '{}'. Write-back through WASM views is not yet supported.",
                    view.name
                ))
            })?;

            // Group mutation fields by target source schema
            let mut redirected: HashMap<String, HashMap<String, serde_json::Value>> =
                HashMap::new();

            for (field_name, value) in &mutation.fields_and_values {
                let (source_schema, source_field) = field_map.get(field_name).ok_or_else(|| {
                    SchemaError::InvalidField(format!(
                        "Field '{}' not found in view '{}'",
                        field_name, view.name
                    ))
                })?;

                redirected
                    .entry(source_schema.clone())
                    .or_default()
                    .insert(source_field.clone(), value.clone());
            }

            // Create one redirected mutation per source schema
            for (target_schema, fields_and_values) in redirected {
                result.push(Mutation {
                    uuid: uuid::Uuid::new_v4().to_string(),
                    schema_name: target_schema,
                    fields_and_values,
                    key_value: mutation.key_value.clone(),
                    pub_key: mutation.pub_key.clone(),
                    mutation_type: mutation.mutation_type.clone(),
                    synchronous: mutation.synchronous,
                    source_file_name: mutation.source_file_name.clone(),
                    metadata: mutation.metadata.clone(),
                });
            }
        }

        Ok(result)
    }

    /// Invalidate view caches that depend on mutated source fields, and spawn
    /// background precomputation for deep views. Operates at the view level
    /// (not per-field).
    pub async fn invalidate_on_mutation(
        &self,
        schema_name: &str,
        fields_affected: &[String],
    ) -> Result<(), SchemaError> {
        // Collect all view names that depend on any of the affected fields
        let dependent_views: HashSet<String> = {
            let registry = self
                .schema_manager
                .view_registry()
                .lock()
                .map_err(|_| SchemaError::InvalidData("view_registry lock".to_string()))?;

            let mut views = HashSet::new();
            for field_name in fields_affected {
                let deps = registry
                    .dependency_tracker
                    .get_dependents(schema_name, field_name);
                for view_name in deps {
                    views.insert(view_name.clone());
                }
            }
            views
        };

        // Collect ALL views to invalidate (direct + transitive) in one pass
        let mut all_invalidated: Vec<String> = Vec::new();
        let mut visited = HashSet::new();
        for view_name in &dependent_views {
            all_invalidated.push(view_name.clone());
            self.collect_cascade_views(view_name, &mut visited, &mut all_invalidated)?;
        }

        // Invalidate all collected views (both Cached and Computing).
        // Computing views must also be reset: a background precompute task
        // started before this mutation holds stale source data. Resetting to
        // Empty ensures the precompute task's check-before-store sees Empty
        // and the view will be re-precomputed with fresh data.
        for view_name in &all_invalidated {
            let current_state = self.db_ops.get_view_cache_state(view_name).await?;

            if !current_state.is_empty() {
                self.db_ops
                    .set_view_cache_state(view_name, &ViewCacheState::Empty)
                    .await?;
                log::debug!(
                    "Invalidated view cache '{}' ({:?} → Empty, source {}.{} mutated)",
                    view_name,
                    current_state,
                    schema_name,
                    fields_affected.first().unwrap_or(&String::new())
                );
            }
        }

        // Identify views deeper than level 1 (depend on other views) and
        // spawn background precomputation. All invalidated views are passed
        // in bottom-up order so leaf views compute first, but only deep views
        // (level 2+) are marked Computing — level 1 views stay Empty and can
        // also be lazily queried.
        let (all_ordered, deep_views) =
            self.partition_views_for_precomputation(&all_invalidated)?;
        if !deep_views.is_empty() {
            self.spawn_background_precomputation(all_ordered, deep_views)
                .await?;
        }

        Ok(())
    }

    /// Collect all transitive cascade views in one pass (single lock acquisition).
    fn collect_cascade_views(
        &self,
        view_name: &str,
        visited: &mut HashSet<String>,
        result: &mut Vec<String>,
    ) -> Result<(), SchemaError> {
        if !visited.insert(view_name.to_string()) {
            return Ok(());
        }

        let cascade_views: Vec<String> = {
            let registry = self
                .schema_manager
                .view_registry()
                .lock()
                .map_err(|_| SchemaError::InvalidData("view_registry lock".to_string()))?;
            registry
                .dependency_tracker
                .get_all_dependents_of_schema(view_name)
        };

        for dep in &cascade_views {
            if !visited.contains(dep) {
                result.push(dep.clone());
                self.collect_cascade_views(dep, visited, result)?;
            }
        }

        Ok(())
    }

    /// Partition invalidated views into:
    /// - `all_ordered`: all views in bottom-up order (leaves first) for precomputation
    /// - `deep_only`: subset that depends on other views (level 2+), to be marked Computing
    fn partition_views_for_precomputation(
        &self,
        invalidated: &[String],
    ) -> Result<(Vec<String>, HashSet<String>), SchemaError> {
        let registry = self
            .schema_manager
            .view_registry()
            .lock()
            .map_err(|_| SchemaError::InvalidData("view_registry lock".to_string()))?;

        let invalidated_set: HashSet<&str> = invalidated.iter().map(|s| s.as_str()).collect();

        // Classify each view as level-1 (only schema sources) or deep (has view sources).
        // Also build an adjacency map for topological sorting.
        let mut deep: HashSet<String> = HashSet::new();
        let mut in_degree: HashMap<String, usize> = HashMap::new();
        // view_name → list of views that depend on it (within the invalidated set)
        let mut dependents_of: HashMap<String, Vec<String>> = HashMap::new();

        for view_name in invalidated {
            if let Some(view) = registry.get_view(view_name) {
                let view_sources_in_set: Vec<String> = view
                    .source_schemas()
                    .into_iter()
                    .filter(|source| {
                        registry.get_view(source).is_some()
                            && invalidated_set.contains(source.as_str())
                    })
                    .collect();

                if !view_sources_in_set.is_empty() {
                    deep.insert(view_name.clone());
                }

                in_degree.insert(view_name.clone(), view_sources_in_set.len());
                for source in view_sources_in_set {
                    dependents_of
                        .entry(source)
                        .or_default()
                        .push(view_name.clone());
                }
            }
        }

        // Kahn's algorithm: topological sort so leaves (in_degree=0) come first
        let mut queue: VecDeque<String> = in_degree
            .iter()
            .filter(|(_, &deg)| deg == 0)
            .map(|(name, _)| name.clone())
            .collect();
        let mut all: Vec<String> = Vec::new();

        while let Some(current) = queue.pop_front() {
            all.push(current.clone());
            if let Some(deps) = dependents_of.get(&current) {
                for dep in deps {
                    if let Some(deg) = in_degree.get_mut(dep) {
                        *deg = deg.saturating_sub(1);
                        if *deg == 0 {
                            queue.push_back(dep.clone());
                        }
                    }
                }
            }
        }

        Ok((all, deep))
    }

    /// Mark deep views as Computing and spawn a background task to precompute
    /// all views in bottom-up order. Level 1 views are computed first (they
    /// only depend on schemas) so that deep views can resolve against them.
    async fn spawn_background_precomputation(
        &self,
        all_ordered: Vec<String>,
        deep_views: HashSet<String>,
    ) -> Result<(), SchemaError> {
        // Only mark deep views as Computing (level 1 stays Empty for lazy query)
        for view_name in &deep_views {
            self.db_ops
                .set_view_cache_state(view_name, &ViewCacheState::Computing)
                .await?;
            log::debug!(
                "View '{}' marked as Computing for background precomputation",
                view_name
            );
        }

        // Spawn background task that computes ALL views bottom-up
        let schema_manager = Arc::clone(&self.schema_manager);
        let db_ops = Arc::clone(&self.db_ops);

        tokio::spawn(async move {
            if let Err(e) = Self::precompute_views(schema_manager, db_ops, all_ordered).await {
                log::error!("Background view precomputation failed: {}", e);
            }
        });

        Ok(())
    }

    /// Background task: precompute views in bottom-up order.
    /// Each view's sources must be Cached before it can be computed.
    async fn precompute_views(
        schema_manager: Arc<SchemaCore>,
        db_ops: Arc<DbOperations>,
        views_to_compute: Vec<String>,
    ) -> Result<(), SchemaError> {
        let wasm_engine = {
            let registry = schema_manager
                .view_registry()
                .lock()
                .map_err(|_| SchemaError::InvalidData("view_registry lock".to_string()))?;
            Arc::clone(registry.wasm_engine())
        };

        for view_name in &views_to_compute {
            // Get view definition
            let view = {
                let registry = schema_manager
                    .view_registry()
                    .lock()
                    .map_err(|_| SchemaError::InvalidData("view_registry lock".to_string()))?;
                match registry.get_view(view_name) {
                    Some(v) => v.clone(),
                    None => {
                        log::warn!("View '{}' disappeared during precomputation", view_name);
                        continue;
                    }
                }
            };

            // Check current state:
            // - Computing: deep view, precompute and store
            // - Empty: level-1 view, precompute and store (needed by deeper views)
            // - Cached: already computed (perhaps by a lazy query), skip
            let state = db_ops.get_view_cache_state(view_name).await?;
            if matches!(state, ViewCacheState::Cached { .. }) {
                log::debug!(
                    "View '{}' already Cached, skipping precomputation",
                    view_name
                );
                continue;
            }

            // Build source query for resolution
            let source_query = StandardSourceQuery::new_precompute(
                Arc::clone(&schema_manager),
                Arc::clone(&db_ops),
                ViewResolver::new(Arc::clone(&wasm_engine)),
            );

            let resolver = ViewResolver::new(Arc::clone(&wasm_engine));
            match resolver
                .resolve(&view, &[], &ViewCacheState::Empty, &source_query)
                .await
            {
                Ok((_, new_cache)) => {
                    // Only store if not re-invalidated since we started
                    let current = db_ops.get_view_cache_state(view_name).await?;
                    if !matches!(current, ViewCacheState::Cached { .. }) {
                        db_ops.set_view_cache_state(view_name, &new_cache).await?;
                        log::info!("View '{}' precomputed successfully", view_name);
                    }
                }
                Err(e) => {
                    log::error!("Failed to precompute view '{}': {}", view_name, e);
                    // Reset Computing to Empty so it can be lazily computed on next query
                    let current = db_ops.get_view_cache_state(view_name).await?;
                    if current.is_computing() {
                        db_ops
                            .set_view_cache_state(view_name, &ViewCacheState::Empty)
                            .await?;
                    }
                }
            }
        }

        Ok(())
    }
}
