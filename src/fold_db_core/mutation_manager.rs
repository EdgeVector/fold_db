//! Mutation Manager - Handles all mutation operations
//!
//! This module contains the MutationManager that handles the core mutation logic
//! previously located in FoldDB. It manages the execution of mutations, including
//! schema updates, atom persistence, and event publishing. It can also listen
//! for MutationRequest events and handle them automatically.

use std::sync::Arc;
use std::thread;
use std::time::Duration;

use crate::db_operations::DbOperations;
use crate::schema::types::{Mutation, KeyValue};
use crate::schema::{SchemaCore, SchemaError};
use super::infrastructure::message_bus::events::query_events::MutationExecuted;
use super::infrastructure::message_bus::request_events::MutationRequest;
use super::infrastructure::MessageBus;
use log::{error, warn};

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
            }
        }

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
                error!(
                    "Field '{}' not found in runtime_fields for schema '{}'. Available fields: {:?}",
                    field_name,
                    mutation_request.mutation.schema_name,
                    schema.runtime_fields.keys().collect::<Vec<_>>()
                );
            }
        }

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
