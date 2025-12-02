use crate::error::{FoldDbError, FoldDbResult};
use crate::schema::types::{Mutation, Query, KeyValue};
use serde_json::Value;
use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::Mutex;
use crate::schema::types::operations::{MutationType, Operation};
use super::DataFoldNode;
use super::response_types::QueryResultMap;

/// Centralized operation processor that handles all operation types consistently.
/// 
/// This eliminates code duplication across HTTP routes, TCP server, CLI, and direct API usage.
/// All operation execution goes through this single processor to ensure consistent behavior.
pub struct OperationProcessor {
    node: Arc<Mutex<DataFoldNode>>,
}

impl OperationProcessor {
    /// Creates a new operation processor with a reference to the DataFoldNode.
    pub fn new(node: Arc<Mutex<DataFoldNode>>) -> Self {
        Self { node }
    }

    /// Executes a query and returns raw structured results, not JSON.
    pub async fn execute_query_map(
        &self,
        query: Query,
    ) -> FoldDbResult<QueryResultMap> {
        let node_guard = self.node.lock().await;
        let results = DataFoldNode::query(&node_guard, query)?;
        Ok(results)
    }

    /// Executes a mutation operation and returns its mutation ID.
    pub async fn execute_mutation(
        &self,
        schema: String,
        fields_and_values: HashMap<String, Value>,
        key_value: KeyValue,
        mutation_type: MutationType,
    ) -> FoldDbResult<String> {
        if fields_and_values.is_empty() {
            return Err(FoldDbError::Config("No fields to mutate".to_string()));
        }

        let schema_name = schema.clone();
        let mutation = Mutation::new(
            schema,
            fields_and_values,
            key_value,
            String::new(),
            0,
            mutation_type,
        );

        // Check if we're using DynamoDB - if so, avoid spawn_blocking to prevent deadlocks
        let is_dynamodb = {
            let node_guard = self.node.lock().await;
            let db_guard = node_guard.get_fold_db()?;
            db_guard.db_ops.is_dynamodb()
        };
        
        log::info!("🔄 Starting mutation execution for schema: {}", schema_name);
        
        let mut ids = if is_dynamodb {
            // For DynamoDB, call mutation directly in async context (no spawn_blocking)
            // This avoids the deadlock from block_on inside spawn_blocking
            log::info!("📊 Using direct async path for DynamoDB to avoid deadlocks");
            let node_guard = self.node.lock().await;
            let mut db_guard = node_guard.get_fold_db()?;
            // Call mutate_batch directly - it will use run_async which should work
            // since we're already in an async context (not spawn_blocking)
            db_guard.mutation_manager.write_mutations_batch(vec![mutation])
                .map_err(|e| {
                    log::error!("❌ Mutation execution failed: {}", e);
                    FoldDbError::Config(format!("Mutation execution failed: {}", e))
                })?
        } else {
            // For non-DynamoDB backends, use spawn_blocking as before
            let node = {
                let node_guard = self.node.lock().await;
                node_guard.clone()
            };
            
            match tokio::task::spawn_blocking(move || {
                log::info!("📝 Executing mutation in blocking context");
                node.mutate_batch(vec![mutation])
            })
            .await
            {
                Ok(result) => {
                    log::info!("✅ Mutation task completed, processing result");
                    result.map_err(|e| {
                        log::error!("❌ Mutation execution failed: {}", e);
                        FoldDbError::Config(format!("Mutation execution failed: {}", e))
                    })?
                }
                Err(e) => {
                    log::error!("❌ Failed to spawn blocking task for mutation: {}", e);
                    return Err(FoldDbError::Config(format!("Failed to execute mutation: {}", e)));
                }
            }
        };
        
        log::info!("📊 Mutation returned {} IDs", ids.len());
        match ids.pop() {
            Some(id) => {
                log::info!("✅ Mutation succeeded with ID: {}", id);
                Ok(id)
            }
            None => {
                log::error!("❌ Batch mutation returned no IDs");
                Err(FoldDbError::Config("Batch mutation returned no IDs".to_string()))
            }
        }
    }

    /// Executes multiple mutations in a batch for improved performance.
    pub async fn execute_mutations_batch(
        &self,
        mutations_data: Vec<Value>,
    ) -> FoldDbResult<Vec<String>> {
        if mutations_data.is_empty() {
            return Ok(Vec::new());
        }

        let mut mutations = Vec::new();

        // Parse each mutation from the input data
        for mutation_data in mutations_data {
            let (schema, fields_and_values, key_value, mutation_type, source_file_name) = match serde_json::from_value::<Operation>(mutation_data) {
                Ok(Operation::Mutation { schema, fields_and_values, key_value, mutation_type, source_file_name }) => {
                    (schema, fields_and_values, key_value, mutation_type, source_file_name)
                },
                Err(e) => {
                    return Err(FoldDbError::Config(format!("Failed to parse mutation: {}", e)));
                }
            };

            if fields_and_values.is_empty() {
                return Err(FoldDbError::Config("No fields to mutate".to_string()));
            }

            let mut mutation = Mutation::new(
                schema,
                fields_and_values,
                key_value,
                String::new(),
                0,
                mutation_type,
            );

            // Add source_file_name if provided
            if let Some(filename) = source_file_name {
                mutation = mutation.with_source_file_name(filename);
            }

            mutations.push(mutation);
        }

        // Clone the node (Arc-based, so this is cheap) to move into blocking task
        let node = {
            let node_guard = self.node.lock().await;
            node_guard.clone()
        };
        
        // Execute the blocking mutation operation in a blocking thread pool
        // This prevents deadlocks when mutate_batch uses block_on internally
        let mutation_ids = tokio::task::spawn_blocking(move || {
            node.mutate_batch(mutations)
        })
        .await
        .map_err(|e| FoldDbError::Config(format!("Failed to execute mutations: {}", e)))?
        .map_err(|e| FoldDbError::Config(format!("Mutation execution failed: {}", e)))?;

        Ok(mutation_ids)
    }

    // Removed execute_sync as part of eliminating the generic execute path
}

#[cfg(test)]
mod tests {
    #[tokio::test]
    async fn test_operation_processor_creation() {
        // This test would require a mock DataFoldNode
        // For now, just test that the struct can be created
        // In a real test, you'd create a test DataFoldNode instance
    }
}
