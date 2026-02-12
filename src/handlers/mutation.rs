//! Shared Mutation Handlers
//!
//! Framework-agnostic handlers for mutation operations.
//! These can be called by both HTTP server routes and Lambda handlers.

use crate::fold_node::node::FoldNode;
use crate::fold_node::OperationProcessor;
use crate::handlers::response::{ApiResponse, HandlerError, HandlerResult};
use crate::schema::types::key_value::KeyValue;
use crate::schema::types::operations::{Mutation, MutationType};
use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::collections::HashMap;
use std::time::Duration;

#[cfg(feature = "ts-bindings")]
use ts_rs::TS;

/// Default timeout for waiting on background tasks after batch mutations.
const DEFAULT_BACKGROUND_TASK_TIMEOUT: Duration = Duration::from_secs(5);

/// Response for mutation execution
#[derive(Debug, Clone, Serialize, Deserialize)]
#[cfg_attr(feature = "ts-bindings", derive(TS))]
#[cfg_attr(
    feature = "ts-bindings",
    ts(export, export_to = "src/fold_node/static-react/src/types/")
)]
pub struct MutationResponse {
    /// The mutation IDs that were executed
    pub mutation_ids: Vec<String>,
    /// Number of mutations executed
    pub count: usize,
}

/// Single mutation response (for backward compatibility with existing API)
#[derive(Debug, Clone, Serialize, Deserialize)]
#[cfg_attr(feature = "ts-bindings", derive(TS))]
#[cfg_attr(
    feature = "ts-bindings",
    ts(export, export_to = "src/fold_node/static-react/src/types/")
)]
pub struct SingleMutationResponse {
    /// The mutation ID
    pub mutation_id: String,
    /// Success flag
    pub success: bool,
}

/// Execute a single mutation
pub async fn execute_mutation(
    mutation: Mutation,
    user_hash: &str,
    node: &FoldNode,
) -> HandlerResult<MutationResponse> {
    execute_mutations_batch(vec![mutation], user_hash, node).await
}

/// Execute a single mutation from components (used by HTTP server)
pub async fn execute_mutation_from_components(
    schema: String,
    fields_and_values: HashMap<String, Value>,
    key_value: KeyValue,
    mutation_type: MutationType,
    user_hash: &str,
    node: &FoldNode,
) -> HandlerResult<SingleMutationResponse> {
    let processor = OperationProcessor::new(node.clone());

    match processor
        .execute_mutation(schema, fields_and_values, key_value, mutation_type)
        .await
    {
        Ok(mutation_id) => Ok(ApiResponse::success_with_user(
            SingleMutationResponse {
                mutation_id,
                success: true,
            },
            user_hash,
        )),
        Err(e) => Err(HandlerError::Internal(format!("Mutation failed: {}", e))),
    }
}

/// Execute multiple mutations in a batch
pub async fn execute_mutations_batch(
    mutations: Vec<Mutation>,
    user_hash: &str,
    node: &FoldNode,
) -> HandlerResult<MutationResponse> {
    let count = mutations.len();

    match node.mutate_batch(mutations).await {
        Ok(mutation_ids) => {
            // Wait for background tasks (indexing) to complete
            node.wait_for_background_tasks(DEFAULT_BACKGROUND_TASK_TIMEOUT)
                .await;

            Ok(ApiResponse::success_with_user(
                MutationResponse {
                    mutation_ids,
                    count,
                },
                user_hash,
            ))
        }
        Err(e) => Err(HandlerError::Internal(format!("Mutation failed: {}", e))),
    }
}

/// Execute mutations batch from JSON values (used by HTTP server)
pub async fn execute_mutations_batch_from_json(
    mutations_data: Vec<Value>,
    user_hash: &str,
    node: &FoldNode,
) -> HandlerResult<MutationResponse> {
    let processor = OperationProcessor::new(node.clone());
    let count = mutations_data.len();

    match processor.execute_mutations_batch(mutations_data).await {
        Ok(mutation_ids) => Ok(ApiResponse::success_with_user(
            MutationResponse {
                mutation_ids,
                count,
            },
            user_hash,
        )),
        Err(e) => Err(HandlerError::Internal(format!(
            "Batch mutations failed: {}",
            e
        ))),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_mutation_response_serialization() {
        let response = MutationResponse {
            mutation_ids: vec!["id1".to_string()],
            count: 1,
        };
        let json = serde_json::to_string(&response).unwrap();
        assert!(json.contains("mutation_ids"));
        assert!(json.contains("count"));
    }
}
