//! Query Executor
//!
//! Main query execution logic extracted from FoldDB core, handling all query types
//! including HashRange schemas with proper delegation to specialized processors.

use crate::access::{self, AccessContext, PaymentGate};
use crate::db_operations::DbOperations;
use crate::schema::types::field::{Field, FieldValue};
use crate::schema::types::key_value::KeyValue;
use crate::schema::types::Query;
use crate::schema::SchemaError;
use crate::schema::{SchemaCore, SchemaState};
use crate::storage::SledPool;
use crate::view::registry::ViewState;
use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::RwLock;

use super::super::view_orchestrator::ViewOrchestrator;
use super::hash_range_query::HashRangeQueryProcessor;

/// Main query executor that handles all query operations
pub struct QueryExecutor {
    schema_manager: Arc<SchemaCore>,
    hash_range_processor: HashRangeQueryProcessor,
    /// Late-bound orchestrator handle. Used by the cold-read path to fire
    /// a view inline (run WASM, dual-write atoms, return output) when the
    /// atom store is empty for the requested fields.
    ///
    /// Wired post-construction in `fold_db.rs`, mirroring the
    /// `set_derived_mutation_writer` pattern. Optional so unit tests that
    /// stub a partial executor still compile; production paths always set it.
    view_orchestrator: Arc<RwLock<Option<Arc<ViewOrchestrator>>>>,
}

impl QueryExecutor {
    /// Create a new query executor with storage abstraction.
    ///
    /// `sled_pool` is optional but required to enable cross-user shared-data
    /// lookup via ShareSubscriptions. When `None`, queries only scan the
    /// personal (and org) namespaces.
    pub fn new(
        db_ops: Arc<DbOperations>,
        schema_manager: Arc<SchemaCore>,
        sled_pool: Option<Arc<SledPool>>,
    ) -> Self {
        let hash_range_processor =
            HashRangeQueryProcessor::new(Arc::clone(&db_ops), sled_pool.clone());

        Self {
            schema_manager,
            hash_range_processor,
            view_orchestrator: Arc::new(RwLock::new(None)),
        }
    }

    /// Wire in the view orchestrator after construction. Idempotent —
    /// re-calling replaces the prior handle.
    pub async fn set_view_orchestrator(&self, orchestrator: Arc<ViewOrchestrator>) {
        *self.view_orchestrator.write().await = Some(orchestrator);
    }

    async fn snapshot_orchestrator(&self) -> Option<Arc<ViewOrchestrator>> {
        self.view_orchestrator.read().await.clone()
    }

    /// Query multiple fields from a schema or view (legacy — no access control)
    pub async fn query(
        &self,
        query: Query,
    ) -> Result<HashMap<String, HashMap<KeyValue, FieldValue>>, SchemaError> {
        self.query_internal(query, None, None).await
    }

    /// Query with access control enforcement.
    ///
    /// For schema queries: fields where the caller lacks read access are filtered out.
    /// For view queries: all-or-nothing — the view is the access unit.
    pub async fn query_with_access(
        &self,
        query: Query,
        access_context: &AccessContext,
        payment_gate: Option<&PaymentGate>,
    ) -> Result<HashMap<String, HashMap<KeyValue, FieldValue>>, SchemaError> {
        self.query_internal(query, Some(access_context), payment_gate)
            .await
    }

