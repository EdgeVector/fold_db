use super::types::TransformRunner;
use super::result_storage::ResultStorage;
use crate::schema::types::SchemaError;
use log::{error, info};
use serde_json::Value as JsonValue;
use std::collections::HashSet;

/// Deprecated
impl TransformRunner for super::TransformManager {
    fn execute_transform_with_context(
        &self,
        transform_id: &str,
        mutation_context: &Option<
            crate::fold_db_core::infrastructure::message_bus::atom_events::MutationContext,
        >,
    ) -> Result<JsonValue, SchemaError> {
        info!(
            "🚀 DIAGNOSTIC: TransformManager executing transform with context: {}",
            transform_id
        );

        // Load the transform from the database
        let transform = match self.db_ops.get_transform(transform_id) {
            Ok(Some(transform)) => {
                transform
            }
            Ok(None) => {
                error!(
                    "❌ DIAGNOSTIC: Transform '{}' not found in database",
                    transform_id
                );
                return Err(SchemaError::InvalidData(format!(
                    "Transform '{}' not found",
                    transform_id
                )));
            }
            Err(e) => {
                error!(
                    "❌ DIAGNOSTIC: Failed to load transform '{}': {}",
                    transform_id, e
                );
                return Err(SchemaError::InvalidData(format!(
                    "Failed to load transform: {}",
                    e
                )));
            }
        };

        // Log mutation context if available
        if let Some(ref context) = mutation_context {
            info!("🎯 DIAGNOSTIC: Transform execution with mutation context - key_config: {:?}, incremental: {}", 
                  context.key_config, context.incremental);
        }

        // Execute the transform using the execution module with mutation context
        println!(
            "🔧 About to call execute_single_transform with context for transform: {}",
            transform_id
        );
        let result = super::TransformManager::execute_single_transform_with_context(
            transform_id,
            &transform,
            &self.db_ops,
            mutation_context,
            None, // FoldDB not available in this context - will use fallback
        )?;
        println!(
            "🔧 execute_single_transform with context completed with result: {}",
            result
        );

        info!(
            "✅ DIAGNOSTIC: Transform '{}' executed successfully with context, result: {}",
            transform_id, result
        );

        // Store the result using message bus
        let mut result_map = std::collections::HashMap::new();
        result_map.insert("result".to_string(), result.clone());
        match ResultStorage::store_transform_result_generic(
            &transform,
            result_map,
            mutation_context.as_ref().unwrap().key_config.clone().unwrap(),
            Some(&self.message_bus)
        ) {
            Ok(_) => {
            }
            Err(e) => {
                return Err(e);
            }
        }

        info!(
            "✅ Transform '{}' executed successfully with context: {}",
            transform_id, result
        );
        Ok(result)
    }

    fn transform_exists(&self, transform_id: &str) -> Result<bool, SchemaError> {
        let registered_transforms = self.registered_transforms.read().map_err(|_| {
            SchemaError::InvalidData("Failed to acquire registered_transforms lock".to_string())
        })?;
        let in_memory_exists = registered_transforms.contains_key(transform_id);

        // Cross-check with database
        let db_exists = self.db_ops.get_transform(transform_id)?.is_some();

        info!(
            "🔍 DIAGNOSTIC: TransformManager.transform_exists('{}') - in_memory: {}, database: {}",
            transform_id, in_memory_exists, db_exists
        );

        if in_memory_exists != db_exists {
            error!(
                "🚨 INCONSISTENCY DETECTED: Transform '{}' - in_memory: {}, database: {}",
                transform_id, in_memory_exists, db_exists
            );
        }

        Ok(in_memory_exists)
    }

    fn get_transforms_for_field(
        &self,
        schema_name: &str,
        field_name: &str,
    ) -> Result<HashSet<String>, SchemaError> {
        let key = format!("{}.{}", schema_name, field_name);
        let field_to_transforms = self.schema_field_to_transforms.read().map_err(|_| {
            SchemaError::InvalidData("Failed to acquire field_to_transforms lock".to_string())
        })?;
        Ok(field_to_transforms.get(&key).cloned().unwrap_or_default())
    }

    fn get_transforms_for_schema(&self, schema_name: &str) -> Result<HashSet<String>, SchemaError> {
        // Delegate to the public method implementation
        self.get_transforms_for_schema(schema_name)
    }
}
