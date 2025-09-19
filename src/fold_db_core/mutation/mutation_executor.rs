//! Mutation Executor
//! 
//! Main mutation execution logic extracted from FoldDB core, handling all mutation operations
//! including write_schema and wait_for_mutation functionality.

use crate::schema::SchemaError;
use crate::schema::types::Mutation;
use crate::db_operations::DbOperations;
use crate::schema::SchemaCore;
use crate::fold_db_core::infrastructure::MessageBus;
use crate::fold_db_core::mutation_completion_handler::{MutationCompletionHandler, MutationCompletionError};
use crate::fold_db_core::services::mutation::MutationService;
use crate::logging::features::{log_feature, LogFeature};
use crate::fold_db_core::infrastructure::message_bus::query_events::MutationExecuted;
use log::warn;
use std::sync::Arc;
use std::time::Instant;
use uuid;

use super::mutation_processor::MutationProcessor;

/// Main mutation executor that handles all mutation operations
pub struct MutationExecutor {
    db_ops: Arc<DbOperations>,
    message_bus: Arc<MessageBus>,
    completion_handler: Arc<MutationCompletionHandler>,
    mutation_processor: MutationProcessor,
}

impl MutationExecutor {
    /// Create a new mutation executor
    pub fn new(
        db_ops: Arc<DbOperations>,
        schema_manager: Arc<SchemaCore>,
        message_bus: Arc<MessageBus>,
        completion_handler: Arc<MutationCompletionHandler>,
    ) -> Self {
        let mutation_processor = MutationProcessor::new(
            Arc::clone(&schema_manager),
        );
        
        Self {
            db_ops,
            message_bus,
            completion_handler,
            mutation_processor,
        }
    }

    /// Write schema operation - main orchestration method for mutations
    pub fn write_schema(&mut self, mutation: Mutation) -> Result<String, SchemaError> {
        let start_time = Instant::now();
        let fields_count = mutation.fields_and_values.len();
        let operation_type = format!("{:?}", mutation.mutation_type);
        let schema_name = mutation.schema_name.clone();
        
        // Generate unique mutation ID
        let mutation_id = uuid::Uuid::new_v4().to_string();
        
        log_feature!(LogFeature::Mutation, info,
            "Starting mutation execution for schema: {} with ID: {}",
            mutation.schema_name, mutation_id
        );
        log_feature!(LogFeature::Mutation, info, "Mutation type: {:?}", mutation.mutation_type);
        log_feature!(LogFeature::Mutation, info,
            "Fields to mutate: {:?}",
            mutation.fields_and_values.keys().collect::<Vec<_>>()
        );

        if mutation.fields_and_values.is_empty() {
            return Err(SchemaError::InvalidData("No fields to write".to_string()));
        }

        // Register mutation with completion handler for tracking BEFORE execution starts
        log_feature!(LogFeature::Mutation, info, "Generated mutation ID {} for completion tracking", mutation_id);
        
        // Pre-register the mutation to ensure wait_for_mutation can find it
        let completion_handler = Arc::clone(&self.completion_handler);
        let mutation_id_for_registration = mutation_id.clone();
        
        // Register the mutation synchronously to avoid race conditions
        let _receiver = completion_handler.register_mutation_sync(mutation_id_for_registration);
        log_feature!(LogFeature::Mutation, info, "Pre-registered mutation {} with completion handler", mutation_id);

        // 1. Prepare mutation and validate schema
        let (schema, processed_mutation, mutation_hash) = self.mutation_processor.prepare_mutation_and_schema(mutation)?;

        // 2. Create mutation service and delegate field updates
        let mutation_service = MutationService::new(Arc::clone(&self.message_bus));
        let result = self.mutation_processor.process_field_mutations_via_service(&mutation_service, &schema, &processed_mutation, &mutation_hash);
        
        // 3. Publish MutationExecuted event
        let execution_time_ms = start_time.elapsed().as_millis() as u64;
        let mutation_event = MutationExecuted::new(
            operation_type,
            schema_name,
            execution_time_ms,
            fields_count,
        );

        if let Err(e) = self.message_bus.publish(mutation_event) {
            warn!("Failed to publish MutationExecuted event: {}", e);
        }

        // 4. Signal completion for mutation tracking AFTER database operations are complete
        // This ensures wait_for_mutation doesn't return until atoms are persisted
        if processed_mutation.synchronous.unwrap_or(false) {
            // Synchronous mode: signal completion immediately
            log_feature!(LogFeature::Mutation, info, "Synchronous mode: signaling completion immediately for mutation ID {}", mutation_id);
            if let Err(e) = self.completion_handler.signal_completion_sync(&mutation_id) {
                warn!("Failed to signal synchronous completion for mutation {}: {}", mutation_id, e);
            }
        } else {
            // Asynchronous mode: spawn task to signal completion AFTER database persistence
            let completion_handler = Arc::clone(&self.completion_handler);
            let mutation_id_clone = mutation_id.clone();
            let db_ops = Arc::clone(&self.db_ops);

            tokio::spawn(async move {
                // Wait a bit to ensure database operations are fully persisted
                tokio::time::sleep(tokio::time::Duration::from_millis(100)).await;

                // Flush the database to ensure all writes are persisted
                if let Err(e) = db_ops.db().flush() {
                    log_feature!(LogFeature::Mutation, warn, "Failed to flush database before signaling completion: {}", e);
                } else {
                    log_feature!(LogFeature::Mutation, info, "Database flushed before signaling mutation completion");
                }

                // Now signal completion
                completion_handler.signal_completion(&mutation_id_clone).await;
                log_feature!(LogFeature::Mutation, info, "✅ Mutation {} completion signaled after database persistence", mutation_id_clone);
            });
        }

        // Return the mutation ID regardless of result
        match result {
            Ok(()) => Ok(mutation_id),
            Err(e) => Err(e),
        }
    }

