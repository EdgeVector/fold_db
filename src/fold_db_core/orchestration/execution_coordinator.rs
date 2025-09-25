//! Execution coordination component for the Transform Orchestrator
//!
//! Handles transform execution logic, validation, and result publishing,
//! extracted from the main TransformOrchestrator for better separation of concerns.

use super::queue_manager::QueueItem;
use crate::fold_db_core::infrastructure::message_bus::MessageBus;
use crate::fold_db_core::transform_manager::types::TransformRunner;
use crate::fold_db_core::transform_manager::utils::EventPublisher;
use crate::schema::SchemaError;
use log::{error, info};
use serde_json::Value as JsonValue;
use std::sync::Arc;
use std::time::Instant;

/// Coordinates transform execution with proper validation and event publishing
pub struct ExecutionCoordinator {
    manager: Arc<dyn TransformRunner>,
    message_bus: Arc<MessageBus>,
    _db_ops: Arc<crate::db_operations::DbOperations>,
}

impl ExecutionCoordinator {
    /// Create a new ExecutionCoordinator
    pub fn new(
        manager: Arc<dyn TransformRunner>,
        message_bus: Arc<MessageBus>,
        db_ops: Arc<crate::db_operations::DbOperations>,
    ) -> Self {
        Self {
            manager,
            message_bus,
            _db_ops: db_ops,
        }
    }

    /// Execute a transform with full coordination (validation, execution, publishing)
    pub fn execute_transform(
        &self,
        item: &QueueItem,
        already_processed: bool,
    ) -> Result<JsonValue, SchemaError> {
        let transform_id = &item.id;
        let mutation_hash = &item.mutation_hash;

        info!("🚀 EXECUTING TRANSFORM: {}", transform_id);
        info!(
            "🔧 Transform details - ID: {}, mutation_hash: {}, already_processed: {}",
            transform_id, mutation_hash, already_processed
        );

        if already_processed {
            info!(
                "⏭️ Transform {} already processed, skipping execution",
                transform_id
            );
            return Ok(serde_json::json!({
                "status": "skipped_already_processed",
                "transform_id": transform_id,
                "mutation_hash": mutation_hash
            }));
        }

        // Validate transform exists before execution
        self.validate_transform_exists(transform_id)?;

        // Execute the transform
        let execution_result = self.execute_transform_with_context(transform_id, &None);

        // Publish execution result using EventPublisher utility
        EventPublisher::handle_execution_result_and_publish(
            &self.message_bus,
            transform_id,
            &execution_result,
        );

        execution_result
    }

    /// Validate that a transform exists before execution using ValidationHelper patterns
    pub fn validate_transform_exists(&self, transform_id: &str) -> Result<(), SchemaError> {
        info!("🔍 Validating transform exists: {}", transform_id);

        // Use consistent validation pattern from ValidationHelper
        if transform_id.trim().is_empty() {
            let error_msg = "Transform ID cannot be empty".to_string();
            error!("❌ {}", error_msg);
            return Err(SchemaError::InvalidData(error_msg));
        }

        match self.manager.transform_exists(transform_id) {
            Ok(exists) => {
                if !exists {
                    let error_msg = format!("Transform '{}' not found", transform_id);
                    error!("❌ {}", error_msg);
                    Err(SchemaError::InvalidData(error_msg))
                } else {
                    info!("✅ Transform exists: {}", transform_id);
                    Ok(())
                }
            }
            Err(e) => {
                let error_msg = format!(
                    "Error checking transform existence for {}: {}",
                    transform_id, e
                );
                error!("❌ {}", error_msg);
                Err(SchemaError::InvalidData(error_msg))
            }
        }
    }

