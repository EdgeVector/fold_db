//! Shared Schema Handlers
//!
//! Framework-agnostic handlers for schema operations.
//! Shared between HTTP server routes and Lambda handlers.

use crate::fold_node::node::FoldNode;
use crate::fold_node::OperationProcessor;
use crate::handlers::response::{ApiResponse, HandlerError, HandlerResult, IntoHandlerError, SuccessResponse};
use serde::{Deserialize, Serialize};

#[cfg(feature = "ts-bindings")]
use ts_rs::TS;

#[derive(Debug, Clone, Serialize, Deserialize)]
#[cfg_attr(feature = "ts-bindings", derive(TS))]
#[cfg_attr(feature = "ts-bindings", ts(export, export_to = "src/fold_node/static-react/src/types/"))]
pub struct SchemaListResponse {
    pub schemas: serde_json::Value,
    pub count: usize,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[cfg_attr(feature = "ts-bindings", derive(TS))]
#[cfg_attr(feature = "ts-bindings", ts(export, export_to = "src/fold_node/static-react/src/types/"))]
pub struct SchemaResponse {
    pub schema: serde_json::Value,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[cfg_attr(feature = "ts-bindings", derive(TS))]
#[cfg_attr(feature = "ts-bindings", ts(export, export_to = "src/fold_node/static-react/src/types/"))]
pub struct SchemaLoadResponse {
    /// Number of available schemas found
    pub available_schemas_loaded: usize,
    /// Number successfully loaded to DB
    pub schemas_loaded_to_db: usize,
    pub failed_schemas: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[cfg_attr(feature = "ts-bindings", derive(TS))]
#[cfg_attr(feature = "ts-bindings", ts(export, export_to = "src/fold_node/static-react/src/types/"))]
pub struct SchemaApproveResponse {
    pub backfill_hash: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[cfg_attr(feature = "ts-bindings", derive(TS))]
#[cfg_attr(feature = "ts-bindings", ts(export, export_to = "src/fold_node/static-react/src/types/"))]
pub struct SchemaKeysResponse {
    pub keys: Vec<crate::schema::types::KeyValue>,
    pub total_count: usize,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[cfg_attr(feature = "ts-bindings", derive(TS))]
#[cfg_attr(feature = "ts-bindings", ts(export, export_to = "src/fold_node/static-react/src/types/"))]
pub struct BackfillStatusResponse {
    pub backfill: serde_json::Value,
}

pub async fn list_schemas(user_hash: &str, node: &FoldNode) -> HandlerResult<SchemaListResponse> {
    let schemas = OperationProcessor::new(node.clone())
        .list_schemas()
        .await
        .handler_err("list schemas")?;
    let count = schemas.len();
    let schemas_json =
        serde_json::to_value(&schemas).unwrap_or_else(|_| serde_json::Value::Array(vec![]));
    Ok(ApiResponse::success_with_user(
        SchemaListResponse { schemas: schemas_json, count },
        user_hash,
    ))
}

pub async fn get_schema(
    schema_name: &str,
    user_hash: &str,
    node: &FoldNode,
) -> HandlerResult<SchemaResponse> {
    let schema_with_state = OperationProcessor::new(node.clone())
        .get_schema(schema_name)
        .await
        .handler_err("get schema")?
        .ok_or_else(|| HandlerError::NotFound(format!("Schema not found: {}", schema_name)))?;
    let schema_json = serde_json::to_value(&schema_with_state).unwrap_or(serde_json::Value::Null);
    Ok(ApiResponse::success_with_user(SchemaResponse { schema: schema_json }, user_hash))
}

pub async fn approve_schema(
    schema_name: &str,
    user_hash: &str,
    node: &FoldNode,
) -> HandlerResult<SchemaApproveResponse> {
    let backfill_hash = OperationProcessor::new(node.clone())
        .approve_schema(schema_name)
        .await
        .handler_err("approve schema")?;
    Ok(ApiResponse::success_with_user(SchemaApproveResponse { backfill_hash }, user_hash))
}

pub async fn block_schema(
    schema_name: &str,
    user_hash: &str,
    node: &FoldNode,
) -> HandlerResult<SuccessResponse> {
    OperationProcessor::new(node.clone())
        .block_schema(schema_name)
        .await
        .handler_err("block schema")?;
    Ok(ApiResponse::success_with_user(
        SuccessResponse { success: true, message: None },
        user_hash,
    ))
}

pub async fn load_schemas(user_hash: &str, node: &FoldNode) -> HandlerResult<SchemaLoadResponse> {
    let (available_schemas_loaded, schemas_loaded_to_db, failed_schemas) =
        OperationProcessor::new(node.clone())
            .load_schemas()
            .await
            .handler_err("load schemas")?;
    Ok(ApiResponse::success_with_user(
        SchemaLoadResponse { available_schemas_loaded, schemas_loaded_to_db, failed_schemas },
        user_hash,
    ))
}

pub async fn list_schema_keys(
    schema_name: &str,
    offset: usize,
    limit: usize,
    user_hash: &str,
    node: &FoldNode,
) -> HandlerResult<SchemaKeysResponse> {
    let (keys, total_count) = OperationProcessor::new(node.clone())
        .list_schema_keys(schema_name, offset, limit)
        .await
        .handler_err("list keys")?;
    Ok(ApiResponse::success_with_user(SchemaKeysResponse { keys, total_count }, user_hash))
}

pub async fn get_backfill_status(
    backfill_hash: &str,
    user_hash: &str,
    node: &FoldNode,
) -> HandlerResult<BackfillStatusResponse> {
    let info = OperationProcessor::new(node.clone())
        .get_backfill(backfill_hash)
        .await
        .handler_err("get backfill status")?
        .ok_or_else(|| HandlerError::NotFound(format!("Backfill not found: {}", backfill_hash)))?;
    let backfill = serde_json::to_value(&info).unwrap_or(serde_json::Value::Null);
    Ok(ApiResponse::success_with_user(BackfillStatusResponse { backfill }, user_hash))
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
