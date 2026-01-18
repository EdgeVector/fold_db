//! Background ingestion task spawner

use serde_json::Value;
use std::sync::Arc;
use tokio::sync::RwLock;

use crate::datafold_node::DataFoldNode;
use crate::ingestion::config::IngestionConfig;
use crate::ingestion::core::IngestionRequest;
use crate::ingestion::progress::ProgressService;
use crate::ingestion::simple_service::SimpleIngestionService;
use crate::ingestion::IngestionError;
use crate::ingestion::ProgressTracker;
use crate::log_feature;
use crate::logging::features::LogFeature;

/// Configuration for spawning background ingestion
pub struct IngestionSpawnConfig {
    pub json_data: Value,
    pub auto_execute: bool,
    pub trust_distance: u32,
    pub pub_key: String,
    pub source_file_name: Option<String>,
    pub ingestion_config: IngestionConfig,
}

/// Spawn background ingestion task and return progress_id
pub async fn spawn_background_ingestion(
    config: IngestionSpawnConfig,
    progress_tracker: &ProgressTracker,
    node: Arc<RwLock<DataFoldNode>>,
) -> String {
    let progress_id = uuid::Uuid::new_v4().to_string();

    // Start progress tracking
    let progress_service = ProgressService::new(progress_tracker.clone());
    progress_service.start_progress(progress_id.clone()).await;

    // Create ingestion request
    let ingestion_request = IngestionRequest {
        data: config.json_data,
        auto_execute: Some(config.auto_execute),
        trust_distance: Some(config.trust_distance),
        pub_key: Some(config.pub_key),
        source_file_name: config.source_file_name,
    };

    // Clone for the spawned task
    let progress_id_clone = progress_id.clone();
    let ingestion_config = config.ingestion_config;

    // Spawn the background task
    tokio::spawn(async move {
        if let Err(e) = run_background_ingestion(
            ingestion_request,
            node,
            progress_service,
            progress_id_clone,
            ingestion_config,
        )
        .await
        {
            log_feature!(
                LogFeature::Ingestion,
                error,
                "Background ingestion setup failed: {}",
                e
            );
        }
    });

    progress_id
}

/// Run the actual ingestion process in background
async fn run_background_ingestion(
    ingestion_request: IngestionRequest,
    node: Arc<RwLock<DataFoldNode>>,
    progress_service: ProgressService,
    progress_id: String,
    ingestion_config: IngestionConfig,
) -> Result<(), String> {
    log_feature!(
        LogFeature::Ingestion,
        info,
        "Starting background ingestion for uploaded file with progress_id: {}",
        progress_id
    );

    // Create ingestion service
    let service = match create_simple_ingestion_service(ingestion_config).await {
        Ok(s) => s,
        Err(e) => {
            let error_msg = format!("Ingestion service not available: {}", e);
            log_feature!(
                LogFeature::Ingestion,
                error,
                "Failed to initialize ingestion service: {}",
                e
            );
            progress_service
                .fail_progress(&progress_id, error_msg.clone())
                .await;
            return Err(error_msg);
        }
    };

    // Process the ingestion
    // Process the ingestion
    // Lock the node
    {
        let node_guard = node.read().await;
        match service
            .process_json_with_node_and_progress(
                ingestion_request,
                &node_guard,
                &progress_service,
                progress_id.clone(),
            )
            .await
        {
            Ok(response) => {
                if response.success {
                    log_feature!(
                        LogFeature::Ingestion,
                        info,
                        "File ingestion completed successfully: {}",
                        progress_id
                    );
                } else {
                    log_feature!(
                        LogFeature::Ingestion,
                        error,
                        "File ingestion failed: {:?}",
                        response.errors
                    );
                }
                Ok(())
            }
            Err(e) => {
                let error_msg = format!("Processing failed: {}", e);
                log_feature!(
                    LogFeature::Ingestion,
                    error,
                    "File ingestion processing failed: {}",
                    e
                );
                progress_service
                    .fail_progress(&progress_id, error_msg.clone())
                    .await;
                Err(error_msg)
            }
        }
    }
}

/// Create a simple ingestion service with potentially updated config
async fn create_simple_ingestion_service(
    config: IngestionConfig,
) -> Result<SimpleIngestionService, IngestionError> {
    SimpleIngestionService::new(config)
}
