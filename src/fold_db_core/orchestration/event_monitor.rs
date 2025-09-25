//! Event monitoring component for the Transform Orchestrator
//!
//! Handles FieldValueSet event monitoring and transform discovery,
//! extracted from the main TransformOrchestrator for better separation of concerns.

use super::persistence_manager::PersistenceManager;
use crate::fold_db_core::infrastructure::message_bus::{
    schema_events::{TransformExecuted, TransformTriggered},
    MessageBus,
};
use crate::transform::manager::types::TransformRunner;
use crate::schema::SchemaError;
use log::{error, info};
use std::sync::Arc;
use std::thread;
use std::time::Duration;

/// Handles monitoring of MutationExecuted and TransformTriggered events for automatic transform execution
pub struct EventMonitor {
    message_bus: Arc<MessageBus>,
    manager: Arc<dyn TransformRunner>,
    persistence: PersistenceManager,
    /// Single monitoring thread for all events
    _monitoring_thread: Option<thread::JoinHandle<()>>,
}

impl EventMonitor {
    /// Create a new EventMonitor and start monitoring
    pub fn new(
        message_bus: Arc<MessageBus>,
        manager: Arc<dyn TransformRunner>,
        persistence: PersistenceManager,
    ) -> Self {
        let monitoring_thread = Self::start_unified_monitoring(
            Arc::clone(&message_bus),
            Arc::clone(&manager),
        );

        Self {
            message_bus,
            manager,
            persistence,
            _monitoring_thread: Some(monitoring_thread),
        }
    }

    /// Start unified monitoring for MutationExecuted and TransformTriggered events
    fn start_unified_monitoring(
        message_bus: Arc<MessageBus>,
        manager: Arc<dyn TransformRunner>,
    ) -> thread::JoinHandle<()> {
        let mut mutation_executed_consumer = message_bus.subscribe::<crate::fold_db_core::infrastructure::message_bus::query_events::MutationExecuted>();
        let mut triggered_consumer = message_bus.subscribe::<TransformTriggered>();

        thread::spawn(move || {
            info!("EventMonitor: Starting unified monitoring of MutationExecuted and TransformTriggered events");

            loop {
                // Check MutationExecuted events - trigger transforms after mutation completion
                if let Ok(event) = mutation_executed_consumer.try_recv() {
                    if let Err(e) =
                        Self::handle_mutation_executed_event(&event, &manager, &message_bus)
                    {
                        error!("Error handling mutation executed event: {}", e);
                    }
                }

                // Check TransformTriggered events
                if let Ok(event) = triggered_consumer.try_recv() {
                    if let Err(e) =
                        Self::handle_transform_triggered_event(&event, &manager, &message_bus)
                    {
                        error!("Error handling TransformTriggered event: {}", e);
                    }
                }

                // Small sleep to prevent busy waiting
                thread::sleep(Duration::from_millis(10));
            }
        })
    }

    /// Handle a TransformTriggered event by executing the transform
    fn handle_transform_triggered_event(
        event: &TransformTriggered,
        manager: &Arc<dyn TransformRunner>,
        message_bus: &Arc<MessageBus>,
    ) -> Result<(), SchemaError> {
        let result = manager.execute_transform_with_context(&event.transform_id, &event.mutation_context);

        match result {
            Ok(result) => {
                info!("Transform {} executed successfully: {}", event.transform_id, result);

                // Publish TransformExecuted event
                Self::publish_transform_executed(
                    message_bus,
                    &event.transform_id,
                    &result.to_string(),
                )?;

                Ok(())
            }
            Err(e) => {
                error!("Transform {} execution failed: {}", event.transform_id, e);

                // Publish TransformExecuted event with error
                Self::publish_transform_executed(
                    message_bus,
                    &event.transform_id,
                    &format!("error: {}", e),
                )?;

                Err(e)
            }
        }
    }

    /// Publish TransformExecuted event
    fn publish_transform_executed(
        message_bus: &Arc<MessageBus>,
        transform_id: &str,
        result: &str,
    ) -> Result<(), SchemaError> {
        let executed_event = TransformExecuted {
            transform_id: transform_id.to_string(),
            result: result.to_string(),
        };

        message_bus.publish(executed_event).map_err(|e| {
            error!("Failed to publish TransformExecuted event for {}: {}", transform_id, e);
            SchemaError::InvalidData(format!("Failed to publish TransformExecuted event: {}", e))
        })?;

        Ok(())
    }

    /// Handle a MutationExecuted event by triggering transforms for the schema
    /// @tomtang: this is the entry point for the transform orchestration
    fn handle_mutation_executed_event(
        event: &crate::fold_db_core::infrastructure::message_bus::query_events::MutationExecuted,
        manager: &Arc<dyn TransformRunner>,
        message_bus: &Arc<MessageBus>,
    ) -> Result<(), SchemaError> {
        let mut unique_transform_ids = std::collections::HashSet::new();
        for field_name in event.fields_affected.clone() {
            match manager.get_transforms_for_field(&event.schema, &field_name) {
                Ok(transform_ids) => {
                    unique_transform_ids.extend(transform_ids);
                }
                Err(e) => {
                    error!("Failed to get transforms for field {}.{}: {}", event.schema, field_name, e);
                    return Err(e);
                }
            }
        }
        Self::add_transforms_to_queue(
            &unique_transform_ids,
            message_bus,
            &None,
        )?;
        Ok(())
    }


    /// REMOVED: add_transforms_to_queue - EventMonitor should not manage persistence directly
    /// This responsibility belongs to PersistenceManager through TransformOrchestrator
    fn add_transforms_to_queue(
        transform_ids: &std::collections::HashSet<String>,
        message_bus: &Arc<MessageBus>,
        mutation_context: &Option<
            crate::fold_db_core::infrastructure::message_bus::atom_events::MutationContext,
        >,
    ) -> Result<(), SchemaError> {
        // Publish TransformTriggered events for each discovered transform
        for transform_id in transform_ids {
            let triggered_event = if let Some(ref context) = mutation_context {
                if context.incremental {
                    TransformTriggered::with_context(transform_id.clone(), context.clone())
                } else {
                    TransformTriggered::new(transform_id.clone())
                }
            } else {
                TransformTriggered::new(transform_id.clone())
            };

            match message_bus.publish(triggered_event) {
                Ok(()) => {
                    // TransformTriggered event published successfully
                }
                Err(e) => {
                    error!("Failed to publish TransformTriggered event for {}: {}", transform_id, e);
                    return Err(SchemaError::InvalidData(format!(
                        "Failed to publish TransformTriggered event for {}: {}",
                        transform_id, e
                    )));
                }
            }
        }

        Ok(())
    }


    /// Stop monitoring (the thread will be stopped when the EventMonitor is dropped)
    pub fn stop_monitoring(&mut self) {
        if let Some(_handle) = self._monitoring_thread.take() {
            // In a real implementation, we would send a shutdown signal
            // For now, the thread will be stopped when the handle is dropped
            info!("EventMonitor: Stopping unified event monitoring");
            // Note: In a production system, you would want to implement
            // a proper shutdown mechanism using channels or atomic flags
        }
    }
}

impl Drop for EventMonitor {
    fn drop(&mut self) {
        self.stop_monitoring();
    }
}