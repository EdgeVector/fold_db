//! Logging abstraction for Lambda deployments
//!
//! Re-exports generic logging functionality from `crate::logging::core`.

pub use crate::logging::core::*;

use super::context::LambdaContext;
use crate::ingestion::IngestionError;
use serde_json::Value;

impl LambdaContext {
    /// List logs
    ///
    /// # Arguments
    ///
    /// * `since` - Optional timestamp to filter logs
    /// * `limit` - Optional limit on number of logs (default 1000)
    /// * `user_id` - User ID for node context
    pub async fn list_logs(
        since: Option<i64>,
        limit: Option<usize>,
        user_id: String,
    ) -> Result<Vec<Value>, IngestionError> {
        let processor = {
            let node_mutex = Self::get_node(&user_id).await?;
            let node_guard = node_mutex.lock().await;
            crate::datafold_node::OperationProcessor::new(node_guard.clone())
        };

        let limit = limit.unwrap_or(1000);
        let logs = processor.list_logs(since, Some(limit)).await;

        Ok(logs
            .into_iter()
            .map(|log| {
                serde_json::to_value(log)
                    .unwrap_or(serde_json::json!({"error": "Failed to serialize log"}))
            })
            .collect())
    }

    /// Get log configuration
    pub async fn get_log_config(user_id: String) -> Result<Value, IngestionError> {
        let processor = {
            let node_mutex = Self::get_node(&user_id).await?;
            let node_guard = node_mutex.lock().await;
            crate::datafold_node::OperationProcessor::new(node_guard.clone())
        };

        if let Some(config) = processor.get_log_config().await {
            Ok(serde_json::json!({
                "config": config
            }))
        } else {
            let current_level = log::max_level().to_string();
            Ok(serde_json::json!({
                "message": "Basic logging configuration",
                "current_level": current_level
            }))
        }
    }

    /// Reload log configuration
    pub async fn reload_log_config(user_id: String) -> Result<(), IngestionError> {
        let processor = {
            let node_mutex = Self::get_node(&user_id).await?;
            let node_guard = node_mutex.lock().await;
            crate::datafold_node::OperationProcessor::new(node_guard.clone())
        };

        processor
            .reload_log_config("config/logging.toml")
            .await
            .map_err(|e| {
                IngestionError::InvalidInput(format!("Failed to reload configuration: {}", e))
            })
    }

    /// Get log features
    pub async fn get_log_features(user_id: String) -> Result<Value, IngestionError> {
        let processor = {
            let node_mutex = Self::get_node(&user_id).await?;
            let node_guard = node_mutex.lock().await;
            crate::datafold_node::OperationProcessor::new(node_guard.clone())
        };

        if let Some(features) = processor.get_log_features().await {
            Ok(serde_json::json!({
                "features": features,
                "available_levels": ["TRACE", "DEBUG", "INFO", "WARN", "ERROR"]
            }))
        } else {
            let current_level = log::max_level().to_string();
            Ok(serde_json::json!({
                "features": {
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
                },
                "available_levels": ["TRACE", "DEBUG", "INFO", "WARN", "ERROR"]
            }))
        }
    }

    /// Update log feature level
    pub async fn update_log_feature_level(
        feature: &str,
        level: &str,
        user_id: String,
    ) -> Result<(), IngestionError> {
        let valid_levels = ["TRACE", "DEBUG", "INFO", "WARN", "ERROR"];
        if !valid_levels.contains(&level) {
            return Err(IngestionError::InvalidInput(format!(
                "Invalid log level: {}",
                level
            )));
        }

        let processor = {
            let node_mutex = Self::get_node(&user_id).await?;
            let node_guard = node_mutex.lock().await;
            crate::datafold_node::OperationProcessor::new(node_guard.clone())
        };

        processor
            .update_log_feature_level(feature, level)
            .await
            .map_err(|e| IngestionError::InvalidInput(format!("Failed to update log level: {}", e)))
    }
}
