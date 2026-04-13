//! Mutation Manager - Handles all mutation operations
//!
//! This module contains the MutationManager that handles the core mutation logic
//! previously located in FoldDB. It manages the execution of mutations, including
//! schema updates, atom persistence, and event publishing. It can also listen
//! for MutationRequest events and handle them automatically.

use std::collections::HashMap;
use std::sync::Arc;

use super::orchestration::index_status::IndexStatusTracker;
use super::view_invalidation::ViewInvalidationService;
use crate::atom::{Atom, FieldKey, MutationEvent};
use crate::db_operations::{DbOperations, MoleculeData};
use crate::messaging::events::query_events::MutationExecuted;
use crate::messaging::{AsyncMessageBus, Event};
use crate::schema::types::field::{Field, FieldVariant};
use crate::schema::types::{KeyValue, Mutation, Schema};
use crate::schema::{SchemaCore, SchemaError};
use chrono::Utc;
use log::{debug, error, warn};
use sha2::{Digest, Sha256};
use std::collections::HashSet;

/// Manages mutation operations for the FoldDB system
pub struct MutationManager {
    /// Database operations for persistence
    db_ops: Arc<DbOperations>,
    /// Schema manager for schema operations
    schema_manager: Arc<SchemaCore>,
    /// Message bus for event publishing and listening
    message_bus: Arc<AsyncMessageBus>,
    /// View lifecycle service (redirect/invalidate/precompute)
    view_invalidation_service: Arc<ViewInvalidationService>,
    /// Index status tracker for reporting indexing progress
    index_status_tracker: Option<IndexStatusTracker>,
    /// Flag to track if the event listener is running
    is_listening: Arc<std::sync::atomic::AtomicBool>,
}

impl MutationManager {
    /// Creates a new MutationManager instance
    pub fn new(
        db_ops: Arc<DbOperations>,
        schema_manager: Arc<SchemaCore>,
        message_bus: Arc<AsyncMessageBus>,
        view_invalidation_service: Arc<ViewInvalidationService>,
        index_status_tracker: Option<IndexStatusTracker>,
    ) -> Self {
        Self {
            db_ops,
            schema_manager,
            message_bus,
            view_invalidation_service,
            index_status_tracker,
            is_listening: Arc::new(std::sync::atomic::AtomicBool::new(false)),
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

        // Phase 0: Redirect identity view mutations to source schemas
        let mutations = self
            .view_invalidation_service
            .redirect_mutation(mutations)
            .await?;

        log::info!(
            "🔄 write_mutations_batch_async: Starting batch of {} mutations",
            mutations.len()
        );
        log::debug!(
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
            log::info!(
                "All {} mutations were idempotency duplicates, skipping processing",
                already_seen_ids.len()
            );
            return Ok(already_seen_ids);
        }

        log::debug!(
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

            // Phase 7.5: Invalidate dependent view caches
            let fields_affected: Vec<String> = schema_mutations
                .iter()
                .flat_map(|m| m.fields_and_values.keys().cloned())
                .collect::<HashSet<_>>()
                .into_iter()
                .collect();
            self.view_invalidation_service
                .invalidate_on_mutation(&schema_name, &fields_affected)
                .await?;
        }

        // Phase 8: Finalize — flush, store idempotency records, publish events
        self.finalize_batch(&hash_to_uuid, batch_events, &mut timing_breakdown)
            .await?;

        let total_time = start_time.elapsed();

        // Combine already-seen ids with newly processed mutation ids
        let mut all_ids = already_seen_ids;
        all_ids.extend(mutation_ids.iter().cloned());

        log::info!(
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
        log::debug!(
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
                    log::debug!(
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
                sorted.sort_by(|(a, _), (b, _)| a.cmp(b));
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
                    DeclarativeSchemaType::HashRange => {
                        if key_value.hash.is_none() || key_value.range.is_none() {
                            return Err(SchemaError::InvalidData(format!(
                                "HashRange schema '{}' mutation {} requires both hash and range keys, got hash={:?} range={:?}",
                                schema_name, mutation.uuid, key_value.hash, key_value.range
                            )));
                        }
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
                    &mutation.pub_key,
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
            log::info!("💾 Batch storing {} atoms", atoms_to_store.len());
            self.db_ops
                .batch_store_atoms(atoms_to_store, schema.org_hash.as_deref())
                .await?;
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
            log::info!("💾 Batch storing {} molecules", molecules_to_store.len());
            self.db_ops
                .batch_store_molecules(molecules_to_store, schema.org_hash.as_deref())
                .await?;
        }

        // Store mutation events for point-in-time queries
        if !mutation_events.is_empty() {
            let phase4_start = std::time::Instant::now();
            log::debug!("💾 Storing {} mutation events", mutation_events.len());
            self.db_ops
                .batch_store_mutation_events(mutation_events, schema.org_hash.as_deref())
                .await?;
            *timing_breakdown
                .entry("  - store_mutation_events")
                .or_insert(std::time::Duration::ZERO) += phase4_start.elapsed();
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
        log::debug!("💾 Flushing database after batch mutations");
        let flush_start = std::time::Instant::now();
        self.db_ops.flush().await.map_err(|e| {
            log::error!("❌ Failed to flush database after batch mutations: {}", e);
            SchemaError::InvalidData(format!("Flush failed: {}", e))
        })?;
        timing_breakdown.insert("flush", flush_start.elapsed());
        log::debug!("✅ Database flushed in {:?}", flush_start.elapsed());

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
                    log::error!("Failed to store idempotency entries: {}", e);
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
        let view_invalidation_service = Arc::clone(&self.view_invalidation_service);
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
                                    Arc::clone(&view_invalidation_service),
                                    None,
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
