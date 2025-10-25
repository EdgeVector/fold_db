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

    /// Executes a mutation operation.
    pub async fn execute_mutation(
        &self,
        schema: String,
        fields_and_values: HashMap<String, Value>,
        key_value: KeyValue,
        mutation_type: MutationType,
    ) -> FoldDbResult<()> {
        if fields_and_values.is_empty() {
            return Err(FoldDbError::Config("No fields to mutate".to_string()));
        }

        // Convert HashMap<String, Value> to HashMap<String, Value> (already correct type)
        let fields_and_values = match serde_json::to_value(&fields_and_values) {
            Ok(Value::Object(map)) => map.into_iter().collect(),
            _ => {
                return Err(FoldDbError::Config(
                    "Mutation fields_and_values must be an object".into(),
                ))
            }
        };

        let mutation = Mutation::new(
            schema,
            fields_and_values,
            key_value,
            String::new(),
            0,
            mutation_type,
        );

        let node_guard = self.node.lock().await;
        node_guard.mutate(mutation)?;

        Ok(())
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
            let (schema, fields_and_values, key_value, mutation_type) = match serde_json::from_value::<Operation>(mutation_data) {
                Ok(Operation::Mutation { schema, fields_and_values, key_value, mutation_type }) => {
                    (schema, fields_and_values, key_value, mutation_type)
                },
                Err(e) => {
                    return Err(FoldDbError::Config(format!("Failed to parse mutation: {}", e)));
                }
            };

            if fields_and_values.is_empty() {
                return Err(FoldDbError::Config("No fields to mutate".to_string()));
            }

            // Convert HashMap<String, Value> to HashMap<String, Value> (already correct type)
            let fields_and_values = match serde_json::to_value(&fields_and_values) {
                Ok(Value::Object(map)) => map.into_iter().collect(),
                _ => {
                    return Err(FoldDbError::Config(
                        "Mutation fields_and_values must be an object".into(),
                    ))
                }
            };

            let mutation = Mutation::new(
                schema,
                fields_and_values,
                key_value,
                String::new(),
                0,
                mutation_type,
            );

            mutations.push(mutation);
        }

        let node_guard = self.node.lock().await;
        let mutation_ids = node_guard.mutate_batch(mutations)?;

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
