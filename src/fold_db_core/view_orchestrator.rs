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

use async_trait::async_trait;
use tokio::sync::RwLock;

use crate::atom::provenance::{MoleculeRef, Provenance};
use crate::db_operations::DbOperations;
use crate::schema::types::operations::MutationType;
use crate::schema::types::{KeyValue, Mutation};
use crate::schema::{SchemaCore, SchemaError};
use crate::view::derived_metadata::DerivedMetadata;
use crate::view::resolver::ViewResolver;
use crate::view::transform_field_override::TransformFieldOverride;
use crate::view::types::{TransformView, ViewCacheState};

use super::query::StandardSourceQuery;

/// Back-door that lets [`ViewOrchestrator`] submit derived mutations through
/// [`super::mutation_manager::MutationManager`] without forming a static
/// circular dependency. `ViewOrchestrator` is constructed first (and owned by
/// `MutationManager`), so the writer is wired post-construction via
/// [`ViewOrchestrator::set_derived_mutation_writer`], mirroring the existing
/// `set_trigger_dispatcher` pattern in `fold_db.rs`.
///
/// The trait abstraction also keeps the orchestrator testable — tests can
/// plug in a mock writer that records submitted mutations without spinning
/// up a real `MutationManager`.
#[async_trait]
pub trait DerivedMutationWriter: Send + Sync {
    /// Submit a batch of derived-provenance mutations. Mirrors
    /// `MutationManager::write_mutations_batch_async` in every respect except
    /// that it is expected to bypass identity-view redirection and
    /// override-persistence when the mutation carries `Provenance::Derived`.
    async fn write_derived_batch(
        &self,
        mutations: Vec<Mutation>,
    ) -> Result<Vec<String>, SchemaError>;
}

/// Orchestrates view lifecycle: dependency-graph traversal, invalidation,
/// and precomputation of derived views triggered by mutations.
pub struct ViewOrchestrator {
    schema_manager: Arc<SchemaCore>,
    db_ops: Arc<DbOperations>,
    /// Post-construction late-binding slot for the derived-mutation writer.
    /// `None` in tests and until `fold_db.rs` wires the `MutationManager` in.
    /// When `None`, the orchestrator still updates `ViewCacheState::Cached` as
    /// before — the derived-mutation path is the additive side of the
    /// dual-write phase of `projects/view-compute-as-mutations`.
    derived_writer: Arc<RwLock<Option<Arc<dyn DerivedMutationWriter>>>>,
}

impl ViewOrchestrator {
    /// Create a new ViewOrchestrator.
    pub fn new(schema_manager: Arc<SchemaCore>, db_ops: Arc<DbOperations>) -> Self {
        Self {
            schema_manager,
            db_ops,
            derived_writer: Arc::new(RwLock::new(None)),
        }
    }

    /// Wire in the derived-mutation writer after construction. Idempotent —
    /// re-calling replaces the prior writer. Called once from `fold_db.rs`
    /// after both `ViewOrchestrator` and `MutationManager` exist.
    pub async fn set_derived_mutation_writer(&self, writer: Arc<dyn DerivedMutationWriter>) {
        *self.derived_writer.write().await = Some(writer);
    }

