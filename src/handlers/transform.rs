//! Shared Transform Handlers
//!
//! Framework-agnostic handlers for transform and backfill operations.

use crate::datafold_node::node::DataFoldNode;
use crate::datafold_node::OperationProcessor;
use crate::handlers::response::{ApiResponse, HandlerError, HandlerResult};
use serde::{Deserialize, Serialize};

#[cfg(feature = "ts-bindings")]
use ts_rs::TS;

/// Response for transform list
#[derive(Debug, Clone, Serialize, Deserialize)]
#[cfg_attr(feature = "ts-bindings", derive(TS))]
#[cfg_attr(
    feature = "ts-bindings",
    ts(export, export_to = "src/datafold_node/static-react/src/types/")
)]
pub struct TransformListResponse {
    pub transforms: serde_json::Value,
}

/// Response for transform queue
#[derive(Debug, Clone, Serialize, Deserialize)]
#[cfg_attr(feature = "ts-bindings", derive(TS))]
#[cfg_attr(
    feature = "ts-bindings",
    ts(export, export_to = "src/datafold_node/static-react/src/types/")
)]
pub struct TransformQueueResponse {
    pub length: usize,
    pub queued_transforms: Vec<String>,
}

/// Simple success response (for transform operations)
#[derive(Debug, Clone, Serialize, Deserialize)]
#[cfg_attr(feature = "ts-bindings", derive(TS))]
#[cfg_attr(
    feature = "ts-bindings",
    ts(
        export,
        rename = "TransformSuccessResponse",
        export_to = "src/datafold_node/static-react/src/types/"
    )
)]
pub struct SuccessResponse {
    pub success: bool,
    pub message: String,
}

/// Response for backfill list
#[derive(Debug, Clone, Serialize, Deserialize)]
#[cfg_attr(feature = "ts-bindings", derive(TS))]
#[cfg_attr(
    feature = "ts-bindings",
    ts(export, export_to = "src/datafold_node/static-react/src/types/")
)]
pub struct BackfillListResponse {
    pub backfills: serde_json::Value,
}

/// Response for single backfill
#[derive(Debug, Clone, Serialize, Deserialize)]
#[cfg_attr(feature = "ts-bindings", derive(TS))]
#[cfg_attr(
    feature = "ts-bindings",
    ts(export, export_to = "src/datafold_node/static-react/src/types/")
)]
pub struct BackfillResponse {
    pub backfill: serde_json::Value,
}

/// Response for transform statistics
#[derive(Debug, Clone, Serialize, Deserialize)]
#[cfg_attr(feature = "ts-bindings", derive(TS))]
#[cfg_attr(
    feature = "ts-bindings",
    ts(export, export_to = "src/datafold_node/static-react/src/types/")
)]
pub struct TransformStatsResponse {
    pub stats: serde_json::Value,
}

/// Response for backfill statistics
#[derive(Debug, Clone, Serialize, Deserialize)]
#[cfg_attr(feature = "ts-bindings", derive(TS))]
#[cfg_attr(
    feature = "ts-bindings",
    ts(export, export_to = "src/datafold_node/static-react/src/types/")
)]
pub struct BackfillStatsResponse {
    pub stats: serde_json::Value,
}

/// List all transforms
pub async fn list_transforms(
    user_hash: &str,
    node: &DataFoldNode,
) -> HandlerResult<TransformListResponse> {
    let processor = OperationProcessor::new(node.clone());

    match processor.list_transforms().await {
        Ok(map) => {
            let transforms_json = serde_json::to_value(&map)
                .unwrap_or_else(|_| serde_json::Value::Object(serde_json::Map::new()));
            Ok(ApiResponse::success_with_user(
                TransformListResponse {
                    transforms: transforms_json,
                },
                user_hash,
            ))
        }
        Err(e) => Err(HandlerError::Internal(format!(
            "Failed to list transforms: {}",
            e
        ))),
    }
}

/// Get transform queue
pub async fn get_transform_queue(
    user_hash: &str,
    node: &DataFoldNode,
) -> HandlerResult<TransformQueueResponse> {
    let processor = OperationProcessor::new(node.clone());

    match processor.get_transform_queue().await {
        Ok((len, queued)) => Ok(ApiResponse::success_with_user(
            TransformQueueResponse {
                length: len,
                queued_transforms: queued,
            },
            user_hash,
        )),
        Err(e) => Err(HandlerError::Internal(format!(
            "Failed to get transform queue: {}",
            e
        ))),
    }
}

