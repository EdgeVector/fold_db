//! Shared System Handlers
//!
//! Framework-agnostic handlers for system operations.

use crate::fold_node::config::DatabaseConfig;
use crate::fold_node::node::FoldNode;
use crate::fold_node::OperationProcessor;
use crate::handlers::response::{ApiResponse, HandlerError, HandlerResult};
use crate::progress::{Job, JobType, ProgressTracker};
use serde::{Deserialize, Serialize};
use serde_json::Value;

#[cfg(feature = "ts-bindings")]
use ts_rs::TS;

// ============================================================================
// Response Types
// ============================================================================

/// Response for system status
#[derive(Debug, Clone, Serialize, Deserialize)]
#[cfg_attr(feature = "ts-bindings", derive(TS))]
#[cfg_attr(
    feature = "ts-bindings",
    ts(export, export_to = "src/fold_node/static-react/src/types/")
)]
pub struct SystemStatusResponse {
    pub status: String,
    pub uptime: u64,
    pub version: String,
    /// Schema service URL configured on the backend (None = local/embedded)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub schema_service_url: Option<String>,
}

/// Response for node key
#[derive(Debug, Clone, Serialize, Deserialize)]
#[cfg_attr(feature = "ts-bindings", derive(TS))]
#[cfg_attr(
    feature = "ts-bindings",
    ts(export, export_to = "src/fold_node/static-react/src/types/")
)]
pub struct NodeKeyResponse {
    pub success: bool,
    pub key: String,
    pub message: String,
}

/// Response for indexing status
#[derive(Debug, Clone, Serialize, Deserialize)]
#[cfg_attr(feature = "ts-bindings", derive(TS))]
#[cfg_attr(
    feature = "ts-bindings",
    ts(export, export_to = "src/fold_node/static-react/src/types/")
)]
pub struct IndexingStatusResponse {
    pub status: serde_json::Value,
}

/// Request for database reset
#[derive(Debug, Clone, Serialize, Deserialize)]
#[cfg_attr(feature = "ts-bindings", derive(TS))]
#[cfg_attr(
    feature = "ts-bindings",
    ts(export, export_to = "src/fold_node/static-react/src/types/")
)]
pub struct ResetDatabaseRequest {
    pub confirm: bool,
}

/// Response for database reset
#[derive(Debug, Clone, Serialize, Deserialize)]
#[cfg_attr(feature = "ts-bindings", derive(TS))]
#[cfg_attr(
    feature = "ts-bindings",
    ts(export, export_to = "src/fold_node/static-react/src/types/")
)]
pub struct ResetDatabaseResponse {
    pub success: bool,
    pub message: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub job_id: Option<String>,
}

/// Response for schema service reset
#[derive(Debug, Clone, Serialize, Deserialize)]
#[cfg_attr(feature = "ts-bindings", derive(TS))]
#[cfg_attr(
    feature = "ts-bindings",
    ts(export, export_to = "src/fold_node/static-react/src/types/")
)]
pub struct ResetSchemaServiceResponse {
    pub success: bool,
    pub message: String,
}

/// Database config response (simplified for API)
#[derive(Debug, Clone, Serialize, Deserialize)]
#[cfg_attr(feature = "ts-bindings", derive(TS))]
#[cfg_attr(
    feature = "ts-bindings",
    ts(export, export_to = "src/fold_node/static-react/src/types/")
)]
pub struct DatabaseConfigResponse {
    pub config_type: String,
    pub details: Value,
}

/// Security key response
#[derive(Debug, Clone, Serialize, Deserialize)]
#[cfg_attr(feature = "ts-bindings", derive(TS))]
#[cfg_attr(
    feature = "ts-bindings",
    ts(export, export_to = "src/fold_node/static-react/src/types/")
)]
pub struct SecurityKeyResponse {
    pub success: bool,
    pub key: Option<Value>,
    pub error: Option<String>,
}

// ============================================================================
// Handler Functions
// ============================================================================

/// Get system status
pub async fn get_system_status(
    user_hash: &str,
    node: &FoldNode,
) -> HandlerResult<SystemStatusResponse> {
    Ok(ApiResponse::success_with_user(
        SystemStatusResponse {
            status: "running".to_string(),
            uptime: std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap_or_default()
                .as_secs(),
            version: env!("CARGO_PKG_VERSION").to_string(),
            schema_service_url: node.schema_service_url(),
        },
        user_hash,
    ))
}

