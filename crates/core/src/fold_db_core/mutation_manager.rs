//! Mutation Manager - Handles all mutation operations
//!
//! This module contains the MutationManager that handles the core mutation logic
//! previously located in FoldDB. It manages the execution of mutations, including
//! schema updates, atom persistence, and event publishing. It can also listen
//! for MutationRequest events and handle them automatically.

use std::collections::HashMap;
use std::sync::Arc;

use super::orchestration::index_status::IndexStatusTracker;
use super::trigger_runner::TriggerDispatcher;
use super::view_orchestrator::{DerivedMutationWriter, ViewOrchestrator};
use crate::atom::{Atom, FieldKey, MutationEvent};
use crate::db_operations::{DbOperations, MoleculeData};
use crate::messaging::events::query_events::MutationExecuted;
use crate::messaging::{AsyncMessageBus, Event};
use crate::schema::types::field::{Field, FieldVariant};
use crate::schema::types::{KeyValue, Mutation, Schema};
use crate::schema::{SchemaCore, SchemaError};
use crate::storage::SledPool;
use chrono::Utc;
use sha2::{Digest, Sha256};
use std::collections::HashSet;
use tracing::{debug, error, warn};

/// Manages mutation operations for the FoldDB system
pub struct MutationManager {
    /// Database operations for persistence
    db_ops: Arc<DbOperations>,
    /// Schema manager for schema operations
    schema_manager: Arc<SchemaCore>,
    /// Message bus for event publishing and listening
    message_bus: Arc<AsyncMessageBus>,
    /// View lifecycle service. Retained for mutation redirection (identity
    /// view writes → source schemas). The old "invalidate every view
    /// dependent on the mutated fields" cascade now goes through the
    /// trigger runner; see `trigger_dispatcher` below.
    view_orchestrator: Arc<ViewOrchestrator>,
    /// Trigger runner notification channel. Installed after construction
    /// via `set_trigger_dispatcher` because the runner needs a reference
    /// back to the mutation manager to write TriggerFiring audit rows.
    /// `None` is legal only in tests or boot paths that predate trigger
    /// wiring — in those cases mutations commit without any fire.
    trigger_dispatcher: std::sync::RwLock<Option<Arc<dyn TriggerDispatcher>>>,
    /// Index status tracker for reporting indexing progress
    index_status_tracker: Option<IndexStatusTracker>,
    /// Sled pool for on-demand access to the org memberships tree.
    /// When present, mutations against org-scoped schemas are gated on
    /// the node actually being a member of that org. When absent (e.g.
    /// non-Sled backends, some unit tests), the check is skipped.
    sled_pool: Option<Arc<SledPool>>,
    /// Flag to track if the event listener is running
    is_listening: Arc<std::sync::atomic::AtomicBool>,
    /// Signing keypair for molecule signatures
    signer: Arc<crate::security::Ed25519KeyPair>,
}

impl MutationManager {
    /// Creates a new MutationManager instance
    pub fn new(
        db_ops: Arc<DbOperations>,
        schema_manager: Arc<SchemaCore>,
        message_bus: Arc<AsyncMessageBus>,
        view_orchestrator: Arc<ViewOrchestrator>,
        index_status_tracker: Option<IndexStatusTracker>,
        sled_pool: Option<Arc<SledPool>>,
        signer: Arc<crate::security::Ed25519KeyPair>,
    ) -> Self {
        Self {
            db_ops,
            schema_manager,
            message_bus,
            view_orchestrator,
            trigger_dispatcher: std::sync::RwLock::new(None),
            index_status_tracker,
            sled_pool,
            is_listening: Arc::new(std::sync::atomic::AtomicBool::new(false)),
            signer,
        }
    }

    /// Install the trigger dispatcher after construction. Needed because
    /// the dispatcher holds a back-reference to this manager (to write
    /// TriggerFiring rows), so the two must be constructed in order:
    /// MutationManager first, TriggerRunner second, then wire here.
    pub fn set_trigger_dispatcher(&self, dispatcher: Arc<dyn TriggerDispatcher>) {
        *self
            .trigger_dispatcher
            .write()
            .expect("trigger_dispatcher poisoned") = Some(dispatcher);
    }

    /// Drop the dispatcher reference. Call from shutdown to break the
    /// Arc cycle between this manager and the TriggerRunner.
    pub fn clear_trigger_dispatcher(&self) {
        *self
            .trigger_dispatcher
            .write()
            .expect("trigger_dispatcher poisoned") = None;
    }

    fn snapshot_trigger_dispatcher(&self) -> Option<Arc<dyn TriggerDispatcher>> {
        self.trigger_dispatcher
            .read()
            .expect("trigger_dispatcher poisoned")
            .clone()
    }

    /// Validate that the node is a member of the given org before allowing
    /// writes against it. Returns `Ok(())` if the membership exists (or if
    /// membership validation is disabled because no sled pool is available).
    ///
    /// This is the authoritative check for org-scoped mutations: a local
    /// attacker cannot inject writes for an arbitrary `org_hash` they are
    /// not a member of, because those writes would otherwise get prefixed
    /// with that org hash and queued for sync.
    fn validate_org_membership(&self, org_hash: &str) -> Result<(), SchemaError> {
        let Some(pool) = self.sled_pool.as_ref() else {
            // No sled pool means we cannot consult the org_memberships tree.
            // This path is only hit in limited test/non-Sled configurations
            // where org sync is not in play. Fail-open here would be unsafe
            // in production, but in production the pool is always present.
            return Ok(());
        };

        match crate::org::operations::get_org(pool, org_hash) {
            Ok(Some(_)) => Ok(()),
            Ok(None) => Err(SchemaError::PermissionDenied(format!(
                "Mutation rejected: node is not a member of org '{}'",
                org_hash
            ))),
            Err(e) => Err(SchemaError::InvalidData(format!(
                "Failed to check org membership for '{}': {}",
                org_hash, e
            ))),
        }
    }

