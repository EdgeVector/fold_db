//! Shared Log Handlers
//!
//! Framework-agnostic handlers for logging operations.
//! These can be called by both HTTP server routes and Lambda handlers.

use crate::datafold_node::node::DataFoldNode;
use crate::datafold_node::OperationProcessor;
use crate::handlers::response::{ApiResponse, HandlerError, HandlerResult};
use crate::logging::core::{LogEntry, Logger};
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};

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

/// Simple success response
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SuccessResponse {
    pub success: bool,
    pub message: String,
}

/// Response for user log query
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UserLogQueryResponse {
    pub logs: Vec<Value>,
    pub count: usize,
    pub source: String,
    pub timestamp: String,
}

/// Response for logger test
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LoggerTestResponse {
    pub tests_run: usize,
    pub tests_passed: usize,
    pub tests_failed: usize,
    pub results: Vec<String>,
    pub failed: Vec<String>,
}

// ============================================================================
// Backend-agnostic Handlers (via Logger trait)
// ============================================================================

/// Query logs using a Logger implementation
///
/// This is the primary way to query logs in a multi-tenant system.
/// Works with any Logger implementation (DynamoDB, file-based, etc.)
///
/// # Arguments
/// * `logger` - Logger implementation to query
/// * `user_hash` - User identifier for isolation
/// * `limit` - Maximum number of logs to return
/// * `from_timestamp` - Optional start timestamp (milliseconds)
///
/// # Returns
/// * `HandlerResult<UserLogQueryResponse>` - Query results
pub async fn query_user_logs<L: Logger>(
    logger: &L,
    user_hash: &str,
    limit: Option<usize>,
    from_timestamp: Option<i64>,
    source_name: &str,
) -> HandlerResult<UserLogQueryResponse> {
    log::info!(
        "Querying logs for user: {}, source: {}",
        user_hash,
        source_name
    );

    match logger.query(user_hash, limit, from_timestamp).await {
        Ok(log_entries) => {
            let logs: Vec<Value> = log_entries
                .into_iter()
                .map(|entry| {
                    json!({
                        "id": entry.id,
                        "user_id": entry.user_id.unwrap_or_else(|| user_hash.to_string()),
                        "timestamp": entry.timestamp,
                        "level": format!("{:?}", entry.level).to_lowercase(),
                        "event_type": entry.event_type,
                        "message": entry.message,
                        "metadata": entry.metadata,
                    })
                })
                .collect();

            let count = logs.len();
            log::info!("Found {} logs for user {}", count, user_hash);

            Ok(ApiResponse::success_with_user(
                UserLogQueryResponse {
                    logs,
                    count,
                    source: source_name.to_string(),
                    timestamp: chrono::Utc::now().to_rfc3339(),
                },
                user_hash,
            ))
        }
        Err(e) => {
            log::error!("Failed to query logs: {}", e);
            Err(HandlerError::Internal(format!(
                "Failed to query logs: {}",
                e
            )))
        }
    }
}

