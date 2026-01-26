use crate::error::{FoldDbError, FoldDbResult};
use crate::schema::types::{KeyValue, Mutation, Query};
use serde_json::Value;
use std::collections::HashMap;

use super::response_types::QueryResultMap;
use super::DataFoldNode;
use crate::datafold_node::config::DatabaseConfig;
use crate::datafold_node::NodeConfig;
use crate::db_operations::IndexResult;
use crate::fold_db_core::infrastructure::backfill_tracker::{
    BackfillInfo, BackfillStatistics, BackfillStatus,
};
use crate::fold_db_core::orchestration::IndexingStatus;
use crate::schema::types::operations::{MutationType, Operation};
use crate::schema::types::Transform;
use crate::schema::{SchemaState, SchemaWithState};
use std::fs;
use std::io::Write;

/// Centralized operation processor that handles all operation types consistently.
///
/// This eliminates code duplication across HTTP routes, TCP server, CLI, and direct API usage.
/// All operation execution goes through this single processor to ensure consistent behavior.
pub struct OperationProcessor {
    node: DataFoldNode,
}

impl OperationProcessor {
    /// Creates a new operation processor with a DataFoldNode instance.
    pub fn new(node: DataFoldNode) -> Self {
        Self { node }
    }

    /// Executes a query and returns raw structured results, not JSON.
    pub async fn execute_query_map(&self, query: Query) -> FoldDbResult<QueryResultMap> {
        let db = self
            .node
            .get_fold_db()
            .await
            .map_err(|e| FoldDbError::Database(e.to_string()))?;
        let results = db.query_executor.query(query).await;
        Ok(results?)
    }

    /// Executes a query and returns formatted JSON records.
    /// This provides a consistent JSON representation for API responses.
    pub async fn execute_query_json(&self, query: Query) -> FoldDbResult<Vec<Value>> {
        let result_map = self.execute_query_map(query).await?;
        let records_map = crate::fold_db_core::query::records_from_field_map(&result_map);

        let results: Vec<Value> = records_map
            .into_iter()
            .map(|(key, record)| {
                serde_json::json!({
                    "key": key,
                    "fields": record.fields,
                    "metadata": record.metadata
                })
            })
            .collect();

        Ok(results)
    }

    // --- Logging Operations ---

    /// List logs with optional filtering.
    pub async fn list_logs(
        &self,
        since: Option<i64>,
        limit: Option<usize>,
    ) -> Vec<crate::logging::core::LogEntry> {
        crate::logging::LoggingSystem::query_logs(limit, since)
            .await
            .unwrap_or_default()
    }

    /// Get current logging configuration.
    pub async fn get_log_config(&self) -> Option<crate::logging::config::LogConfig> {
        crate::logging::LoggingSystem::get_config().await
    }

    /// Reload logging configuration from file.
    pub async fn reload_log_config(&self, path: &str) -> FoldDbResult<()> {
        crate::logging::LoggingSystem::reload_config_from_file(path)
            .await
            .map_err(|e| FoldDbError::Config(format!("Failed to reload log config: {}", e)))
    }

    /// Get available log features and their levels.
    pub async fn get_log_features(&self) -> Option<HashMap<String, String>> {
        crate::logging::LoggingSystem::get_features().await
    }

    /// Update log level for a specific feature.
    pub async fn update_log_feature_level(&self, feature: &str, level: &str) -> FoldDbResult<()> {
        crate::logging::LoggingSystem::update_feature_level(feature, level)
            .await
            .map_err(|e| FoldDbError::Config(format!("Failed to update log level: {}", e)))
    }

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
            0,
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
                0,
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

    // Removed execute_sync as part of eliminating the generic execute path

    /// Search the native word index for a term.
    pub async fn native_index_search(&self, term: &str) -> FoldDbResult<Vec<IndexResult>> {
        let term = term.trim();
        if term.is_empty() {
            return Err(FoldDbError::Config("Term cannot be empty".to_string()));
        }

        let db = self
            .node
            .get_fold_db()
            .await
            .map_err(|e| FoldDbError::Database(e.to_string()))?;

        db.native_search_all_classifications(term)
            .await
            .map_err(|e| FoldDbError::Database(e.to_string()))
    }

