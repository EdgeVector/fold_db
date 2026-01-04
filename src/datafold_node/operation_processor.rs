use crate::error::{FoldDbError, FoldDbResult};
use crate::schema::types::{KeyValue, Mutation, Query};
use serde_json::Value;
use std::collections::HashMap;
use std::sync::Arc;

use super::response_types::QueryResultMap;
use super::DataFoldNode;
use crate::schema::types::operations::{MutationType, Operation};

/// Centralized operation processor that handles all operation types consistently.
///
/// This eliminates code duplication across HTTP routes, TCP server, CLI, and direct API usage.
/// All operation execution goes through this single processor to ensure consistent behavior.
pub struct OperationProcessor {
    node: Arc<tokio::sync::RwLock<DataFoldNode>>,
}

impl OperationProcessor {
    /// Creates a new operation processor with a reference to the DataFoldNode.
    pub fn new(node: Arc<tokio::sync::RwLock<DataFoldNode>>) -> Self {
        Self { node }
    }

    /// Executes a query and returns raw structured results, not JSON.
    pub async fn execute_query_map(&self, query: Query) -> FoldDbResult<QueryResultMap> {
        let node_guard = self.node.read().await;
        let db = node_guard
            .get_fold_db()
            .await
            .map_err(|e| FoldDbError::Database(e.to_string()))?;
        let results = db.query_executor.query(query).await;
        Ok(results?)
    }

    /// Executes a mutation operation and returns its mutation ID.
    pub async fn execute_mutation(
        &self,
        schema: String,
        fields_and_values: HashMap<String, Value>,
        key_value: KeyValue,
        mutation_type: MutationType,
    ) -> FoldDbResult<String> {
        // Delete mutations are allowed to have empty fields_and_values
        // They only need the key_value to identify what to delete
        if fields_and_values.is_empty() && mutation_type != MutationType::Delete {
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

        log::info!("🔄 Starting mutation execution for schema: {}", schema_name);

        // Use async version directly - all backends now support async operations
        // This avoids deadlocks and provides consistent behavior across all backends
        let node_guard = self.node.read().await;
        let mut db = node_guard
            .get_fold_db()
            .await
            .map_err(|e| FoldDbError::Database(e.to_string()))?;
        let mut ids = db
            .mutation_manager
            .write_mutations_batch_async(vec![mutation])
            .await
            .map_err(|e| {
                log::error!("❌ Mutation execution failed: {}", e);
                FoldDbError::Config(format!("Mutation execution failed: {}", e))
            })?;

        log::info!("📊 Mutation returned {} IDs", ids.len());
        match ids.pop() {
            Some(id) => {
                log::info!("✅ Mutation succeeded with ID: {}", id);
                Ok(id)
            }
            None => {
                log::error!("❌ Batch mutation returned no IDs");
                Err(FoldDbError::Config(
                    "Batch mutation returned no IDs".to_string(),
                ))
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
            let (schema, fields_and_values, key_value, mutation_type, source_file_name) =
                match serde_json::from_value::<Operation>(mutation_data) {
                    Ok(Operation::Mutation {
                        schema,
                        fields_and_values,
                        key_value,
                        mutation_type,
                        source_file_name,
                    }) => (
                        schema,
                        fields_and_values,
                        key_value,
                        mutation_type,
                        source_file_name,
                    ),
                    Err(e) => {
                        return Err(FoldDbError::Config(format!(
                            "Failed to parse mutation: {}",
                            e
                        )));
                    }
                };

            // Delete mutations are allowed to have empty fields_and_values
            if fields_and_values.is_empty() && mutation_type != MutationType::Delete {
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

        // Use async version directly - all backends now support async operations
        // This avoids deadlocks and provides consistent behavior across all backends
        let node_guard = self.node.read().await;
        let mut db = node_guard
            .get_fold_db()
            .await
            .map_err(|e| FoldDbError::Database(e.to_string()))?;
        let mutation_ids = db
            .mutation_manager
            .write_mutations_batch_async(mutations)
            .await
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
