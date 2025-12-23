//! Mutation Manager - Handles all mutation operations
//!
//! This module contains the MutationManager that handles the core mutation logic
//! previously located in FoldDB. It manages the execution of mutations, including
//! schema updates, atom persistence, and event publishing. It can also listen
//! for MutationRequest events and handle them automatically.

use std::collections::HashMap;
use std::sync::Arc;
use std::thread;
use std::time::Duration;

use super::infrastructure::message_bus::events::query_events::MutationExecuted;
use super::infrastructure::message_bus::request_events::MutationRequest;
use super::infrastructure::MessageBus;
use super::orchestration::index_status::IndexStatusTracker;
use crate::atom::Atom;
use crate::db_operations::DbOperations;
use crate::schema::types::field::{Field, FieldVariant};
use crate::schema::types::{KeyValue, Mutation};
use crate::schema::{SchemaCore, SchemaError};
use crate::storage::traits::TypedStore;
use futures_util::future::join_all;
use log::{debug, error, warn};
use std::collections::HashSet;
use std::future::Future;
use std::pin::Pin;

/// Manages mutation operations for the FoldDB system
pub struct MutationManager {
    /// Database operations for persistence
    db_ops: Arc<DbOperations>,
    /// Schema manager for schema operations
    schema_manager: Arc<SchemaCore>,
    /// Message bus for event publishing and listening
    message_bus: Arc<MessageBus>,
    /// Index status tracker for reporting indexing progress
    index_status_tracker: Option<IndexStatusTracker>,
    /// Flag to track if the event listener is running
    is_listening: Arc<std::sync::atomic::AtomicBool>,
}

impl MutationManager {
    /// Helper to run async code from sync context, handling both cases where we're
    /// already in a runtime (use block_in_place) or not (create new runtime)
    ///
    /// # Deprecated
    /// This helper is deprecated. All storage operations are now async, so this
    /// sync wrapper is no longer needed. Use async methods directly.
    ///
    /// WARNING: When called from spawn_blocking context, this can deadlock.
    /// The issue is that block_on inside spawn_blocking can deadlock if the runtime is busy.
    /// Prefer using async methods directly instead of this helper.
    fn run_async<F, T>(future: F) -> Result<T, SchemaError>
    where
        F: std::future::Future<Output = Result<T, SchemaError>>,
    {
        // Check if we're in a blocking context (spawn_blocking)
        // This can cause deadlocks when calling async operations
        let is_blocking = std::thread::current()
            .name()
            .map(|n| n.contains("tokio-worker"))
            .unwrap_or(false);

        match tokio::runtime::Handle::try_current() {
            Ok(handle) => {
                // We're already in a runtime
                // If we're in spawn_blocking, block_in_place + block_on can deadlock
                // Prefer using async methods directly instead of this helper
                if is_blocking {
                    log::warn!("⚠️ run_async called from blocking context - this may deadlock");
                }

                // Use block_in_place to avoid nested runtime error
                // NOTE: This can still deadlock if called from spawn_blocking with a busy runtime
                tokio::task::block_in_place(|| {
                    // Use a timeout to detect potential deadlocks
                    let timeout_duration = std::time::Duration::from_secs(60);
                    log::debug!(
                        "⏱️ run_async: Starting with {}s timeout",
                        timeout_duration.as_secs()
                    );
                    let start = std::time::Instant::now();
                    let result = handle
                        .block_on(async { tokio::time::timeout(timeout_duration, future).await });
                    let elapsed = start.elapsed();
                    log::debug!("⏱️ run_async: Completed in {:?}", elapsed);
                    match result {
                        Ok(Ok(result)) => {
                            log::debug!("✅ run_async: Operation succeeded");
                            Ok(result)
                        }
                        Ok(Err(e)) => {
                            log::error!("❌ run_async: Operation failed: {}", e);
                            Err(e)
                        }
                        Err(_) => {
                            log::error!("❌ run_async: Operation timed out after {:?} - possible deadlock in spawn_blocking context", timeout_duration);
                            Err(SchemaError::InvalidData(
                                format!("Operation timed out after {:?} - possible deadlock in spawn_blocking context. Use async methods directly instead.", timeout_duration)
                            ))
                        }
                    }
                })
            }
            Err(_) => {
                log::debug!("🔄 run_async: No runtime handle, creating new runtime");
                // No runtime, create one
                tokio::runtime::Runtime::new()
                    .map_err(|e| {
                        SchemaError::InvalidData(format!("Failed to create runtime: {}", e))
                    })?
                    .block_on(future)
            }
        }
    }