/// Get indexing status
pub async fn get_indexing_status(
    user_hash: &str,
    node: &FoldNode,
) -> HandlerResult<IndexingStatusResponse> {
    let processor = OperationProcessor::new(node.clone());

    match processor.get_indexing_status().await {
        Ok(status) => {
            // Convert to JSON Value
            let status_json = serde_json::to_value(&status).unwrap_or(serde_json::Value::Null);
            Ok(ApiResponse::success_with_user(
                IndexingStatusResponse {
                    status: status_json,
                },
                user_hash,
            ))
        }
        Err(e) => Err(HandlerError::Internal(format!(
            "Failed to get indexing status: {}",
            e
        ))),
    }
}

/// Get node private key
pub async fn get_node_private_key(
    user_hash: &str,
    node: &FoldNode,
) -> HandlerResult<NodeKeyResponse> {
    let private_key = node.get_node_private_key();

    Ok(ApiResponse::success_with_user(
        NodeKeyResponse {
            success: true,
            key: private_key.to_string(),
            message: "Node private key retrieved successfully".to_string(),
        },
        user_hash,
    ))
}

/// Get node public key
pub async fn get_node_public_key(
    user_hash: &str,
    node: &FoldNode,
) -> HandlerResult<NodeKeyResponse> {
    let public_key = node.get_node_public_key();

    Ok(ApiResponse::success_with_user(
        NodeKeyResponse {
            success: true,
            key: public_key.to_string(),
            message: "Node public key retrieved successfully".to_string(),
        },
        user_hash,
    ))
}

/// Reset database (starts background job, returns immediately)
///
/// This is a destructive operation that clears all user data.
/// Returns a job_id that can be used to track progress.
pub async fn reset_database(
    request: ResetDatabaseRequest,
    user_hash: &str,
    tracker: &ProgressTracker,
    node: &FoldNode,
) -> HandlerResult<ResetDatabaseResponse> {
    // Require explicit confirmation
    if !request.confirm {
        return Err(HandlerError::BadRequest(
            "Reset confirmation required. Set 'confirm' to true.".to_string(),
        ));
    }

    // Generate a unique job ID
    let job_id = format!("reset_{}", uuid::Uuid::new_v4());

    // Create the job entry
    let mut job = Job::new(job_id.clone(), JobType::Other("database_reset".to_string()));
    job = job.with_user(user_hash.to_string());
    job.update_progress(5, "Initializing database reset...".to_string());

    // Save initial job state
    tracker
        .save(&job)
        .await
        .map_err(|e| HandlerError::Internal(format!("Failed to create reset job: {}", e)))?;

    // Clone what we need for the background task
    let node_clone = node.clone();
    let job_id_clone = job_id.clone();
    let user_hash_clone = user_hash.to_string();
    let user_hash_for_reset = user_hash.to_string();
    let user_hash_for_complete = user_hash.to_string();
    let tracker_clone = tracker.clone();

    // Spawn background reset task
    tokio::spawn(async move {
        crate::logging::core::run_with_user(&user_hash_clone, async move {
            // Update progress
            if let Ok(Some(mut job)) = tracker_clone.load(&job_id_clone).await {
                job.update_progress(10, "Clearing user data from storage...".to_string());
                let _ = tracker_clone.save(&job).await;
            }

            let processor = OperationProcessor::new(node_clone.clone());

            // Step 2: Perform the storage reset
            if let Err(e) = processor
                .perform_database_reset(Some(&user_hash_for_reset))
                .await
            {
                log::error!("Database reset failed: {}", e);
                if let Ok(Some(mut job)) = tracker_clone.load(&job_id_clone).await {
                    job.fail(format!("Database reset failed: {}", e));
                    let _ = tracker_clone.save(&job).await;
                }
                return;
            }

            // Mark job as complete
            if let Ok(Some(mut job)) = tracker_clone.load(&job_id_clone).await {
                job.complete(Some(serde_json::json!({
                    "user_id": user_hash_for_complete,
                    "message": "Database reset successfully. All data has been cleared."
                })));
                let _ = tracker_clone.save(&job).await;
            }
        })
        .await;
    });

    // Return immediately with job_id
    Ok(ApiResponse::success_with_user(
        ResetDatabaseResponse {
            success: true,
            message:
                "Database reset started. Monitor progress via /api/ingestion/progress endpoint."
                    .to_string(),
            job_id: Some(job_id),
        },
        user_hash,
    ))
}

