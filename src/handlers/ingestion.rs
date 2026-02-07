//! Shared Ingestion Handlers
//!
//! Framework-agnostic handlers for ingestion operations.
//! These can be called by both HTTP server routes and Lambda handlers.

use crate::datafold_node::node::DataFoldNode;
use crate::handlers::response::{ApiResponse, HandlerError, HandlerResult};
use crate::ingestion::config::{IngestionConfig, SavedConfig};
use crate::ingestion::progress::{IngestionProgress, ProgressService, ProgressTracker};
use crate::ingestion::ingestion_service::IngestionService;
use crate::ingestion::IngestionRequest;
use crate::progress::JobType;
use serde::{Deserialize, Serialize};
use serde_json::Value;

#[cfg(feature = "ts-bindings")]
use ts_rs::TS;

// ============================================================================
// Request/Response Types
// ============================================================================

/// Re-export IngestionRequest as ProcessJsonRequest for backward compatibility
/// with Lambda handlers in exemem-infra.
pub type ProcessJsonRequest = IngestionRequest;

/// Response for process_json (immediate response)
#[derive(Debug, Clone, Serialize, Deserialize)]
#[cfg_attr(feature = "ts-bindings", derive(TS))]
#[cfg_attr(
    feature = "ts-bindings",
    ts(export, export_to = "src/datafold_node/static-react/src/types/")
)]
pub struct ProcessJsonResponse {
    pub success: bool,
    pub progress_id: String,
    pub message: String,
}

/// Response type for get_all_progress
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProgressListResponse {
    /// List of progress items
    pub progress: Vec<IngestionProgress>,
}

/// Response for ingestion status
#[derive(Debug, Clone, Serialize, Deserialize)]
#[cfg_attr(feature = "ts-bindings", derive(TS))]
#[cfg_attr(
    feature = "ts-bindings",
    ts(export, export_to = "src/datafold_node/static-react/src/types/")
)]
pub struct IngestionStatusResponse {
    pub enabled: bool,
    pub configured: bool,
    pub provider: String,
    pub model: String,
}

/// Response for config operations
#[derive(Debug, Clone, Serialize, Deserialize)]
#[cfg_attr(feature = "ts-bindings", derive(TS))]
#[cfg_attr(
    feature = "ts-bindings",
    ts(export, export_to = "src/datafold_node/static-react/src/types/")
)]
pub struct ConfigSaveResponse {
    pub success: bool,
    pub message: String,
}

// ============================================================================
// Helper Functions
// ============================================================================

/// Extract ingestion data from a payload
///
/// Handles both wrapped format { data: {...}, progress_id: "...", ... }
/// and direct data format { field1: "...", field2: "..." }
pub fn extract_ingestion_data(payload: &Value) -> Result<(Value, Option<String>), HandlerError> {
    // Check if payload has a "data" field (wrapped format)
    if let Some(data) = payload.get("data") {
        let progress_id = payload
            .get("progress_id")
            .and_then(|v| v.as_str())
            .map(|s| s.to_string());
        return Ok((data.clone(), progress_id));
    }

    // Check if payload looks like a wrapper without data (error case)
    if payload.get("progress_id").is_some() || payload.get("auto_execute").is_some() {
        return Err(HandlerError::BadRequest(
            "Payload must contain a 'data' field with the JSON to ingest".to_string(),
        ));
    }

    // Treat the whole payload as data (direct format)
    Ok((payload.clone(), None))
}

/// Create ingestion service from environment config
fn create_ingestion_service() -> Result<IngestionService, HandlerError> {
    IngestionService::from_env().map_err(|e| {
        HandlerError::ServiceUnavailable(format!("Ingestion service not available: {}", e))
    })
}

// ============================================================================
// Handler Functions
// ============================================================================