    /// List all schemas with their states.
    pub async fn list_schemas(&self) -> FoldDbResult<Vec<SchemaWithState>> {
        let db = self
            .node
            .get_fold_db()
            .await
            .map_err(|e| FoldDbError::Database(e.to_string()))?;

        db.schema_manager
            .get_schemas_with_states()
            .map_err(|e| FoldDbError::Database(e.to_string()))
    }

    /// Get a specific schema by name with its state.
    pub async fn get_schema(&self, name: &str) -> FoldDbResult<Option<SchemaWithState>> {
        let db = self
            .node
            .get_fold_db()
            .await
            .map_err(|e| FoldDbError::Database(e.to_string()))?;

        let mgr = &db.schema_manager;
        match mgr
            .get_schema(name)
            .map_err(|e| FoldDbError::Database(e.to_string()))?
        {
            Some(schema) => {
                let states = mgr
                    .get_schema_states()
                    .map_err(|e| FoldDbError::Database(e.to_string()))?;
                let state = states.get(name).copied().unwrap_or_default();
                Ok(Some(SchemaWithState::new(schema, state)))
            }
            None => Ok(None),
        }
    }

    /// Approve a schema.
    pub async fn approve_schema(&self, schema_name: &str) -> FoldDbResult<Option<String>> {
        let db = self
            .node
            .get_fold_db()
            .await
            .map_err(|e| FoldDbError::Database(e.to_string()))?;

        let schema_mgr = &db.schema_manager;
        let transform_mgr = &db.transform_manager;

        // Check if schema is already approved
        let states = schema_mgr
            .get_schema_states()
            .map_err(|e| FoldDbError::Database(e.to_string()))?;
        let current_state = states.get(schema_name).copied().unwrap_or_default();

        if current_state == SchemaState::Approved {
            return Ok(None);
        }

        let is_transform = transform_mgr.transform_exists(schema_name).unwrap_or(false);

        // Logic to generate backfill hash needs to be moved here or reused.
        // For now, I will create a private helper or copy the logic since it was in the route handler.
        // But `generate_backfill_hash_for_transform` was in `src/server/routes/schema.rs`.
        // Integrating it here.
        let backfill_hash = if is_transform {
            Self::generate_backfill_hash_for_transform(transform_mgr, schema_name).await
        } else {
            None
        };

        schema_mgr
            .set_schema_state_with_backfill(
                schema_name,
                SchemaState::Approved,
                backfill_hash.clone(),
            )
            .await
            .map_err(|e| FoldDbError::Database(e.to_string()))?;

        Ok(backfill_hash)
    }

    /// Block a schema.
    pub async fn block_schema(&self, schema_name: &str) -> FoldDbResult<()> {
        let db = self
            .node
            .get_fold_db()
            .await
            .map_err(|e| FoldDbError::Database(e.to_string()))?;

        db.schema_manager
            .block_schema(schema_name)
            .await
            .map_err(|e| FoldDbError::Database(e.to_string()))
    }

    /// Load schemas from the schema service (Standard fetch & load logic).
    /// Returns (available_count, loaded_count, failed_schemas).
    pub async fn load_schemas(&self) -> FoldDbResult<(usize, usize, Vec<String>)> {
        // Need to drop lock between fetch and load to avoid holding it too long?
        // The original implementation dropped it.
        let schemas = {
            self.node
                .fetch_available_schemas()
                .await
                .map_err(|e| FoldDbError::Database(e.to_string()))?
        };

        let schema_count = schemas.len();
        let mut loaded_count = 0;
        let mut failed_schemas = Vec::new();

        for schema in schemas {
            let schema_name = schema.name.clone();
            let result = {
                let db = self
                    .node
                    .get_fold_db()
                    .await
                    .map_err(|e| FoldDbError::Database(e.to_string()))?;

                db.schema_manager
                    .load_schema_internal(schema)
                    .await
                    .map_err(|e| FoldDbError::Database(e.to_string()))
            };

            match result {
                Ok(_) => loaded_count += 1,
                Err(e) => {
                    log::error!("Failed to load schema {}: {}", schema_name, e);
                    failed_schemas.push(schema_name);
                }
            }
        }

        Ok((schema_count, loaded_count, failed_schemas))
    }

