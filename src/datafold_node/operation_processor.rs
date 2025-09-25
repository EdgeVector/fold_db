use crate::error::{FoldDbError, FoldDbResult};
use crate::log_feature;
use crate::logging::features::LogFeature;
use crate::schema::types::{Mutation, Operation, Query};
use serde_json::Value;
use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::Mutex;
use crate::schema::types::key_config::KeyConfig;
use crate::schema::types::operations::MutationType;
use super::DataFoldNode;

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

    /// Executes an operation and returns the result.
    /// 
    /// This is the single source of truth for operation execution across all entry points.
    /// 
    /// # Arguments
    /// 
    /// * `operation` - The operation to execute
    /// 
    /// # Returns
    /// 
    /// * `Ok(Value)` - The operation result
    /// * `Err(SchemaError)` - If the operation failed
    pub async fn execute(&self, operation: Operation) -> FoldDbResult<Value> {
        log_feature!(
            LogFeature::Query,
            info,
            "Executing operation: {:?}",
            operation
        );

        match operation {
            Operation::Query {
                schema,
                fields,
                filter,
            } => self.execute_query(schema, fields, filter).await,
            Operation::Mutation {
                schema,
                fields_and_values,
                key_config,
                mutation_type,
            } => self.execute_mutation(schema, fields_and_values, key_config, mutation_type).await,
        }
    }

    /// Executes a query operation.
    async fn execute_query(
        &self,
        schema: String,
        fields: Vec<String>,
        filter: Option<Value>,
    ) -> FoldDbResult<Value> {
        let query = Query {
            schema_name: schema,
            fields,
            pub_key: String::new(),
            trust_distance: 0,
            filter,
        };

        let mut node_guard = self.node.lock().await;
        let results = node_guard.query(query)?;
        
        // Convert Vec<Result<Value, SchemaError>> to Vec<Value> with errors as JSON
        let unwrapped: Vec<Value> = results
            .into_iter()
            .map(|r| r.unwrap_or_else(|e| serde_json::json!({"error": e.to_string()})))
            .collect();

        Ok(serde_json::to_value(&unwrapped)?)
    }

    /// Executes a mutation operation.
    async fn execute_mutation(
        &self,
        schema: String,
        fields_and_values: HashMap<String, Value>,
        key_config: KeyConfig,
        mutation_type: MutationType,
    ) -> FoldDbResult<Value> {
        // Validate that fields_and_values is not empty
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

        let mutation = Mutation {
            schema_name: schema,
            fields_and_values,
            key_config,
            pub_key: String::new(),
            trust_distance: 0,
            mutation_type,
            synchronous: None,
        };

        let node_guard = self.node.lock().await;
        node_guard.mutate(mutation)?;

        Ok(serde_json::json!({ "success": true }))
    }


    /// Synchronous wrapper for CLI and other synchronous contexts.
    /// 
    /// This method provides a synchronous interface to the async operation processor
    /// by using tokio::runtime::Handle::current() to run the async operation.
    /// 
    /// # Arguments
    /// 
    /// * `operation` - The operation to execute
    /// 
    /// # Returns
    /// 
    /// * `Ok(Value)` - The operation result
    /// * `Err(SchemaError)` - If the operation failed
    pub fn execute_sync(&self, operation: Operation) -> FoldDbResult<Value> {
        let rt = tokio::runtime::Handle::current();
        rt.block_on(self.execute(operation))
    }
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