    pub async fn get_indexing_status(&self) -> super::orchestration::IndexingStatus {
        if let Some(tracker) = &self.index_status_tracker {
            tracker.get_status().await
        } else {
            super::orchestration::IndexingStatus::default()
        }
    }

    pub async fn is_indexing(&self) -> bool {
        if let Some(tracker) = &self.index_status_tracker {
            tracker.is_indexing().await
        } else {
            false
        }
    }

    /// Write mutations with access control enforcement.
    ///
    /// Checks per-field write access for every field in every mutation.
    /// Mutations are all-or-nothing: if any field is denied, the entire batch
    /// for that schema is rejected.
    pub async fn write_mutations_with_access(
        &self,
        mutations: Vec<Mutation>,
        access_context: &crate::access::AccessContext,
        payment_gate: Option<&crate::access::PaymentGate>,
    ) -> Result<Vec<String>, SchemaError> {
        use crate::access;
        use crate::schema::types::field::Field;

        // Pre-check access for all mutations before processing any
        for mutation in &mutations {
            let schema = self
                .schema_manager
                .get_schema_metadata(&mutation.schema_name)?
                .ok_or_else(|| {
                    SchemaError::InvalidData(format!("Schema '{}' not found", mutation.schema_name))
                })?;

            // Security check: org-scoped schemas may only be written by
            // members of the org. `write_mutations_batch_async` re-checks
            // this as a defense-in-depth chokepoint, but we fail fast here
            // so the caller gets a clear error before any side effects.
            if let Some(org_hash) = schema.org_hash.as_deref() {
                self.validate_org_membership(org_hash)?;
            }

            for field_name in mutation.fields_and_values.keys() {
                let policy = schema
                    .runtime_fields
                    .get(field_name)
                    .map(|fv| fv.common().access_policy.as_ref())
                    .unwrap_or(None);

                let decision = access::check_access(
                    policy,
                    access_context,
                    &mutation.schema_name,
                    payment_gate,
                    true,
                );

                if let access::AccessDecision::Denied(reason) = decision {
                    return Err(SchemaError::PermissionDenied(format!(
                        "Write denied for field '{}' on schema '{}': {}",
                        field_name, mutation.schema_name, reason
                    )));
                }
            }
        }

        // All access checks passed — delegate to the standard batch writer
        self.write_mutations_batch_async(mutations).await
    }

