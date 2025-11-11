//! Mutation Manager - Handles all mutation operations
//!
//! This module contains the MutationManager that handles the core mutation logic
//! previously located in FoldDB. It manages the execution of mutations, including
//! schema updates, atom persistence, and event publishing. It can also listen
//! for MutationRequest events and handle them automatically.

use std::sync::Arc;
use std::thread;
use std::time::Duration;
use std::collections::HashMap;

use crate::db_operations::DbOperations;
use crate::schema::types::{Mutation, KeyValue};
use crate::schema::types::field::Field;
use crate::schema::{SchemaCore, SchemaError};
use super::infrastructure::message_bus::events::query_events::MutationExecuted;
use super::infrastructure::message_bus::request_events::MutationRequest;
use super::infrastructure::MessageBus;
use log::{debug, error, warn};

/// Manages mutation operations for the FoldDB system
pub struct MutationManager {
    /// Database operations for persistence
    db_ops: Arc<DbOperations>,
    /// Schema manager for schema operations
    schema_manager: Arc<SchemaCore>,
    /// Message bus for event publishing and listening
    message_bus: Arc<MessageBus>,
    /// Flag to track if the event listener is running
    is_listening: Arc<std::sync::atomic::AtomicBool>,
}

impl MutationManager {
    /// Creates a new MutationManager instance
    pub fn new(
        db_ops: Arc<DbOperations>,
        schema_manager: Arc<SchemaCore>,
        message_bus: Arc<MessageBus>,
    ) -> Self {
        Self {
            db_ops,
            schema_manager,
            message_bus,
            is_listening: Arc::new(std::sync::atomic::AtomicBool::new(false)),
        }
    }