    /// Execute a transform with consolidated execution logic (no helper dependency)
    fn execute_transform_with_context(&self, transform_id: &str, _mutation_context: &Option<crate::fold_db_core::infrastructure::message_bus::atom_events::MutationContext>) -> Result<JsonValue, SchemaError> {
        info!(
            "🔧 ExecutionCoordinator: Executing transform directly: {}",
            transform_id
        );

        let execution_start_time = Instant::now();

        // Execute transform using the TransformRunner interface
        match self.manager.execute_transform_with_context(transform_id, _mutation_context) {
            Ok(result) => {
                let duration = execution_start_time.elapsed();
                info!(
                    "✅ Transform {} executed successfully in {:?}: {}",
                    transform_id, duration, result
                );

                // Publish success event
                self.publish_success_event(transform_id, &result.to_string())?;

                Ok(serde_json::json!({
                    "status": "executed_from_queue",
                    "transform_id": transform_id,
                    "result": result,
                    "method": "delegated_execution",
                    "duration_ms": duration.as_millis(),
                    "execution_time": chrono::Utc::now().to_rfc3339()
                }))
            }
            Err(e) => {
                let duration = execution_start_time.elapsed();
                error!(
                    "❌ Transform {} failed during execution after {:?}: {}",
                    transform_id, duration, e
                );
                error!("❌ Execution error details: {:?}", e);

                // Publish failure event
                self.publish_failure_event(transform_id, &e.to_string())?;

                Err(SchemaError::InvalidData(format!(
                    "Transform execution failed: {}",
                    e
                )))
            }
        }
    }

    /// Publish success event with consistent error handling
    fn publish_success_event(&self, transform_id: &str, result: &str) -> Result<(), SchemaError> {
        use crate::fold_db_core::infrastructure::message_bus::schema_events::TransformExecuted;

        info!("📢 Publishing TransformExecuted success event...");

        let executed_event = TransformExecuted {
            transform_id: transform_id.to_string(),
            result: format!("computed_result: {}", result),
        };

        self.message_bus.publish(executed_event).map_err(|e| {
            error!(
                "❌ Failed to publish TransformExecuted success event for {}: {}",
                transform_id, e
            );
            SchemaError::InvalidData(format!("Failed to publish success event: {}", e))
        })?;

        info!(
            "✅ Published TransformExecuted success event for: {}",
            transform_id
        );
        Ok(())
    }

    /// Publish failure event with consistent error handling
    fn publish_failure_event(
        &self,
        transform_id: &str,
        error_msg: &str,
    ) -> Result<(), SchemaError> {
        use crate::fold_db_core::infrastructure::message_bus::schema_events::TransformExecuted;

        info!(
            "📢 Publishing TransformExecuted failure event for: {}",
            transform_id
        );

        let executed_event = TransformExecuted {
            transform_id: transform_id.to_string(),
            result: format!("execution_error: {}", error_msg),
        };

        self.message_bus.publish(executed_event).map_err(|e| {
            error!(
                "❌ Failed to publish TransformExecuted failure event for {}: {}",
                transform_id, e
            );
            SchemaError::InvalidData(format!("Failed to publish failure event: {}", e))
        })?;

        info!(
            "✅ Published TransformExecuted failure event for transform: {}",
            transform_id
        );
        Ok(())
    }

    /// Execute multiple transforms in sequence
    pub fn execute_transforms_batch(
        &self,
        items: Vec<(QueueItem, bool)>,
    ) -> Vec<Result<JsonValue, SchemaError>> {
        info!(
            "🚀 BATCH EXECUTION START - executing {} transforms",
            items.len()
        );

        let mut results = Vec::with_capacity(items.len());

        for (index, (item, already_processed)) in items.into_iter().enumerate() {
            info!(
                "🔄 Executing transform {}: {} (batch item {}/{})",
                index + 1,
                item.id,
                index + 1,
                results.capacity()
            );

            let result = self.execute_transform(&item, already_processed);

            match &result {
                Ok(value) => {
                    info!(
                        "✅ Batch item {} completed successfully: {:?}",
                        index + 1,
                        value
                    );
                }
                Err(e) => {
                    error!("❌ Batch item {} failed: {:?}", index + 1, e);
                }
            }

            results.push(result);
        }

        info!(
            "🏁 BATCH EXECUTION COMPLETE - processed {} transforms",
            results.len()
        );
        results
    }

