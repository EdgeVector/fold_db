//! Shared System Handlers
//!
//! Framework-agnostic handlers for system operations.

use crate::fold_node::node::FoldNode;
use crate::fold_node::OperationProcessor;
use crate::handlers::response::{ApiResponse, HandlerResult, IntoHandlerError};
use serde::{Deserialize, Serialize};

#[cfg(feature = "ts-bindings")]
use ts_rs::TS;

#[derive(Debug, Clone, Serialize, Deserialize)]
#[cfg_attr(feature = "ts-bindings", derive(TS))]
#[cfg_attr(feature = "ts-bindings", ts(export, export_to = "src/fold_node/static-react/src/types/"))]
pub struct SystemStatusResponse {
    pub status: String,
    pub uptime: u64,
    pub version: String,
    /// Schema service URL configured on the backend (None = local/embedded)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub schema_service_url: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[cfg_attr(feature = "ts-bindings", derive(TS))]
#[cfg_attr(feature = "ts-bindings", ts(export, export_to = "src/fold_node/static-react/src/types/"))]
pub struct NodeKeyResponse {
    pub success: bool,
    pub key: String,
    pub message: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[cfg_attr(feature = "ts-bindings", derive(TS))]
#[cfg_attr(feature = "ts-bindings", ts(export, export_to = "src/fold_node/static-react/src/types/"))]
pub struct IndexingStatusResponse {
    pub status: serde_json::Value,
}

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

pub async fn get_indexing_status(
    user_hash: &str,
    node: &FoldNode,
) -> HandlerResult<IndexingStatusResponse> {
    let status = OperationProcessor::new(node.clone())
        .get_indexing_status()
        .await
        .handler_err("get indexing status")?;
    let status_json = serde_json::to_value(&status).unwrap_or(serde_json::Value::Null);
    Ok(ApiResponse::success_with_user(
        IndexingStatusResponse { status: status_json },
        user_hash,
    ))
}

pub async fn get_node_private_key(
    user_hash: &str,
    node: &FoldNode,
) -> HandlerResult<NodeKeyResponse> {
    Ok(ApiResponse::success_with_user(
        NodeKeyResponse {
            success: true,
            key: node.get_node_private_key().to_string(),
            message: "Node private key retrieved successfully".to_string(),
        },
        user_hash,
    ))
}

pub async fn get_node_public_key(
    user_hash: &str,
    node: &FoldNode,
) -> HandlerResult<NodeKeyResponse> {
    Ok(ApiResponse::success_with_user(
        NodeKeyResponse {
            success: true,
            key: node.get_node_public_key().to_string(),
            message: "Node public key retrieved successfully".to_string(),
        },
        user_hash,
    ))
}
