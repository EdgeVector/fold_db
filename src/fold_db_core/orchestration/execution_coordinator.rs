//! Execution coordination component for the Transform Orchestrator
//!
//! Handles transform execution logic, validation, and result publishing,
//! extracted from the main TransformOrchestrator for better separation of concerns.

use super::queue_manager::QueueItem;
use crate::fold_db_core::infrastructure::message_bus::{AsyncMessageBus, Event};
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
    message_bus: Arc<AsyncMessageBus>,
}

impl ExecutionCoordinator {
    /// Create a new ExecutionCoordinator
    pub fn new(
        manager: Arc<TransformManager>,
        message_bus: Arc<AsyncMessageBus>,
    ) -> Self {
        Self {
            manager,
            message_bus,
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
                )
                .await?;

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
                self.publish_failure_event(transform_id, &e.to_string())
                    .await?;

                Err(SchemaError::InvalidData(format!(
                    "Transform execution failed: {}",
                    e
                )))
            }
        }
    }

    /// Publish success event with consistent error handling
    async fn publish_success_event(
        &self,
        transform_id: &str,
        result: &str,
    ) -> Result<(), SchemaError> {
        use crate::fold_db_core::infrastructure::message_bus::schema_events::TransformExecuted;

        info!("📢 Publishing TransformExecuted success event...");

        let executed_event = TransformExecuted {
            transform_id: transform_id.to_string(),
            result: format!("computed_result: {}", result),
        };

        self.message_bus
            .publish_event(Event::TransformExecuted(executed_event))
            .await
            .map_err(|e| {
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
    async fn publish_failure_event(
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

        self.message_bus
            .publish_event(Event::TransformExecuted(executed_event))
            .await
            .map_err(|e| {
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

    /// Get access to the underlying transform manager
    pub fn get_manager(&self) -> &Arc<TransformManager> {
        &self.manager
    }

}
