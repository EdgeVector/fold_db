//! Shared System Handlers
//!
//! Framework-agnostic handlers for system operations.

use crate::fold_node::config::DatabaseConfig;
use crate::fold_node::node::FoldNode;
use crate::fold_node::OperationProcessor;
use crate::handlers::response::{ApiResponse, HandlerError, HandlerResult};
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