    /// Write multiple mutations in a batch for improved performance (async version)
    /// Groups mutations by schema to minimize schema reloads and uses true batching
    ///
    /// This is the preferred async version that avoids deadlocks.
    /// All storage operations use direct async/await instead of run_async.
    pub async fn write_mutations_batch_async(
        &self,
        mutations: Vec<Mutation>,
    ) -> Result<Vec<String>, SchemaError> {
        if mutations.is_empty() {
            return Ok(Vec::new());
        }

        // Phase -1: Shape-check derived mutations before anything else touches
        // them. A mutation carrying `Provenance::Derived` must name a
        // registered WASM view whose compiled bytes hash to the provenance's
        // `wasm_hash`, and the `encoding_version` must be supported. Catches
        // forged / stale derived writes before they pollute atoms. User
        // mutations (`Provenance::User` or `None`) are untouched.
        for mutation in &mutations {
            validate_derived_provenance(mutation, &self.schema_manager)?;
        }

        // Phase 0: Redirect identity view mutations to source schemas
        let mutations = self.view_orchestrator.redirect_mutation(mutations).await?;

        tracing::info!(
            "🔄 write_mutations_batch_async: Starting batch of {} mutations",
            mutations.len()
        );
        tracing::debug!(
            "DEBUG: MutationManager::write_mutations_batch_async started for {} mutations",
            mutations.len()
        );

        let start_time = std::time::Instant::now();
        let mut timing_breakdown = std::collections::HashMap::new();

        // Phase 1: Filter out already-processed mutations
        let idem_start = std::time::Instant::now();
        let (already_seen_ids, new_mutations, new_hashes) =
            self.filter_idempotent_mutations(mutations).await?;
        timing_breakdown.insert("idempotency_check", idem_start.elapsed());

        if new_mutations.is_empty() {
            tracing::info!(
                "All {} mutations were idempotency duplicates, skipping processing",
                already_seen_ids.len()
            );
            return Ok(already_seen_ids);
        }

        tracing::debug!(
            "Idempotency: {} new, {} duplicates",
            new_mutations.len(),
            already_seen_ids.len()
        );

        // Build hash->uuid map before grouping (uuid is stable, set at creation time)
        let hash_to_uuid: Vec<(String, String)> = new_mutations
            .iter()
            .zip(new_hashes.iter())
            .map(|(m, h)| (h.clone(), m.uuid.clone()))
            .collect();

        // Group mutations by schema to minimize schema reloads
        let group_start = std::time::Instant::now();
        let grouped_mutations = self.group_mutations_by_schema(new_mutations);
        timing_breakdown.insert("grouping", group_start.elapsed());

        let mut mutation_ids = Vec::new();
        let mut batch_events = Vec::new();

        for (schema_name, schema_mutations) in grouped_mutations {
            // Load schema once for all mutations in this schema
            let load_start = std::time::Instant::now();
            let mut schema = self
                .schema_manager
                .get_schema_metadata(&schema_name)?
                .ok_or_else(|| {
                    SchemaError::InvalidData(format!("Schema '{}' not found", schema_name))
                })?;
            *timing_breakdown
                .entry("schema_load")
                .or_insert(std::time::Duration::ZERO) += load_start.elapsed();

            // Security check: if this schema is org-scoped, verify the node
            // is a member of the org before persisting. Writes to org-scoped
            // schemas produce `{org_hash}:`-prefixed sled keys which would
            // otherwise be queued for sync under that org's prefix.
            if let Some(org_hash) = schema.org_hash.as_deref() {
                self.validate_org_membership(org_hash)?;
            }

            // Phase 2: Create atoms and compute key values
            let phase1_start = std::time::Instant::now();
            let (mutation_key_values, atom_results) = self
                .prepare_atoms_and_key_values(&schema_name, &schema, &schema_mutations)
                .await?;
            *timing_breakdown
                .entry("  - create_atoms_batch")
                .or_insert(std::time::Duration::ZERO) += phase1_start.elapsed();

            // Phase 3: Restore molecules missing from memory
            self.restore_missing_molecules(&mut schema).await;

            // Phase 4: Apply mutations to molecules in memory
            let phase2_start = std::time::Instant::now();
            let (mutation_events, modified_fields) = self.apply_mutations_to_molecules(
                &mut schema,
                &schema_mutations,
                &mutation_key_values,
                atom_results,
            )?;
            *timing_breakdown
                .entry("  - update_memory_serial")
                .or_insert(std::time::Duration::ZERO) += phase2_start.elapsed();

            // Phase 5: Persist modified molecules and mutation events
            let phase3_start = std::time::Instant::now();
            self.persist_modified_molecules(
                &schema,
                modified_fields,
                mutation_events,
                &mut timing_breakdown,
            )
            .await?;
            *timing_breakdown
                .entry("  - write_molecules_batch")
                .or_insert(std::time::Duration::ZERO) += phase3_start.elapsed();

            // Phase 6: Inline index mutations
            let inline_index_start = std::time::Instant::now();
            self.inline_index_mutations(&schema_name, &schema_mutations, &mutation_key_values)
                .await;
            *timing_breakdown
                .entry("inline_indexing")
                .or_insert(std::time::Duration::ZERO) += inline_index_start.elapsed();

            // Sync molecule UUIDs to the persisted field before storing
            let sync_start = std::time::Instant::now();
            schema.sync_molecule_uuids();
            *timing_breakdown
                .entry("sync_uuids")
                .or_insert(std::time::Duration::ZERO) += sync_start.elapsed();

            // Single schema persist and reload for this schema group (async)
            let store_start = std::time::Instant::now();
            self.db_ops.store_schema(&schema_name, &schema).await?;
            *timing_breakdown
                .entry("schema_store")
                .or_insert(std::time::Duration::ZERO) += store_start.elapsed();

            // Collect molecule versions before schema is moved into load_schema_internal
            let mut mol_versions: HashSet<u64> = HashSet::new();
            for schema_field in schema.runtime_fields.values() {
                if let Some(v) = schema_field.molecule_version() {
                    mol_versions.insert(v);
                }
            }

            let reload_start = std::time::Instant::now();
            self.schema_manager.load_schema_internal(schema).await?;
            *timing_breakdown
                .entry("schema_reload")
                .or_insert(std::time::Duration::ZERO) += reload_start.elapsed();

            // Phase 7: Build events for batch publishing
            let (events, ids) = self.build_batch_events(
                &schema_name,
                &schema_mutations,
                mutation_key_values,
                &mol_versions,
            );
            batch_events.extend(events);
            mutation_ids.extend(ids);

            // Phase 7.5: Notify the trigger runner. The runner decides
            // which views fire based on their declared triggers and
            // dispatches via ViewOrchestrator::fire_view. The old
            // implicit "every mutation invalidates every dependent view"
            // cascade is gone — this is the ONLY fire path now.
            if let Some(dispatcher) = self.snapshot_trigger_dispatcher() {
                let fields_affected: Vec<String> = schema_mutations
                    .iter()
                    .flat_map(|m| m.fields_and_values.keys().cloned())
                    .collect::<HashSet<_>>()
                    .into_iter()
                    .collect();
                if let Err(e) = dispatcher.on_mutation(&schema_name, &fields_affected).await {
                    warn!(
                        "TriggerDispatcher::on_mutation failed for '{}': {}. \
                         Mutation already committed; view fires skipped.",
                        schema_name, e
                    );
                }
            }
        }

        // Phase 8: Finalize — flush, store idempotency records, publish events
        self.finalize_batch(&hash_to_uuid, batch_events, &mut timing_breakdown)
            .await?;

        let total_time = start_time.elapsed();

        // Combine already-seen ids with newly processed mutation ids
        let mut all_ids = already_seen_ids;
        all_ids.extend(mutation_ids.iter().cloned());

        tracing::info!(
            "✅ write_mutations_batch_async: Completed {} mutations ({} new, {} cached) in {:.2}ms",
            all_ids.len(),
            mutation_ids.len(),
            all_ids.len() - mutation_ids.len(),
            total_time.as_millis()
        );

        Self::log_timing_breakdown(&timing_breakdown, total_time);

        Ok(all_ids)
    }

    /// Log a sorted timing breakdown for batch mutation phases.
    fn log_timing_breakdown(
        timing_breakdown: &HashMap<&str, std::time::Duration>,
        total_time: std::time::Duration,
    ) {
        tracing::debug!(
            "Batch mutation timing breakdown (total: {:.2}ms):",
            total_time.as_millis()
        );
        let mut sorted_timings: Vec<_> = timing_breakdown.iter().collect();
        sorted_timings.sort_by(|a, b| b.1.cmp(a.1));
        let total_ms = total_time.as_millis() as f64;
        for (operation, duration) in sorted_timings {
            let percentage = (duration.as_millis() as f64 / total_ms) * 100.0;
            debug!(
                "  - {}: {:.2}ms ({:.1}%)",
                operation,
                duration.as_millis(),
                percentage
            );
        }
    }

    // --- Helpers for write_mutations_batch_async ---