/// Reset database synchronously (awaits completion before returning)
///
/// Same as `reset_database()` but runs the reset inline instead of spawning a
/// background task. Use this in Lambda where `tokio::spawn` tasks get frozen
/// after the handler responds.
pub async fn reset_database_sync(
    request: ResetDatabaseRequest,
    user_hash: &str,
    tracker: &ProgressTracker,
    node: &FoldNode,
) -> HandlerResult<ResetDatabaseResponse> {
    // Require explicit confirmation
    if !request.confirm {
        return Err(HandlerError::BadRequest(
            "Reset confirmation required. Set 'confirm' to true.".to_string(),
        ));
    }

    // Generate a unique job ID
    let job_id = format!("reset_{}", uuid::Uuid::new_v4());

    // Create the job entry
    let mut job = Job::new(job_id.clone(), JobType::Other("database_reset".to_string()));
    job = job.with_user(user_hash.to_string());
    job.update_progress(5, "Initializing database reset...".to_string());

    // Save initial job state
    tracker
        .save(&job)
        .await
        .map_err(|e| HandlerError::Internal(format!("Failed to create reset job: {}", e)))?;

    // Run reset inline (no tokio::spawn)
    let result = crate::logging::core::run_with_user(user_hash, {
        let job_id = job_id.clone();
        let tracker = tracker.clone();
        let node = node.clone();
        let user_hash = user_hash.to_string();
        async move {
            // Update progress
            if let Ok(Some(mut job)) = tracker.load(&job_id).await {
                job.update_progress(10, "Clearing user data from storage...".to_string());
                let _ = tracker.save(&job).await;
            }

            let processor = OperationProcessor::new(node);

            // Perform the storage reset
            if let Err(e) = processor
                .perform_database_reset(Some(&user_hash))
                .await
            {
                log::error!("Database reset failed: {}", e);
                if let Ok(Some(mut job)) = tracker.load(&job_id).await {
                    job.fail(format!("Database reset failed: {}", e));
                    let _ = tracker.save(&job).await;
                }
                return Err(format!("Database reset failed: {}", e));
            }

            // Mark job as complete
            if let Ok(Some(mut job)) = tracker.load(&job_id).await {
                job.complete(Some(serde_json::json!({
                    "user_id": user_hash,
                    "message": "Database reset successfully. All data has been cleared."
                })));
                let _ = tracker.save(&job).await;
            }

            Ok(())
        }
    })
    .await;

    match result {
        Ok(()) => Ok(ApiResponse::success_with_user(
            ResetDatabaseResponse {
                success: true,
                message: "Database reset completed successfully. All data has been cleared."
                    .to_string(),
                job_id: Some(job_id),
            },
            user_hash,
        )),
        Err(e) => Err(HandlerError::Internal(e)),
    }
}

/// Reset schema service
pub async fn reset_schema_service(
    request: ResetDatabaseRequest,
    user_hash: &str,
    node: &FoldNode,
) -> HandlerResult<ResetSchemaServiceResponse> {
    // Require explicit confirmation
    if !request.confirm {
        return Err(HandlerError::BadRequest(
            "Reset confirmation required. Set 'confirm' to true.".to_string(),
        ));
    }

    let schema_client = node.get_schema_client();

    match schema_client.reset_schema_service().await {
        Ok(()) => Ok(ApiResponse::success_with_user(
            ResetSchemaServiceResponse {
                success: true,
                message:
                    "Schema service database reset successfully. All schemas have been cleared."
                        .to_string(),
            },
            user_hash,
        )),
        Err(e) => Err(HandlerError::Internal(format!(
            "Schema service reset failed: {}",
            e
        ))),
    }
}

/// Get database configuration
pub async fn get_database_config(
    user_hash: &str,
    node: &FoldNode,
) -> HandlerResult<DatabaseConfigResponse> {
    let config = &node.config;

    let (config_type, details) = match &config.database {
        DatabaseConfig::Local { path } => (
            "local".to_string(),
            serde_json::json!({ "path": path.to_string_lossy() }),
        ),
        #[cfg(feature = "aws-backend")]
        DatabaseConfig::Cloud(cloud_config) => (
            "cloud".to_string(),
            serde_json::json!({
                "region": cloud_config.region,
                "auto_create": cloud_config.auto_create,
                "user_id": cloud_config.user_id,
            }),
        ),
        DatabaseConfig::Exemem { api_url, .. } => (
            "exemem".to_string(),
            serde_json::json!({ "api_url": api_url }),
        ),
    };

    Ok(ApiResponse::success_with_user(
        DatabaseConfigResponse {
            config_type,
            details,
        },
        user_hash,
    ))
}

/// Get system public key (security manager)
pub async fn get_system_public_key(
    user_hash: &str,
    node: &FoldNode,
) -> HandlerResult<SecurityKeyResponse> {
    let security_manager = node.get_security_manager();

    match security_manager.get_system_public_key() {
        Ok(Some(key_info)) => Ok(ApiResponse::success_with_user(
            SecurityKeyResponse {
                success: true,
                key: Some(serde_json::to_value(key_info).unwrap_or(Value::Null)),
                error: None,
            },
            user_hash,
        )),
        Ok(None) => Err(HandlerError::NotFound("System key not found".to_string())),
        Err(e) => Err(HandlerError::Internal(e.to_string())),
    }
}