    /// Write schema operation - main orchestration method for mutations
    /// 
    /// # Deprecated
    /// Use `write_mutations_batch()` instead for better performance.
    /// Single mutations cause a flush-per-operation, while batching allows a single flush.
    #[deprecated(since = "0.1.0", note = "Use write_mutations_batch() instead for better performance")]
    #[allow(deprecated)]
    pub fn write_mutation(&mut self, mutation: Mutation) -> Result<String, SchemaError> {
        let start_time = std::time::Instant::now();
        
        // Capture backfill_hash before mutation is consumed
        let backfill_hash = mutation.backfill_hash.clone();
        
        // Get the schema definition
        let mut schema = self.schema_manager.get_schema(&mutation.schema_name)?
            .ok_or_else(|| SchemaError::InvalidData(format!("Schema '{}' not found", mutation.schema_name)))?;
        
        let key_config = schema.key.clone();
        let key_value = KeyValue::from_mutation(&mutation.fields_and_values, key_config.as_ref().unwrap());
        let mutation_id = mutation.uuid.clone();
        
        // Validate all field values against their topologies before processing
        for (field_name, value) in &mutation.fields_and_values {
            schema.validate_field_value(field_name, value)?;
        }
        
        // Process each field in the mutation
        let fields_affected: Vec<String> = mutation.fields_and_values.keys().cloned().collect();
        for (field_name, value) in mutation.fields_and_values {
            // Get field classifications BEFORE mutable borrow
            let field_classifications = schema.get_field_classifications(&field_name);
            
            if let Some(schema_field) = schema.runtime_fields.get_mut(&field_name) {
                // Use the new db_operations method with classifications
                self.db_ops.process_mutation_field_with_schema(
                    &mutation.schema_name,
                    &field_name,
                    &mutation.pub_key,
                    value,
                    &key_value,
                    schema_field,
                    field_classifications,
                )?;
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
        self.db_ops.native_index_manager().flush()?;

        // Sync molecule UUIDs to the persisted field before storing
        schema.sync_molecule_uuids();

        // Persist the updated schema back to the database and schema_manager
        let schema_name = schema.name.clone();
        self.db_ops.store_schema(&schema_name, &schema)?;
        self.schema_manager.load_schema_internal(schema)?;

        // Calculate execution time
        let execution_time_ms = start_time.elapsed().as_millis() as u64;
        
        // Create mutation context for transform execution
        let mutation_context = Some(crate::fold_db_core::infrastructure::message_bus::atom_events::MutationContext {
            key_value: Some(key_value.clone()),
            mutation_hash: Some(mutation_id.clone()),
            incremental: true,
            backfill_hash: backfill_hash.clone(), // Preserve backfill_hash from mutation
        });
        
        // Publish MutationExecuted event to trigger transforms
        let event = MutationExecuted::with_context(
            "write_mutation",
            mutation.schema_name.clone(),
            execution_time_ms,
            fields_affected,
            mutation_context,
        );
        
        self.message_bus.publish(event)?;
        
        // Flush database to ensure mutation is persisted to disk
        self.db_ops.flush()?;
        
        // Return the mutation ID
        Ok(mutation_id)
    }

    /// Write multiple mutations in a batch for improved performance
    /// Groups mutations by schema to minimize schema reloads and uses true batching
    pub fn write_mutations_batch(&mut self, mutations: Vec<Mutation>) -> Result<Vec<String>, SchemaError> {
        if mutations.is_empty() {
            return Ok(Vec::new());
        }

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
            let mut schema = self.schema_manager.get_schema(&schema_name)?
                .ok_or_else(|| SchemaError::InvalidData(format!("Schema '{}' not found", schema_name)))?;
            *timing_breakdown.entry("schema_load").or_insert(std::time::Duration::ZERO) += load_start.elapsed();

            let mut mutation_contexts = Vec::new();
            let mut validation_time = std::time::Duration::ZERO;
            let mut field_processing_time = std::time::Duration::ZERO;
            let mut refresh_time = std::time::Duration::ZERO;
            let mut atom_time = std::time::Duration::ZERO;
            let mut molecule_time = std::time::Duration::ZERO;
            let mut index_time = std::time::Duration::ZERO;
            
            // Collect all index operations for batch processing
            let mut index_operations = Vec::new();

            // Process all mutations for this schema using deferred storage
            for mutation in schema_mutations {
                let mutation_id = mutation.uuid.clone();
                let backfill_hash = mutation.backfill_hash.clone();
                let key_config = schema.key.clone();
                let key_value = KeyValue::from_mutation(&mutation.fields_and_values, key_config.as_ref()
                    .ok_or_else(|| SchemaError::InvalidData(format!(
                        "Schema '{}' has no key configuration. Cannot execute mutation.",
                        schema_name
                    )))?);
                
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
                
                // Process each field using deferred storage
                let field_start = std::time::Instant::now();
                for (field_name, value) in mutation.fields_and_values {
                    // Get field classifications before mutable borrow
                    let field_classifications = schema.get_field_classifications(&field_name);
                    
                    // Process the field in a separate scope to release the mutable borrow
                    {
                        let schema_field = schema.runtime_fields.get_mut(&field_name)
                            .ok_or_else(|| SchemaError::InvalidData(format!(
                                "Field '{}' not found in runtime_fields for schema '{}'",
                                field_name,
                                mutation.schema_name
                            )))?;
                        
                        // Refresh field from database
                        let refresh_start = std::time::Instant::now();
                        schema_field.refresh_from_db(&self.db_ops);
                        refresh_time += refresh_start.elapsed();
                        
                        // Create and store atom using deferred storage
                        let atom_start = std::time::Instant::now();
                        let new_atom = self.db_ops.create_and_store_atom_for_mutation_deferred(
                            &mutation.schema_name,
                            &mutation.pub_key,
                            value.clone(),
                            mutation.source_file_name.clone(),
                        )?;
                        atom_time += atom_start.elapsed();
                        
                        // Write mutation to field (updates in-memory molecule)
                        let mol_start = std::time::Instant::now();
                        schema_field.write_mutation(&key_value, new_atom, mutation.pub_key.clone());
                        
                        // Persist molecule using deferred storage
                        if let Some(molecule_uuid) = schema_field.common().molecule_uuid() {
                            self.db_ops.persist_field_molecule_deferred(schema_field, molecule_uuid)?;
                        }
                        molecule_time += mol_start.elapsed();
                    } // Mutable borrow ends here
                    
                    // Collect index operation for batch processing
                    index_operations.push((
                        mutation.schema_name.clone(),
                        field_name,
                        key_value.clone(),
                        value,
                        field_classifications,
                    ));
                }
                field_processing_time += field_start.elapsed();

                // Store mutation context for event publishing
                mutation_contexts.push((mutation_id, backfill_hash, key_value));
            }
            
            *timing_breakdown.entry("validation").or_insert(std::time::Duration::ZERO) += validation_time;
            *timing_breakdown.entry("field_processing").or_insert(std::time::Duration::ZERO) += field_processing_time;
            *timing_breakdown.entry("  - refresh_fields").or_insert(std::time::Duration::ZERO) += refresh_time;
            *timing_breakdown.entry("  - create_atoms").or_insert(std::time::Duration::ZERO) += atom_time;
            *timing_breakdown.entry("  - write_molecules").or_insert(std::time::Duration::ZERO) += molecule_time;
            *timing_breakdown.entry("  - index_fields").or_insert(std::time::Duration::ZERO) += index_time;
            
            // Publish batch index request for background processing
            if !index_operations.is_empty() {
                let index_start = std::time::Instant::now();
                debug!(
                    "MutationManager: Publishing BatchIndexRequest with {} operations for schema '{}'",
                    index_operations.len(),
                    schema_name
                );
                
                let index_requests: Vec<_> = index_operations.into_iter().map(|(schema_name, field_name, key_value, value, classifications)| {
                    super::infrastructure::message_bus::request_events::IndexRequest {
                        schema_name,
                        field_name,
                        key_value,
                        value,
                        classifications,
                    }
                }).collect();
                
                let batch_request = super::infrastructure::message_bus::request_events::BatchIndexRequest {
                    operations: index_requests,
                };
                
                self.message_bus.publish(batch_request)?;
                debug!(
                    "MutationManager: Successfully published BatchIndexRequest for schema '{}'",
                    schema_name
                );
                index_time += index_start.elapsed();
            } else {
                debug!(
                    "MutationManager: No index operations to publish for schema '{}'",
                    schema_name
                );
            }

            // Sync molecule UUIDs to the persisted field before storing
            let sync_start = std::time::Instant::now();
            schema.sync_molecule_uuids();
            *timing_breakdown.entry("sync_uuids").or_insert(std::time::Duration::ZERO) += sync_start.elapsed();

            // Single schema persist and reload for this schema group
            let store_start = std::time::Instant::now();
            self.db_ops.store_schema(&schema_name, &schema)?;
            *timing_breakdown.entry("schema_store").or_insert(std::time::Duration::ZERO) += store_start.elapsed();
            
            let reload_start = std::time::Instant::now();
            self.schema_manager.load_schema_internal(schema)?;
            *timing_breakdown.entry("schema_reload").or_insert(std::time::Duration::ZERO) += reload_start.elapsed();

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
                    0, // Execution time will be calculated for the batch
                    vec![], // Fields affected - could be populated if needed
                    mutation_context,
                );
                
                batch_events.push((event, mutation_id.clone()));
                mutation_ids.push(mutation_id.clone());
            }
        }