    async fn query_internal(
        &self,
        query: Query,
        access_context: Option<&AccessContext>,
        payment_gate: Option<&PaymentGate>,
    ) -> Result<HashMap<String, HashMap<KeyValue, FieldValue>>, SchemaError> {
        // Views are registered as both a view (with WASM + triggers — still
        // what fires the transform) AND a synthesized schema (atom store
        // populated by the derived-mutation fire path, PR 2). The reader
        // serves from the synthesized schema's atom store first; if every
        // requested field is populated there, we return atoms directly —
        // carrying real `atom_uuid` / `molecule_uuid` / `written_at` /
        // `Provenance::Derived` provenance — and skip the WASM round-trip.
        //
        // When the atom store is cold (no fire has landed atoms for the
        // requested fields), we fire the view inline through the
        // orchestrator: run WASM, dual-write derived atoms, return the
        // computed output. Subsequent reads serve from atoms.
        let is_view = {
            let registry = self
                .schema_manager
                .view_registry()
                .lock()
                .map_err(|_| SchemaError::InvalidData("view_registry lock".to_string()))?;
            registry.get_view(&query.schema_name).is_some()
        };

        if is_view {
            if let Some(results) = self.read_view_atoms(&query).await? {
                return Ok(results);
            }
            return self.fire_view_for_query(&query).await;
        }

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

                let results = self
                    .hash_range_processor
                    .query_with_filter(&mut schema, &query.fields, query.filter, query.as_of)
                    .await?;

                // Apply per-field access control filtering if context is provided
                if let Some(ctx) = access_context {
                    Ok(Self::filter_fields_by_access(
                        results,
                        &schema,
                        ctx,
                        &query.schema_name,
                        payment_gate,
                    ))
                } else {
                    Ok(results)
                }
            }
            None => Err(SchemaError::InvalidData(format!(
                "'{}' not found as schema or view",
                query.schema_name
            ))),
        }
    }

    /// Filter query results by per-field access policies.
    /// Fields where the caller lacks read access are removed from the results.
    fn filter_fields_by_access(
        mut results: HashMap<String, HashMap<KeyValue, FieldValue>>,
        schema: &crate::schema::types::Schema,
        context: &AccessContext,
        schema_name: &str,
        payment_gate: Option<&PaymentGate>,
    ) -> HashMap<String, HashMap<KeyValue, FieldValue>> {
        let fields_to_remove: Vec<String> = results
            .keys()
            .filter(|field_name| {
                let policy = schema
                    .runtime_fields
                    .get(*field_name)
                    .map(|fv| fv.common().access_policy.as_ref())
                    .unwrap_or(None);
                let decision =
                    access::check_access(policy, context, schema_name, payment_gate, false);
                decision.is_denied()
            })
            .cloned()
            .collect();

        for field_name in fields_to_remove {
            results.remove(&field_name);
        }

        results
    }

    /// Try to serve a view query directly from its synthesized schema's
    /// atom store (the molecules written by
    /// `ViewOrchestrator::write_derived_mutations`). Returns `Ok(Some(_))`
    /// when every requested field has at least one entry — a signal that
    /// the view has fired at least once — and `Ok(None)` when the atom
    /// store is cold and the caller should fall through to a fresh fire.
    /// `Err` only propagates schema-level failures (blocked state); a
    /// legitimately empty atom store is not an error.
    async fn read_view_atoms(
        &self,
        query: &Query,
    ) -> Result<Option<HashMap<String, HashMap<KeyValue, FieldValue>>>, SchemaError> {
        let Some(mut schema) = self.schema_manager.get_schema(&query.schema_name).await? else {
            return Ok(None);
        };

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

        let results = self
            .hash_range_processor
            .query_with_filter(
                &mut schema,
                &query.fields,
                query.filter.clone(),
                query.as_of,
            )
            .await?;

        // Determine whether the atom store has results for every requested
        // field. An empty `fields` list in the query means "all output
        // fields"; we ask the view for its declared output fields so we
        // fall through consistently when any one field is cold.
        let requested_fields: Vec<String> = if query.fields.is_empty() {
            let registry = self
                .schema_manager
                .view_registry()
                .lock()
                .map_err(|_| SchemaError::InvalidData("view_registry lock".to_string()))?;
            let view = registry
                .get_view(&query.schema_name)
                .ok_or_else(|| SchemaError::InvalidData("view disappeared".to_string()))?;
            view.output_fields.keys().cloned().collect()
        } else {
            query.fields.clone()
        };

        let all_fields_populated = !requested_fields.is_empty()
            && requested_fields.iter().all(|field_name| {
                results
                    .get(field_name)
                    .map(|entries| !entries.is_empty())
                    .unwrap_or(false)
            });

        if !all_fields_populated {
            return Ok(None);
        }

        Ok(Some(results))
    }

    /// Cold-path: fire the view inline via the orchestrator (runs WASM,
    /// dual-writes atoms, returns computed output) and return the
    /// requested fields.
    ///
    /// Honors `View::Blocked` state. Errors thrown by the resolver
    /// (gas exceeded, compile, trap, type validation) propagate as
    /// `InvalidTransform` for the caller to surface.
    async fn fire_view_for_query(
        &self,
        query: &Query,
    ) -> Result<HashMap<String, HashMap<KeyValue, FieldValue>>, SchemaError> {
        // Validate the view exists and isn't blocked. Cloning the registry
        // entry here avoids holding the lock across the orchestrator call.
        {
            let registry = self.schema_manager.view_registry().lock().map_err(|_| {
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

            let state = registry
                .get_view_state(&query.schema_name)
                .unwrap_or(ViewState::Available);
            if state == ViewState::Blocked {
                return Err(SchemaError::InvalidData(format!(
                    "View '{}' is blocked and cannot be queried",
                    query.schema_name
                )));
            }

            let _ = view; // hold the borrow lifetime; registry lock released at scope end
        }

        let orchestrator = self.snapshot_orchestrator().await.ok_or_else(|| {
            SchemaError::InvalidData(
                "Internal: ViewOrchestrator not wired into QueryExecutor".to_string(),
            )
        })?;

        let output = orchestrator.fire_view(&query.schema_name).await?;

        // The resolver returns FieldValues with blank atom-level provenance
        // because it works against in-flight WASM output, not landed atoms.
        // Once the dual-write completes, atoms are persisted with full
        // provenance — this in-line path returns the computed values
        // immediately so the caller doesn't have to wait for a re-read.
        // Subsequent reads hit `read_view_atoms` and serve real atoms.
        if query.fields.is_empty() {
            Ok(output)
        } else {
            // Filter to requested fields, preserving the validate-fields-exist
            // semantics that the resolver enforced internally.
            let mut filtered = HashMap::new();
            for field_name in &query.fields {
                if let Some(entries) = output.get(field_name) {
                    filtered.insert(field_name.clone(), entries.clone());
                }
            }
            Ok(filtered)
        }
    }
}
