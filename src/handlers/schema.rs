//! Shared Schema Handlers
//!
//! Framework-agnostic handlers for schema operations.
//! These can be called by both HTTP server routes and Lambda handlers.

use crate::datafold_node::node::DataFoldNode;
use crate::datafold_node::OperationProcessor;
use crate::handlers::response::{ApiResponse, HandlerError, HandlerResult};
use serde::{Deserialize, Serialize};

#[cfg(feature = "ts-bindings")]
use ts_rs::TS;

/// Response for listing schemas
#[derive(Debug, Clone, Serialize, Deserialize)]
#[cfg_attr(feature = "ts-bindings", derive(TS))]
#[cfg_attr(
    feature = "ts-bindings",
    ts(export, export_to = "src/datafold_node/static-react/src/types/")
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
    ts(export, export_to = "src/datafold_node/static-react/src/types/")
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
    ts(export, export_to = "src/datafold_node/static-react/src/types/")
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
    ts(export, export_to = "src/datafold_node/static-react/src/types/")
)]
pub struct SchemaApproveResponse {
    /// Backfill hash if transform, null otherwise
    pub backfill_hash: Option<String>,
}

/// Simple success response
#[derive(Debug, Clone, Serialize, Deserialize)]
#[cfg_attr(feature = "ts-bindings", derive(TS))]
#[cfg_attr(
    feature = "ts-bindings",
    ts(export, export_to = "src/datafold_node/static-react/src/types/")
)]
pub struct SuccessResponse {
    pub success: bool,
}

/// List all schemas
pub async fn list_schemas(
    user_hash: &str,
    node: &DataFoldNode,
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

/// List schemas with auto-load if empty (for ephemeral environments)
pub async fn list_schemas_with_autoload(
    user_hash: &str,
    node: &DataFoldNode,
) -> HandlerResult<SchemaListResponse> {
    let processor = OperationProcessor::new(node.clone());

    let mut schemas = processor
        .list_schemas()
        .await
        .map_err(|e| HandlerError::Internal(format!("Failed to list schemas: {}", e)))?;

    // Auto-load if empty (self-healing for ephemeral environments)
    if schemas.is_empty() {
        log::info!(
            "Schema list is empty for user: {}. Attempting auto-load...",
            user_hash
        );

        if let Ok((_avail, loaded, _failed)) = processor.load_schemas().await {
            log::info!("Auto-loaded {} schemas.", loaded);
            // Refresh list
            schemas = processor.list_schemas().await.map_err(|e| {
                HandlerError::Internal(format!("Failed to list schemas after auto-load: {}", e))
            })?;
        }
    }

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

/// Get a single schema by name
pub async fn get_schema(
    schema_name: &str,
    user_hash: &str,
    node: &DataFoldNode,
) -> HandlerResult<SchemaResponse> {
    let processor = OperationProcessor::new(node.clone());

    match processor.get_schema(schema_name).await {
        Ok(Some(schema_with_state)) => {
            // Convert to JSON Value
            let schema_json = serde_json::to_value(&schema_with_state)
                .unwrap_or(serde_json::Value::Null);
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
    node: &DataFoldNode,
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
    node: &DataFoldNode,
) -> HandlerResult<SuccessResponse> {
    let processor = OperationProcessor::new(node.clone());

    match processor.block_schema(schema_name).await {
        Ok(_) => Ok(ApiResponse::success_with_user(
            SuccessResponse { success: true },
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
    node: &DataFoldNode,
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
