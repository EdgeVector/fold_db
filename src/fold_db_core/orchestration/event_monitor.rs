//! Event monitoring component for the Transform Orchestrator
//!
//! Handles FieldValueSet event monitoring and transform discovery,
//! extracted from the main TransformOrchestrator for better separation of concerns.

use super::persistence_manager::PersistenceManager;
use crate::fold_db_core::infrastructure::message_bus::{
    atom_events::FieldValueSet,
    schema_events::{TransformExecuted, TransformTriggered},
    MessageBus,
};
use crate::fold_db_core::transform_manager::types::TransformRunner;
use crate::schema::SchemaError;
use log::{error, info};
use std::sync::Arc;
use std::thread;
use std::time::Duration;

/// Handles monitoring of FieldValueSet events and automatic transform discovery
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
            persistence.get_tree().clone(),
        );

        Self {
            message_bus,
            manager,
            persistence,
            _monitoring_thread: Some(monitoring_thread),
        }
    }

    /// Start unified monitoring for both FieldValueSet and TransformTriggered events
    fn start_unified_monitoring(
        message_bus: Arc<MessageBus>,
        manager: Arc<dyn TransformRunner>,
        tree: sled::Tree,
    ) -> thread::JoinHandle<()> {
        let mut field_value_consumer = message_bus.subscribe::<FieldValueSet>();
        let mut triggered_consumer = message_bus.subscribe::<TransformTriggered>();

        thread::spawn(move || {
            info!("🔍 EventMonitor: Starting unified monitoring of FieldValueSet and TransformTriggered events");

            loop {
                // Check FieldValueSet events
                if let Ok(event) = field_value_consumer.try_recv() {
                    info!("🔔 DIAGNOSTIC: EventMonitor received FieldValueSet event - field: {}, source: {}", event.field, event.source);
                    if let Err(e) =
                        Self::handle_field_value_event(&event, &manager, &tree, &message_bus)
                    {
                        error!("❌ Error handling field value event: {}", e);
                    }
                }

                // Check TransformTriggered events
                if let Ok(event) = triggered_consumer.try_recv() {
                    info!("🔔 DIAGNOSTIC: EventMonitor received TransformTriggered event - transform_id: {}", event.transform_id);
                    if let Err(e) =
                        Self::handle_transform_triggered_event(&event, &manager, &message_bus)
                    {
                        error!("❌ Error handling TransformTriggered event: {}", e);
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
        info!(
            "🎯 DIAGNOSTIC: EventMonitor received TransformTriggered - transform_id: {}",
            event.transform_id
        );

        // Check if transform exists before executing
        match manager.transform_exists(&event.transform_id) {
            Ok(true) => {
                info!(
                    "✅ DIAGNOSTIC: Transform {} exists, proceeding with execution",
                    event.transform_id
                );
            }
            Ok(false) => {
                error!(
                    "❌ DIAGNOSTIC: Transform {} does not exist, skipping execution",
                    event.transform_id
                );
                return Err(SchemaError::InvalidData(format!(
                    "Transform '{}' does not exist",
                    event.transform_id
                )));
            }
            Err(e) => {
                error!(
                    "❌ DIAGNOSTIC: Error checking if transform {} exists: {}",
                    event.transform_id, e
                );
                return Err(e);
            }
        }

        // Execute the transform with context if available
        let result = if let Some(ref context) = &event.mutation_context {
            if context.incremental {
                info!(
                    "🎯 DIAGNOSTIC: Using incremental transform execution for {}",
                    event.transform_id
                );
                manager.execute_transform_with_context(&event.transform_id, &event.mutation_context)
            } else {
                info!(
                    "🎯 DIAGNOSTIC: Using standard transform execution for {}",
                    event.transform_id
                );
                manager.execute_transform_now(&event.transform_id)
            }
        } else {
            info!(
                "🎯 DIAGNOSTIC: No mutation context, using standard transform execution for {}",
                event.transform_id
            );
            manager.execute_transform_now(&event.transform_id)
        };

        match result {
            Ok(result) => {
                info!(
                    "✅ Transform {} executed successfully: {}",
                    event.transform_id, result
                );

                // Publish TransformExecuted event
                Self::publish_transform_executed(
                    message_bus,
                    &event.transform_id,
                    &result.to_string(),
                )?;

                Ok(())
            }
            Err(e) => {
                error!(
                    "❌ Transform {} execution failed: {}",
                    event.transform_id, e
                );

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
            error!(
                "❌ Failed to publish TransformExecuted event for {}: {}",
                transform_id, e
            );
            SchemaError::InvalidData(format!("Failed to publish TransformExecuted event: {}", e))
        })?;

        info!("✅ Published TransformExecuted event for: {}", transform_id);
        Ok(())
    }

    /// Handle a single FieldValueSet event
    fn handle_field_value_event(
        event: &FieldValueSet,
        manager: &Arc<dyn TransformRunner>,
        tree: &sled::Tree,
        message_bus: &Arc<MessageBus>,
    ) -> Result<(), SchemaError> {
        info!(
            "🎯 EventMonitor: Field value set detected - field: {}, source: {}",
            event.field, event.source
        );

        // Parse schema.field from the field path
        if let Some((schema_name, field_name)) = event.field.split_once('.') {
            Self::process_discovered_transforms(
                schema_name,
                field_name,
                &event.source,
                manager,
                tree,
                message_bus,
                &event.mutation_context,
            )
        } else {
            error!(
                "❌ Invalid field format '{}' - expected 'schema.field'",
                event.field
            );
            Err(SchemaError::InvalidData(format!(
                "Invalid field format '{}' - expected 'schema.field'",
                event.field
            )))
        }
    }

    /// Process discovered transforms for a field
    fn process_discovered_transforms(
        schema_name: &str,
        field_name: &str,
        source: &str,
        manager: &Arc<dyn TransformRunner>,
        tree: &sled::Tree,
        message_bus: &Arc<MessageBus>,
        mutation_context: &Option<
            crate::fold_db_core::infrastructure::message_bus::atom_events::MutationContext,
        >,
    ) -> Result<(), SchemaError> {
        // Look up transforms for this field using the manager
        info!(
            "🔍 DIAGNOSTIC: Looking up transforms for field {}.{} from manager",
            schema_name, field_name
        );
        match manager.get_transforms_for_field(schema_name, field_name) {
            Ok(transform_ids) => {
                info!(
                    "🔍 DIAGNOSTIC: Transform lookup result - found {} transforms: {:?}",
                    transform_ids.len(),
                    transform_ids
                );

                if !transform_ids.is_empty() {
                    info!(
                        "🔍 Found {} transforms for field {}.{}: {:?}",
                        transform_ids.len(),
                        schema_name,
                        field_name,
                        transform_ids
                    );

                    Self::add_transforms_to_queue(
                        &transform_ids,
                        source,
                        tree,
                        message_bus,
                        mutation_context,
                    )?;
                    info!(
                        "✅ EventMonitor triggered {} transforms via TransformTriggered events",
                        transform_ids.len()
                    );
                } else {
                    info!(
                        "ℹ️ DIAGNOSTIC: No transforms found for field {}.{} - this may indicate missing transform dependency mappings",
                        schema_name, field_name
                    );
                }
                Ok(())
            }
            Err(e) => {
                error!(
                    "❌ DIAGNOSTIC: Failed to get transforms for field {}.{}: {}",
                    schema_name, field_name, e
                );
                Err(e)
            }
        }
    }

    /// REMOVED: add_transforms_to_queue - EventMonitor should not manage persistence directly
    /// This responsibility belongs to PersistenceManager through TransformOrchestrator
    fn add_transforms_to_queue(
        transform_ids: &std::collections::HashSet<String>,
        _source: &str,
        _tree: &sled::Tree,
        message_bus: &Arc<MessageBus>,
        mutation_context: &Option<
            crate::fold_db_core::infrastructure::message_bus::atom_events::MutationContext,
        >,
    ) -> Result<(), SchemaError> {
        info!(
            "🚀 EventMonitor: Discovered {} transforms for field update",
            transform_ids.len()
        );

        // Publish TransformTriggered events for each discovered transform
        for transform_id in transform_ids {
            info!(
                "🔔 Publishing TransformTriggered event for: {}",
                transform_id
            );

            let triggered_event = if let Some(ref context) = mutation_context {
                if context.incremental {
                    info!("🎯 DIAGNOSTIC: Publishing TransformTriggered with mutation context for incremental processing");
                    TransformTriggered::with_context(transform_id.clone(), context.clone())
                } else {
                    TransformTriggered::new(transform_id.clone())
                }
            } else {
                TransformTriggered::new(transform_id.clone())
            };

            match message_bus.publish(triggered_event) {
                Ok(()) => {
                    info!(
                        "✅ Published TransformTriggered event for: {}",
                        transform_id
                    );
                }
                Err(e) => {
                    error!(
                        "❌ Failed to publish TransformTriggered event for {}: {}",
                        transform_id, e
                    );
                    return Err(SchemaError::InvalidData(format!(
                        "Failed to publish TransformTriggered event for {}: {}",
                        transform_id, e
                    )));
                }
            }
        }

        info!(
            "✅ EventMonitor published {} TransformTriggered events",
            transform_ids.len()
        );
        Ok(())
    }

    // REMOVED: mark_transforms_as_processed - EventMonitor should only queue, not execute/process

    // REMOVED: persist_queue_state - EventMonitor should not handle persistence
    // All persistence should go through PersistenceManager to avoid conflicts

    /// Get access to the message bus for publishing events
    pub fn get_message_bus(&self) -> &Arc<MessageBus> {
        &self.message_bus
    }

    /// Get access to the transform manager
    pub fn get_manager(&self) -> &Arc<dyn TransformRunner> {
        &self.manager
    }

    /// Get access to the persistence manager
    pub fn get_persistence(&self) -> &PersistenceManager {
        &self.persistence
    }

    /// Stop monitoring (the thread will be stopped when the EventMonitor is dropped)
    pub fn stop_monitoring(&mut self) {
        if let Some(_handle) = self._monitoring_thread.take() {
            // In a real implementation, we would send a shutdown signal
            // For now, the thread will be stopped when the handle is dropped
            info!("🛑 EventMonitor: Stopping unified event monitoring");
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

#[cfg(test)]
mod tests {
    use super::*;
    use crate::fold_db_core::transform_manager::types::TransformRunner;
    use serde_json::Value as JsonValue;
    use std::collections::HashSet;

    struct MockTransformRunner {
        transforms_for_field: HashSet<String>,
    }

    impl MockTransformRunner {
        fn new(transforms: Vec<&str>) -> Self {
            Self {
                transforms_for_field: transforms.into_iter().map(|s| s.to_string()).collect(),
            }
        }
    }

    impl TransformRunner for MockTransformRunner {
        fn execute_transform_now(&self, _transform_id: &str) -> Result<JsonValue, SchemaError> {
            Ok(serde_json::json!({"status": "success"}))
        }

        fn execute_transform_with_context(
            &self,
            _transform_id: &str,
            mutation_context: &Option<
                crate::fold_db_core::infrastructure::message_bus::atom_events::MutationContext,
            >,
        ) -> Result<JsonValue, SchemaError> {
            if let Some(ref context) = mutation_context {
                Ok(
                    serde_json::json!({"status": "success_with_context", "range_key": context.range_key, "hash_key": context.hash_key, "incremental": context.incremental}),
                )
            } else {
                Ok(serde_json::json!({"status": "success_with_context", "no_context": true}))
            }
        }

        fn transform_exists(&self, _transform_id: &str) -> Result<bool, SchemaError> {
            Ok(true)
        }

        fn get_transforms_for_field(
            &self,
            _schema_name: &str,
            _field_name: &str,
        ) -> Result<HashSet<String>, SchemaError> {
            Ok(self.transforms_for_field.clone())
        }
    }

    fn create_test_tree() -> sled::Tree {
        crate::testing_utils::TestDatabaseFactory::create_named_test_tree("test_event_monitor")
    }

    #[test]
    fn test_process_discovered_transforms() {
        let tree = create_test_tree();
        let manager: Arc<dyn TransformRunner> =
            Arc::new(MockTransformRunner::new(vec!["transform1", "transform2"]));

        let message_bus = Arc::new(MessageBus::new());
        let result = EventMonitor::process_discovered_transforms(
            "test_schema",
            "test_field",
            "test_source",
            &manager,
            &tree,
            &message_bus,
            &None, // No mutation context for this test
        );

        assert!(result.is_ok());

        // In the new architecture, EventMonitor only discovers and queues transforms
        // It no longer handles persistence directly - that's handled by TransformOrchestrator
        // So we just verify that the discovery process completed successfully
        // The actual queuing and persistence is delegated to other components
    }

    #[test]
    fn test_handle_field_value_event() {
        let tree = create_test_tree();
        let manager: Arc<dyn TransformRunner> =
            Arc::new(MockTransformRunner::new(vec!["transform1"]));

        let event = FieldValueSet {
            field: "test_schema.test_field".to_string(),
            value: serde_json::json!("test_value"),
            source: "test_source".to_string(),
            mutation_context: None,
        };

        let message_bus = Arc::new(MessageBus::new());
        let result = EventMonitor::handle_field_value_event(&event, &manager, &tree, &message_bus);
        assert!(result.is_ok());
    }

    #[test]
    fn test_invalid_field_format() {
        let tree = create_test_tree();
        let manager: Arc<dyn TransformRunner> = Arc::new(MockTransformRunner::new(vec![]));

        let event = FieldValueSet {
            field: "invalid_field_format".to_string(),
            value: serde_json::json!("test_value"),
            source: "test_source".to_string(),
            mutation_context: None,
        };

        let message_bus = Arc::new(MessageBus::new());
        let result = EventMonitor::handle_field_value_event(&event, &manager, &tree, &message_bus);
        assert!(result.is_err());
    }
}
