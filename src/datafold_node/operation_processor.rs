use crate::error::{FoldDbError, FoldDbResult};
use crate::schema::types::{Mutation, Query, KeyValue};
use serde_json::Value;
use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::Mutex;
use crate::schema::types::operations::MutationType;
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
    ) -> FoldDbResult<Value> {
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

        Ok(serde_json::json!(true))
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
