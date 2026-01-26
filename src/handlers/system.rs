//! Shared System Handlers
//!
//! Framework-agnostic handlers for system operations.

use crate::datafold_node::node::DataFoldNode;
use crate::datafold_node::OperationProcessor;
use crate::handlers::response::{ApiResponse, HandlerError, HandlerResult};
use serde::{Deserialize, Serialize};

#[cfg(feature = "ts-bindings")]
use ts_rs::TS;

/// Response for system status
#[derive(Debug, Clone, Serialize, Deserialize)]
#[cfg_attr(feature = "ts-bindings", derive(TS))]
#[cfg_attr(
    feature = "ts-bindings",
    ts(export, export_to = "src/datafold_node/static-react/src/types/")
)]
pub struct SystemStatusResponse {
    pub status: String,
    pub uptime: u64,
    pub version: String,
}

/// Response for node key
#[derive(Debug, Clone, Serialize, Deserialize)]
#[cfg_attr(feature = "ts-bindings", derive(TS))]
#[cfg_attr(
    feature = "ts-bindings",
    ts(export, export_to = "src/datafold_node/static-react/src/types/")
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
    ts(export, export_to = "src/datafold_node/static-react/src/types/")
)]
pub struct IndexingStatusResponse {
    pub status: serde_json::Value,
}

/// Get system status
pub async fn get_system_status(
    user_hash: &str,
    _node: &DataFoldNode,
) -> HandlerResult<SystemStatusResponse> {
    Ok(ApiResponse::success_with_user(
        SystemStatusResponse {
            status: "running".to_string(),
            uptime: std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap_or_default()
                .as_secs(),
            version: env!("CARGO_PKG_VERSION").to_string(),
        },
        user_hash,
    ))
}

/// Get indexing status
pub async fn get_indexing_status(
    user_hash: &str,
    node: &DataFoldNode,
) -> HandlerResult<IndexingStatusResponse> {
    let processor = OperationProcessor::new(node.clone());

    match processor.get_indexing_status().await {
        Ok(status) => {
            // Convert to JSON Value
            let status_json =
                serde_json::to_value(&status).unwrap_or_else(|_| serde_json::Value::Null);
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
    node: &DataFoldNode,
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
    node: &DataFoldNode,
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
