//! Shared Schema Handlers
//!
//! Framework-agnostic handlers for schema operations.
//! These can be called by both HTTP server routes and Lambda handlers.

use crate::fold_node::node::FoldNode;
use crate::fold_node::OperationProcessor;
use crate::handlers::response::{ApiResponse, HandlerError, HandlerResult, SuccessResponse};
use serde::{Deserialize, Serialize};

#[cfg(feature = "ts-bindings")]
use ts_rs::TS;

/// Response for listing schemas
#[derive(Debug, Clone, Serialize, Deserialize)]
#[cfg_attr(feature = "ts-bindings", derive(TS))]
#[cfg_attr(
    feature = "ts-bindings",
    ts(export, export_to = "src/fold_node/static-react/src/types/")
)]
pub struct SchemaListResponse {
    /// List of schemas with their states
    pub schemas: serde_json::Value,
    /// Total count
    pub count: usize,
}

/// Response for a single schema
#[derive(Debug, Clone, Serialize, Deserialize)]
#[cfg_attr(feature = "ts-bindings", derive(TS))]
#[cfg_attr(
    feature = "ts-bindings",
    ts(export, export_to = "src/fold_node/static-react/src/types/")
)]
pub struct SchemaResponse {
    /// The schema data
    pub schema: serde_json::Value,
}

/// Response for schema load operation
#[derive(Debug, Clone, Serialize, Deserialize)]
#[cfg_attr(feature = "ts-bindings", derive(TS))]
#[cfg_attr(
    feature = "ts-bindings",
    ts(export, export_to = "src/fold_node/static-react/src/types/")
)]
pub struct SchemaLoadResponse {
    /// Number of available schemas found
    pub available_schemas_loaded: usize,
    /// Number successfully loaded to DB
    pub schemas_loaded_to_db: usize,
    /// List of failed schema names
    pub failed_schemas: Vec<String>,
}

/// Response for schema approval
#[derive(Debug, Clone, Serialize, Deserialize)]
#[cfg_attr(feature = "ts-bindings", derive(TS))]
#[cfg_attr(
    feature = "ts-bindings",
    ts(export, export_to = "src/fold_node/static-react/src/types/")
)]
pub struct SchemaApproveResponse {
    /// Backfill hash if transform, null otherwise
    pub backfill_hash: Option<String>,
}


/// List all schemas
pub async fn list_schemas(
    user_hash: &str,
    node: &FoldNode,
) -> HandlerResult<SchemaListResponse> {
    let processor = OperationProcessor::new(node.clone());

    match processor.list_schemas().await {
        Ok(schemas) => {
            let count = schemas.len();
            // Convert to JSON Value
            let schemas_json =
                serde_json::to_value(&schemas).unwrap_or_else(|_| serde_json::Value::Array(vec![]));
            Ok(ApiResponse::success_with_user(
                SchemaListResponse {
                    schemas: schemas_json,
                    count,
                },
                user_hash,
            ))
        }
        Err(e) => Err(HandlerError::Internal(format!(
            "Failed to list schemas: {}",
            e
        ))),
    }
}

/// Get a single schema by name
pub async fn get_schema(
    schema_name: &str,
    user_hash: &str,
    node: &FoldNode,
) -> HandlerResult<SchemaResponse> {
    let processor = OperationProcessor::new(node.clone());

    match processor.get_schema(schema_name).await {
        Ok(Some(schema_with_state)) => {
            // Convert to JSON Value
            let schema_json =
                serde_json::to_value(&schema_with_state).unwrap_or(serde_json::Value::Null);
            Ok(ApiResponse::success_with_user(
                SchemaResponse {
                    schema: schema_json,
                },
                user_hash,
            ))
        }
        Ok(None) => Err(HandlerError::NotFound(format!(
            "Schema not found: {}",
            schema_name
        ))),
        Err(e) => Err(HandlerError::Internal(format!(
            "Failed to get schema: {}",
            e
        ))),
    }
}