    /// Execute transforms with retry logic
    pub fn execute_transform_with_retry(
        &self,
        item: &QueueItem,
        already_processed: bool,
        max_retries: u32,
    ) -> Result<JsonValue, SchemaError> {
        let mut attempts = 0;
        let mut last_error = None;

        while attempts <= max_retries {
            if attempts > 0 {
                info!("🔄 Retry attempt {} for transform: {}", attempts, item.id);
            }

            match self.execute_transform(item, already_processed) {
                Ok(result) => {
                    if attempts > 0 {
                        info!(
                            "✅ Transform {} succeeded on retry attempt {}",
                            item.id, attempts
                        );
                    }
                    return Ok(result);
                }
                Err(e) => {
                    attempts += 1;
                    last_error = Some(e);

                    if attempts <= max_retries {
                        let delay = std::time::Duration::from_millis(100 * attempts as u64);
                        error!(
                            "❌ Transform {} failed on attempt {}, retrying in {:?}",
                            item.id, attempts, delay
                        );
                        std::thread::sleep(delay);
                    }
                }
            }
        }

        let final_error = last_error.unwrap_or_else(|| {
            SchemaError::InvalidData("Unknown error during retry execution".to_string())
        });

        error!(
            "❌ Transform {} failed after {} attempts",
            item.id, attempts
        );
        Err(final_error)
    }

    /// Get execution statistics for monitoring
    pub fn get_execution_stats(&self) -> ExecutionStats {
        // In a real implementation, this would track actual statistics
        ExecutionStats {
            total_executions: 0,
            successful_executions: 0,
            failed_executions: 0,
            average_execution_time_ms: 0,
        }
    }

    /// Get access to the underlying transform manager
    pub fn get_manager(&self) -> &Arc<dyn TransformRunner> {
        &self.manager
    }

    /// Get access to the message bus
    pub fn get_message_bus(&self) -> &Arc<MessageBus> {
        &self.message_bus
    }
}

/// Statistics for transform execution monitoring
#[derive(Debug, Clone)]
pub struct ExecutionStats {
    pub total_executions: u64,
    pub successful_executions: u64,
    pub failed_executions: u64,
    pub average_execution_time_ms: u64,
}

impl ExecutionStats {
    pub fn success_rate(&self) -> f64 {
        if self.total_executions == 0 {
            0.0
        } else {
            self.successful_executions as f64 / self.total_executions as f64
        }
    }