    /// List transforms.
    pub async fn list_transforms(&self) -> FoldDbResult<HashMap<String, Transform>> {
        let db = self
            .node
            .get_fold_db()
            .await
            .map_err(|e| FoldDbError::Database(e.to_string()))?;

        db.transform_manager
            .list_transforms()
            .map_err(|e| FoldDbError::Database(e.to_string()))
    }

    /// Add transform to queue.
    pub async fn add_to_transform_queue(
        &self,
        transform_id: &str,
        trigger: &str,
    ) -> FoldDbResult<()> {
        let db = self
            .node
            .get_fold_db()
            .await
            .map_err(|e| FoldDbError::Database(e.to_string()))?;

        if let Some(orchestrator) = db.transform_orchestrator() {
            orchestrator
                .add_transform(transform_id, trigger)
                .await
                .map_err(|e| FoldDbError::Config(e.to_string()))
        } else {
            Err(FoldDbError::Config(
                "Transform orchestrator not available".to_string(),
            ))
        }
    }

    /// Get transform queue info.
    /// Returns (length, queued_transforms).
    pub async fn get_transform_queue(&self) -> FoldDbResult<(usize, Vec<String>)> {
        let db = self
            .node
            .get_fold_db()
            .await
            .map_err(|e| FoldDbError::Database(e.to_string()))?;

        if let Some(orchestrator) = db.transform_orchestrator() {
            let queued = orchestrator
                .list_queued_transforms()
                .map_err(|e| FoldDbError::Config(e.to_string()))?;
            let len = orchestrator.len().unwrap_or(0);
            Ok((len, queued))
        } else {
            Err(FoldDbError::Config(
                "Transform orchestrator not available".to_string(),
            ))
        }
    }

    /// Get all backfills.
    pub async fn get_all_backfills(&self) -> FoldDbResult<Vec<BackfillInfo>> {
        let db = self
            .node
            .get_fold_db()
            .await
            .map_err(|e| FoldDbError::Database(e.to_string()))?;
        Ok(db.get_all_backfills())
    }

    /// Get active backfills.
    pub async fn get_active_backfills(&self) -> FoldDbResult<Vec<BackfillInfo>> {
        let db = self
            .node
            .get_fold_db()
            .await
            .map_err(|e| FoldDbError::Database(e.to_string()))?;
        Ok(db.get_active_backfills())
    }

    /// Get backfill by ID/Hash.
    pub async fn get_backfill(&self, id: &str) -> FoldDbResult<Option<BackfillInfo>> {
        // Access via get_fold_db is standardized
        let db = self
            .node
            .get_fold_db()
            .await
            .map_err(|e| FoldDbError::Database(e.to_string()))?;
        Ok(db.get_backfill(id))
    }

    /// Get backfill statistics.
    pub async fn get_backfill_statistics(&self) -> FoldDbResult<BackfillStatistics> {
        let backfills = self.get_all_backfills().await?;

        let active_count = backfills
            .iter()
            .filter(|b| b.status == BackfillStatus::InProgress)
            .count();
        let completed_count = backfills
            .iter()
            .filter(|b| b.status == BackfillStatus::Completed)
            .count();
        let failed_count = backfills
            .iter()
            .filter(|b| b.status == BackfillStatus::Failed)
            .count();

        Ok(BackfillStatistics {
            total_backfills: backfills.len(),
            active_backfills: active_count,
            completed_backfills: completed_count,
            failed_backfills: failed_count,
            total_mutations_expected: backfills.iter().map(|b| b.mutations_expected).sum(),
            total_mutations_completed: backfills.iter().map(|b| b.mutations_completed).sum(),
            total_mutations_failed: backfills.iter().map(|b| b.mutations_failed).sum(),
            total_records_produced: backfills.iter().map(|b| b.records_produced).sum(),
        })
    }