    /// Filters out already-processed mutations using content-hash idempotency.
    /// Returns (already_seen_ids, new_mutations, new_hashes).
    async fn filter_idempotent_mutations(
        &self,
        mutations: Vec<Mutation>,
    ) -> Result<(Vec<String>, Vec<Mutation>, Vec<String>), SchemaError> {
        let mut already_seen_ids: Vec<String> = Vec::new();
        let mut new_mutations: Vec<Mutation> = Vec::new();
        let mut new_hashes: Vec<String> = Vec::new();

        for mutation in mutations {
            let hash = mutation.content_hash();
            let key = format!("idem:{}", hash);
            match self
                .db_ops
                .metadata()
                .get_idempotency_item::<String>(&key)
                .await
            {
                Ok(Some(cached_id)) => {
                    tracing::debug!(
                        "Idempotency hit for mutation hash {}, returning cached id {}",
                        hash,
                        cached_id
                    );
                    already_seen_ids.push(cached_id);
                }
                _ => {
                    new_hashes.push(hash);
                    new_mutations.push(mutation);
                }
            }
        }

        Ok((already_seen_ids, new_mutations, new_hashes))
    }

    /// Creates atoms and computes key values for all mutations in a schema group.
    /// Returns (mutation_key_values, atom_results) and batch-stores atoms as a side effect.
    async fn prepare_atoms_and_key_values(
        &self,
        schema_name: &str,
        schema: &Schema,
        schema_mutations: &[Mutation],
    ) -> Result<(Vec<KeyValue>, Vec<(usize, String, Atom)>), SchemaError> {
        let mut atoms_to_store: Vec<Atom> = Vec::new();
        let mut atom_results: Vec<(usize, String, Atom)> = Vec::new();
        let mut mutation_key_values = Vec::with_capacity(schema_mutations.len());

        for (idx, mutation) in schema_mutations.iter().enumerate() {
            // Prefer the pre-computed key_value from the mutation (set by the
            // ingestion service with date normalization and proper field extraction).
            // Fall back to KeyValue::from_mutation() only when both fields are None.
            let mut key_value =
                if mutation.key_value.hash.is_some() || mutation.key_value.range.is_some() {
                    mutation.key_value.clone()
                } else {
                    let key_config = schema.key.clone();
                    KeyValue::from_mutation(
                        &mutation.fields_and_values,
                        key_config.as_ref().ok_or_else(|| {
                            SchemaError::InvalidData(format!(
                                "Schema '{}' has no key configuration. Cannot execute mutation.",
                                schema_name
                            ))
                        })?,
                    )
                };

            // Safety net: if key is still empty after both extraction paths,
            // generate a deterministic content hash so the mutation is stored
            // and retrievable rather than silently lost.
            if key_value.hash.is_none() && key_value.range.is_none() {
                let mut hasher = Sha256::new();
                let mut sorted: Vec<_> = mutation.fields_and_values.iter().collect();
                sorted.sort_by_key(|(a, _)| (*a).clone());
                for (k, v) in sorted {
                    hasher.update(k.as_bytes());
                    hasher.update(v.to_string().as_bytes());
                }
                let fallback = format!("{:x}", hasher.finalize());
                let short = &fallback[..16];
                warn!(
                    "Key resolution produced empty key for schema '{}', mutation {}; using content hash '{}'",
                    schema_name, mutation.uuid, short
                );
                key_value.hash = Some(short.to_string());
            }

            // Validate that the key matches the schema type.
            // A mismatch means a bug upstream — fail loudly.
            {
                use crate::schema::types::schema::DeclarativeSchemaType;
                match &schema.schema_type {
                    DeclarativeSchemaType::Hash if key_value.hash.is_none() => {
                        return Err(SchemaError::InvalidData(format!(
                            "Hash schema '{}' mutation {} has no hash key",
                            schema_name, mutation.uuid
                        )));
                    }
                    DeclarativeSchemaType::Range if key_value.range.is_none() => {
                        return Err(SchemaError::InvalidData(format!(
                            "Range schema '{}' mutation {} has no range key",
                            schema_name, mutation.uuid
                        )));
                    }
                    DeclarativeSchemaType::HashRange
                        if key_value.hash.is_none() || key_value.range.is_none() =>
                    {
                        return Err(SchemaError::InvalidData(format!(
                            "HashRange schema '{}' mutation {} requires both hash and range keys, got hash={:?} range={:?}",
                            schema_name, mutation.uuid, key_value.hash, key_value.range
                        )));
                    }
                    _ => {}
                }
            }

            mutation_key_values.push(key_value);

            // Create atoms in memory (no storage yet)
            for (field_name, value) in &mutation.fields_and_values {
                // Validate value against declared field type
                let field_type = schema.get_field_type(field_name);
                if let Err(type_err) = field_type.validate(value) {
                    return Err(SchemaError::InvalidData(format!(
                        "Type error in field '{}' of schema '{}': {}. Expected {}, got {}",
                        field_name,
                        schema_name,
                        type_err,
                        field_type,
                        serde_json::to_string(value).unwrap_or_else(|_| "?".to_string())
                    )));
                }

                let atom = DbOperations::create_atom(
                    schema_name,
                    value.clone(),
                    mutation.source_file_name.clone(),
                    mutation.metadata.clone(),
                );
                atoms_to_store.push(atom.clone());
                atom_results.push((idx, field_name.clone(), atom));
            }
        }

        // Batch store all atoms at once
        if !atoms_to_store.is_empty() {
            tracing::info!("💾 Batch storing {} atoms", atoms_to_store.len());
            self.db_ops
                .batch_store_atoms(atoms_to_store.clone(), schema.org_hash.as_deref())
                .await?;

            // Push to share prefixes for personal data
            if schema.org_hash.is_none() {
                if let Some(pool) = &self.sled_pool {
                    if let Ok(rules) = crate::sharing::store::list_share_rules(pool) {
                        for rule in rules {
                            if rule.active && rule.scope_matches(schema_name) {
                                self.db_ops
                                    .batch_store_atoms(
                                        atoms_to_store.clone(),
                                        Some(&rule.share_prefix),
                                    )
                                    .await?;
                            }
                        }
                    }
                }
            }
        }

        Ok((mutation_key_values, atom_results))
    }

