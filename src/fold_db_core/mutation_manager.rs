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
use log::{error, info, warn};

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
        
        // Get the schema definition
        let mut schema = self.schema_manager.get_schema(&mutation.schema_name)?
            .ok_or_else(|| SchemaError::InvalidData(format!("Schema '{}' not found", mutation.schema_name)))?;
        
        let key_config = schema.key.clone();
        let key_value = KeyValue::from_mutation(&mutation.fields_and_values, key_config.as_ref().unwrap());
        let mutation_id = mutation.uuid.clone();
        
        // Process each field in the mutation
        let fields_affected: Vec<String> = mutation.fields_and_values.keys().cloned().collect();
        for (field_name, value) in mutation.fields_and_values {
            if let Some(schema_field) = schema.fields.get_mut(&field_name) {
                // Use the new db_operations method to handle the entire field mutation process
                self.db_ops.process_mutation_field(
                    &mutation.schema_name,
                    &mutation.pub_key,
                    value,
                    &key_value,
                    schema_field,
                )?;
            }
        }

        // Persist the updated schema back to the database and schema_manager
        let schema_name = schema.name.clone();
        log::info!("🔄 Persisting schema '{}' with updated field molecule UUIDs", schema_name);
        self.db_ops.store_schema(&schema_name, &schema)?;
        self.schema_manager.load_schema_internal(schema)?;
        log::info!("✅ Schema '{}' persisted successfully", schema_name);

        // Calculate execution time
        let execution_time_ms = start_time.elapsed().as_millis() as u64;
        
        // Create mutation context for transform execution
        let mutation_context = Some(crate::fold_db_core::infrastructure::message_bus::atom_events::MutationContext {
            key_value: Some(key_value.clone()),
            mutation_hash: Some(mutation_id.clone()),
            incremental: true,
        });
        
        // Publish MutationExecuted event to trigger transforms
        let event = MutationExecuted::with_context(
            "write_mutation",
            mutation.schema_name.clone(),
            execution_time_ms,
            fields_affected,
            mutation_context,
        );
        
        if let Err(e) = self.message_bus.publish(event) {
            log::warn!("Failed to publish MutationExecuted event: {}", e);
            // Don't fail the mutation if event publishing fails
        }
        
        // Flush database to ensure mutation is persisted to disk
        if let Err(e) = self.db_ops.flush() {
            log::warn!("Failed to flush database after mutation completion: {}", e);
            // Don't fail the mutation if flush fails, but log the warning
        }
        
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
            info!("🔄 Starting MutationManager event listener background thread");
            
            // Subscribe to MutationRequest events
            let mut consumer = message_bus.subscribe::<MutationRequest>();

            info!("✅ MutationManager subscribed to MutationRequest events");

            // Main event processing loop
            while is_listening.load(std::sync::atomic::Ordering::Acquire) {
                match consumer.try_recv() {
                    Ok(mutation_request) => {
                        info!("📨 MutationManager received MutationRequest event: {:?}", mutation_request);
                        
                        if let Err(e) = Self::handle_mutation_request_event(&mutation_request, &db_ops, &schema_manager, &message_bus) {
                            error!("❌ MutationManager failed to handle mutation request: {}", e);
                        }
                    }
                    Err(std::sync::mpsc::TryRecvError::Empty) => {
                        // No events available, sleep briefly to avoid busy waiting
                        thread::sleep(Duration::from_millis(10));
                    }
                    Err(std::sync::mpsc::TryRecvError::Disconnected) => {
                        error!("❌ MutationManager message bus consumer disconnected");
                        break;
                    }
                }
            }

            info!("🛑 MutationManager event listener background thread stopped");
        });

        info!("✅ MutationManager event listener started successfully");
        Ok(())
    }

    /// Stop the event listener
    pub fn stop_event_listener(&self) {
        self.is_listening.store(false, std::sync::atomic::Ordering::Release);
        info!("🛑 MutationManager event listener stop requested");
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
        
        info!(
            "🔧 MutationManager processing MutationRequest event for schema: {}, correlation_id: {}",
            mutation_request.mutation.schema_name, mutation_request.correlation_id
        );

        // Log mutation details
        info!(
            "📊 MutationManager mutation details - Schema: {}, Type: {:?}, Fields: {:?}",
            mutation_request.mutation.schema_name,
            mutation_request.mutation.mutation_type,
            mutation_request.mutation.fields_and_values.keys().collect::<Vec<_>>()
        );

        // Get the schema definition
        let mut schema = schema_manager.get_schema(&mutation_request.mutation.schema_name)?
            .ok_or_else(|| SchemaError::InvalidData(format!("Schema '{}' not found", mutation_request.mutation.schema_name)))?;
        
        let key_config = schema.key.clone();
        let key_value = KeyValue::from_mutation(&mutation_request.mutation.fields_and_values, key_config.as_ref().unwrap());
        let mutation_id = mutation_request.mutation.uuid.clone();
        
        // Process each field in the mutation
        let fields_affected: Vec<String> = mutation_request.mutation.fields_and_values.keys().cloned().collect();
        for (field_name, value) in mutation_request.mutation.fields_and_values.clone() {
            if let Some(schema_field) = schema.fields.get_mut(&field_name) {
                // Use the new db_operations method to handle the entire field mutation process
                db_ops.process_mutation_field(
                    &mutation_request.mutation.schema_name,
                    &mutation_request.mutation.pub_key,
                    value,
                    &key_value,
                    schema_field,
                )?;
            }
        }

        // Persist the updated schema back to the database and schema_manager
        let schema_name = schema.name.clone();
        log::info!("🔄 Persisting schema '{}' with updated field molecule UUIDs", schema_name);
        db_ops.store_schema(&schema_name, &schema)?;
        schema_manager.load_schema_internal(schema)?;
        log::info!("✅ Schema '{}' persisted successfully", schema_name);

        // Calculate execution time
        let execution_time_ms = start_time.elapsed().as_millis() as u64;
        
        // Create mutation context for transform execution
        let mutation_context = Some(crate::fold_db_core::infrastructure::message_bus::atom_events::MutationContext {
            key_value: Some(key_value.clone()),
            mutation_hash: Some(mutation_id.clone()),
            incremental: true,
        });
        
        // Publish MutationExecuted event to trigger transforms
        let event = MutationExecuted::with_context(
            "mutation_request_handler",
            mutation_request.mutation.schema_name.clone(),
            execution_time_ms,
            fields_affected,
            mutation_context,
        );
        
        if let Err(e) = message_bus.publish(event) {
            log::warn!("Failed to publish MutationExecuted event: {}", e);
            // Don't fail the mutation if event publishing fails
        }

        // Flush database to ensure mutation is persisted to disk
        if let Err(e) = db_ops.flush() {
            log::warn!("Failed to flush database after mutation completion: {}", e);
            // Don't fail the mutation if flush fails, but log the warning
        }

        info!(
            "✅ MutationManager successfully executed mutation from event. Mutation ID: {}, Execution time: {}ms",
            mutation_id, execution_time_ms
        );
        Ok(())
    }
}
