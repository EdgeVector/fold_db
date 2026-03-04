//! Shared Log Handlers
//!
//! Framework-agnostic handlers for logging operations.
//! Shared between HTTP server routes and Lambda handlers.

use crate::fold_node::node::FoldNode;
use crate::fold_node::OperationProcessor;
use crate::handlers::response::{ApiResponse, HandlerError, HandlerResult, IntoHandlerError, SuccessResponse};
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LogListResponse {
    pub logs: serde_json::Value,
    pub count: usize,
    pub timestamp: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LogConfigResponse {
    pub config: serde_json::Value,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LogFeaturesResponse {
    pub features: serde_json::Value,
    pub available_levels: Vec<String>,
}

const LOG_LEVELS: &[&str] = &["TRACE", "DEBUG", "INFO", "WARN", "ERROR"];

pub async fn list_logs(
    since: Option<i64>,
    user_hash: &str,
    node: &FoldNode,
) -> HandlerResult<LogListResponse> {
    let logs = OperationProcessor::new(node.clone())
        .list_logs(since, Some(1000))
        .await;
    let count = logs.len();
    let logs_json = serde_json::to_value(&logs)
        .handler_err("serialize logs")?;
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

pub async fn get_log_config(
    user_hash: &str,
    node: &FoldNode,
) -> HandlerResult<LogConfigResponse> {
    let config = OperationProcessor::new(node.clone())
        .get_log_config()
        .await
        .ok_or_else(|| HandlerError::Internal("Log configuration not available".to_string()))?;
    Ok(ApiResponse::success_with_user(
        LogConfigResponse {
            config: serde_json::to_value(config).unwrap_or(serde_json::Value::Null),
        },
        user_hash,
    ))
}

pub async fn get_log_features(
    user_hash: &str,
    node: &FoldNode,
) -> HandlerResult<LogFeaturesResponse> {
    let features = OperationProcessor::new(node.clone())
        .get_log_features()
        .await
        .ok_or_else(|| HandlerError::Internal("Log features not available".to_string()))?;
    Ok(ApiResponse::success_with_user(
        LogFeaturesResponse {
            features: serde_json::to_value(features).unwrap_or(serde_json::Value::Null),
            available_levels: LOG_LEVELS.iter().map(|s| s.to_string()).collect(),
        },
        user_hash,
    ))
}

pub async fn update_log_feature_level(
    feature: &str,
    level: &str,
    user_hash: &str,
    node: &FoldNode,
) -> HandlerResult<SuccessResponse> {
    OperationProcessor::new(node.clone())
        .update_log_feature_level(feature, level)
        .await
        .map_err(HandlerError::from)?;
    Ok(ApiResponse::success_with_user(
        SuccessResponse {
            success: true,
            message: Some(format!("Updated {} log level to {}", feature, level)),
        },
        user_hash,
    ))
}

pub async fn reload_log_config(
    config_path: &str,
    user_hash: &str,
    node: &FoldNode,
) -> HandlerResult<SuccessResponse> {
    OperationProcessor::new(node.clone())
        .reload_log_config(config_path)
        .await
        .map_err(HandlerError::from)?;
    Ok(ApiResponse::success_with_user(
        SuccessResponse {
            success: true,
            message: Some("Configuration reloaded successfully".to_string()),
        },
        user_hash,
    ))
}
