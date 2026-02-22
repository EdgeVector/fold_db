use crate::error::{FoldDbError, FoldDbResult};
use crate::fold_db_core::infrastructure::backfill_tracker::{
    BackfillInfo, BackfillStatistics, BackfillStatus,
};
use crate::fold_db_core::orchestration::IndexingStatus;
use crate::fold_node::config::DatabaseConfig;
use crate::fold_node::NodeConfig;
use crate::ingestion::ingestion_service::IngestionService;
use crate::schema::types::Transform;
use std::collections::HashMap;
use std::fs;
use std::io::Write;

use super::OperationProcessor;

impl OperationProcessor {
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

    // --- Transform Operations ---

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

    // --- Backfill Operations ---

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

    // --- Security Operations ---

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

    // --- Config / Reset Operations ---

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

    /// Reset the database (destructive operation).
    /// Handles closing DB and clearing storage (Local or DynamoDB).
    /// Note: Schema service reset is NOT included - use reset_schema_service() separately if needed.
    pub async fn perform_database_reset(
        &self,
        #[allow(unused_variables)] user_id_override: Option<&str>,
    ) -> FoldDbResult<()> {
        // 1. Get config and path before closing
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
                // Recreate the empty data directory so subsequent operations can use it
                if let Err(e) = std::fs::create_dir_all(&db_path) {
                    log::error!("Failed to recreate database folder: {}", e);
                    return Err(FoldDbError::Io(e));
                }
            }
            DatabaseConfig::Exemem { .. } => {
                return Err(FoldDbError::Other(
                    "Database reset is not supported for Exemem backend".to_string(),
                ));
            }
        }

        Ok(())
    }

    // --- Ingestion Operations ---

    /// Scan a folder using LLM to classify files and return recommendations.
    pub async fn smart_folder_scan(
        &self,
        folder_path: &std::path::Path,
        max_depth: usize,
        max_files: usize,
    ) -> FoldDbResult<crate::ingestion::smart_folder::SmartFolderScanResponse> {
        crate::ingestion::smart_folder::perform_smart_folder_scan(
            folder_path,
            max_depth,
            max_files,
            None,
        )
        .await
        .map_err(|e| FoldDbError::Other(e.to_string()))
    }

    /// Ingest a single file through the AI ingestion pipeline.
    ///
    /// Tries the native parser first for known formats (json, js/Twitter, csv, txt, md),
    /// then falls back to file_to_json for everything else (images, PDFs, YAML, etc.).
    pub async fn ingest_single_file(
        &self,
        file_path: &std::path::Path,
        auto_execute: bool,
    ) -> FoldDbResult<crate::ingestion::IngestionResponse> {
        self.ingest_single_file_with_tracker(file_path, auto_execute, None).await
    }

    /// Like `ingest_single_file` but accepts an optional external `ProgressTracker`
    /// so callers (e.g. TUI) can poll progress while ingestion runs.
    /// Returns `(progress_id, IngestionResponse)`.
    pub async fn ingest_single_file_with_tracker(
        &self,
        file_path: &std::path::Path,
        auto_execute: bool,
        external_tracker: Option<crate::ingestion::ProgressTracker>,
    ) -> FoldDbResult<crate::ingestion::IngestionResponse> {
        use crate::ingestion::IngestionRequest;
        use crate::ingestion::json_processor::convert_file_to_json;
        use crate::ingestion::progress::ProgressService;
        use crate::ingestion::smart_folder;

        // Try native parser first (handles json, js/Twitter, csv, txt, md without LLM),
        // fall back to file_to_json for unsupported types (images, PDFs, etc.)
        let data = match smart_folder::read_file_as_json(file_path) {
            Ok(json) => json,
            Err(_) => convert_file_to_json(&file_path.to_path_buf())
                .await
                .map_err(|e| FoldDbError::Other(e.to_string()))?,
        };

        let progress_id = uuid::Uuid::new_v4().to_string();
        let pub_key = self.get_node_public_key();

        let request = IngestionRequest {
            data,
            auto_execute,
            trust_distance: 0,
            pub_key,
            source_file_name: file_path
                .file_name()
                .and_then(|n| n.to_str())
                .map(|s| s.to_string()),
            progress_id: Some(progress_id.clone()),
            file_hash: None,
        };

        let service = IngestionService::from_env().map_err(|e| FoldDbError::Other(e.to_string()))?;

        let progress_tracker = match external_tracker {
            Some(t) => t,
            None => crate::ingestion::create_progress_tracker(None).await,
        };
        let progress_service = ProgressService::new(progress_tracker);
        progress_service
            .start_progress(progress_id.clone(), "cli".to_string())
            .await;

        let response = service
            .process_json_with_node_and_progress(
                request,
                &self.node,
                &progress_service,
                progress_id,
            )
            .await
            .map_err(|e| FoldDbError::Other(e.to_string()))?;

        Ok(response)
    }

    // --- LLM Query Operations ---

    /// Run an LLM agent query against the database.
    ///
    /// Creates an LlmQueryService, loads all schemas, and runs the agent
    /// which can autonomously use tools (query, list_schemas, search) to answer.
    pub async fn llm_query(
        &self,
        user_query: &str,
        user_hash: &str,
        max_iterations: usize,
    ) -> FoldDbResult<(
        String,
        Vec<crate::fold_node::llm_query::types::ToolCallRecord>,
    )> {
        use crate::fold_node::llm_query::service::LlmQueryService;
        use crate::ingestion::config::IngestionConfig;

        let config = IngestionConfig::from_env_allow_empty();
        let service = LlmQueryService::new(config).map_err(FoldDbError::Other)?;

        let schemas = self.list_schemas().await?;

        service
            .run_agent_query(user_query, &schemas, &self.node, user_hash, max_iterations)
            .await
            .map_err(FoldDbError::Other)
    }

    // --- Configuration Operations ---

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