        // Single flush for entire batch
        let flush_start = std::time::Instant::now();
        self.db_ops.flush()?;
        timing_breakdown.insert("flush", flush_start.elapsed());

        // Batch publish events
        let publish_start = std::time::Instant::now();
        self.publish_batch_events(batch_events)?;
        timing_breakdown.insert("event_publish", publish_start.elapsed());

        let total_time = start_time.elapsed();
        
        // Log timing breakdown
        debug!("Batch mutation timing breakdown (total: {:.2}ms):", total_time.as_millis());
        let mut sorted_timings: Vec<_> = timing_breakdown.iter().collect();
        sorted_timings.sort_by(|a, b| b.1.cmp(a.1));
        for (operation, duration) in sorted_timings {
            let percentage = (duration.as_millis() as f64 / total_time.as_millis() as f64) * 100.0;
            debug!("  - {}: {:.2}ms ({:.1}%)", operation, duration.as_millis(), percentage);
        }

        Ok(mutation_ids)
    }

    /// Groups mutations by schema name for efficient batch processing
    fn group_mutations_by_schema(&self, mutations: Vec<Mutation>) -> HashMap<String, Vec<Mutation>> {
        let mut grouped: HashMap<String, Vec<Mutation>> = HashMap::new();
        
        for mutation in mutations {
            grouped.entry(mutation.schema_name.clone())
                .or_default()
                .push(mutation);
        }
        
        grouped
    }


    /// Publishes all batch events at once
    fn publish_batch_events(&self, events: Vec<(MutationExecuted, String)>) -> Result<(), SchemaError> {
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
                        if let Err(e) = Self::handle_mutation_request_event(&mutation_request, &db_ops, &schema_manager, &message_bus) {
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
        self.is_listening.store(false, std::sync::atomic::Ordering::Release);
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
        let mut schema = schema_manager.get_schema(&mutation_request.mutation.schema_name)?
            .ok_or_else(|| SchemaError::InvalidData(format!("Schema '{}' not found", mutation_request.mutation.schema_name)))?;
        
        let key_config = schema.key.clone();
        let key_value = KeyValue::from_mutation(&mutation_request.mutation.fields_and_values, key_config.as_ref().unwrap());
        let mutation_id = mutation_request.mutation.uuid.clone();
        
        // Validate all field values against their topologies before processing
        for (field_name, value) in &mutation_request.mutation.fields_and_values {
            schema.validate_field_value(field_name, value)?;
        }
        
        // Process each field in the mutation
        let fields_affected: Vec<String> = mutation_request.mutation.fields_and_values.keys().cloned().collect();
        for (field_name, value) in mutation_request.mutation.fields_and_values.clone() {
            // Get field classifications BEFORE mutable borrow
            let field_classifications = schema.get_field_classifications(&field_name);
            
            if let Some(schema_field) = schema.runtime_fields.get_mut(&field_name) {
                // Use the new db_operations method with classifications
                db_ops.process_mutation_field_with_schema(
                    &mutation_request.mutation.schema_name,
                    &field_name,
                    &mutation_request.mutation.pub_key,
                    value,
                    &key_value,
                    schema_field,
                    field_classifications,
                )?;
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
        db_ops.native_index_manager().flush()?;

        // Sync molecule UUIDs to the persisted field before storing
        schema.sync_molecule_uuids();

        // Persist the updated schema back to the database and schema_manager
        let schema_name = schema.name.clone();
        db_ops.store_schema(&schema_name, &schema)?;
        schema_manager.load_schema_internal(schema)?;

        // Calculate execution time
        let execution_time_ms = start_time.elapsed().as_millis() as u64;
        
        // Create mutation context for transform execution
        let mutation_context = Some(crate::fold_db_core::infrastructure::message_bus::atom_events::MutationContext {
            key_value: Some(key_value.clone()),
            mutation_hash: Some(mutation_id.clone()),
            incremental: true,
            backfill_hash: mutation_request.mutation.backfill_hash.clone(), // Pass through backfill_hash
        });
        
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
        db_ops.flush()?;

        Ok(())
    }
}