    /// Get event/transform statistics.
    pub async fn get_transform_statistics(
        &self,
    ) -> FoldDbResult<crate::fold_db_core::infrastructure::event_statistics::EventStatistics> {
        let db = self
            .node
            .get_fold_db()
            .await
            .map_err(|e| FoldDbError::Database(e.to_string()))?;
        Ok(db.get_event_statistics())
    }

    /// Get indexing status.
    pub async fn get_indexing_status(&self) -> FoldDbResult<IndexingStatus> {
        let db = self
            .node
            .get_fold_db()
            .await
            .map_err(|e| FoldDbError::Database(e.to_string()))?;
        Ok(db.get_indexing_status().await)
    }

    /// Get the node's private key
    pub fn get_node_private_key(&self) -> String {
        self.node.get_node_private_key().to_string()
    }

    /// Get the node's public key
    pub fn get_node_public_key(&self) -> String {
        self.node.get_node_public_key().to_string()
    }

    /// Get the system public key
    pub fn get_system_public_key(&self) -> FoldDbResult<Option<crate::security::PublicKeyInfo>> {
        let security_manager = self.node.get_security_manager();
        security_manager
            .get_system_public_key()
            .map_err(|e| FoldDbError::Other(e.to_string()))
    }

    /// Reset schema service
    pub async fn reset_schema_service(&self) -> FoldDbResult<()> {
        let schema_client = self.node.get_schema_client();
        schema_client
            .reset_schema_service()
            .await
            .map_err(|e| FoldDbError::Other(format!("Schema service reset failed: {}", e)))
    }

    /// Get database configuration
    pub fn get_database_config(&self) -> DatabaseConfig {
        self.node.config.database.clone()
    }

    // Helper for approve_schema
    async fn generate_backfill_hash_for_transform(
        transform_manager: &crate::transform::manager::TransformManager,
        schema_name: &str,
    ) -> Option<String> {
        let transforms = match transform_manager.list_transforms() {
            Ok(t) => t,
            Err(e) => {
                log::warn!("Failed to list transforms for {}: {}", schema_name, e);
                return None;
            }
        };

        let transform = match transforms.get(schema_name) {
            Some(t) => t,
            None => {
                log::debug!("Transform {} not found in transform list", schema_name);
                return None;
            }
        };

        // Look up the transform's schema from the database
        let declarative_schema = match transform_manager
            .db_ops
            .get_schema(transform.get_schema_name())
            .await
        {
            Ok(Some(s)) => s,
            Ok(None) => {
                log::warn!("Transform {} schema not found in database", schema_name);
                return None;
            }
            Err(e) => {
                log::warn!("Failed to get schema for transform {}: {}", schema_name, e);
                return None;
            }
        };

        let inputs = declarative_schema.get_inputs();
        let first_input = match inputs.first() {
            Some(i) => i,
            None => {
                log::warn!(
                    "Transform {} has no inputs in declarative schema",
                    schema_name
                );
                return None;
            }
        };

        let source_schema_name = match first_input.split('.').next() {
            Some(s) => s,
            None => {
                log::warn!("Failed to parse source schema from input: {}", first_input);
                return None;
            }
        };

        Some(
            crate::fold_db_core::infrastructure::backfill_tracker::BackfillTracker::generate_hash(
                schema_name,
                source_schema_name,
            ),
        )
    }

