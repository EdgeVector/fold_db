//! Mutation Manager - Handles all mutation operations
//!
//! This module contains the MutationManager that handles the core mutation logic
//! previously located in FoldDB. It manages the execution of mutations, including
//! schema updates, atom persistence, and event publishing. It can also listen
//! for MutationRequest events and handle them automatically.

use std::collections::HashMap;
use std::sync::Arc;

use super::infrastructure::message_bus::events::query_events::MutationExecuted;

use super::infrastructure::message_bus::{AsyncMessageBus, Event};
use super::orchestration::index_status::IndexStatusTracker;
use chrono::Utc;
use crate::atom::{Atom, FieldKey, MutationEvent};
use crate::db_operations::{DbOperations, MoleculeData};
use crate::schema::types::field::{Field, FieldVariant};
use crate::schema::types::{KeyValue, Mutation};
use crate::schema::{SchemaCore, SchemaError};
use log::{debug, error, warn};
use std::collections::HashSet;

/// Manages mutation operations for the FoldDB system
pub struct MutationManager {
    /// Database operations for persistence
    db_ops: Arc<DbOperations>,
    /// Schema manager for schema operations
    schema_manager: Arc<SchemaCore>,
    /// Message bus for event publishing and listening
    message_bus: Arc<AsyncMessageBus>,
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
        index_status_tracker: Option<IndexStatusTracker>,
    ) -> Self {
        Self {
            db_ops,
            schema_manager,
            message_bus,
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

    /// Write multiple mutations in a batch for improved performance (async version)
    /// Groups mutations by schema to minimize schema reloads and uses true batching
    ///
    /// This is the preferred async version that avoids deadlocks.
    /// All storage operations use direct async/await instead of run_async.
    pub async fn write_mutations_batch_async(
        &mut self,
        mutations: Vec<Mutation>,
    ) -> Result<Vec<String>, SchemaError> {
        if mutations.is_empty() {
            return Ok(Vec::new());
        }

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

        // Group mutations by schema to minimize schema reloads
        let group_start = std::time::Instant::now();
        let grouped_mutations = self.group_mutations_by_schema(mutations);
        timing_breakdown.insert("grouping", group_start.elapsed());

        let mut mutation_ids = Vec::new();
        let mut batch_events = Vec::new();

        for (schema_name, schema_mutations) in grouped_mutations {
            // Load schema once for all mutations in this schema
            let load_start = std::time::Instant::now();
            let mut schema = self
                .schema_manager
                .get_schema(&schema_name)?
                .ok_or_else(|| {
                    SchemaError::InvalidData(format!("Schema '{}' not found", schema_name))
                })?;
            *timing_breakdown
                .entry("schema_load")
                .or_insert(std::time::Duration::ZERO) += load_start.elapsed();

            let mut mutation_contexts = Vec::new();
            let mut validation_time = std::time::Duration::ZERO;
            // let mut field_processing_time = std::time::Duration::ZERO; // No longer needed as phases track their own time
            // let mut refresh_time = std::time::Duration::ZERO; // No longer needed
            // let mut atom_time = std::time::Duration::ZERO; // No longer needed
            // let mut molecule_time = std::time::Duration::ZERO; // No longer needed
            // let mut index_time = std::time::Duration::ZERO;
            // Collect all index operations for batch processing - DEPRECATED/REMOVED
            // Indexing is now handled asynchronously via IndexOrchestrator subscribing to MutationExecuted events

            // Process all mutations for this schema with 3-phase optimization
            // Phase 1: Create atoms in memory, then batch store
            let phase1_start = std::time::Instant::now();
            let mut atoms_to_store: Vec<Atom> = Vec::new();
            let mut atom_results: Vec<(usize, String, Atom)> = Vec::new();

            // Pre-calculate key values for all mutations to avoid doing it in the inner loop
            let mut mutation_key_values = Vec::with_capacity(schema_mutations.len());

            for (idx, mutation) in schema_mutations.iter().enumerate() {
                let key_config = schema.key.clone();
                let key_value = KeyValue::from_mutation(
                    &mutation.fields_and_values,
                    key_config.as_ref().ok_or_else(|| {
                        SchemaError::InvalidData(format!(
                            "Schema '{}' has no key configuration. Cannot execute mutation.",
                            schema_name
                        ))
                    })?,
                );
                mutation_key_values.push(key_value);

                // Validate all field values against their topologies before processing
                let val_start = std::time::Instant::now();
                for (field_name, value) in &mutation.fields_and_values {
                    schema.validate_field_value(field_name, value)
                        .map_err(|e| {
                            SchemaError::InvalidData(format!(
                                "Topology validation failed for field '{}' in schema '{}': {}. Value received: {:?}",
                                field_name, mutation.schema_name, e, value
                            ))
                        })?;
                }
                validation_time += val_start.elapsed();

                // Create atoms in memory (no storage yet)
                for (field_name, value) in &mutation.fields_and_values {
                    let atom = DbOperations::create_atom(
                        &schema_name,
                        &mutation.pub_key,
                        value.clone(),
                        mutation.source_file_name.clone(),
                    );
                    atoms_to_store.push(atom.clone());
                    atom_results.push((idx, field_name.clone(), atom));
                }
            }

            // Batch store all atoms at once
            if !atoms_to_store.is_empty() {
                log::info!("💾 Batch storing {} atoms", atoms_to_store.len());
                self.db_ops.batch_store_atoms(atoms_to_store).await?;
            }

            *timing_breakdown
                .entry("  - create_atoms_batch")
                .or_insert(std::time::Duration::ZERO) += phase1_start.elapsed();

            // Phase 2: Serial Memory Update and Index Collection
            // Also collect molecule persistence tasks
            let phase2_start = std::time::Instant::now();
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
                    FieldVariant::Single(f) => {
                        f.base.molecule.as_ref().map(|m| m.get_atom_uuid().to_string())
                    }
                    FieldVariant::Range(f) => {
                        key_value.range.as_ref().and_then(|r| {
                            f.base.molecule.as_ref().and_then(|m| m.get_atom_uuid(r).cloned())
                        })
                    }
                    FieldVariant::HashRange(f) => {
                        key_value.hash.as_ref().zip(key_value.range.as_ref()).and_then(|(h, r)| {
                            f.base.molecule.as_ref().and_then(|m| m.get_atom_uuid(h, r).cloned())
                        })
                    }
                };

                // Write mutation to memory
                schema_field.write_mutation(key_value, atom.clone(), mutation.pub_key.clone());

                // Build mutation event if something actually changed
                if old_atom_uuid.as_deref() != Some(atom.uuid()) {
                    let field_key = match schema_field {
                        FieldVariant::Single(_) => FieldKey::Single,
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
                        });
                    }
                }

                // Track for persistence
                if schema_field.common().molecule_uuid().is_some() {
                    modified_fields.insert(field_name.clone());
                }
            }
            *timing_breakdown
                .entry("  - update_memory_serial")
                .or_insert(std::time::Duration::ZERO) += phase2_start.elapsed();

            // Phase 3: Batch Molecule Persistence
            let phase3_start = std::time::Instant::now();
            let mut molecules_to_store: Vec<(String, MoleculeData)> = Vec::new();

            for field_name in modified_fields {
                let schema_field = schema.runtime_fields.get(&field_name).unwrap();
                let molecule_uuid = schema_field.common().molecule_uuid().unwrap().to_string(); // verified is_some above

                match schema_field {
                    FieldVariant::Single(f) => {
                        if let Some(molecule) = f.base.molecule.clone() {
                            molecules_to_store.push((molecule_uuid, MoleculeData::Single(molecule)));
                        }
                    }
                    FieldVariant::Range(f) => {
                        if let Some(molecule) = f.base.molecule.clone() {
                            molecules_to_store.push((molecule_uuid, MoleculeData::Range(molecule)));
                        }
                    }
                    FieldVariant::HashRange(f) => {
                        if let Some(molecule) = f.base.molecule.clone() {
                            molecules_to_store.push((molecule_uuid, MoleculeData::HashRange(molecule)));
                        }
                    }
                }
            }

            // Batch store all molecules at once
            if !molecules_to_store.is_empty() {
                log::info!("💾 Batch storing {} molecules", molecules_to_store.len());
                self.db_ops.batch_store_molecules(molecules_to_store).await?;
            }

            *timing_breakdown
                .entry("  - write_molecules_batch")
                .or_insert(std::time::Duration::ZERO) += phase3_start.elapsed();

            // Phase 4: Store mutation events for point-in-time queries
            if !mutation_events.is_empty() {
                let phase4_start = std::time::Instant::now();
                log::debug!("💾 Storing {} mutation events", mutation_events.len());
                self.db_ops.batch_store_mutation_events(mutation_events).await?;
                *timing_breakdown
                    .entry("  - store_mutation_events")
                    .or_insert(std::time::Duration::ZERO) += phase4_start.elapsed();
            }

            // Collect molecule versions for each mutated field
            let mut mol_versions: HashMap<String, u64> = HashMap::new();
            for (field_name, schema_field) in &schema.runtime_fields {
                if let Some(v) = schema_field.molecule_version() {
                    mol_versions.insert(field_name.clone(), v);
                }
            }

            // Populate mutation contexts for events
            for (idx, mutation) in schema_mutations.iter().enumerate() {
                let mutation_id = mutation.uuid.clone();
                let backfill_hash = mutation.backfill_hash.clone();
                let key_value = mutation_key_values[idx].clone();
                let data = mutation.fields_and_values.clone();
                mutation_contexts.push((mutation_id, backfill_hash, key_value, data, mol_versions.clone()));
            }

            *timing_breakdown
                .entry("validation")
                .or_insert(std::time::Duration::ZERO) += validation_time;

            // Process batch index operations synchronously - REMOVED
            // Indexing now handled by IndexOrchestrator via event bus
            if false {
                // Kept empty block structure to minimize diff churn if needed, or just allow compiler to optimize out
            }

            // Indexing timing removed

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

            let reload_start = std::time::Instant::now();
            self.schema_manager.load_schema_internal(schema).await?;
            *timing_breakdown
                .entry("schema_reload")
                .or_insert(std::time::Duration::ZERO) += reload_start.elapsed();

            // Create events for batch publishing
            for (mutation_id, backfill_hash, key_value, data, versions) in mutation_contexts {
                let mutation_context = Some(crate::fold_db_core::infrastructure::message_bus::atom_events::MutationContext {
                    key_value: Some(key_value),
                    mutation_hash: Some(mutation_id.clone()),
                    incremental: true,
                    backfill_hash,
                });

                let mol_versions = if versions.is_empty() { None } else { Some(versions) };

                let event = MutationExecuted {
                    operation: "write_mutations_batch".to_string(),
                    schema: schema_name.clone(),
                    execution_time_ms: 0,
                    fields_affected: data.keys().cloned().collect(),
                    mutation_context,
                    data: Some(vec![data]), // Single data row for this mutation
                    user_id: crate::logging::core::get_current_user_id(),
                    molecule_versions: mol_versions,
                };

                batch_events.push((event, mutation_id.clone()));
                mutation_ids.push(mutation_id.clone());
            }
        }

        // Flush native index after all mutations in the batch - REMOVED
        // Handled by IndexOrchestrator
        let native_index_flush_start = std::time::Instant::now();
        timing_breakdown.insert("native_index_flush", native_index_flush_start.elapsed());

        // Single flush for entire batch (async)
        log::debug!("💾 Flushing database after batch mutations");
        let flush_start = std::time::Instant::now();
        self.db_ops.flush().await.map_err(|e| {
            log::error!("❌ Failed to flush database after batch mutations: {}", e);
            SchemaError::InvalidData(format!("Flush failed: {}", e))
        })?;
        timing_breakdown.insert("flush", flush_start.elapsed());
        log::debug!("✅ Database flushed in {:?}", flush_start.elapsed());

        // Batch publish events
        let publish_start = std::time::Instant::now();
        self.publish_batch_events_async(batch_events).await?;
        timing_breakdown.insert("event_publish", publish_start.elapsed());

        let total_time = start_time.elapsed();

        log::info!(
            "✅ write_mutations_batch_async: Completed {} mutations in {:.2}ms",
            mutation_ids.len(),
            total_time.as_millis()
        );

        // Log timing breakdown
        log::debug!(
            "Batch mutation timing breakdown (total: {:.2}ms):",
            total_time.as_millis()
        );
        let mut sorted_timings: Vec<_> = timing_breakdown.iter().collect();
        sorted_timings.sort_by(|a, b| b.1.cmp(a.1));
        for (operation, duration) in sorted_timings {
            let percentage = (duration.as_millis() as f64 / total_time.as_millis() as f64) * 100.0;
            debug!(
                "  - {}: {:.2}ms ({:.1}%)",
                operation,
                duration.as_millis(),
                percentage
            );
            log::debug!(
                "DEBUG: Step '{}': {:.2}ms ({:.1}%)",
                operation,
                duration.as_millis(),
                percentage
            );
        }

        Ok(mutation_ids)
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
    pub async fn start_event_listener(&self) -> Result<(), SchemaError> {
        if self.is_listening.load(std::sync::atomic::Ordering::Acquire) {
            warn!("MutationManager event listener is already running");
            return Ok(());
        }

        let db_ops = Arc::clone(&self.db_ops);
        let schema_manager = Arc::clone(&self.schema_manager);
        let message_bus = Arc::clone(&self.message_bus);
        let is_listening = Arc::clone(&self.is_listening);

        is_listening.store(true, std::sync::atomic::Ordering::Release);

        // Use tokio::spawn for async background task
        tokio::spawn(async move {
            // Subscribe to MutationRequest events
            let mut consumer = message_bus.subscribe("MutationRequest").await;

            // Main event processing loop
            while is_listening.load(std::sync::atomic::Ordering::Acquire) {
                // Async receive
                if let Some(event) = consumer.recv().await {
                    match event {
                        Event::MutationRequest(mutation_request) => {
                            let backfill_hash = mutation_request.mutation.backfill_hash.clone();

                            // Inline async handling instead of blocking call
                            let mut temp_manager = Self::new(
                                Arc::clone(&db_ops),
                                Arc::clone(&schema_manager),
                                Arc::clone(&message_bus),
                                None,
                            );

                            if let Err(e) = temp_manager
                                .write_mutations_batch_async(vec![mutation_request
                                    .mutation
                                    .clone()])
                                .await
                            {
                                error!("MutationManager failed to handle mutation request: {}", e);

                                // If this was part of a backfill, publish a failure event
                                if let Some(hash) = backfill_hash {
                                    let fail_event = crate::fold_db_core::infrastructure::message_bus::request_events::BackfillMutationFailed {
                                        backfill_hash: hash,
                                        error: e.to_string(),
                                    };
                                    let _ = message_bus
                                        .publish_event(Event::BackfillMutationFailed(fail_event))
                                        .await;
                                }
                            }
                        }
                        _ => {
                            // Ignore other events if any leaked (shouldn't happen with filtered subscribe??)
                            // Wait, subscribe("MutationRequest") filters? No, the implementation of subscribe does add_subscriber.
                            // But `AsyncConsumer` recv gets `Event` from channel.
                            // If `message_bus` routes correctly, we only get events sent to that topic.
                            // The `AsyncSubscriberRegistry` uses "MutationRequest" as key.
                            // Events published with `publish_event` check `event.event_type()`.
                            // So yes, strictly filtered.
                        }
                    }
                } else {
                    // Disconnected
                    error!("MutationManager message bus consumer disconnected");
                    break;
                }
            }
        });
        Ok(())
    }

    /// Stop the event listener
    pub fn stop_event_listener(&self) {
        self.is_listening
            .store(false, std::sync::atomic::Ordering::Release);
    }

    /// Check if the event listener is currently running
    pub fn is_listening(&self) -> bool {
        self.is_listening.load(std::sync::atomic::Ordering::Acquire)
    }
}