    pub fn failure_rate(&self) -> f64 {
        if self.total_executions == 0 {
            0.0
        } else {
            self.failed_executions as f64 / self.total_executions as f64
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::fold_db_core::transform_manager::types::TransformRunner;
    use std::collections::HashSet;

    struct MockTransformRunner {
        should_succeed: bool,
        execution_delay_ms: u64,
    }

    impl MockTransformRunner {
        fn new(should_succeed: bool) -> Self {
            Self {
                should_succeed,
                execution_delay_ms: 0,
            }
        }
    }

    impl TransformRunner for MockTransformRunner {

        fn execute_transform_with_context(
            &self,
            transform_id: &str,
            mutation_context: &Option<
                crate::fold_db_core::infrastructure::message_bus::atom_events::MutationContext,
            >,
        ) -> Result<JsonValue, SchemaError> {
            if self.execution_delay_ms > 0 {
                std::thread::sleep(std::time::Duration::from_millis(self.execution_delay_ms));
            }

            if self.should_succeed {
                if let Some(ref context) = mutation_context {
                    Ok(serde_json::json!({
                        "status": "success_with_context",
                        "transform_id": transform_id,
                        "range_key": context.range_key,
                        "hash_key": context.hash_key,
                        "incremental": context.incremental
                    }))
                } else {
                    Ok(serde_json::json!({
                        "status": "success_with_context",
                        "transform_id": transform_id,
                        "no_context": true
                    }))
                }
            } else {
                Err(SchemaError::InvalidData(
                    "Mock execution failure".to_string(),
                ))
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
            Ok(HashSet::new())
        }

        fn get_transforms_for_schema(&self, _schema_name: &str) -> Result<HashSet<String>, SchemaError> {
            Ok(HashSet::new())
        }
    }

    #[test]
    fn test_successful_execution() {
        let message_bus =
            Arc::new(crate::fold_db_core::infrastructure::message_bus::MessageBus::new());
        let manager = Arc::new(MockTransformRunner::new(true));
        let (db_ops, _message_bus) =
            crate::testing_utils::TestDatabaseFactory::create_test_environment().unwrap();
        let coordinator = ExecutionCoordinator::new(manager, message_bus, db_ops);

        let item = QueueItem {
            id: "test_transform".to_string(),
            mutation_hash: "test_hash".to_string(),
        };

        let result = coordinator.execute_transform(&item, false);
        assert!(result.is_ok());

        let value = result.unwrap();
        assert_eq!(value["status"], "executed_from_queue");
        assert_eq!(value["transform_id"], "test_transform");
    }

    #[test]
    fn test_already_processed_skip() {
        let message_bus =
            Arc::new(crate::fold_db_core::infrastructure::message_bus::MessageBus::new());
        let manager = Arc::new(MockTransformRunner::new(true));
        let (db_ops, _message_bus) =
            crate::testing_utils::TestDatabaseFactory::create_test_environment().unwrap();
        let coordinator = ExecutionCoordinator::new(manager, message_bus, db_ops);

        let item = QueueItem {
            id: "test_transform".to_string(),
            mutation_hash: "test_hash".to_string(),
        };

        let result = coordinator.execute_transform(&item, true);
        assert!(result.is_ok());

        let value = result.unwrap();
        assert_eq!(value["status"], "skipped_already_processed");
    }

    #[test]
    fn test_failed_execution() {
        let message_bus =
            Arc::new(crate::fold_db_core::infrastructure::message_bus::MessageBus::new());
        let manager = Arc::new(MockTransformRunner::new(false));
        let (db_ops, _message_bus) =
            crate::testing_utils::TestDatabaseFactory::create_test_environment().unwrap();
        let coordinator = ExecutionCoordinator::new(manager, message_bus, db_ops);

        let item = QueueItem {
            id: "test_transform".to_string(),
            mutation_hash: "test_hash".to_string(),
        };

        let result = coordinator.execute_transform(&item, false);
        assert!(result.is_err());
    }

    #[test]
    fn test_batch_execution() {
        let message_bus =
            Arc::new(crate::fold_db_core::infrastructure::message_bus::MessageBus::new());
        let manager = Arc::new(MockTransformRunner::new(true));
        let (db_ops, _message_bus) =
            crate::testing_utils::TestDatabaseFactory::create_test_environment().unwrap();
        let coordinator = ExecutionCoordinator::new(manager, message_bus, db_ops);

        let items = vec![
            (
                QueueItem {
                    id: "transform1".to_string(),
                    mutation_hash: "hash1".to_string(),
                },
                false,
            ),
            (
                QueueItem {
                    id: "transform2".to_string(),
                    mutation_hash: "hash2".to_string(),
                },
                true,
            ), // Already processed
        ];

        let results = coordinator.execute_transforms_batch(items);
        assert_eq!(results.len(), 2);
        assert!(results[0].is_ok());
        assert!(results[1].is_ok());

        // First should be executed, second should be skipped
        assert_eq!(
            results[0].as_ref().unwrap()["status"],
            "executed_from_queue"
        );
        assert_eq!(
            results[1].as_ref().unwrap()["status"],
            "skipped_already_processed"
        );
    }

    #[test]
    fn test_retry_execution() {
        let message_bus =
            Arc::new(crate::fold_db_core::infrastructure::message_bus::MessageBus::new());
        let manager = Arc::new(MockTransformRunner::new(false)); // Always fails
        let (db_ops, _message_bus) =
            crate::testing_utils::TestDatabaseFactory::create_test_environment().unwrap();
        let coordinator = ExecutionCoordinator::new(manager, message_bus, db_ops);

        let item = QueueItem {
            id: "test_transform".to_string(),
            mutation_hash: "test_hash".to_string(),
        };

        let result = coordinator.execute_transform_with_retry(&item, false, 2);
        assert!(result.is_err());
    }
}