    /// Ensures molecules are loaded from DB for fields that have molecule_uuid
    /// but no molecule in memory (e.g. after server restart or schema reload).
    async fn restore_missing_molecules(&self, schema: &mut Schema) {
        let fields_needing_refresh: Vec<String> = schema
            .runtime_fields
            .iter()
            .filter(|(_, field)| field.common().molecule_uuid().is_some() && !field.has_molecule())
            .map(|(name, _)| name.clone())
            .collect();

        for field_name in fields_needing_refresh {
            if let Some(field) = schema.runtime_fields.get_mut(&field_name) {
                field.refresh_from_db(&self.db_ops).await;
            }
        }
    }

    /// Applies atom writes to in-memory molecules, collecting mutation events
    /// and the set of modified field names for later persistence.
    fn apply_mutations_to_molecules(
        &self,
        schema: &mut Schema,
        schema_mutations: &[Mutation],
        mutation_key_values: &[KeyValue],
        atom_results: Vec<(usize, String, Atom)>,
    ) -> Result<(Vec<MutationEvent>, HashSet<String>), SchemaError> {
        let mut modified_fields = HashSet::new();
        let mut mutation_events: Vec<MutationEvent> = Vec::new();

        for (idx, field_name, atom) in atom_results {
            let mutation = &schema_mutations[idx];
            let key_value = &mutation_key_values[idx];

            let schema_field = schema.runtime_fields.get_mut(&field_name).ok_or_else(|| {
                SchemaError::InvalidData(format!(
                    "Field '{}' not found in runtime_fields for schema '{}'",
                    field_name, mutation.schema_name
                ))
            })?;

            // Capture old atom UUID before write_mutation
            let old_atom_uuid: Option<String> = match schema_field {
                FieldVariant::Single(f) => f
                    .base
                    .molecule
                    .as_ref()
                    .map(|m| m.get_atom_uuid().to_string()),
                FieldVariant::Hash(f) => key_value.hash.as_ref().and_then(|h| {
                    f.base
                        .molecule
                        .as_ref()
                        .and_then(|m| m.get_atom_uuid(h).cloned())
                }),
                FieldVariant::Range(f) => key_value.range.as_ref().and_then(|r| {
                    f.base
                        .molecule
                        .as_ref()
                        .and_then(|m| m.get_atom_uuid(r).cloned())
                }),
                FieldVariant::HashRange(f) => key_value
                    .hash
                    .as_ref()
                    .zip(key_value.range.as_ref())
                    .and_then(|(h, r)| {
                        f.base
                            .molecule
                            .as_ref()
                            .and_then(|m| m.get_atom_uuid(h, r).cloned())
                    }),
            };

            // Write mutation to memory
            schema_field.write_mutation(
                key_value,
                crate::schema::types::field::WriteContext {
                    atom: atom.clone(),
                    pub_key: mutation.pub_key.clone(),
                    source_file_name: mutation.source_file_name.clone(),
                    metadata: mutation.metadata.clone(),
                    schema_name: mutation.schema_name.clone(),
                    field_name: field_name.clone(),
                    signer: Arc::clone(&self.signer),
                },
            );

            // Build mutation event if something actually changed
            if old_atom_uuid.as_deref() != Some(atom.uuid()) {
                let field_key = match schema_field {
                    FieldVariant::Single(_) => FieldKey::Single,
                    FieldVariant::Hash(_) => FieldKey::Hash {
                        hash: key_value.hash.clone().unwrap_or_default(),
                    },
                    FieldVariant::Range(_) => FieldKey::Range {
                        range: key_value.range.clone().unwrap_or_default(),
                    },
                    FieldVariant::HashRange(_) => FieldKey::HashRange {
                        hash: key_value.hash.clone().unwrap_or_default(),
                        range: key_value.range.clone().unwrap_or_default(),
                    },
                };

                if let Some(mol_uuid) = schema_field.common().molecule_uuid() {
                    mutation_events.push(MutationEvent {
                        molecule_uuid: mol_uuid.to_string(),
                        timestamp: Utc::now(),
                        field_key,
                        old_atom_uuid,
                        new_atom_uuid: atom.uuid().to_string(),
                        version: schema_field.molecule_version().unwrap_or(0),
                        is_conflict: false,
                        conflict_loser_atom: None,
                        writer_pubkey: String::new(),
                        signature: String::new(),
                        // Propagate the originating mutation's provenance onto
                        // the event. If the mutation was submitted with a
                        // signature (`Some(Provenance::User{..})`), the event
                        // records it; otherwise `None`, matching pre-PR-5
                        // behavior.
                        provenance: mutation.provenance.clone(),
                    });
                }
            }

            // Track for persistence
            if schema_field.common().molecule_uuid().is_some() {
                modified_fields.insert(field_name.clone());
            }
        }

        Ok((mutation_events, modified_fields))
    }

