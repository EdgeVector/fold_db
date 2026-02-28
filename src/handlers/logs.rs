//! Shared Log Handlers
//!
//! Framework-agnostic handlers for logging operations.
//! These can be called by both HTTP server routes and Lambda handlers.

use crate::fold_node::node::FoldNode;
use crate::fold_node::OperationProcessor;
use crate::handlers::response::{ApiResponse, HandlerError, HandlerResult, SuccessResponse};
use serde::{Deserialize, Serialize};

// ============================================================================
// Response Types
// ============================================================================

/// Response for log list
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LogListResponse {
    pub logs: serde_json::Value,
    pub count: usize,
    pub timestamp: u64,
}

/// Response for log config
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LogConfigResponse {
    pub config: serde_json::Value,
}

/// Response for log features
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LogFeaturesResponse {
    pub features: serde_json::Value,
    pub available_levels: Vec<String>,
}

// ============================================================================
// Node-based Handlers (for HTTP server using OperationProcessor)
// ============================================================================

/// List logs using OperationProcessor
pub async fn list_logs(
    since: Option<i64>,
    user_hash: &str,
    node: &FoldNode,
) -> HandlerResult<LogListResponse> {
    let processor = OperationProcessor::new(node.clone());

    let logs = processor.list_logs(since, Some(1000)).await;
    let count = logs.len();

    // Convert to JSON Value
    let logs_json =
        serde_json::to_value(&logs).unwrap_or_else(|_| serde_json::Value::Array(vec![]));

    Ok(ApiResponse::success_with_user(
        LogListResponse {
            logs: logs_json,
            count,
            timestamp: std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap_or_default()
                .as_secs(),
        },
        user_hash,
    ))
}

/// Get log configuration
pub async fn get_log_config(
    user_hash: &str,
    node: &FoldNode,
) -> HandlerResult<LogConfigResponse> {
    let processor = OperationProcessor::new(node.clone());

    if let Some(config) = processor.get_log_config().await {
        Ok(ApiResponse::success_with_user(
            LogConfigResponse {
                config: serde_json::to_value(config).unwrap_or(serde_json::Value::Null),
            },
            user_hash,
        ))
    } else {
        let current_level = log::max_level().to_string();
        Ok(ApiResponse::success_with_user(
            LogConfigResponse {
                config: serde_json::json!({
                    "message": "Basic logging configuration",
                    "current_level": current_level
                }),
            },
            user_hash,
        ))
    }
}

/// Get available log features
pub async fn get_log_features(
    user_hash: &str,
    node: &FoldNode,
) -> HandlerResult<LogFeaturesResponse> {
    let processor = OperationProcessor::new(node.clone());
    let available_levels = vec![
        "TRACE".to_string(),
        "DEBUG".to_string(),
        "INFO".to_string(),
        "WARN".to_string(),
        "ERROR".to_string(),
    ];

    if let Some(features) = processor.get_log_features().await {
        Ok(ApiResponse::success_with_user(
            LogFeaturesResponse {
                features: serde_json::to_value(features).unwrap_or(serde_json::Value::Null),
                available_levels,
            },
            user_hash,
        ))
    } else {
        let current_level = log::max_level().to_string();
        Ok(ApiResponse::success_with_user(
            LogFeaturesResponse {
                features: serde_json::json!({
                    "transform": current_level,
                    "network": current_level,
                    "database": current_level,
                    "schema": current_level,
                    "query": current_level,
                    "mutation": current_level,
                    "permissions": current_level,
                    "http_server": current_level,
                    "tcp_server": current_level,
                    "ingestion": current_level
                }),
                available_levels,
            },
            user_hash,
        ))
    }
}

/// Update log feature level
pub async fn update_log_feature_level(
    feature: &str,
    level: &str,
    user_hash: &str,
    node: &FoldNode,
) -> HandlerResult<SuccessResponse> {
    let processor = OperationProcessor::new(node.clone());

    match processor.update_log_feature_level(feature, level).await {
        Ok(_) => Ok(ApiResponse::success_with_user(
            SuccessResponse {
                success: true,
                message: Some(format!("Updated {} log level to {}", feature, level)),
            },
            user_hash,
        )),
        Err(e) => Err(HandlerError::Internal(format!(
            "Failed to update log level: {}",
            e
        ))),
    }
}

/// Reload log configuration
pub async fn reload_log_config(
    config_path: &str,
    user_hash: &str,
    node: &FoldNode,
) -> HandlerResult<SuccessResponse> {
    let processor = OperationProcessor::new(node.clone());

    match processor.reload_log_config(config_path).await {
        Ok(_) => Ok(ApiResponse::success_with_user(
            SuccessResponse {
                success: true,
                message: Some("Configuration reloaded successfully".to_string()),
            },
            user_hash,
        )),
        Err(e) => Err(HandlerError::Internal(format!(
            "Failed to reload configuration: {}",
            e
        ))),
    }
}