/// Get all ingestion/indexing progress for a user
///
/// # Arguments
/// * `user_hash` - The user's hash for isolation
/// * `tracker` - Progress tracker instance
///
/// # Returns
/// * `HandlerResult<ProgressListResponse>` - List of progress items wrapped in standard envelope
pub async fn get_all_progress(
    user_hash: &str,
    tracker: &ProgressTracker,
) -> HandlerResult<ProgressListResponse> {
    let jobs = tracker
        .list_by_user(user_hash)
        .await
        .map_err(|e| HandlerError::Internal(format!("Failed to list progress: {}", e)))?;

    let progress: Vec<IngestionProgress> = jobs
        .into_iter()
        .filter(|j| matches!(j.job_type, JobType::Ingestion | JobType::Indexing))
        .map(|j| j.into())
        .collect();

    Ok(ApiResponse::success_with_user(
        ProgressListResponse { progress },
        user_hash,
    ))
}

/// Get progress for a specific job
///
/// # Arguments
/// * `id` - The progress ID
/// * `user_hash` - The user's hash for isolation
/// * `tracker` - Progress tracker instance
///
/// # Returns
/// * `HandlerResult<IngestionProgress>` - Progress item wrapped in standard envelope
pub async fn get_progress(
    id: &str,
    user_hash: &str,
    tracker: &ProgressTracker,
) -> HandlerResult<IngestionProgress> {
    let progress_service = ProgressService::new(tracker.clone());

    match progress_service.get_progress(id).await {
        Some(progress) => Ok(ApiResponse::success_with_user(progress, user_hash)),
        None => Err(HandlerError::NotFound(format!(
            "Progress not found for ID: {}",
            id
        ))),
    }
}

/// Get ingestion service status
///
/// # Arguments
/// * `user_hash` - The user's hash for context
///
/// # Returns
/// * `HandlerResult<IngestionStatusResponse>` - Status wrapped in standard envelope
pub async fn get_status(user_hash: &str) -> HandlerResult<IngestionStatusResponse> {
    match create_ingestion_service() {
        Ok(service) => match service.get_status() {
            Ok(status) => Ok(ApiResponse::success_with_user(
                IngestionStatusResponse {
                    enabled: status.enabled,
                    configured: status.configured,
                    provider: format!("{:?}", status.provider),
                    model: status.model,
                },
                user_hash,
            )),
            Err(e) => Err(HandlerError::Internal(format!(
                "Failed to get status: {}",
                e
            ))),
        },
        Err(_e) => {
            // Return a disabled status rather than an error
            Ok(ApiResponse::success_with_user(
                IngestionStatusResponse {
                    enabled: false,
                    configured: false,
                    provider: "None".to_string(),
                    model: "".to_string(),
                },
                user_hash,
            ))
        }
    }
}

/// Get ingestion configuration
///
/// # Arguments
/// * `user_hash` - The user's hash for context
///
/// # Returns
/// * `HandlerResult<IngestionConfig>` - Config wrapped in standard envelope
pub async fn get_config(user_hash: &str) -> HandlerResult<IngestionConfig> {
    let config = IngestionConfig::from_env_allow_empty();

    Ok(ApiResponse::success_with_user(config.redacted(), user_hash))
}

/// Save ingestion configuration
///
/// # Arguments
/// * `config` - The configuration to save
/// * `user_hash` - The user's hash for context
///
/// # Returns
/// * `HandlerResult<ConfigSaveResponse>` - Result wrapped in standard envelope
pub async fn save_config(
    config: SavedConfig,
    user_hash: &str,
) -> HandlerResult<ConfigSaveResponse> {
    match IngestionConfig::save_to_file(&config) {
        Ok(()) => Ok(ApiResponse::success_with_user(
            ConfigSaveResponse {
                success: true,
                message: "Configuration saved successfully".to_string(),
            },
            user_hash,
        )),
        Err(e) => Err(HandlerError::Internal(format!(
            "Failed to save configuration: {}",
            e
        ))),
    }
}