    /// Reset the database (destructive operation).
    /// Handles schema service reset, closing DB, and clearing storage (Local or DynamoDB).
    pub async fn perform_database_reset(
        &self,
        #[allow(unused_variables)] user_id_override: Option<&str>,
    ) -> FoldDbResult<()> {
        // 1. Reset Schema Service
        if let Err(e) = self.reset_schema_service().await {
            log::warn!(
                "Failed to reset schema service during database reset: {}",
                e
            );
            // Continue
        } else {
            log::info!("Schema service database reset successfully");
        }

        // 2. Get config and path before closing
        let config = self.node.config.clone();
        let db_path = config.get_storage_path();

        // 3. Close the current database
        if let Ok(db) = self.node.get_fold_db().await {
            if let Err(e) = db.close() {
                log::warn!("Failed to close database during reset: {}", e);
            }
        }

        // 4. Handle storage reset
        match &config.database {
            #[cfg(feature = "aws-backend")]
            DatabaseConfig::Cloud(cloud_config) => {
                let aws_config = aws_config::defaults(aws_config::BehaviorVersion::latest())
                    .region(aws_sdk_dynamodb::config::Region::new(
                        cloud_config.region.clone(),
                    ))
                    .load()
                    .await;
                let client = std::sync::Arc::new(aws_sdk_dynamodb::Client::new(&aws_config));

                // Priority: 1) explicit override, 2) current user context from HTTP request,
                // 3) config user_id, 4) node public key
                let uid = user_id_override
                    .map(|s| s.to_string())
                    .or_else(crate::logging::core::get_current_user_id)
                    .or_else(|| cloud_config.user_id.clone())
                    .unwrap_or_else(|| self.node.get_node_public_key().to_string());

                log::info!(
                    "Resetting database for user_id={} using scan-free DynamoDbResetManager",
                    uid
                );

                let manager = crate::storage::reset_manager::DynamoDbResetManager::new(
                    client.clone(),
                    cloud_config.tables.clone(),
                );

                if let Err(e) = manager.reset_user(&uid).await {
                    log::error!("Failed to reset user data: {}", e);
                    return Err(FoldDbError::Other(format!(
                        "Failed to reset user data: {}",
                        e
                    )));
                }
            }
            DatabaseConfig::Local { .. } => {
                if db_path.exists() {
                    if let Err(e) = std::fs::remove_dir_all(&db_path) {
                        log::error!("Failed to delete database folder: {}", e);
                        return Err(FoldDbError::Io(e));
                    }
                }
            }
        }

        Ok(())
    }

    /// Update database configuration and write to disk.
    /// Returns the new NodeConfig so the caller can recreate the node.
    pub async fn update_database_configuration(
        &self,
        new_db_config: DatabaseConfig,
    ) -> FoldDbResult<NodeConfig> {
        let mut config = self.node.config.clone();
        config.database = new_db_config;

        let config_path =
            std::env::var("NODE_CONFIG").unwrap_or_else(|_| "config/node_config.json".to_string());

        // Ensure config directory exists
        if let Some(parent) = std::path::Path::new(&config_path).parent() {
            if let Err(e) = fs::create_dir_all(parent) {
                return Err(FoldDbError::Other(format!(
                    "Failed to create config directory: {}",
                    e
                )));
            }
        }

        // Serialize and write config
        let config_json = serde_json::to_string_pretty(&config)
            .map_err(|e| FoldDbError::Config(format!("Failed to serialize config: {}", e)))?;

        let mut file = fs::File::create(&config_path)
            .map_err(|e| FoldDbError::Other(format!("Failed to create config file: {}", e)))?;

        file.write_all(config_json.as_bytes())
            .map_err(|e| FoldDbError::Other(format!("Failed to write config file: {}", e)))?;

        // Close current DB (best effort)
        if let Ok(db) = self.node.get_fold_db().await {
            if let Err(e) = db.close() {
                log::warn!("Failed to close database during config update: {}", e);
            }
        }

        Ok(config)
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

    #[tokio::test]
    async fn test_logging_methods_signature() {
        // This test ensures the logging methods are available on OperationProcessor
        // without needing to instantiate a full DataFoldNode (which is complex).
        // It relies on the fact that if this compiles, the methods exist.
        async fn check_methods(processor: &crate::datafold_node::OperationProcessor) {
            let _ = processor.list_logs(None, None).await;
            let _ = processor.get_log_config().await;
            let _ = processor.get_log_features().await;
        }
        // check_methods is defined but not called, which satisfies the compiler checking the body.
        // To strictly avoid "unused" warnings we might want to use it in a phantom way?
        // But the original code was: let _ = |...| ...
        // We can just define it. The compiler checks the body of the function.
        let _ = check_methods;
    }
}
