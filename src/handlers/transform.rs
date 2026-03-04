//! Shared Transform Handlers
//!
//! Framework-agnostic handlers for transform and backfill operations.

use crate::fold_node::node::FoldNode;
use crate::fold_node::OperationProcessor;
use crate::handlers::response::{ApiResponse, HandlerError, HandlerResult, IntoHandlerError, SuccessResponse};
use serde::{Deserialize, Serialize};

#[cfg(feature = "ts-bindings")]
use ts_rs::TS;

/// Response for transform list
#[derive(Debug, Clone, Serialize, Deserialize)]
#[cfg_attr(feature = "ts-bindings", derive(TS))]
#[cfg_attr(
    feature = "ts-bindings",
    ts(export, export_to = "src/fold_node/static-react/src/types/")
)]
pub struct TransformListResponse {
    pub transforms: serde_json::Value,
}

/// Response for transform queue
#[derive(Debug, Clone, Serialize, Deserialize)]
#[cfg_attr(feature = "ts-bindings", derive(TS))]
#[cfg_attr(
    feature = "ts-bindings",
    ts(export, export_to = "src/fold_node/static-react/src/types/")
)]
pub struct TransformQueueResponse {
    pub length: usize,
    pub queued_transforms: Vec<String>,
}


/// Response for backfill list
#[derive(Debug, Clone, Serialize, Deserialize)]
#[cfg_attr(feature = "ts-bindings", derive(TS))]
#[cfg_attr(
    feature = "ts-bindings",
    ts(export, export_to = "src/fold_node/static-react/src/types/")
)]
pub struct BackfillListResponse {
    pub backfills: serde_json::Value,
}

/// Response for single backfill
#[derive(Debug, Clone, Serialize, Deserialize)]
#[cfg_attr(feature = "ts-bindings", derive(TS))]
#[cfg_attr(
    feature = "ts-bindings",
    ts(export, export_to = "src/fold_node/static-react/src/types/")
)]
pub struct BackfillResponse {
    pub backfill: serde_json::Value,
}

/// Response for transform statistics
#[derive(Debug, Clone, Serialize, Deserialize)]
#[cfg_attr(feature = "ts-bindings", derive(TS))]
#[cfg_attr(
    feature = "ts-bindings",
    ts(export, export_to = "src/fold_node/static-react/src/types/")
)]
pub struct TransformStatsResponse {
    pub stats: serde_json::Value,
}

/// Response for backfill statistics
#[derive(Debug, Clone, Serialize, Deserialize)]
#[cfg_attr(feature = "ts-bindings", derive(TS))]
#[cfg_attr(
    feature = "ts-bindings",
    ts(export, export_to = "src/fold_node/static-react/src/types/")
)]
pub struct BackfillStatsResponse {
    pub stats: serde_json::Value,
}

/// List all transforms
pub async fn list_transforms(
    user_hash: &str,
    node: &FoldNode,
) -> HandlerResult<TransformListResponse> {
    let map = OperationProcessor::new(node.clone())
        .list_transforms()
        .await
        .handler_err("list transforms")?;
    let transforms_json = serde_json::to_value(&map)
        .unwrap_or_else(|_| serde_json::Value::Object(serde_json::Map::new()));
    Ok(ApiResponse::success_with_user(
        TransformListResponse { transforms: transforms_json },
        user_hash,
    ))
}

/// Get transform queue
pub async fn get_transform_queue(
    user_hash: &str,
    node: &FoldNode,
) -> HandlerResult<TransformQueueResponse> {
    let (length, queued_transforms) = OperationProcessor::new(node.clone())
        .get_transform_queue()
        .await
        .handler_err("get transform queue")?;
    Ok(ApiResponse::success_with_user(
        TransformQueueResponse { length, queued_transforms },
        user_hash,
    ))
}

/// Add to transform queue
pub async fn add_to_transform_queue(
    transform_id: &str,
    user_hash: &str,
    node: &FoldNode,
) -> HandlerResult<SuccessResponse> {
    OperationProcessor::new(node.clone())
        .add_to_transform_queue(transform_id, "manual_api_trigger")
        .await
        .handler_err("add transform to queue")?;
    Ok(ApiResponse::success_with_user(
        SuccessResponse {
            success: true,
            message: Some(format!("Transform '{}' added to queue", transform_id)),
        },
        user_hash,
    ))
}

/// Shared helper for backfill list endpoints.
async fn get_backfills_filtered(
    user_hash: &str,
    node: &FoldNode,
    active_only: bool,
) -> HandlerResult<BackfillListResponse> {
    let processor = OperationProcessor::new(node.clone());
    let backfills = if active_only {
        processor.get_active_backfills().await
    } else {
        processor.get_all_backfills().await
    }
    .handler_err("get backfills")?;

    let backfills_json =
        serde_json::to_value(&backfills).unwrap_or_else(|_| serde_json::Value::Array(vec![]));
    Ok(ApiResponse::success_with_user(
        BackfillListResponse { backfills: backfills_json },
        user_hash,
    ))
}

/// Get all backfills
pub async fn get_all_backfills(
    user_hash: &str,
    node: &FoldNode,
) -> HandlerResult<BackfillListResponse> {
    get_backfills_filtered(user_hash, node, false).await
}

/// Get active backfills
pub async fn get_active_backfills(
    user_hash: &str,
    node: &FoldNode,
) -> HandlerResult<BackfillListResponse> {
    get_backfills_filtered(user_hash, node, true).await
}

/// Get specific backfill
pub async fn get_backfill(
    backfill_id: &str,
    user_hash: &str,
    node: &FoldNode,
) -> HandlerResult<BackfillResponse> {
    let backfill = OperationProcessor::new(node.clone())
        .get_backfill(backfill_id)
        .await
        .handler_err("get backfill")?
        .ok_or_else(|| HandlerError::NotFound(format!("Backfill not found: {}", backfill_id)))?;
    let backfill_json = serde_json::to_value(&backfill).unwrap_or(serde_json::Value::Null);
    Ok(ApiResponse::success_with_user(
        BackfillResponse { backfill: backfill_json },
        user_hash,
    ))
}

/// Get transform statistics
pub async fn get_transform_statistics(
    user_hash: &str,
    node: &FoldNode,
) -> HandlerResult<TransformStatsResponse> {
    let stats = OperationProcessor::new(node.clone())
        .get_transform_statistics()
        .await
        .handler_err("get transform statistics")?;
    let stats_json = serde_json::to_value(&stats).unwrap_or(serde_json::Value::Null);
    Ok(ApiResponse::success_with_user(
        TransformStatsResponse { stats: stats_json },
        user_hash,
    ))
}

/// Get backfill statistics
pub async fn get_backfill_statistics(
    user_hash: &str,
    node: &FoldNode,
) -> HandlerResult<BackfillStatsResponse> {
    let stats = OperationProcessor::new(node.clone())
        .get_backfill_statistics()
        .await
        .handler_err("get backfill statistics")?;
    let stats_json = serde_json::to_value(&stats).unwrap_or(serde_json::Value::Null);
    Ok(ApiResponse::success_with_user(
        BackfillStatsResponse { stats: stats_json },
        user_hash,
    ))
}