/// Approve a schema for queries and mutations
pub async fn approve_schema(
    schema_name: &str,
    user_hash: &str,
    node: &FoldNode,
) -> HandlerResult<SchemaApproveResponse> {
    let processor = OperationProcessor::new(node.clone());

    match processor.approve_schema(schema_name).await {
        Ok(backfill_hash) => Ok(ApiResponse::success_with_user(
            SchemaApproveResponse { backfill_hash },
            user_hash,
        )),
        Err(e) => Err(HandlerError::Internal(format!(
            "Failed to approve schema: {}",
            e
        ))),
    }
}

/// Block a schema from queries and mutations
pub async fn block_schema(
    schema_name: &str,
    user_hash: &str,
    node: &FoldNode,
) -> HandlerResult<SuccessResponse> {
    let processor = OperationProcessor::new(node.clone());

    match processor.block_schema(schema_name).await {
        Ok(_) => Ok(ApiResponse::success_with_user(
            SuccessResponse { success: true, message: None },
            user_hash,
        )),
        Err(e) => Err(HandlerError::Internal(format!(
            "Failed to block schema: {}",
            e
        ))),
    }
}

/// Load schemas from standard directories
pub async fn load_schemas(
    user_hash: &str,
    node: &FoldNode,
) -> HandlerResult<SchemaLoadResponse> {
    let processor = OperationProcessor::new(node.clone());

    match processor.load_schemas().await {
        Ok((available, loaded, failed)) => Ok(ApiResponse::success_with_user(
            SchemaLoadResponse {
                available_schemas_loaded: available,
                schemas_loaded_to_db: loaded,
                failed_schemas: failed,
            },
            user_hash,
        )),
        Err(e) => Err(HandlerError::Internal(format!(
            "Failed to load schemas: {}",
            e
        ))),
    }
}

/// Response for listing keys in a schema
#[derive(Debug, Clone, Serialize, Deserialize)]
#[cfg_attr(feature = "ts-bindings", derive(TS))]
#[cfg_attr(
    feature = "ts-bindings",
    ts(export, export_to = "src/fold_node/static-react/src/types/")
)]
pub struct SchemaKeysResponse {
    /// Keys in this page
    pub keys: Vec<crate::schema::types::KeyValue>,
    /// Total number of keys across all pages
    pub total_count: usize,
}

/// List keys for a schema with pagination
pub async fn list_schema_keys(
    schema_name: &str,
    offset: usize,
    limit: usize,
    user_hash: &str,
    node: &FoldNode,
) -> HandlerResult<SchemaKeysResponse> {
    let processor = OperationProcessor::new(node.clone());

    match processor.list_schema_keys(schema_name, offset, limit).await {
        Ok((keys, total_count)) => Ok(ApiResponse::success_with_user(
            SchemaKeysResponse { keys, total_count },
            user_hash,
        )),
        Err(e) => Err(HandlerError::Internal(format!(
            "Failed to list keys: {}",
            e
        ))),
    }
}

/// Response for backfill status
#[derive(Debug, Clone, Serialize, Deserialize)]
#[cfg_attr(feature = "ts-bindings", derive(TS))]
#[cfg_attr(
    feature = "ts-bindings",
    ts(export, export_to = "src/fold_node/static-react/src/types/")
)]
pub struct BackfillStatusResponse {
    /// Backfill information
    pub backfill: serde_json::Value,
}

/// Get backfill status by hash
pub async fn get_backfill_status(
    backfill_hash: &str,
    user_hash: &str,
    node: &FoldNode,
) -> HandlerResult<BackfillStatusResponse> {
    let processor = OperationProcessor::new(node.clone());

    match processor.get_backfill(backfill_hash).await {
        Ok(Some(info)) => {
            let backfill_json = serde_json::to_value(&info).unwrap_or(serde_json::Value::Null);
            Ok(ApiResponse::success_with_user(
                BackfillStatusResponse {
                    backfill: backfill_json,
                },
                user_hash,
            ))
        }
        Ok(None) => Err(HandlerError::NotFound(format!(
            "Backfill not found: {}",
            backfill_hash
        ))),
        Err(e) => Err(HandlerError::Internal(format!(
            "Failed to get backfill status: {}",
            e
        ))),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_schema_list_response_serialization() {
        let response = SchemaListResponse {
            schemas: serde_json::Value::Array(vec![]),
            count: 0,
        };
        let json = serde_json::to_string(&response).unwrap();
        assert!(json.contains("schemas"));
        assert!(json.contains("count"));
    }
}
