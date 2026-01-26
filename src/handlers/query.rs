//! Shared Query Handlers
//!
//! Framework-agnostic handlers for query operations.
//! These can be called by both HTTP server routes and Lambda handlers.

use crate::datafold_node::node::DataFoldNode;
use crate::datafold_node::OperationProcessor;
use crate::handlers::response::{ApiResponse, HandlerError, HandlerResult};
use crate::schema::types::operations::Query;
use serde::{Deserialize, Serialize};

#[cfg(feature = "ts-bindings")]
use ts_rs::TS;

/// Response for query execution
#[derive(Debug, Clone, Serialize, Deserialize)]
#[cfg_attr(feature = "ts-bindings", derive(TS))]
#[cfg_attr(
    feature = "ts-bindings",
    ts(export, export_to = "src/datafold_node/static-react/src/types/")
)]
pub struct QueryResponse {
    /// Query results
    pub results: serde_json::Value,
}

/// Response for native index search
#[derive(Debug, Clone, Serialize, Deserialize)]
#[cfg_attr(feature = "ts-bindings", derive(TS))]
#[cfg_attr(
    feature = "ts-bindings",
    ts(export, export_to = "src/datafold_node/static-react/src/types/")
)]
pub struct IndexSearchResponse {
    /// Search results
    pub results: serde_json::Value,
}

/// Execute a query
pub async fn execute_query(
    query: Query,
    user_hash: &str,
    node: &DataFoldNode,
) -> HandlerResult<QueryResponse> {
    let processor = OperationProcessor::new(node.clone());

    match processor.execute_query_json(query).await {
        Ok(results) => {
            // Convert Vec<Value> to Value::Array
            let results_json = serde_json::Value::Array(results);
            Ok(ApiResponse::success_with_user(
                QueryResponse {
                    results: results_json,
                },
                user_hash,
            ))
        }
        Err(e) => Err(HandlerError::Internal(format!(
            "Query execution failed: {}",
            e
        ))),
    }
}

/// Execute a native index search
pub async fn native_index_search(
    query_string: &str,
    user_hash: &str,
    node: &DataFoldNode,
) -> HandlerResult<IndexSearchResponse> {
    let processor = OperationProcessor::new(node.clone());

    match processor.native_index_search(query_string).await {
        Ok(results) => {
            // Convert results to JSON Value
            let results_json =
                serde_json::to_value(&results).unwrap_or_else(|_| serde_json::Value::Array(vec![]));
            Ok(ApiResponse::success_with_user(
                IndexSearchResponse {
                    results: results_json,
                },
                user_hash,
            ))
        }
        Err(e) => Err(HandlerError::Internal(format!(
            "Index search failed: {}",
            e
        ))),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_query_response_serialization() {
        let response = QueryResponse {
            results: serde_json::json!([]),
        };
        let json = serde_json::to_string(&response).unwrap();
        assert!(json.contains("results"));
    }
}