/// Process JSON ingestion (starts background task and returns immediately)
///
/// This is the shared handler for JSON ingestion. It:
/// 1. Validates the input data
/// 2. Starts a progress tracking job
/// 3. Spawns background ingestion
/// 4. Returns immediately with progress_id
///
/// # Arguments
/// * `request` - The ingestion request with data and options
/// * `user_hash` - The user's hash for isolation
/// * `tracker` - Progress tracker
/// * `node` - The DataFold node
///
/// # Returns
/// * `HandlerResult<ProcessJsonResponse>` - Response with progress_id
pub async fn process_json(
    request: IngestionRequest,
    user_hash: &str,
    tracker: &ProgressTracker,
    node: &DataFoldNode,
) -> HandlerResult<ProcessJsonResponse> {
    // Validate data is not empty
    if request.data.is_null() {
        return Err(HandlerError::BadRequest("Data cannot be null".to_string()));
    }

    if let Value::Object(ref obj) = request.data {
        if obj.is_empty() {
            return Err(HandlerError::BadRequest("Data cannot be empty".to_string()));
        }
    }

    // Generate or use provided progress_id
    let progress_id = request
        .progress_id
        .clone()
        .unwrap_or_else(|| uuid::Uuid::new_v4().to_string());

    // Start progress tracking
    let progress_service = ProgressService::new(tracker.clone());
    progress_service
        .start_progress(progress_id.clone(), user_hash.to_string())
        .await;

    // Create ingestion service
    let service = match create_ingestion_service() {
        Ok(s) => s,
        Err(e) => {
            progress_service
                .fail_progress(
                    &progress_id,
                    format!("Ingestion service not available: {}", e),
                )
                .await;
            return Err(e);
        }
    };

    // Clone what we need for the background task
    let node_clone = node.clone();
    let progress_id_clone = progress_id.clone();
    let user_hash_clone = user_hash.to_string();
    let tracker_clone = tracker.clone();

    // Spawn background ingestion
    tokio::spawn(async move {
        crate::logging::core::run_with_user(&user_hash_clone, async move {
            let progress_service = ProgressService::new(tracker_clone);

            match service
                .process_json_with_node_and_progress(
                    request,
                    &node_clone,
                    &progress_service,
                    progress_id_clone.clone(),
                )
                .await
            {
                Ok(response) => {
                    if !response.success {
                        crate::log_feature!(
                            crate::logging::features::LogFeature::Ingestion,
                            error,
                            "Background ingestion failed: {:?}",
                            response.errors
                        );
                    }
                }
                Err(e) => {
                    crate::log_feature!(
                        crate::logging::features::LogFeature::Ingestion,
                        error,
                        "Background ingestion processing failed: {}",
                        e
                    );
                    progress_service
                        .fail_progress(&progress_id_clone, format!("Processing failed: {}", e))
                        .await;
                }
            }
        })
        .await;
    });

    // Return immediately with progress_id
    Ok(ApiResponse::success_with_user(
        ProcessJsonResponse {
            success: true,
            progress_id,
            message: "Ingestion started. Use progress_id to track status.".to_string(),
        },
        user_hash,
    ))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_progress_list_response_serialization() {
        let response = ProgressListResponse { progress: vec![] };
        let json = serde_json::to_string(&response).unwrap();
        assert!(json.contains("progress"));
    }

    #[test]
    fn test_extract_ingestion_data_wrapped() {
        let payload = serde_json::json!({
            "data": {"field": "value"},
            "progress_id": "test-123",
            "auto_execute": true
        });

        let (data, progress_id) = extract_ingestion_data(&payload).unwrap();
        assert_eq!(data, serde_json::json!({"field": "value"}));
        assert_eq!(progress_id, Some("test-123".to_string()));
    }

    #[test]
    fn test_extract_ingestion_data_direct() {
        let payload = serde_json::json!({
            "field": "value",
            "another": 123
        });

        let (data, progress_id) = extract_ingestion_data(&payload).unwrap();
        assert_eq!(data, payload);
        assert_eq!(progress_id, None);
    }

    #[test]
    fn test_extract_ingestion_data_error() {
        let payload = serde_json::json!({
            "progress_id": "test-123",
            "auto_execute": true
        });

        let result = extract_ingestion_data(&payload);
        assert!(result.is_err());
    }
}
