//! Mutation Event Manager for handling mutation events from transform completion
//!
//! This manager listens for mutation events and provides a centralized way to handle
//! mutation-related operations with access to database operations.

use crate::fold_db_core::infrastructure::message_bus::{
    MessageBus, query_events::MutationExecuted, request_events::MutationRequest
};
use crate::schema::SchemaError;
use crate::schema::types::Mutation;
use log::{error, info, warn};
use serde_json::Value;
use std::sync::Arc;
use std::thread;
use std::time::Duration;

/// Manages mutation events from transform completion and provides database operations access
pub struct MutationEventManager {
    mutation_executor: Arc<dyn Fn(Mutation) -> Result<String, SchemaError> + Send + Sync>,
    message_bus: Arc<MessageBus>,
    is_running: Arc<std::sync::atomic::AtomicBool>,
}

impl MutationEventManager {
    /// Create a new MutationEventManager with a mutation executor closure
    pub fn new<F>(mutation_executor: F, message_bus: Arc<MessageBus>) -> Self 
    where
        F: Fn(Mutation) -> Result<String, SchemaError> + Send + Sync + 'static,
    {
        Self {
            mutation_executor: Arc::new(mutation_executor),
            message_bus,
            is_running: Arc::new(std::sync::atomic::AtomicBool::new(false)),
        }
    }

    /// Start the mutation event manager in a background thread
    pub fn start(&self) -> Result<(), SchemaError> {
        if self.is_running.load(std::sync::atomic::Ordering::Acquire) {
            warn!("MutationEventManager is already running");
            return Ok(());
        }

        let mutation_executor = self.mutation_executor.clone();
        let message_bus = Arc::clone(&self.message_bus);
        let is_running = Arc::clone(&self.is_running);

        is_running.store(true, std::sync::atomic::Ordering::Release);

        thread::spawn(move || {
            info!("🔄 Starting MutationEventManager background thread");
            
            // Subscribe to MutationRequest events (emitted when transforms complete and store results)
            let mut consumer = message_bus.subscribe::<MutationRequest>();

            info!("✅ MutationEventManager subscribed to MutationRequest events");

            // Main event processing loop
            while is_running.load(std::sync::atomic::Ordering::Acquire) {
                match consumer.try_recv() {
                    Ok(mutation_request) => {
                        info!("📨 Received MutationRequest event: {:?}", mutation_request);
                        
                        if let Err(e) = Self::handle_mutation_request(&mutation_request, &mutation_executor, &message_bus) {
                            error!("❌ Failed to handle mutation request: {}", e);
                        }
                    }
                    Err(std::sync::mpsc::TryRecvError::Empty) => {
                        // No events available, sleep briefly to avoid busy waiting
                        thread::sleep(Duration::from_millis(10));
                    }
                    Err(std::sync::mpsc::TryRecvError::Disconnected) => {
                        error!("❌ Message bus consumer disconnected");
                        break;
                    }
                }
            }

            info!("🛑 MutationEventManager background thread stopped");
        });

        info!("✅ MutationEventManager started successfully");
        Ok(())
    }

    /// Stop the mutation event manager
    pub fn stop(&self) {
        self.is_running.store(false, std::sync::atomic::Ordering::Release);
        info!("🛑 MutationEventManager stop requested");
    }

    /// Check if the manager is currently running
    pub fn is_running(&self) -> bool {
        self.is_running.load(std::sync::atomic::Ordering::Acquire)
    }

