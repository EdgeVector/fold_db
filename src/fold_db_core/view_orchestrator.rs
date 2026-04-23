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
use crate::view::transform_field_override::TransformFieldOverride;
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

    /// Route mutations targeting transform views.
    ///
    /// Identity views: writes go through to the source schema (the inverse
    /// of identity is itself).
    ///
    /// WASM views (no inverse): writes are persisted as
    /// `TransformFieldOverride` molecules per `transform_views_design.md` —
    /// the field flips into the `Overridden` state and the source link is
    /// marked stale. The override is stored in its own namespace so it
    /// participates in the unified sync log and converges across replicas
    /// via LWW on `written_at`. These mutations do NOT propagate further
    /// down the pipeline (they are not source-schema mutations); the
    /// returned vector contains only the rewritten source mutations and any
    /// non-view mutations that pass through unchanged.
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

            if let Some(field_map) = view.source_field_map() {
                // Identity view — rewrite to source mutations.
                let mut redirected: HashMap<String, HashMap<String, serde_json::Value>> =
                    HashMap::new();

                for (field_name, value) in &mutation.fields_and_values {
                    let (source_schema, source_field) =
                        field_map.get(field_name).ok_or_else(|| {
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
            } else {
                // Irreversible (WASM) view — persist each field as an override.
                self.persist_overrides_for_mutation(&view.name, &mutation)
                    .await?;
            }
        }

        Ok(result)
    }

    /// Persist a `TransformFieldOverride` for every (field, key) in the
    /// mutation. The mutation's `pub_key` becomes the override's writer
    /// pubkey; its `key_value` becomes the per-key handle. Each call stamps
    /// `written_at = now`, so concurrent writes on different replicas resolve
    /// via LWW once they meet through the sync log.
    async fn persist_overrides_for_mutation(
        &self,
        view_name: &str,
        mutation: &Mutation,
    ) -> Result<(), SchemaError> {
        // The view must be registered for us to honor the write — otherwise we
        // would silently drop user data. The caller has already established
        // the view exists, so this is just a defensive lookup against schema
        // mutations between the registry read and now.
        let view_exists = {
            let registry = self.schema_manager.view_registry().lock().map_err(|_| {
                SchemaError::InvalidData("Failed to acquire view_registry lock".to_string())
            })?;
            registry.get_view(view_name).is_some()
        };
        if !view_exists {
            return Err(SchemaError::NotFound(format!(
                "View '{}' disappeared mid-mutation",
                view_name
            )));
        }

        let key_str = mutation.key_value.to_string();
        let view_store = self.db_ops.views();

        for (field_name, value) in &mutation.fields_and_values {
            let override_mol = TransformFieldOverride::new(value.clone(), mutation.pub_key.clone());
            view_store
                .put_transform_field_override(view_name, field_name, &key_str, &override_mol)
                .await?;
        }
        Ok(())
    }

    /// Invalidate a single named view (and its cascade), then spawn a
    /// background precompute for the deep tier. Phase 1 trigger runner
    /// entry: the runner decides which views fire, then calls this to
    /// perform the actual cache lifecycle work.
    pub async fn invalidate_view(&self, view_name: &str) -> Result<(), SchemaError> {
        // Confirm the view exists before doing work.
        {
            let registry = self
                .schema_manager
                .view_registry()
                .lock()
                .map_err(|_| SchemaError::InvalidData("view_registry lock".to_string()))?;
            if registry.get_view(view_name).is_none() {
                return Err(SchemaError::NotFound(format!(
                    "View '{}' not found",
                    view_name
                )));
            }
        }

        let mut all_invalidated: Vec<String> = vec![view_name.to_string()];
        let mut visited = std::collections::HashSet::new();
        self.collect_cascade_views(view_name, &mut visited, &mut all_invalidated)?;

        for v in &all_invalidated {
            let current_state = self.db_ops.get_view_cache_state(v).await?;
            if !current_state.is_empty() {
                self.db_ops
                    .set_view_cache_state(v, &ViewCacheState::Empty)
                    .await?;
                log::debug!(
                    "Invalidated view cache '{}' ({:?} → Empty, trigger-driven)",
                    v,
                    current_state
                );
            }
        }

        let (all_ordered, deep_views) =
            self.partition_views_for_precomputation(&all_invalidated)?;
        if !deep_views.is_empty() {
            self.spawn_background_precomputation(all_ordered, deep_views)
                .await?;
        }
        Ok(())
    }

    // NOTE: `invalidate_on_mutation(schema, fields)` used to be the
    // implicit fire path — every mutation would re-run every view
    // dependent on the mutated fields. Phase 1 task 3 replaces that with
    // explicit triggers on each view and routes dispatch through
    // `TriggerRunner`, which calls `invalidate_view` per view it decides
    // to fire. No call site remains for the old method, so it's removed.

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
            // - Unavailable: compute already attempted and failed on this
            //   input (sticky per input); skip — a source mutation will
            //   invalidate it back to Empty before the next retry.
            let state = db_ops.get_view_cache_state(view_name).await?;
            if matches!(
                state,
                ViewCacheState::Cached { .. } | ViewCacheState::Unavailable { .. }
            ) {
                log::debug!(
                    "View '{}' already in terminal state ({:?}), skipping precomputation",
                    view_name,
                    state
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
            // Precompute also has to honor overrides — otherwise the
            // background pass would write a `Cached` state that ignores the
            // user's pin, and the next read would briefly serve the wrong
            // value before being corrected by `resolve_with_overrides`.
            let overrides = db_ops
                .views()
                .scan_transform_field_overrides(view_name)
                .await?;
            match resolver
                .resolve_with_overrides(
                    &view,
                    &[],
                    &ViewCacheState::Empty,
                    &source_query,
                    &overrides,
                )
                .await
            {
                Ok((_, new_cache)) => {
                    // Only store if not re-invalidated since we started
                    // (e.g., a source mutation landed mid-compute and moved
                    // the view back to Empty). Persist both successful
                    // Cached and terminal Unavailable states — the latter
                    // is what prevents retry storms.
                    let current = db_ops.get_view_cache_state(view_name).await?;
                    if !matches!(current, ViewCacheState::Cached { .. }) {
                        db_ops.set_view_cache_state(view_name, &new_cache).await?;
                        match &new_cache {
                            ViewCacheState::Unavailable { reason } => {
                                log::warn!(
                                    "View '{}' precomputation → Unavailable: {}",
                                    view_name,
                                    reason
                                );
                            }
                            _ => log::info!("View '{}' precomputed successfully", view_name),
                        }
                    }
                }
                Err(e) => {
                    log::error!("Failed to precompute view '{}': {}", view_name, e);
                    // Reset Computing to Empty so it can be lazily computed on next query.
                    // Note: per-input WASM failures never reach this branch — the
                    // resolver converts them to `Ok(Unavailable)`. This path only
                    // fires for infrastructure errors (lock poisoning, source query
                    // failure) that are worth retrying.
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