    /// Accessor used by the background precompute task to grab the current
    /// writer (if set) without holding the lock across the write operation.
    async fn snapshot_derived_writer(&self) -> Option<Arc<dyn DerivedMutationWriter>> {
        self.derived_writer.read().await.clone()
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
            // `Provenance::Derived` mutations are produced by the transform
            // fire path itself (see `precompute_views` below). They are writes
            // to the view's own output molecules, NOT user-originated writes
            // against the view, so they must bypass identity-view redirection
            // AND override persistence — both of which assume a user pin.
            // Pass them straight through to the normal mutation pipeline so
            // atoms land on the view schema's fields.
            if matches!(mutation.provenance, Some(Provenance::Derived { .. })) {
                result.push(mutation);
                continue;
            }

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
                        provenance: mutation.provenance.clone(),
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
        let derived_writer = self.snapshot_derived_writer().await;

        tokio::spawn(async move {
            if let Err(e) =
                Self::precompute_views(schema_manager, db_ops, all_ordered, derived_writer).await
            {
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
        derived_writer: Option<Arc<dyn DerivedMutationWriter>>,
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
                .resolve_with_overrides_and_derived(
                    &view,
                    &[],
                    &ViewCacheState::Empty,
                    &source_query,
                    &overrides,
                )
                .await
            {
                Ok((output, new_cache, derived)) => {
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

                    // Dual-write to the atom layer: on a successful WASM fire
                    // (`derived` is `Some`), submit the output as a batch of
                    // mutations carrying `Provenance::Derived` through
                    // `MutationManager`. The cache write above keeps reads
                    // working while PR 4 flips the read path to atoms; the
                    // mutation write is what the future cache-free world
                    // consumes. Identity views, sticky-Unavailable, and the
                    // no-writer-configured case all land in this branch with
                    // `derived = None` and skip the dual-write.
                    if let (Some(metadata), Some(writer)) = (derived, derived_writer.as_ref()) {
                        if let Err(e) = Self::write_derived_mutations(
                            writer.as_ref(),
                            &db_ops,
                            &view,
                            &output,
                            metadata,
                        )
                        .await
                        {
                            // Derived-write failure should not abort
                            // precomputation — the cache write already
                            // happened, so the view is readable. Log loudly
                            // so regressions surface in integration tests.
                            log::error!(
                                "Derived-mutation dual-write failed for view '{}': {}",
                                view_name,
                                e
                            );
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

    /// Turn a successful WASM fire's output into a batch of derived-provenance
    /// mutations and submit them through the configured writer. The output is
    /// pivoted from `field -> key -> value` to `key -> (field -> value)` so
    /// each resulting mutation writes every field for a single row, matching
    /// `MutationManager::write_mutations_batch_async`'s expectation that one
    /// `Mutation` corresponds to one `(schema, key)` tuple. After the writer
    /// returns, lineage index entries are inserted so reverse queries
    /// ("which derived molecules came from source X?") can find these mutations.
    ///
    /// Best-effort: errors here do NOT abort precomputation — the cache write
    /// is the authoritative result for readers until PR 4 of
    /// `projects/view-compute-as-mutations` flips the read path.
    async fn write_derived_mutations(
        writer: &dyn DerivedMutationWriter,
        db_ops: &Arc<DbOperations>,
        view: &TransformView,
        output: &HashMap<String, HashMap<KeyValue, crate::schema::types::field::FieldValue>>,
        metadata: DerivedMetadata,
    ) -> Result<(), SchemaError> {
        let mutations = build_derived_mutations(view, output, &metadata);
        if mutations.is_empty() {
            return Ok(());
        }

        // Mutation uuids are stable from construction; remember them so we
        // can seed the forward lineage index keyed by the same uuid the
        // mutation pipeline persists.
        let mutation_uuids: Vec<String> = mutations.iter().map(|m| m.uuid.clone()).collect();
        let sources: &[MoleculeRef] = &metadata.sources;

        // Submit the batch. Propagate the mutation-pipeline error rather than
        // swallowing it — the caller logs and continues.
        let _ids = writer.write_derived_batch(mutations).await?;

        // Populate lineage forward/reverse entries for every derived mutation.
        // Uses the `Mutation::uuid` as the derived identifier so the index is
        // keyed the same way mutation events are emitted. Best-effort per
        // uuid so a single failure doesn't starve the rest.
        let lineage = db_ops.lineage();
        for uuid in mutation_uuids {
            if let Err(e) = lineage.insert(&uuid, sources).await {
                log::warn!(
                    "Lineage index insert failed for derived mutation '{}' on view '{}': {}",
                    uuid,
                    view.name,
                    e
                );
            }
        }

        Ok(())
    }
}

/// Pivot the `field -> key -> value` output map from the resolver into one
/// [`Mutation`] per distinct key. Each mutation targets the view's own
/// schema, carries `Provenance::Derived` built from [`DerivedMetadata`], and
/// writes every field that produced a value for that row.
///
/// Free function (not a method) so it can be unit-tested without constructing
/// a full `ViewOrchestrator` / database stack.
fn build_derived_mutations(
    view: &TransformView,
    output: &HashMap<String, HashMap<KeyValue, crate::schema::types::field::FieldValue>>,
    metadata: &DerivedMetadata,
) -> Vec<Mutation> {
    // Gather every key that appears anywhere in the output, then collect the
    // per-field values for each key. HashMap iteration is unordered, so the
    // resulting mutation batch is unordered — `MutationManager` does not
    // require ordering.
    let mut by_key: HashMap<KeyValue, HashMap<String, serde_json::Value>> = HashMap::new();
    for (field_name, entries) in output {
        for (key, fv) in entries {
            by_key
                .entry(key.clone())
                .or_default()
                .insert(field_name.clone(), fv.value.clone());
        }
    }

    let provenance = Provenance::derived(
        metadata.wasm_hash.clone(),
        metadata.input_snapshot_hash.clone(),
        metadata.sources_merkle_root.clone(),
    );

    by_key
        .into_iter()
        .map(|(key_value, fields_and_values)| {
            Mutation {
                uuid: uuid::Uuid::new_v4().to_string(),
                schema_name: view.name.clone(),
                fields_and_values,
                key_value,
                // `pub_key` is left empty for derived writes — the writer's
                // identity lives in `provenance.wasm_hash`, not in a human
                // key. The follow-up cleanup PR that removes `pub_key`
                // outright after full wire-through will drop this altogether.
                pub_key: String::new(),
                mutation_type: MutationType::Create,
                synchronous: None,
                source_file_name: None,
                metadata: None,
                provenance: Some(provenance.clone()),
            }
        })
        .collect()
}

#[cfg(test)]
mod derived_mutation_tests {
    //! `build_derived_mutations` is pure — no DB, no async. Exercise it
    //! directly so the row-pivot logic is pinned against regressions.

    use super::*;
    use crate::schema::types::field::FieldValue;
    use crate::schema::types::field_value_type::FieldValueType;
    use crate::schema::types::operations::Query;
    use crate::schema::types::schema::DeclarativeSchemaType;
    use serde_json::json;

    fn view(name: &str) -> TransformView {
        TransformView::new(
            name,
            DeclarativeSchemaType::Single,
            None,
            vec![Query::new("Src".to_string(), vec!["f".to_string()])],
            None,
            HashMap::from([
                ("title".to_string(), FieldValueType::String),
                ("count".to_string(), FieldValueType::Integer),
            ]),
        )
    }

    fn fv(value: serde_json::Value) -> FieldValue {
        FieldValue {
            value,
            atom_uuid: String::new(),
            source_file_name: None,
            metadata: None,
            molecule_uuid: None,
            molecule_version: None,
            writer_pubkey: None,
            written_at: None,
        }
    }

    fn metadata() -> DerivedMetadata {
        DerivedMetadata {
            wasm_hash: "w".repeat(64),
            input_snapshot_hash: "i".repeat(64),
            sources_merkle_root: "r".repeat(64),
            sources: Vec::new(),
        }
    }

    #[test]
    fn one_mutation_per_key_carries_every_field_for_that_row() {
        let v = view("V");
        let key_a = KeyValue::new(Some("a".to_string()), None);
        let key_b = KeyValue::new(Some("b".to_string()), None);

        let mut title_entries = HashMap::new();
        title_entries.insert(key_a.clone(), fv(json!("A-title")));
        title_entries.insert(key_b.clone(), fv(json!("B-title")));
        let mut count_entries = HashMap::new();
        count_entries.insert(key_a.clone(), fv(json!(1)));
        count_entries.insert(key_b.clone(), fv(json!(2)));

        let mut output = HashMap::new();
        output.insert("title".to_string(), title_entries);
        output.insert("count".to_string(), count_entries);

        let muts = build_derived_mutations(&v, &output, &metadata());
        assert_eq!(muts.len(), 2);
        for m in &muts {
            assert_eq!(m.schema_name, "V");
            assert_eq!(m.mutation_type, MutationType::Create);
            assert!(m.pub_key.is_empty(), "derived writes have no user writer");
            assert!(m.provenance.is_some(), "derived provenance must be set");
            assert!(matches!(m.provenance, Some(Provenance::Derived { .. })));
            assert_eq!(m.fields_and_values.len(), 2);
        }

        let by_key: HashMap<&KeyValue, &HashMap<String, serde_json::Value>> = muts
            .iter()
            .map(|m| (&m.key_value, &m.fields_and_values))
            .collect();
        assert_eq!(by_key[&key_a]["title"], json!("A-title"));
        assert_eq!(by_key[&key_a]["count"], json!(1));
        assert_eq!(by_key[&key_b]["title"], json!("B-title"));
        assert_eq!(by_key[&key_b]["count"], json!(2));
    }

    #[test]
    fn missing_field_on_some_keys_yields_partial_mutations() {
        // When a field has no entry for a particular key, the resulting
        // mutation for that key should still be emitted — just without that
        // field in its `fields_and_values`. This matches the resolver's
        // tolerance for non-uniform output shapes.
        let v = view("V");
        let key_only_title = KeyValue::new(Some("a".to_string()), None);
        let key_only_count = KeyValue::new(Some("b".to_string()), None);

        let mut title_entries = HashMap::new();
        title_entries.insert(key_only_title.clone(), fv(json!("only")));
        let mut count_entries = HashMap::new();
        count_entries.insert(key_only_count.clone(), fv(json!(99)));

        let mut output = HashMap::new();
        output.insert("title".to_string(), title_entries);
        output.insert("count".to_string(), count_entries);

        let muts = build_derived_mutations(&v, &output, &metadata());
        assert_eq!(muts.len(), 2);

        let by_key: HashMap<&KeyValue, &HashMap<String, serde_json::Value>> = muts
            .iter()
            .map(|m| (&m.key_value, &m.fields_and_values))
            .collect();
        assert_eq!(by_key[&key_only_title].len(), 1);
        assert_eq!(by_key[&key_only_count].len(), 1);
    }

    #[test]
    fn empty_output_produces_no_mutations() {
        let v = view("V");
        let muts = build_derived_mutations(&v, &HashMap::new(), &metadata());
        assert!(muts.is_empty());
    }

    #[test]
    fn derived_provenance_fields_propagate_from_metadata() {
        let v = view("V");
        let key = KeyValue::new(Some("a".to_string()), None);
        let mut title = HashMap::new();
        title.insert(key, fv(json!("t")));
        let mut output = HashMap::new();
        output.insert("title".to_string(), title);

        let md = DerivedMetadata {
            wasm_hash: "abc".to_string(),
            input_snapshot_hash: "def".to_string(),
            sources_merkle_root: "ghi".to_string(),
            sources: Vec::new(),
        };
        let muts = build_derived_mutations(&v, &output, &md);
        let Some(Provenance::Derived {
            wasm_hash,
            input_snapshot_hash,
            sources_merkle_root,
            encoding_version,
        }) = muts[0].provenance.clone()
        else {
            panic!("expected Derived provenance");
        };
        assert_eq!(wasm_hash, "abc");
        assert_eq!(input_snapshot_hash, "def");
        assert_eq!(sources_merkle_root, "ghi");
        assert_eq!(encoding_version, 1);
    }
}