    /// Handle a mutation request event by actually executing the mutation using the provided closure
    fn handle_mutation_request(
        mutation_request: &MutationRequest,
        mutation_executor: &Arc<dyn Fn(Mutation) -> Result<String, SchemaError> + Send + Sync>,
        message_bus: &MessageBus,
    ) -> Result<(), SchemaError> {
        let start_time = std::time::Instant::now();
        
        info!(
            "🔧 Processing MutationRequest event for schema: {}, correlation_id: {}",
            mutation_request.mutation.schema_name, mutation_request.correlation_id
        );

        // Log mutation details
        info!(
            "📊 Mutation details - Schema: {}, Type: {:?}, Fields: {:?}",
            mutation_request.mutation.schema_name,
            mutation_request.mutation.mutation_type,
            mutation_request.mutation.fields_and_values.keys().collect::<Vec<_>>()
        );

        // Actually execute the mutation using the provided closure
        let mutation_result = mutation_executor(mutation_request.mutation.clone())?;
        
        // Calculate execution time
        let execution_time_ms = start_time.elapsed().as_millis() as u64;
        
        // Extract fields affected from the mutation
        let fields_affected: Vec<String> = mutation_request.mutation.fields_and_values.keys().cloned().collect();
        
        // Create mutation context for the executed mutation
        let mutation_context = Some(crate::fold_db_core::infrastructure::message_bus::atom_events::MutationContext {
            key_value: Some(mutation_request.mutation.key_value.clone()),
            mutation_hash: Some(mutation_request.mutation.uuid.clone()),
            incremental: true,
            backfill_hash: mutation_request.mutation.backfill_hash.clone(),
        });
        
        // Emit a MutationExecuted event for the completed mutation
        let mutation_event = MutationExecuted {
            operation: format!("{:?}", mutation_request.mutation.mutation_type).to_lowercase(),
            schema: mutation_request.mutation.schema_name.clone(),
            execution_time_ms,
            fields_affected,
            mutation_context,
        };

        info!("📢 Emitting MutationExecuted event for completed mutation");
        message_bus.publish(mutation_event).map_err(|e| {
            error!("❌ Failed to emit MutationExecuted event: {}", e);
            SchemaError::InvalidData(format!("Failed to emit mutation event: {}", e))
        })?;

        // Perform any additional database operations or logging as needed
        Self::perform_mutation_request_cleanup(mutation_request)?;
        Self::update_mutation_request_statistics(mutation_request)?;

        info!(
            "✅ Successfully executed mutation and emitted MutationExecuted event. Mutation ID: {}, Execution time: {}ms",
            mutation_result, execution_time_ms
        );
        Ok(())
    }


    /// Perform cleanup operations after a mutation request
    fn perform_mutation_request_cleanup(
        mutation_request: &MutationRequest,
    ) -> Result<(), SchemaError> {
        // Example: Update mutation state or perform cleanup
        // This is where you could add any post-mutation processing
        
        info!(
            "🧹 Performing mutation request cleanup for schema: {}, correlation_id: {}",
            mutation_request.mutation.schema_name, mutation_request.correlation_id
        );

        // Example cleanup operations:
        // - Update mutation tracking
        // - Clean up temporary data
        // - Update statistics
        // - Notify other systems

        Ok(())
    }

    /// Update mutation request statistics
    fn update_mutation_request_statistics(
        mutation_request: &MutationRequest,
    ) -> Result<(), SchemaError> {
        info!(
            "📈 Updating mutation request statistics for schema: {}",
            mutation_request.mutation.schema_name
        );

        // Example: Store mutation request statistics
        // Note: In a real implementation, you might want to store these in the database
        // For now, we'll just log the statistics
        info!(
            "📊 Mutation statistics - Schema: {}, Type: {:?}, Fields: {}, Correlation ID: {}",
            mutation_request.mutation.schema_name,
            mutation_request.mutation.mutation_type,
            mutation_request.mutation.fields_and_values.len(),
            mutation_request.correlation_id
        );

        info!("✅ Updated mutation request statistics");
        Ok(())
    }

    /// Get access to the message bus
    pub fn message_bus(&self) -> &MessageBus {
        &self.message_bus
    }

    /// Emit a mutation event (for use by transform completion)
    pub fn emit_mutation_event(
        &self,
        operation: String,
        schema: String,
        execution_time_ms: u64,
        fields_affected: Vec<String>,
        mutation_context: Option<crate::fold_db_core::infrastructure::message_bus::atom_events::MutationContext>,
    ) -> Result<(), SchemaError> {
        let mutation_event = MutationExecuted {
            operation,
            schema,
            execution_time_ms,
            fields_affected,
            mutation_context,
        };

        info!("📢 Emitting MutationExecuted event: {:?}", mutation_event);

        self.message_bus.publish(mutation_event).map_err(|e| {
            error!("❌ Failed to emit MutationExecuted event: {}", e);
            SchemaError::InvalidData(format!("Failed to emit mutation event: {}", e))
        })?;

        info!("✅ Successfully emitted MutationExecuted event");
        Ok(())
    }