    /// Waits for a specific mutation to complete processing.
    ///
    /// This method allows queries and other operations to wait for specific mutations to finish
    /// processing before executing, solving the race condition where queries try to access data
    /// before mutations finish processing. This is the core functionality that eliminates
    /// "Atom not found" errors.
    ///
    /// # Arguments
    ///
    /// * `mutation_id` - The unique identifier of the mutation to wait for completion
    ///
    /// # Returns
    ///
    /// Returns `Ok(())` if the mutation completed successfully within the timeout period,
    /// or a `SchemaError` if the operation failed.
    ///
    /// # Timeout Behavior
    ///
    /// Uses the default 5-second timeout as defined in `MutationCompletionHandler`.
    /// If the mutation does not complete within this timeframe, a timeout error is returned.
    ///
    /// # Error Handling
    ///
    /// - **Timeout**: Returns `SchemaError::InvalidData("Mutation failed")` when the mutation
    ///   does not complete within the 5-second timeout
    /// - **Invalid ID**: Returns `SchemaError::InvalidData` with details when the mutation ID
    ///   is not found or was never registered
    /// - **System Error**: Returns appropriate `SchemaError` for lock failures or channel errors
    ///
    /// # Usage Examples
    ///
    /// ## Basic Usage
    /// ```no_run
    /// use datafold::fold_db_core::mutation::MutationExecutor;
    /// use datafold::schema::types::{Mutation, MutationType};
    /// use std::collections::HashMap;
    /// use serde_json::Value;
    ///
    /// # async fn example() -> Result<(), Box<dyn std::error::Error>> {
    /// // Create a mutation executor (this would be done by FoldDB internally)
    /// // let executor = MutationExecutor::new(...);
    /// 
    /// // Execute a mutation and get the mutation ID
    /// let fields = HashMap::new();
    /// let mutation = Mutation::new(
    ///     "schema_name".to_string(),
    ///     fields,
    ///     "pub_key".to_string(),
    ///     0,
    ///     MutationType::Update
    /// );
    /// // let mutation_id = executor.write_schema(mutation)?;
    ///
    /// // Wait for the mutation to complete before querying
    /// // executor.wait_for_mutation(&mutation_id).await?;
    /// # Ok(())
    /// # }
    /// ```
    ///
    /// ## Error Handling Example
    /// ```no_run
    /// use datafold::fold_db_core::mutation::MutationExecutor;
    ///
    /// # async fn example() -> Result<(), Box<dyn std::error::Error>> {
    /// // Create a mutation executor (this would be done by FoldDB internally)
    /// // let executor = MutationExecutor::new(...);
    /// 
    /// # Ok(())
    /// # }
    /// ```
    ///
    /// # Implementation Notes
    ///
    /// This method integrates with the `MutationCompletionHandler` to provide efficient
    /// completion tracking. The mutation must be registered with the completion handler
    /// (typically done by `write_schema`) before calling this method.
    ///
    /// The method is async and non-blocking, allowing other operations to continue while
    /// waiting for mutation completion.
    pub async fn wait_for_mutation(&self, mutation_id: &str) -> Result<(), SchemaError> {
        log_feature!(LogFeature::Mutation, info,
            "Waiting for completion of mutation: {}", mutation_id);
        
        // Use the completion handler to wait for the mutation with default timeout
        match self.completion_handler.wait_for_completion(mutation_id).await {
            Ok(()) => {
                log_feature!(LogFeature::Mutation, info,
                    "Mutation {} completed successfully", mutation_id);
                Ok(())
            }
            Err(mutation_error) => {
                // Convert MutationCompletionError to SchemaError with appropriate error messages
                let schema_error = match mutation_error {
                    MutationCompletionError::Timeout(id, duration) => {
                        log_feature!(LogFeature::Mutation, warn,
                            "Mutation {} timed out after {:?}", id, duration);
                        SchemaError::InvalidData("Mutation failed".to_string())
                    }
                    MutationCompletionError::MutationNotFound(id) => {
                        log_feature!(LogFeature::Mutation, warn,
                            "Mutation {} not found in completion tracking system", id);
                        SchemaError::InvalidData(format!(
                            "Mutation '{}' not found in tracking system. The mutation may have already completed, never been registered, or the ID is invalid.",
                            id
                        ))
                    }
                    MutationCompletionError::SignalFailed(id, reason) => {
                        log_feature!(LogFeature::Mutation, error,
                            "Failed to receive completion signal for mutation {}: {}", id, reason);
                        SchemaError::InvalidData(format!(
                            "Mutation '{}' completion signal failed: {}. This may indicate a system error or that the mutation process was interrupted.",
                            id, reason
                        ))
                    }
                    MutationCompletionError::LockFailed(reason) => {
                        log_feature!(LogFeature::Mutation, error,
                            "Failed to acquire lock for mutation completion tracking: {}", reason);
                        SchemaError::InvalidData(format!(
                            "Failed to access mutation tracking system: {}. This may indicate high system load or a concurrency issue.",
                            reason
                        ))
                    }
                };
                
                Err(schema_error)
            }
        }
    }
}