    /// Batch-persists modified molecules and mutation events to storage.
    async fn persist_modified_molecules(
        &self,
        schema: &Schema,
        modified_fields: HashSet<String>,
        mutation_events: Vec<MutationEvent>,
        timing_breakdown: &mut HashMap<&str, std::time::Duration>,
    ) -> Result<(), SchemaError> {
        let mut molecules_to_store: Vec<(String, MoleculeData)> = Vec::new();

        for field_name in modified_fields {
            let schema_field = schema.runtime_fields.get(&field_name).expect(
                "field_name came from modified_fields which was populated from runtime_fields keys",
            );
            let molecule_uuid = schema_field.common().molecule_uuid().unwrap().to_string(); // verified is_some above

            if let Some(mol_data) = schema_field.clone_molecule_data() {
                molecules_to_store.push((molecule_uuid, mol_data));
            }
        }

        // Batch store all molecules at once
        if !molecules_to_store.is_empty() {
            tracing::info!("💾 Batch storing {} molecules", molecules_to_store.len());
            self.db_ops
                .batch_store_molecules(molecules_to_store.clone(), schema.org_hash.as_deref())
                .await?;
        }

        // Store mutation events for point-in-time queries
        if !mutation_events.is_empty() {
            let phase4_start = std::time::Instant::now();
            tracing::debug!("💾 Storing {} mutation events", mutation_events.len());
            self.db_ops
                .batch_store_mutation_events(mutation_events.clone(), schema.org_hash.as_deref())
                .await?;
            *timing_breakdown
                .entry("  - store_mutation_events")
                .or_insert(std::time::Duration::ZERO) += phase4_start.elapsed();
        }

        // Push to share prefixes for personal data
        if schema.org_hash.is_none() {
            if let Some(pool) = &self.sled_pool {
                if let Ok(rules) = crate::sharing::store::list_share_rules(pool) {
                    // MutationEvents are local-only and must NOT be shared
                    // across the wire (per cross-user sharing protocol —
                    // receiver namespace Q3). Only molecules get multiplexed.
                    for rule in rules {
                        if rule.active
                            && rule.scope_matches(&schema.name)
                            && !molecules_to_store.is_empty()
                        {
                            self.db_ops
                                .batch_store_molecules(
                                    molecules_to_store.clone(),
                                    Some(&rule.share_prefix),
                                )
                                .await?;
                        }
                    }
                }
            }
        }

        Ok(())
    }

    /// Embeds all mutations as document vectors (best-effort, logs errors).
    async fn inline_index_mutations(
        &self,
        schema_name: &str,
        schema_mutations: &[Mutation],
        mutation_key_values: &[KeyValue],
    ) {
        let native_index_mgr = match self.db_ops.native_index_manager() {
            Some(mgr) => mgr,
            None => return,
        };

        for (idx, mutation) in schema_mutations.iter().enumerate() {
            let key_value = &mutation_key_values[idx];
            if let Err(e) = native_index_mgr
                .index_record(schema_name, key_value, &mutation.fields_and_values)
                .await
            {
                warn!(
                    "Embedding indexing failed for schema '{}': {}",
                    schema_name, e
                );
            }
        }
    }

    /// Builds MutationExecuted events and collects mutation IDs for a schema group.
    fn build_batch_events(
        &self,
        schema_name: &str,
        schema_mutations: &[Mutation],
        mutation_key_values: Vec<KeyValue>,
        mol_versions: &HashSet<u64>,
    ) -> (Vec<(MutationExecuted, String)>, Vec<String>) {
        let mut events = Vec::new();
        let mut ids = Vec::new();

        for (idx, mutation) in schema_mutations.iter().enumerate() {
            let mutation_id = mutation.uuid.clone();
            let key_value = mutation_key_values[idx].clone();
            let data = mutation.fields_and_values.clone();
            let metadata = mutation.metadata.clone();

            let mutation_context = Some(crate::messaging::atom_events::MutationContext {
                key_value: Some(key_value),
                mutation_hash: Some(mutation_id.clone()),
                incremental: true,
            });

            let mol_versions_opt = if mol_versions.is_empty() {
                None
            } else {
                Some(mol_versions.clone())
            };

            let event = MutationExecuted {
                operation: "write_mutations_batch".to_string(),
                schema: schema_name.to_string(),
                execution_time_ms: 0,
                fields_affected: data.keys().cloned().collect(),
                mutation_context,
                data: Some(vec![data]), // Single data row for this mutation
                user_id: crate::logging::core::get_current_user_id(),
                molecule_versions: mol_versions_opt,
                metadata,
            };

            events.push((event, mutation_id.clone()));
            ids.push(mutation_id);
        }

        (events, ids)
    }

    /// Flushes the database, stores idempotency records, and publishes batch events.
    async fn finalize_batch(
        &self,
        hash_to_uuid: &[(String, String)],
        batch_events: Vec<(MutationExecuted, String)>,
        timing_breakdown: &mut HashMap<&str, std::time::Duration>,
    ) -> Result<(), SchemaError> {
        // Single flush for entire batch (async)
        tracing::debug!("💾 Flushing database after batch mutations");
        let flush_start = std::time::Instant::now();
        self.db_ops.flush().await.map_err(|e| {
            tracing::error!("❌ Failed to flush database after batch mutations: {}", e);
            SchemaError::InvalidData(format!("Flush failed: {}", e))
        })?;
        timing_breakdown.insert("flush", flush_start.elapsed());
        tracing::debug!("✅ Database flushed in {:?}", flush_start.elapsed());

        // Store idempotency entries for successfully processed mutations
        let idem_store_start = std::time::Instant::now();
        let idem_entries: Vec<(String, String)> = hash_to_uuid
            .iter()
            .map(|(hash, uuid)| (format!("idem:{}", hash), uuid.clone()))
            .collect();
        if !idem_entries.is_empty() {
            self.db_ops
                .metadata()
                .batch_put_idempotency(idem_entries)
                .await
                .map_err(|e| {
                    tracing::error!("Failed to store idempotency entries: {}", e);
                    e
                })?;
        }
        timing_breakdown.insert("idempotency_store", idem_store_start.elapsed());

        // Batch publish events
        let publish_start = std::time::Instant::now();
        self.publish_batch_events_async(batch_events).await?;
        timing_breakdown.insert("event_publish", publish_start.elapsed());

        Ok(())
    }

    /// Groups mutations by schema name for efficient batch processing
    fn group_mutations_by_schema(
        &self,
        mutations: Vec<Mutation>,
    ) -> HashMap<String, Vec<Mutation>> {
        let mut grouped: HashMap<String, Vec<Mutation>> = HashMap::new();

        for mutation in mutations {
            grouped
                .entry(mutation.schema_name.clone())
                .or_default()
                .push(mutation);
        }

        grouped
    }

    /// Publishes all batch events at once (Async)
    async fn publish_batch_events_async(
        &self,
        events: Vec<(MutationExecuted, String)>,
    ) -> Result<(), SchemaError> {
        for (event, _mutation_id) in events {
            self.message_bus
                .publish_mutation_executed(event)
                .await
                .map_err(|e| SchemaError::InvalidData(e.to_string()))?;
        }
        Ok(())
    }