    /// Get mutation statistics for a schema
    pub fn get_mutation_statistics(&self, _schema_name: &str) -> Result<Option<Value>, SchemaError> {
        // This is a simplified implementation - in practice you might want to
        // store and retrieve statistics from the database
        info!("📊 Getting mutation statistics (simplified implementation)");
        Ok(None)
    }

    /// List all schemas with mutation statistics
    pub fn list_mutation_statistics(&self) -> Result<Vec<(String, Value)>, SchemaError> {
        // This is a simplified implementation - in practice you might want to
        // iterate through the database to find all mutation_stats:* keys
        info!("📊 Listing mutation statistics (simplified implementation)");
        Ok(Vec::new())
    }
}

impl Drop for MutationEventManager {
    fn drop(&mut self) {
        self.stop();
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::fold_db_core::infrastructure::message_bus::MessageBus;
    use std::sync::Arc;
    use std::time::Duration;

    #[test]
    fn test_mutation_event_manager_creation() {
        let message_bus = Arc::new(MessageBus::new());
        let mutation_executor = |_mutation: Mutation| -> Result<String, SchemaError> {
            Ok("test-mutation-id".to_string())
        };

        let manager = MutationEventManager::new(mutation_executor, message_bus);
        assert!(!manager.is_running());
    }

    #[test]
    fn test_mutation_event_manager_start_stop() {
        let message_bus = Arc::new(MessageBus::new());
        let mutation_executor = |_mutation: Mutation| -> Result<String, SchemaError> {
            Ok("test-mutation-id".to_string())
        };

        let manager = MutationEventManager::new(mutation_executor, message_bus);
        
        // Start the manager
        assert!(manager.start().is_ok());
        assert!(manager.is_running());
        
        // Give it a moment to start
        thread::sleep(Duration::from_millis(100));
        
        // Stop the manager
        manager.stop();
        
        // Give it a moment to stop
        thread::sleep(Duration::from_millis(100));
        assert!(!manager.is_running());
    }

    #[test]
    fn test_emit_mutation_event() {
        let message_bus = Arc::new(MessageBus::new());
        let mutation_executor = |_mutation: Mutation| -> Result<String, SchemaError> {
            Ok("test-mutation-id".to_string())
        };

        let manager = MutationEventManager::new(mutation_executor, message_bus);

        // Test emitting a mutation event
        let result = manager.emit_mutation_event(
            "create".to_string(),
            "TestSchema".to_string(),
            150,
            vec!["field1".to_string(), "field2".to_string()],
            None,
        );

        assert!(result.is_ok());
    }

    #[test]
    fn test_handle_mutation_request() {
        use crate::schema::types::Mutation;
        use crate::MutationType;
        use crate::schema::types::key_value::KeyValue;
        use std::collections::HashMap;
        use serde_json::json;

        let message_bus = Arc::new(MessageBus::new());
        let mutation_executor: Arc<dyn Fn(Mutation) -> Result<String, SchemaError> + Send + Sync> = Arc::new(|_mutation: Mutation| -> Result<String, SchemaError> {
            Ok("test-mutation-id".to_string())
        });

        // Create a test mutation request
        let mut fields_and_values = HashMap::new();
        fields_and_values.insert("field1".to_string(), json!("value1"));
        fields_and_values.insert("field2".to_string(), json!("value2"));

        let mutation = Mutation::new(
            "TestSchema".to_string(),
            fields_and_values,
            KeyValue::new(Some("test_hash".to_string()), Some("test_range".to_string())),
            "test_source".to_string(),
            0,
            MutationType::Create,
        );

        let mutation_request = MutationRequest {
            correlation_id: "test-correlation-id".to_string(),
            mutation,
        };

        // Test that the handler processes the mutation request correctly
        let result = MutationEventManager::handle_mutation_request(
            &mutation_request,
            &mutation_executor,
            &message_bus,
        );

        assert!(result.is_ok());
    }
}
