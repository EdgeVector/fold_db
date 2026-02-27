//! Shared Query Handlers
//!
//! Framework-agnostic handlers for query operations.
//! These can be called by both HTTP server routes and Lambda handlers.

use crate::fold_node::node::FoldNode;
use crate::fold_node::OperationProcessor;
use crate::handlers::response::{get_db_guard, ApiResponse, HandlerError, HandlerResult};
use crate::schema::types::operations::Query;
use serde::{Deserialize, Serialize};

#[cfg(feature = "ts-bindings")]
use ts_rs::TS;

/// Response for query execution
#[derive(Debug, Clone, Serialize, Deserialize)]
#[cfg_attr(feature = "ts-bindings", derive(TS))]
#[cfg_attr(
    feature = "ts-bindings",
    ts(export, export_to = "src/fold_node/static-react/src/types/")
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
    ts(export, export_to = "src/fold_node/static-react/src/types/")
)]
pub struct IndexSearchResponse {
    /// Search results
    pub results: serde_json::Value,
}

/// Execute a query
pub async fn execute_query(
    query: Query,
    user_hash: &str,
    node: &FoldNode,
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
    node: &FoldNode,
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

/// Summary of a single mutation event in a molecule's history
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MutationEventSummary {
    pub timestamp: String,
    pub version: u64,
    pub field_key: serde_json::Value,
    pub old_atom_uuid: Option<String>,
    pub new_atom_uuid: String,
}

/// Response for molecule history
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MoleculeHistoryResponse {
    pub molecule_uuid: String,
    pub events: Vec<MutationEventSummary>,
}

/// Response for atom content
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AtomContentResponse {
    pub atom_uuid: String,
    pub content: serde_json::Value,
    pub source_file_name: Option<String>,
    pub created_at: String,
}

/// Get mutation history for a molecule
pub async fn get_molecule_history(
    molecule_uuid: &str,
    user_hash: &str,
    node: &FoldNode,
) -> HandlerResult<MoleculeHistoryResponse> {
    let db_guard = get_db_guard(node).await?;

    let db_ops = db_guard.get_db_ops();

    let events = db_ops
        .get_mutation_events(molecule_uuid)
        .await
        .map_err(|e| HandlerError::Internal(format!("Failed to load history: {}", e)))?;

    let summaries: Vec<MutationEventSummary> = events
        .into_iter()
        .map(|e| MutationEventSummary {
            timestamp: e.timestamp.to_rfc3339(),
            version: e.version,
            field_key: serde_json::to_value(&e.field_key).unwrap_or_default(),
            old_atom_uuid: e.old_atom_uuid,
            new_atom_uuid: e.new_atom_uuid,
        })
        .collect();

    Ok(ApiResponse::success_with_user(
        MoleculeHistoryResponse {
            molecule_uuid: molecule_uuid.to_string(),
            events: summaries,
        },
        user_hash,
    ))
}

/// Get content of a specific atom by UUID
pub async fn get_atom_content(
    atom_uuid: &str,
    user_hash: &str,
    node: &FoldNode,
) -> HandlerResult<AtomContentResponse> {
    let db_guard = get_db_guard(node).await?;

    let db_ops = db_guard.get_db_ops();

    let atom: crate::atom::Atom = db_ops
        .get_item(&format!("atom:{}", atom_uuid))
        .await
        .map_err(|e| HandlerError::Internal(format!("Failed to fetch atom: {}", e)))?
        .ok_or_else(|| HandlerError::NotFound(format!("Atom '{}' not found", atom_uuid)))?;

    Ok(ApiResponse::success_with_user(
        AtomContentResponse {
            atom_uuid: atom_uuid.to_string(),
            content: atom.content().clone(),
            source_file_name: atom.source_file_name().cloned(),
            created_at: atom.created_at().to_rfc3339(),
        },
        user_hash,
    ))
}

/// Response for process results query
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProcessResultsResponse {
    pub results: Vec<crate::fold_node::node::MutationOutcome>,
}

/// Get process results for a given progress_id (actual stored keys from mutations)
pub async fn get_process_results(
    progress_id: &str,
    user_hash: &str,
    node: &FoldNode,
) -> HandlerResult<ProcessResultsResponse> {
    let results = node
        .get_process_results(progress_id)
        .await
        .map_err(|e| {
            HandlerError::Internal(format!("Failed to get process results: {}", e))
        })?;

    Ok(ApiResponse::success_with_user(
        ProcessResultsResponse { results },
        user_hash,
    ))
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