    /// Start listening for MutationRequest events in a background thread
    pub async fn start_event_listener(&self, user_id: String) -> Result<(), SchemaError> {
        if self.is_listening.load(std::sync::atomic::Ordering::Acquire) {
            warn!("MutationManager event listener is already running");
            return Ok(());
        }

        let db_ops = Arc::clone(&self.db_ops);
        let schema_manager = Arc::clone(&self.schema_manager);
        let message_bus = Arc::clone(&self.message_bus);
        let view_orchestrator = Arc::clone(&self.view_orchestrator);
        let sled_pool = self.sled_pool.clone();
        let signer = Arc::clone(&self.signer);
        let is_listening = Arc::clone(&self.is_listening);

        is_listening.store(true, std::sync::atomic::Ordering::Release);

        // Use tokio::spawn for async background task
        tokio::spawn(async move {
            crate::logging::core::run_with_user(&user_id, async move {
                // Subscribe to MutationRequest events
                let mut consumer = message_bus.subscribe("MutationRequest").await;

                // Main event processing loop
                while is_listening.load(std::sync::atomic::Ordering::Acquire) {
                    // Async receive
                    if let Some(event) = consumer.recv().await {
                        match event {
                            Event::MutationRequest(mutation_request) => {
                                let temp_manager = Self::new(
                                    Arc::clone(&db_ops),
                                    Arc::clone(&schema_manager),
                                    Arc::clone(&message_bus),
                                    Arc::clone(&view_orchestrator),
                                    None,
                                    sled_pool.clone(),
                                    Arc::clone(&signer),
                                );

                                if let Err(e) = temp_manager
                                    .write_mutations_batch_async(vec![mutation_request
                                        .mutation
                                        .clone()])
                                    .await
                                {
                                    error!(
                                        "MutationManager failed to handle mutation request: {}",
                                        e
                                    );
                                }
                            }
                            _ => {
                                // Ignore other events if any leaked (shouldn't happen with filtered subscribe)
                            }
                        }
                    } else {
                        // Disconnected
                        error!("MutationManager message bus consumer disconnected");
                        break;
                    }
                }
            })
            .await
        });
        Ok(())
    }
}

#[async_trait::async_trait]
impl DerivedMutationWriter for MutationManager {
    async fn write_derived_batch(
        &self,
        mutations: Vec<Mutation>,
    ) -> Result<Vec<String>, SchemaError> {
        // Derived mutations skip the access-control wrapper because they are
        // produced internally by the view fire path — there is no user
        // identity to authorize against. The pipeline itself enforces the
        // `Provenance::Derived` pass-through in `ViewOrchestrator::redirect_mutation`,
        // so from here on they flow through exactly like any other batch.
        self.write_mutations_batch_async(mutations).await
    }
}

/// Highest `encoding_version` the runtime knows how to accept for derived
/// provenance. Bumped only when a new canonical byte layout for
/// `input_snapshot_hash` / Merkle leaves is introduced — see the docstring
/// on [`crate::atom::provenance::Provenance::Derived::encoding_version`].
const SUPPORTED_DERIVED_ENCODING_VERSION: u8 = 1;

/// Validate that a mutation carrying `Provenance::Derived` actually
/// corresponds to a registered WASM view whose current compiled bytes
/// hash to the provenance's `wasm_hash`.
///
/// Lightweight shape check — does NOT verify the Merkle root matches a
/// concrete source list (the on-molecule provenance only carries the root,
/// not the sources). The Merkle check happens at replay time via
/// [`crate::db_operations::LineageIndex::verify_merkle_consistency`] once
/// the local forward index has the stored sources.
///
/// Mutations without `Provenance::Derived` (user writes with
/// `Provenance::User { .. }` or `None`) are returned unchecked.
///
/// # Errors
///
/// - [`SchemaError::InvalidData`] if the target schema is not a registered
///   view, the view has no WASM transform (identity view), the wasm hash
///   does not match the view's compiled bytes, or the encoding version is
///   unsupported.
fn validate_derived_provenance(
    mutation: &Mutation,
    schema_manager: &SchemaCore,
) -> Result<(), SchemaError> {
    use crate::atom::provenance::Provenance;
    let (wasm_hash, encoding_version) = match &mutation.provenance {
        Some(Provenance::Derived {
            wasm_hash,
            encoding_version,
            ..
        }) => (wasm_hash.clone(), *encoding_version),
        _ => return Ok(()),
    };

    if encoding_version == 0 || encoding_version > SUPPORTED_DERIVED_ENCODING_VERSION {
        return Err(SchemaError::InvalidData(format!(
            "Derived mutation targets unsupported encoding_version {} (max supported: {})",
            encoding_version, SUPPORTED_DERIVED_ENCODING_VERSION
        )));
    }

    let view = {
        let registry = schema_manager.view_registry().lock().map_err(|_| {
            SchemaError::InvalidData(
                "Failed to acquire view_registry lock during provenance check".to_string(),
            )
        })?;
        registry.get_view(&mutation.schema_name).cloned()
    };

    let Some(view) = view else {
        return Err(SchemaError::InvalidData(format!(
            "Derived mutation targets schema '{}' which is not a registered view",
            mutation.schema_name
        )));
    };

    let Some(spec) = view.wasm_transform.as_ref() else {
        return Err(SchemaError::InvalidData(format!(
            "Derived mutation targets identity view '{}' (no WASM transform to derive from)",
            mutation.schema_name
        )));
    };

    let expected_hash = {
        let mut hasher = Sha256::new();
        hasher.update(&spec.bytes);
        format!("{:x}", hasher.finalize())
    };
    if expected_hash != wasm_hash {
        return Err(SchemaError::InvalidData(format!(
            "Derived mutation wasm_hash mismatch for view '{}': provenance carries '{}', view bytes hash to '{}'",
            mutation.schema_name, wasm_hash, expected_hash
        )));
    }

    Ok(())
}