/// Test a logger by writing test entries at various levels
///
/// # Arguments
/// * `logger` - Logger implementation to test
/// * `user_hash` - User identifier for the test
///
/// # Returns
/// * `HandlerResult<LoggerTestResponse>` - Test results
pub async fn test_logger<L: Logger>(
    logger: &L,
    user_hash: &str,
) -> HandlerResult<LoggerTestResponse> {
    use crate::logging::core::LogLevel;
    use std::collections::HashMap;

    log::info!("Testing logger for user: {}", user_hash);

    let mut results = Vec::new();
    let mut failed_tests = Vec::new();

    // Define test cases
    let test_cases = vec![
        (LogLevel::Info, "test_info", "Testing INFO level logging"),
        (LogLevel::Error, "test_error", "Testing ERROR level logging"),
        (LogLevel::Warn, "test_warn", "Testing WARN level logging"),
        (LogLevel::Debug, "test_debug", "Testing DEBUG level logging"),
        (LogLevel::Trace, "test_trace", "Testing TRACE level logging"),
    ];

    // Run level tests
    for (level, event_type, message) in test_cases {
        let entry = LogEntry {
            level: level.clone(),
            event_type: event_type.to_string(),
            message: message.to_string(),
            metadata: None,
            timestamp: chrono::Utc::now().timestamp_millis(),
            user_id: Some(user_hash.to_string()),
            id: uuid::Uuid::new_v4().to_string(),
        };

        match logger.log(entry).await {
            Ok(_) => results.push(format!("{:?} level test passed", level)),
            Err(e) => failed_tests.push(format!("{:?} test failed: {}", level, e)),
        }
    }

    // Test with metadata
    let mut metadata = HashMap::new();
    metadata.insert("test_key".to_string(), "test_value".to_string());
    metadata.insert("event_id".to_string(), "12345".to_string());

    let entry = LogEntry {
        level: LogLevel::Info,
        event_type: "test_metadata".to_string(),
        message: "Testing logging with custom metadata".to_string(),
        metadata: Some(metadata),
        timestamp: chrono::Utc::now().timestamp_millis(),
        user_id: Some(user_hash.to_string()),
        id: uuid::Uuid::new_v4().to_string(),
    };

    match logger.log(entry).await {
        Ok(_) => results.push("Metadata logging test passed".to_string()),
        Err(e) => failed_tests.push(format!("Metadata test failed: {}", e)),
    }

    let tests_run = results.len() + failed_tests.len();
    let tests_passed = results.len();
    let tests_failed = failed_tests.len();

    if failed_tests.is_empty() {
        log::info!("✅ All logger tests passed ({} tests)", results.len());
        Ok(ApiResponse::success_with_user(
            LoggerTestResponse {
                tests_run,
                tests_passed,
                tests_failed,
                results,
                failed: failed_tests,
            },
            user_hash,
        ))
    } else {
        log::error!("❌ Some logger tests failed: {:?}", failed_tests);
        // Still return success envelope but with failure details
        Ok(ApiResponse::success_with_user(
            LoggerTestResponse {
                tests_run,
                tests_passed,
                tests_failed,
                results,
                failed: failed_tests,
            },
            user_hash,
        ))
    }
}

// ============================================================================
// Node-based Handlers (for HTTP server using OperationProcessor)
// ============================================================================

/// List logs using OperationProcessor
pub async fn list_logs(
    since: Option<i64>,
    user_hash: &str,
    node: &DataFoldNode,
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
    node: &DataFoldNode,
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
    node: &DataFoldNode,
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
    node: &DataFoldNode,
) -> HandlerResult<SuccessResponse> {
    let processor = OperationProcessor::new(node.clone());

    match processor.update_log_feature_level(feature, level).await {
        Ok(_) => Ok(ApiResponse::success_with_user(
            SuccessResponse {
                success: true,
                message: format!("Updated {} log level to {}", feature, level),
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
    node: &DataFoldNode,
) -> HandlerResult<SuccessResponse> {
    let processor = OperationProcessor::new(node.clone());

    match processor.reload_log_config(config_path).await {
        Ok(_) => Ok(ApiResponse::success_with_user(
            SuccessResponse {
                success: true,
                message: "Configuration reloaded successfully".to_string(),
            },
            user_hash,
        )),
        Err(e) => Err(HandlerError::Internal(format!(
            "Failed to reload configuration: {}",
            e
        ))),
    }
}

// ============================================================================
// Transport-Specific Responses
// ============================================================================

/// Returns an informative error for SSE streaming requests in Lambda
///
/// SSE (Server-Sent Events) requires long-lived connections which are not
/// supported in Lambda. This helper provides a clear error message.
pub fn stream_logs_not_supported(_user_hash: &str) -> HandlerResult<SuccessResponse> {
    // Return error in success envelope for consistent response handling
    Err(HandlerError::BadRequest(
        "Server-Sent Events (SSE) streaming is not supported in AWS Lambda. Use GET /api/logs with the 'since' parameter to poll for new logs.".to_string()
    ))
}
