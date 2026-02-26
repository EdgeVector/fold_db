use crate::error::{FoldDbError, FoldDbResult};
use crate::schema::types::operations::{MutationType, Operation};
use crate::schema::types::{KeyValue, Mutation};
use serde_json::Value;
use std::collections::HashMap;

use super::OperationProcessor;

impl OperationProcessor {
    /// Executes a mutation operation from a Mutation struct.
    pub async fn execute_mutation_op(&self, mutation: Mutation) -> FoldDbResult<String> {
        let schema_name = mutation.schema_name.clone();
        log::info!("🔄 Starting mutation execution for schema: {}", schema_name);

        let mut db = self
            .node
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

    /// Executes a mutation operation (legacy wrapper).
    pub async fn execute_mutation(
        &self,
        schema: String,
        fields_and_values: HashMap<String, Value>,
        key_value: KeyValue,
        mutation_type: MutationType,
    ) -> FoldDbResult<String> {
        // Delete mutations are allowed to have empty fields_and_values
        if fields_and_values.is_empty() && mutation_type != MutationType::Delete {
            return Err(FoldDbError::Config("No fields to mutate".to_string()));
        }

        let mutation = Mutation::new(
            schema,
            fields_and_values,
            key_value,
            String::new(),
            mutation_type,
        );

        self.execute_mutation_op(mutation).await
    }

    /// Executes multiple mutations in a batch from Mutation structs.
    pub async fn execute_mutations_batch_ops(
        &self,
        mutations: Vec<Mutation>,
    ) -> FoldDbResult<Vec<String>> {
        let mut db = self
            .node
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

    /// Executes multiple mutations in a batch for improved performance (from JSON).
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
                mutation_type,
            );

            // Add source_file_name if provided
            if let Some(filename) = source_file_name {
                mutation = mutation.with_source_file_name(filename);
            }

            mutations.push(mutation);
        }

        self.execute_mutations_batch_ops(mutations).await
    }
}