#[cfg(test)]
mod derived_provenance_validator_tests {
    //! `validate_derived_provenance` is pure — no DB, no async. Drives a
    //! real `SchemaCore` with registered views through every rejection
    //! branch plus the happy path.

    use super::*;
    use crate::atom::provenance::Provenance;
    use crate::schema::types::field_value_type::FieldValueType;
    use crate::schema::types::operations::{MutationType, Query};
    use crate::schema::types::schema::DeclarativeSchemaType;
    use crate::schema::SchemaCore;
    use crate::view::types::{TransformView, WasmTransformSpec};
    use std::collections::HashMap;

    fn sha256_hex(bytes: &[u8]) -> String {
        let mut h = Sha256::new();
        h.update(bytes);
        format!("{:x}", h.finalize())
    }

    fn view_with_wasm(name: &str, wasm: Vec<u8>) -> TransformView {
        TransformView::new(
            name,
            DeclarativeSchemaType::Single,
            None,
            vec![Query::new("Src".to_string(), vec!["f".to_string()])],
            Some(WasmTransformSpec {
                bytes: wasm,
                max_gas: 1_000_000,
                gas_model: None,
            }),
            HashMap::from([("out".to_string(), FieldValueType::String)]),
        )
    }

    fn identity_view(name: &str) -> TransformView {
        TransformView::new(
            name,
            DeclarativeSchemaType::Single,
            None,
            vec![Query::new("Src".to_string(), vec!["f".to_string()])],
            None,
            HashMap::from([("f".to_string(), FieldValueType::Any)]),
        )
    }

    fn derived_mutation(schema_name: &str, wasm_hash: &str, encoding_version: u8) -> Mutation {
        Mutation {
            uuid: uuid::Uuid::new_v4().to_string(),
            schema_name: schema_name.to_string(),
            fields_and_values: HashMap::new(),
            key_value: KeyValue::new(None, None),
            pub_key: String::new(),
            mutation_type: MutationType::Create,
            synchronous: None,
            source_file_name: None,
            metadata: None,
            provenance: Some(Provenance::Derived {
                wasm_hash: wasm_hash.to_string(),
                input_snapshot_hash: "i".repeat(64),
                sources_merkle_root: "r".repeat(64),
                encoding_version,
            }),
        }
    }

    async fn core_with_view(view: TransformView) -> SchemaCore {
        use crate::test_helpers::TestSchemaBuilder;
        let core = SchemaCore::new_for_testing().await.unwrap();
        // `register_view` validates that every declared source schema exists
        // in the core. All fixtures in this module target a `Src` schema, so
        // register it once before the view.
        core.load_schema_from_json(&TestSchemaBuilder::new("Src").fields(&["f"]).build_json())
            .await
            .unwrap();
        core.register_view(view).await.unwrap();
        core
    }

    #[tokio::test]
    async fn user_mutation_is_not_validated() {
        // No provenance → ok regardless of whether schema exists.
        let core = SchemaCore::new_for_testing().await.unwrap();
        let mut m = derived_mutation("absent", "w", 1);
        m.provenance = None;
        validate_derived_provenance(&m, &core).expect("None provenance passes");

        m.provenance = Some(Provenance::user("pk".to_string(), "sig".to_string()));
        validate_derived_provenance(&m, &core).expect("User provenance passes");
    }

    #[tokio::test]
    async fn derived_with_unregistered_view_rejected() {
        let core = SchemaCore::new_for_testing().await.unwrap();
        let m = derived_mutation("Unregistered", &sha256_hex(b"w"), 1);
        let err = validate_derived_provenance(&m, &core).unwrap_err();
        assert!(
            format!("{}", err).contains("not a registered view"),
            "got: {}",
            err
        );
    }

    #[tokio::test]
    async fn derived_with_identity_view_rejected() {
        let core = core_with_view(identity_view("IdView")).await;
        let m = derived_mutation("IdView", &sha256_hex(b"w"), 1);
        let err = validate_derived_provenance(&m, &core).unwrap_err();
        assert!(format!("{}", err).contains("identity view"), "got: {}", err);
    }

    #[tokio::test]
    async fn derived_with_mismatched_wasm_hash_rejected() {
        let core = core_with_view(view_with_wasm("V", b"correct".to_vec())).await;
        // Provenance claims a different wasm hash than the view's actual bytes.
        let m = derived_mutation("V", &sha256_hex(b"wrong"), 1);
        let err = validate_derived_provenance(&m, &core).unwrap_err();
        assert!(
            format!("{}", err).contains("wasm_hash mismatch"),
            "got: {}",
            err
        );
    }

    #[tokio::test]
    async fn derived_with_zero_encoding_version_rejected() {
        let core = core_with_view(view_with_wasm("V", b"x".to_vec())).await;
        let m = derived_mutation("V", &sha256_hex(b"x"), 0);
        let err = validate_derived_provenance(&m, &core).unwrap_err();
        assert!(
            format!("{}", err).contains("unsupported encoding_version"),
            "got: {}",
            err
        );
    }

    #[tokio::test]
    async fn derived_with_future_encoding_version_rejected() {
        let core = core_with_view(view_with_wasm("V", b"x".to_vec())).await;
        let m = derived_mutation("V", &sha256_hex(b"x"), 2);
        let err = validate_derived_provenance(&m, &core).unwrap_err();
        assert!(
            format!("{}", err).contains("unsupported encoding_version"),
            "got: {}",
            err
        );
    }

    #[tokio::test]
    async fn derived_with_matching_wasm_hash_accepted() {
        let wasm = b"some wasm bytes".to_vec();
        let core = core_with_view(view_with_wasm("V", wasm.clone())).await;
        let m = derived_mutation("V", &sha256_hex(&wasm), 1);
        validate_derived_provenance(&m, &core).expect("matching hash should pass");
    }
}
