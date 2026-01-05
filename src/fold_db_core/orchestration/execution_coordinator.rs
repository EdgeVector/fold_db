//! Execution coordination component for the Transform Orchestrator
//!
//! Handles transform execution logic, validation, and result publishing,
//! extracted from the main TransformOrchestrator for better separation of concerns.

use super::queue_manager::QueueItem;
use crate::fold_db_core::infrastructure::message_bus::MessageBus;
use crate::schema::SchemaError;
use crate::transform::manager::{
    types::{TransformResult, TransformRunner},
    TransformManager,
};
use log::{error, info};
use std::sync::Arc;
use std::time::Instant;

/// Coordinates transform execution with proper validation and event publishing
#[derive(Clone)]
pub struct ExecutionCoordinator {
    manager: Arc<TransformManager>,
    message_bus: Arc<MessageBus>,
    _db_ops: Arc<crate::db_operations::DbOperations>,
}

impl ExecutionCoordinator {
    /// Create a new ExecutionCoordinator
    pub fn new(
        manager: Arc<TransformManager>,
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
    pub async fn execute_transform(
        &self,
        item: &QueueItem,
        already_processed: bool,
    ) -> Result<TransformResult, SchemaError> {
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
            // Return empty TransformResult for already processed transforms
            return Ok(TransformResult::new(vec![]));
        }

        // Execute the transform
        self.execute_transform_with_context(transform_id, &None)
            .await
    }

    /// Execute a transform with consolidated execution logic (no helper dependency)
    async fn execute_transform_with_context(
        &self,
        transform_id: &str,
        _mutation_context: &Option<
            crate::fold_db_core::infrastructure::message_bus::atom_events::MutationContext,
        >,
    ) -> Result<TransformResult, SchemaError> {
        info!(
            "🔧 ExecutionCoordinator: Executing transform directly: {}",
            transform_id
        );

        let execution_start_time = Instant::now();

        // Execute transform using the TransformRunner interface
        match self
            .manager
            .execute_transform_with_context(transform_id, _mutation_context)
            .await
        {
            Ok(transform_result) => {
                let duration = execution_start_time.elapsed();
                info!(
                    "✅ Transform {} executed successfully in {:?}: {} records",
                    transform_id,
                    duration,
                    transform_result.records.len()
                );

                // Publish success event
                self.publish_success_event(
                    transform_id,
                    &format!("{} records produced", transform_result.records.len()),
                )?;

                Ok(transform_result)
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
    pub async fn execute_transforms_batch(
        &self,
        items: Vec<(QueueItem, bool)>,
    ) -> Vec<Result<TransformResult, SchemaError>> {
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

            let result = self.execute_transform(&item, already_processed).await;

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
    pub async fn execute_transform_with_retry(
        &self,
        item: &QueueItem,
        already_processed: bool,
        max_retries: u32,
    ) -> Result<TransformResult, SchemaError> {
        let mut attempts = 0;
        let mut last_error = None;

        while attempts <= max_retries {
            if attempts > 0 {
                info!("🔄 Retry attempt {} for transform: {}", attempts, item.id);
            }

            match self.execute_transform(item, already_processed).await {
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
                        tokio::time::sleep(delay).await;
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
    pub fn get_manager(&self) -> &Arc<TransformManager> {
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