/// Add to transform queue
pub async fn add_to_transform_queue(
    transform_id: &str,
    user_hash: &str,
    node: &DataFoldNode,
) -> HandlerResult<SuccessResponse> {
    let processor = OperationProcessor::new(node.clone());

    match processor
        .add_to_transform_queue(transform_id, "manual_api_trigger")
        .await
    {
        Ok(_) => Ok(ApiResponse::success_with_user(
            SuccessResponse {
                success: true,
                message: format!("Transform '{}' added to queue", transform_id),
            },
            user_hash,
        )),
        Err(e) => Err(HandlerError::Internal(format!(
            "Failed to add transform to queue: {}",
            e
        ))),
    }
}

/// Get all backfills
pub async fn get_all_backfills(
    user_hash: &str,
    node: &DataFoldNode,
) -> HandlerResult<BackfillListResponse> {
    let processor = OperationProcessor::new(node.clone());

    match processor.get_all_backfills().await {
        Ok(backfills) => {
            let backfills_json = serde_json::to_value(&backfills)
                .unwrap_or_else(|_| serde_json::Value::Array(vec![]));
            Ok(ApiResponse::success_with_user(
                BackfillListResponse {
                    backfills: backfills_json,
                },
                user_hash,
            ))
        }
        Err(e) => Err(HandlerError::Internal(format!(
            "Failed to get all backfills: {}",
            e
        ))),
    }
}

/// Get active backfills
pub async fn get_active_backfills(
    user_hash: &str,
    node: &DataFoldNode,
) -> HandlerResult<BackfillListResponse> {
    let processor = OperationProcessor::new(node.clone());

    match processor.get_active_backfills().await {
        Ok(backfills) => {
            let backfills_json = serde_json::to_value(&backfills)
                .unwrap_or_else(|_| serde_json::Value::Array(vec![]));
            Ok(ApiResponse::success_with_user(
                BackfillListResponse {
                    backfills: backfills_json,
                },
                user_hash,
            ))
        }
        Err(e) => Err(HandlerError::Internal(format!(
            "Failed to get active backfills: {}",
            e
        ))),
    }
}

/// Get specific backfill
pub async fn get_backfill(
    backfill_id: &str,
    user_hash: &str,
    node: &DataFoldNode,
) -> HandlerResult<BackfillResponse> {
    let processor = OperationProcessor::new(node.clone());

    match processor.get_backfill(backfill_id).await {
        Ok(Some(backfill)) => {
            let backfill_json = serde_json::to_value(&backfill).unwrap_or(serde_json::Value::Null);
            Ok(ApiResponse::success_with_user(
                BackfillResponse {
                    backfill: backfill_json,
                },
                user_hash,
            ))
        }
        Ok(None) => Err(HandlerError::NotFound(format!(
            "Backfill not found: {}",
            backfill_id
        ))),
        Err(e) => Err(HandlerError::Internal(format!(
            "Failed to get backfill: {}",
            e
        ))),
    }
}

/// Get transform statistics
pub async fn get_transform_statistics(
    user_hash: &str,
    node: &DataFoldNode,
) -> HandlerResult<TransformStatsResponse> {
    let processor = OperationProcessor::new(node.clone());

    match processor.get_transform_statistics().await {
        Ok(stats) => {
            let stats_json = serde_json::to_value(&stats).unwrap_or(serde_json::Value::Null);
            Ok(ApiResponse::success_with_user(
                TransformStatsResponse { stats: stats_json },
                user_hash,
            ))
        }
        Err(e) => Err(HandlerError::Internal(format!(
            "Failed to get transform statistics: {}",
            e
        ))),
    }
}

/// Get backfill statistics
pub async fn get_backfill_statistics(
    user_hash: &str,
    node: &DataFoldNode,
) -> HandlerResult<BackfillStatsResponse> {
    let processor = OperationProcessor::new(node.clone());

    match processor.get_backfill_statistics().await {
        Ok(stats) => {
            let stats_json = serde_json::to_value(&stats).unwrap_or(serde_json::Value::Null);
            Ok(ApiResponse::success_with_user(
                BackfillStatsResponse { stats: stats_json },
                user_hash,
            ))
        }
        Err(e) => Err(HandlerError::Internal(format!(
            "Failed to get backfill statistics: {}",
            e
        ))),
    }
}