    /// Creates a new MutationManager instance
    pub fn new(
        db_ops: Arc<DbOperations>,
        schema_manager: Arc<SchemaCore>,
        message_bus: Arc<MessageBus>,
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

    /// Write schema operation - main orchestration method for mutations
    ///
    /// # Deprecated
    /// Use `write_mutations_batch()` instead for better performance.
    /// Single mutations cause a flush-per-operation, while batching allows a single flush.
    #[deprecated(
        since = "0.1.0",
        note = "Use write_mutations_batch() instead for better performance"
    )]
    #[allow(deprecated)]
    pub fn write_mutation(&mut self, mutation: Mutation) -> Result<String, SchemaError> {
        let start_time = std::time::Instant::now();
        log::info!(
            "🔄 write_mutation: Starting mutation for schema '{}', mutation_id: {}",
            mutation.schema_name,
            mutation.uuid
        );

        // Capture backfill_hash before mutation is consumed
        let backfill_hash = mutation.backfill_hash.clone();

        // Get the schema definition
        log::debug!("📋 Getting schema: {}", mutation.schema_name);
        let mut schema = self
            .schema_manager
            .get_schema(&mutation.schema_name)?
            .ok_or_else(|| {
                log::error!("❌ Schema '{}' not found", mutation.schema_name);
                SchemaError::InvalidData(format!("Schema '{}' not found", mutation.schema_name))
            })?;
        log::debug!("✅ Schema found: {}", mutation.schema_name);

        let key_config = schema.key.clone();
        let key_value =
            KeyValue::from_mutation(&mutation.fields_and_values, key_config.as_ref().unwrap());
        let mutation_id = mutation.uuid.clone();

        // Validate all field values against their topologies before processing
        log::debug!("🔍 Validating {} fields", mutation.fields_and_values.len());
        for (field_name, value) in &mutation.fields_and_values {
            schema
                .validate_field_value(field_name, value)
                .map_err(|e| {
                    log::error!("❌ Field validation failed for '{}': {}", field_name, e);
                    e
                })?;
        }
        log::debug!("✅ All fields validated");

        // Process each field in the mutation
        let fields_affected: Vec<String> = mutation.fields_and_values.keys().cloned().collect();
        for (field_name, value) in mutation.fields_and_values {
            // Get field classifications BEFORE mutable borrow
            let field_classifications = schema.get_field_classifications(&field_name);

            if let Some(schema_field) = schema.runtime_fields.get_mut(&field_name) {
                // NOTE: process_mutation_field_with_schema is deprecated v1 method
                // For now, we'll handle the mutation inline with v2 async methods

                // Create and store atom
                log::debug!("⚛️ Creating atom for field '{}'", field_name);
                let new_atom =
                    Self::run_async(self.db_ops.create_and_store_atom_for_mutation_deferred(
                        &mutation.schema_name,
                        &mutation.pub_key,
                        value.clone(),
                        None, // source_file_name
                    ))
                    .map_err(|e| {
                        log::error!("❌ Failed to create atom for field '{}': {}", field_name, e);
                        e
                    })?;
                log::debug!("✅ Atom created: {}", new_atom.uuid());

                // Write mutation to field
                schema_field.write_mutation(&key_value, new_atom.clone(), mutation.pub_key.clone());
                log::debug!("✅ Mutation written to field '{}'", field_name);

                // Persist molecule if present
                if let Some(molecule_uuid) = schema_field.common().molecule_uuid() {
                    log::info!(
                        "🔗 Sync path: Persisting molecule for field '{}': uuid={}",
                        field_name,
                        molecule_uuid
                    );
                    Self::run_async(
                        self.db_ops
                            .persist_field_molecule_deferred(schema_field, molecule_uuid),
                    )
                    .map_err(|e| {
                        log::error!("❌ Failed to persist molecule '{}': {}", molecule_uuid, e);
                        e
                    })?;
                    log::info!(
                        "✅ Sync path: Molecule persisted for field '{}'",
                        field_name
                    );
                } else {
                    log::warn!(
                        "⚠️ Sync path: No molecule_uuid set for field '{}' after write_mutation",
                        field_name
                    );
                }

                // Index the field value with classifications
                if let Some(native_index_mgr) = self.db_ops.native_index_manager() {
                    // Use batch indexing with a single item
                    let single_operation = vec![(
                        mutation.schema_name.clone(),
                        field_name.clone(),
                        key_value.clone(),
                        value.clone(),
                        field_classifications,
                    )];

                    // Update index status
                    if let Some(tracker) = &self.index_status_tracker {
                        let _ = Self::run_async(async {
                            tracker.start_batch(1).await;
                            Ok(())
                        });
                    }

                    let index_start = std::time::Instant::now();
                    let result = native_index_mgr
                        .batch_index_field_values_with_classifications(&single_operation);

                    // Update index status completion
                    if let Some(tracker) = &self.index_status_tracker {
                        let _ = Self::run_async(async {
                            tracker
                                .complete_batch(1, index_start.elapsed().as_millis())
                                .await;
                            Ok(())
                        });
                    }

                    result?;
                }
            } else {
                return Err(SchemaError::InvalidData(format!(
                    "Field '{}' not found in runtime_fields for schema '{}'. Available fields: {:?}",
                    field_name,
                    mutation.schema_name,
                    schema.runtime_fields.keys().collect::<Vec<_>>()
                )));
            }
        }

        // Flush native index after all field operations (single mutation path only)
        if let Some(native_index_mgr) = self.db_ops.native_index_manager() {
            native_index_mgr.flush()?;
        }

        // Sync molecule UUIDs to the persisted field before storing
        schema.sync_molecule_uuids();

        // Persist the updated schema back to the database and schema_manager
        let schema_name = schema.name.clone();
        let schema_name_for_log = schema_name.clone();
        log::debug!("💾 Persisting schema: {}", schema_name_for_log);
        let db_ops = self.db_ops.clone();
        let schema_manager = self.schema_manager.clone();
        let schema_name_for_error = schema_name.clone();
        Self::run_async(async move {
            db_ops.store_schema(&schema_name, &schema).await?;
            schema_manager.load_schema_internal(schema).await
        })
        .map_err(|e| {
            log::error!(
                "❌ Failed to persist schema '{}': {}",
                schema_name_for_error,
                e
            );
            e
        })?;
        log::debug!("✅ Schema persisted: {}", schema_name_for_log);

        // Calculate execution time
        let execution_time_ms = start_time.elapsed().as_millis() as u64;

        // Create mutation context for transform execution
        let mutation_context = Some(
            crate::fold_db_core::infrastructure::message_bus::atom_events::MutationContext {
                key_value: Some(key_value.clone()),
                mutation_hash: Some(mutation_id.clone()),
                incremental: true,
                backfill_hash: backfill_hash.clone(), // Preserve backfill_hash from mutation
            },
        );

        // Publish MutationExecuted event to trigger transforms
        let event = MutationExecuted::with_context(
            "write_mutation",
            mutation.schema_name.clone(),
            execution_time_ms,
            fields_affected,
            mutation_context,
        );

        log::debug!("📢 Publishing MutationExecuted event");
        self.message_bus.publish(event).map_err(|e| {
            log::error!("❌ Failed to publish mutation event: {}", e);
            e
        })?;

        // Flush database to ensure mutation is persisted to disk
        // Note: This is in the deprecated sync write_mutation method
        // The async version uses flush().await directly
        log::debug!("💾 Flushing database (sync path - deprecated)");
        self.db_ops.flush_sync().map_err(|e| {
            log::error!("❌ Failed to flush database: {}", e);
            e
        })?;
        log::debug!("✅ Database flushed");

        // Return the mutation ID
        log::info!(
            "✅ write_mutation: Completed successfully, mutation_id: {}",
            mutation_id
        );
        Ok(mutation_id)
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
        println!(
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
            let mut index_time = std::time::Duration::ZERO;

            // Collect all index operations for batch processing
            let mut index_operations = Vec::new();

            // Process all mutations for this schema with 3-phase optimization
            // Phase 1: Concurrent Atom Creation
            let phase1_start = std::time::Instant::now();
            let mut atom_tasks = Vec::new();

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

                // Collect atom creation tasks
                for (field_name, value) in &mutation.fields_and_values {
                    let db_ops = self.db_ops.clone();
                    let s_name = schema_name.clone();
                    let p_key = mutation.pub_key.clone();
                    let val = value.clone();
                    let src = mutation.source_file_name.clone();
                    let f_name = field_name.clone();

                    atom_tasks.push(async move {
                        let atom = db_ops
                            .create_and_store_atom_for_mutation_deferred(&s_name, &p_key, val, src)
                            .await?;
                        Ok::<_, SchemaError>((idx, f_name, atom))
                    });
                }
            }

            // Execute Phase 1
            let atom_results_raw = join_all(atom_tasks).await;
            let atom_results: Vec<(usize, String, Atom)> = atom_results_raw
                .into_iter()
                .collect::<Result<_, SchemaError>>()?;
            *timing_breakdown
                .entry("  - create_atoms_parallel")
                .or_insert(std::time::Duration::ZERO) += phase1_start.elapsed();

            // Phase 2: Serial Memory Update and Index Collection
            // Also collect molecule persistence tasks
            let phase2_start = std::time::Instant::now();
            let mut modified_fields = HashSet::new();

            for (idx, field_name, atom) in atom_results {
                let mutation = &schema_mutations[idx];
                let key_value = &mutation_key_values[idx];
                let value = mutation
                    .fields_and_values
                    .get(&field_name)
                    .ok_or_else(|| SchemaError::InvalidData("Field value missing".to_string()))?;

                // Get field classifications before mutable borrow
                let field_classifications = schema.get_field_classifications(&field_name);

                let schema_field = schema.runtime_fields.get_mut(&field_name).ok_or_else(|| {
                    SchemaError::InvalidData(format!(
                        "Field '{}' not found in runtime_fields for schema '{}'",
                        field_name, mutation.schema_name
                    ))
                })?;

                // Write mutation to memory
                schema_field.write_mutation(key_value, atom, mutation.pub_key.clone());

                // Track for persistence
                if schema_field.common().molecule_uuid().is_some() {
                    modified_fields.insert(field_name.clone());
                }

                // Collect index op
                index_operations.push((
                    mutation.schema_name.clone(),
                    field_name,
                    key_value.clone(),
                    value.clone(),
                    field_classifications,
                ));
            }
            *timing_breakdown
                .entry("  - update_memory_serial")
                .or_insert(std::time::Duration::ZERO) += phase2_start.elapsed();

            // Phase 3: Concurrent Molecule Persistence
            let phase3_start = std::time::Instant::now();
            let mut persist_tasks: Vec<
                Pin<Box<dyn Future<Output = Result<(), SchemaError>> + Send>>,
            > = Vec::new();

            for field_name in modified_fields {
                let schema_field = schema.runtime_fields.get(&field_name).unwrap();
                let molecule_uuid = schema_field.common().molecule_uuid().unwrap().to_string(); // verified is_some above
                let ref_key = format!("ref:{}", molecule_uuid);
                let db_ops = self.db_ops.clone();

                match schema_field {
                    FieldVariant::Single(f) => {
                        if let Some(molecule) = f.base.molecule.clone() {
                            persist_tasks.push(Box::pin(async move {
                                db_ops
                                    .molecules_store()
                                    .put_item(&ref_key, &molecule)
                                    .await
                                    .map_err(|e| {
                                        SchemaError::InvalidData(format!(
                                            "Failed to store molecule: {}",
                                            e
                                        ))
                                    })
                            }));
                        }
                    }
                    FieldVariant::Range(f) => {
                        if let Some(molecule) = f.base.molecule.clone() {
                            persist_tasks.push(Box::pin(async move {
                                db_ops
                                    .molecules_store()
                                    .put_item(&ref_key, &molecule)
                                    .await
                                    .map_err(|e| {
                                        SchemaError::InvalidData(format!(
                                            "Failed to store molecule: {}",
                                            e
                                        ))
                                    })
                            }));
                        }
                    }
                    FieldVariant::HashRange(f) => {
                        if let Some(molecule) = f.base.molecule.clone() {
                            persist_tasks.push(Box::pin(async move {
                                db_ops
                                    .molecules_store()
                                    .put_item(&ref_key, &molecule)
                                    .await
                                    .map_err(|e| {
                                        SchemaError::InvalidData(format!(
                                            "Failed to store molecule: {}",
                                            e
                                        ))
                                    })
                            }));
                        }
                    }
                }
            }

            let persist_results_raw = join_all(persist_tasks).await;
            for res in persist_results_raw {
                res?; // Propagate any errors from molecule persistence
            }

            *timing_breakdown
                .entry("  - write_molecules_parallel")
                .or_insert(std::time::Duration::ZERO) += phase3_start.elapsed();

            // Populate mutation contexts for events
            for (idx, mutation) in schema_mutations.iter().enumerate() {
                let mutation_id = mutation.uuid.clone();
                let backfill_hash = mutation.backfill_hash.clone();
                let key_value = mutation_key_values[idx].clone();
                mutation_contexts.push((mutation_id, backfill_hash, key_value));
            }

            *timing_breakdown
                .entry("validation")
                .or_insert(std::time::Duration::ZERO) += validation_time;

            // Process batch index operations synchronously
            if !index_operations.is_empty() {
                let index_start = std::time::Instant::now();
                debug!(
                    "MutationManager: Processing batch index with {} operations for schema '{}'",
                    index_operations.len(),
                    schema_name
                );

                // Call batch indexing directly for synchronous processing
                if let Some(native_index_mgr) = self.db_ops.native_index_manager() {
                    // Update index status
                    if let Some(tracker) = &self.index_status_tracker {
                        tracker.start_batch(index_operations.len()).await;
                    }

                    let batch_index_start = std::time::Instant::now();

                    let result = if native_index_mgr.is_async() {
                        native_index_mgr
                            .batch_index_field_values_with_classifications_async(&index_operations)
                            .await
                    } else {
                        native_index_mgr
                            .batch_index_field_values_with_classifications(&index_operations)
                    };

                    // Update index status completion
                    if let Some(tracker) = &self.index_status_tracker {
                        tracker
                            .complete_batch(
                                index_operations.len(),
                                batch_index_start.elapsed().as_millis(),
                            )
                            .await;
                    }

                    result?;
                }

                debug!(
                    "MutationManager: Successfully processed batch index for schema '{}'",
                    schema_name
                );
                index_time += index_start.elapsed();
            } else {
                debug!(
                    "MutationManager: No index operations to process for schema '{}'",
                    schema_name
                );
            }

            *timing_breakdown
                .entry("  - index_fields")
                .or_insert(std::time::Duration::ZERO) += index_time;

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
            for (mutation_id, backfill_hash, key_value) in mutation_contexts {
                let mutation_context = Some(crate::fold_db_core::infrastructure::message_bus::atom_events::MutationContext {
                    key_value: Some(key_value),
                    mutation_hash: Some(mutation_id.clone()),
                    incremental: true,
                    backfill_hash,
                });

                let event = MutationExecuted::with_context(
                    "write_mutations_batch",
                    schema_name.clone(),
                    0,      // Execution time will be calculated for the batch
                    vec![], // Fields affected - could be populated if needed
                    mutation_context,
                );

                batch_events.push((event, mutation_id.clone()));
                mutation_ids.push(mutation_id.clone());
            }
        }

        // Flush native index after all mutations in the batch
        let native_index_flush_start = std::time::Instant::now();
        if let Some(native_index_mgr) = self.db_ops.native_index_manager() {
            native_index_mgr.flush()?;
        }
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
        self.publish_batch_events(batch_events)?;
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
            println!(
                "DEBUG: Step '{}': {:.2}ms ({:.1}%)",
                operation,
                duration.as_millis(),
                percentage
            );
        }

        Ok(mutation_ids)
    }

    /// Write multiple mutations in a batch for improved performance (sync version)
    /// Groups mutations by schema to minimize schema reloads and uses true batching
    ///
    /// # Deprecated
    /// This sync version uses `block_on` which can deadlock.
    /// Use `write_mutations_batch_async()` instead.
    #[deprecated(note = "Use write_mutations_batch_async() to avoid deadlocks")]
    pub fn write_mutations_batch(
        &mut self,
        mutations: Vec<Mutation>,
    ) -> Result<Vec<String>, SchemaError> {
        // For backward compatibility, call async version from sync context
        // This uses block_on which can deadlock, but maintains compatibility
        match tokio::runtime::Handle::try_current() {
            Ok(handle) => tokio::task::block_in_place(|| {
                handle.block_on(self.write_mutations_batch_async(mutations))
            }),
            Err(_) => {
                // No runtime, create one
                tokio::runtime::Runtime::new()
                    .map_err(|e| {
                        SchemaError::InvalidData(format!("Failed to create runtime: {}", e))
                    })?
                    .block_on(self.write_mutations_batch_async(mutations))
            }
        }
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

    /// Publishes all batch events at once
    fn publish_batch_events(
        &self,
        events: Vec<(MutationExecuted, String)>,
    ) -> Result<(), SchemaError> {
        for (event, _mutation_id) in events {
            self.message_bus.publish(event)?;
        }
        Ok(())
    }

    /// Start listening for MutationRequest events in a background thread
    pub fn start_event_listener(&self) -> Result<(), SchemaError> {
        if self.is_listening.load(std::sync::atomic::Ordering::Acquire) {
            warn!("MutationManager event listener is already running");
            return Ok(());
        }

        let db_ops = Arc::clone(&self.db_ops);
        let schema_manager = Arc::clone(&self.schema_manager);
        let message_bus = Arc::clone(&self.message_bus);
        let is_listening = Arc::clone(&self.is_listening);

        is_listening.store(true, std::sync::atomic::Ordering::Release);

        thread::spawn(move || {
            // Subscribe to MutationRequest events
            let mut consumer = message_bus.subscribe::<MutationRequest>();

            // Main event processing loop
            while is_listening.load(std::sync::atomic::Ordering::Acquire) {
                match consumer.try_recv() {
                    Ok(mutation_request) => {
                        let backfill_hash = mutation_request.mutation.backfill_hash.clone();
                        if let Err(e) = Self::handle_mutation_request_event(
                            &mutation_request,
                            &db_ops,
                            &schema_manager,
                            &message_bus,
                        ) {
                            error!("MutationManager failed to handle mutation request: {}", e);

                            // If this was part of a backfill, publish a failure event
                            if let Some(hash) = backfill_hash {
                                let fail_event = crate::fold_db_core::infrastructure::message_bus::request_events::BackfillMutationFailed {
                                    backfill_hash: hash,
                                    error: e.to_string(),
                                };
                                let _ = message_bus.publish(fail_event);
                            }
                        }
                    }
                    Err(std::sync::mpsc::TryRecvError::Empty) => {
                        // No events available, sleep briefly to avoid busy waiting
                        thread::sleep(Duration::from_millis(10));
                    }
                    Err(std::sync::mpsc::TryRecvError::Disconnected) => {
                        error!("MutationManager message bus consumer disconnected");
                        break;
                    }
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

    /// Handle a mutation request event by executing the mutation
    ///
    /// Note: This is a legacy async event handler that processes single mutations.
    /// For better performance, use the batch mutation APIs directly.
    #[allow(deprecated)]
    fn handle_mutation_request_event(
        mutation_request: &MutationRequest,
        db_ops: &Arc<DbOperations>,
        schema_manager: &Arc<SchemaCore>,
        message_bus: &MessageBus,
    ) -> Result<(), SchemaError> {
        let start_time = std::time::Instant::now();

        // Get the schema definition
        let mut schema = schema_manager
            .get_schema(&mutation_request.mutation.schema_name)?
            .ok_or_else(|| {
                SchemaError::InvalidData(format!(
                    "Schema '{}' not found",
                    mutation_request.mutation.schema_name
                ))
            })?;

        let key_config = schema.key.clone();
        let key_value = KeyValue::from_mutation(
            &mutation_request.mutation.fields_and_values,
            key_config.as_ref().unwrap(),
        );
        let mutation_id = mutation_request.mutation.uuid.clone();

        // Validate all field values against their topologies before processing
        for (field_name, value) in &mutation_request.mutation.fields_and_values {
            schema.validate_field_value(field_name, value)?;
        }

        // Process each field in the mutation
        let fields_affected: Vec<String> = mutation_request
            .mutation
            .fields_and_values
            .keys()
            .cloned()
            .collect();
        for (field_name, value) in mutation_request.mutation.fields_and_values.clone() {
            // Get field classifications BEFORE mutable borrow
            let field_classifications = schema.get_field_classifications(&field_name);

            if let Some(schema_field) = schema.runtime_fields.get_mut(&field_name) {
                // NOTE: process_mutation_field_with_schema is deprecated v1 method
                // Handle mutation inline with v2 async methods

                // Create and store atom
                let new_atom =
                    Self::run_async(db_ops.create_and_store_atom_for_mutation_deferred(
                        &mutation_request.mutation.schema_name,
                        &mutation_request.mutation.pub_key,
                        value.clone(),
                        None,
                    ))?;

                // Write mutation to field
                schema_field.write_mutation(
                    &key_value,
                    new_atom.clone(),
                    mutation_request.mutation.pub_key.clone(),
                );

                // Persist molecule if present
                if let Some(molecule_uuid) = schema_field.common().molecule_uuid() {
                    Self::run_async(
                        db_ops.persist_field_molecule_deferred(schema_field, molecule_uuid),
                    )?;
                }

                // Index the field value
                if let Some(native_index_mgr) = db_ops.native_index_manager() {
                    // Use batch indexing with a single item
                    let single_operation = vec![(
                        mutation_request.mutation.schema_name.clone(),
                        field_name.clone(),
                        key_value.clone(),
                        value.clone(),
                        field_classifications,
                    )];

                    // Note: This static method doesn't have access to self.index_status_tracker
                    // We would need to pass it in or change the method signature
                    // For now, we'll skip tracking for this legacy path or we could pass it if we change signature
                    native_index_mgr
                        .batch_index_field_values_with_classifications(&single_operation)?;
                }
            } else {
                return Err(SchemaError::InvalidData(format!(
                    "Field '{}' not found in runtime_fields for schema '{}'. Available fields: {:?}",
                    field_name,
                    mutation_request.mutation.schema_name,
                    schema.runtime_fields.keys().collect::<Vec<_>>()
                )));
            }
        }

        // Flush native index after all field operations (legacy event handler path)
        if let Some(native_index_mgr) = db_ops.native_index_manager() {
            native_index_mgr.flush()?;
        }

        // Sync molecule UUIDs to the persisted field before storing
        schema.sync_molecule_uuids();

        // Persist the updated schema back to the database and schema_manager
        let schema_name = schema.name.clone();
        Self::run_async(async move {
            db_ops.store_schema(&schema_name, &schema).await?;
            schema_manager.load_schema_internal(schema).await
        })?;

        // Calculate execution time
        let execution_time_ms = start_time.elapsed().as_millis() as u64;

        // Create mutation context for transform execution
        let mutation_context = Some(
            crate::fold_db_core::infrastructure::message_bus::atom_events::MutationContext {
                key_value: Some(key_value.clone()),
                mutation_hash: Some(mutation_id.clone()),
                incremental: true,
                backfill_hash: mutation_request.mutation.backfill_hash.clone(), // Pass through backfill_hash
            },
        );

        // Publish MutationExecuted event to trigger transforms
        let event = MutationExecuted::with_context(
            "mutation_request_handler",
            mutation_request.mutation.schema_name.clone(),
            execution_time_ms,
            fields_affected,
            mutation_context,
        );

        message_bus.publish(event)?;

        // Flush database to ensure mutation is persisted to disk
        // Note: This is in a sync event handler - consider making it async
        // For now, use deprecated flush_sync for backward compatibility
        db_ops.flush_sync()?;

        Ok(())
    }
}
