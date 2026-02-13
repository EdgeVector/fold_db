//! Shared LLM Query Handlers
//!
//! Framework-agnostic handlers for LLM query operations.
//! These can be called by both HTTP server routes and Lambda handlers.

use crate::fold_node::llm_query::{types::*, LlmQueryService, SessionManager};
use crate::fold_node::node::FoldNode;
use crate::fold_node::OperationProcessor;
use crate::db_operations::IndexResult;
use crate::fold_db_core::query::records_from_field_map;
use crate::fold_db_core::FoldDB;
use crate::handlers::response::{ApiResponse, HandlerError, HandlerResult};
use crate::schema::field::HashRangeFilter;
use crate::schema::types::Query;
use crate::schema::SchemaWithState;
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
use std::collections::HashMap;

// ============================================================================
// Response Types
// ============================================================================

/// Response for analyze query
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AnalyzeQueryHandlerResponse {
    pub session_id: String,
    pub query_plan: QueryPlan,
}

/// Response for execute query plan
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ExecuteQueryPlanHandlerResponse {
    pub status: QueryExecutionStatus,
    pub backfill_progress: Option<f64>,
    pub results: Option<Vec<Value>>,
    pub summary: Option<String>,
}

/// Response for chat
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ChatHandlerResponse {
    pub answer: String,
    pub context_used: bool,
}

/// Response for analyze followup
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AnalyzeFollowupHandlerResponse {
    pub needs_query: bool,
    pub query: Option<crate::schema::types::Query>,
    pub reasoning: String,
}

/// Response for backfill status
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BackfillStatusHandlerResponse {
    pub status: String,
    pub progress: f64,
    pub total_records: u64,
    pub processed_records: u64,
    pub estimated_completion: Option<String>,
}

/// Response for AI native index query
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AiNativeIndexHandlerResponse {
    pub ai_interpretation: String,
    pub raw_results: Vec<Value>,
    pub query: String,
    pub session_id: String,
}

/// Request for agent query
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AgentQueryHandlerRequest {
    pub query: String,
    pub session_id: Option<String>,
    pub max_iterations: Option<usize>,
}

/// Response for agent query
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AgentQueryHandlerResponse {
    pub answer: String,
    pub tool_calls: Vec<crate::fold_node::llm_query::types::ToolCallRecord>,
    pub session_id: String,
}

// ============================================================================
// Helper Functions
// ============================================================================

/// Generate a backfill hash for a transform schema
async fn generate_backfill_hash_for_transform(
    transform_manager: &crate::transform::manager::TransformManager,
    schema_name: &str,
) -> Option<String> {
    let transforms = match transform_manager.list_transforms() {
        Ok(t) => t,
        Err(e) => {
            log::warn!("Failed to list transforms for {}: {}", schema_name, e);
            return None;
        }
    };

    let transform = match transforms.get(schema_name) {
        Some(t) => t,
        None => {
            log::debug!("Transform {} not found in transform list", schema_name);
            return None;
        }
    };

    // Look up the transform's schema from the database
    let declarative_schema = match transform_manager
        .db_ops
        .get_schema(transform.get_schema_name())
        .await
    {
        Ok(Some(s)) => s,
        Ok(None) => {
            log::warn!("Transform {} schema not found in database", schema_name);
            return None;
        }
        Err(e) => {
            log::warn!("Failed to get schema for transform {}: {}", schema_name, e);
            return None;
        }
    };

    let inputs = declarative_schema.get_inputs();
    let first_input = match inputs.first() {
        Some(i) => i,
        None => {
            log::warn!(
                "Transform {} has no inputs in declarative schema",
                schema_name
            );
            return None;
        }
    };

    let source_schema_name = match first_input.split('.').next() {
        Some(s) => s,
        None => {
            log::warn!("Failed to parse source schema from input: {}", first_input);
            return None;
        }
    };

    Some(
        crate::fold_db_core::infrastructure::backfill_tracker::BackfillTracker::generate_hash(
            schema_name,
            source_schema_name,
        ),
    )
}

// ============================================================================
// Handler Functions
// ============================================================================

/// Analyze a natural language query
///
/// # Arguments
/// * `request` - The analyze query request
/// * `user_hash` - User identifier for isolation
/// * `service` - LLM query service
/// * `session_manager` - Session manager for tracking conversation state
/// * `node` - DataFold node instance
///
/// # Returns
/// * `HandlerResult<AnalyzeQueryHandlerResponse>` - Analysis result with query plan
pub async fn analyze_query(
    request: AnalyzeQueryRequest,
    user_hash: &str,
    service: &LlmQueryService,
    session_manager: &SessionManager,
    node: &FoldNode,
) -> HandlerResult<AnalyzeQueryHandlerResponse> {
    log::info!(
        "AI Query: Analysis request received for user: {}",
        user_hash
    );

    // Get available schemas
    let schemas: Vec<SchemaWithState> = {
        let db_guard = node
            .get_fold_db()
            .await
            .map_err(|e| HandlerError::Internal(format!("Failed to access database: {}", e)))?;
        db_guard
            .schema_manager()
            .get_schemas_with_states()
            .map_err(|e| HandlerError::Internal(format!("Failed to get schemas: {}", e)))?
    };

    // Create or get session
    let session_id = session_manager
        .create_or_get_session(request.session_id.clone(), request.query.clone())
        .map_err(|e| HandlerError::Internal(format!("Failed to create session: {}", e)))?;

    // Analyze query with LLM
    let query_plan = service
        .analyze_query(&request.query, &schemas)
        .await
        .map_err(|e| HandlerError::Internal(format!("Failed to analyze query: {}", e)))?;

    // Store the query plan in session
    session_manager
        .add_message(
            &session_id,
            "assistant".to_string(),
            format!("Query plan: {}", query_plan.reasoning),
        )
        .map_err(|e| HandlerError::Internal(format!("Failed to update session: {}", e)))?;

    Ok(ApiResponse::success_with_user(
        AnalyzeQueryHandlerResponse {
            session_id,
            query_plan,
        },
        user_hash,
    ))
}

/// Execute a query plan
///
/// # Arguments
/// * `request` - The execute query plan request
/// * `user_hash` - User identifier for isolation
/// * `service` - Optional LLM query service (for summarization)
/// * `session_manager` - Session manager for tracking conversation state
/// * `node` - DataFold node instance
///
/// # Returns
/// * `HandlerResult<ExecuteQueryPlanHandlerResponse>` - Execution result
pub async fn execute_query_plan(
    request: ExecuteQueryPlanRequest,
    user_hash: &str,
    service: Option<&LlmQueryService>,
    session_manager: &SessionManager,
    node: &FoldNode,
) -> HandlerResult<ExecuteQueryPlanHandlerResponse> {
    log::info!(
        "AI Query: Execution request received for session: {:?}, user: {}",
        request.session_id,
        user_hash
    );

    let session_id = &request.session_id;
    let query_plan = &request.query_plan;

    // If index schema is needed, create it
    let mut backfill_hash: Option<String> = None;
    if let Some(ref index_schema) = query_plan.index_schema {
        let schema_name = index_schema.name.clone();

        {
            let db_guard = node
                .get_fold_db()
                .await
                .map_err(|e| HandlerError::Internal(format!("Failed to access database: {}", e)))?;

            // Interpret and load the schema from the definition
            let schema = db_guard
                .schema_manager()
                .interpret_declarative_schema(index_schema.clone())
                .await
                .map_err(|e| {
                    HandlerError::Internal(format!("Failed to interpret schema: {}", e))
                })?;

            db_guard
                .schema_manager()
                .load_schema_internal(schema)
                .await
                .map_err(|e| HandlerError::Internal(format!("Failed to load schema: {}", e)))?;

            // Check if this is a transform schema and generate backfill hash if needed
            let is_transform = match db_guard.transform_manager.transform_exists(&schema_name) {
                Ok(exists) => exists,
                Err(e) => {
                    log::warn!("Failed to check if {} is a transform: {}", schema_name, e);
                    false
                }
            };

            if is_transform {
                backfill_hash =
                    generate_backfill_hash_for_transform(&db_guard.transform_manager, &schema_name)
                        .await;
            }

            // Auto-approve the schema
            if let Err(e) = db_guard
                .schema_manager()
                .approve_with_backfill(&schema_name, backfill_hash.clone())
                .await
            {
                return Err(HandlerError::Internal(format!(
                    "Failed to approve schema: {}",
                    e
                )));
            }
        }

        // Update session with created schema
        session_manager
            .set_schema_created(session_id, schema_name)
            .map_err(|e| HandlerError::Internal(format!("Failed to update session: {}", e)))?;

        // If we have a backfill, check its status
        if let Some(ref hash) = backfill_hash {
            let backfill_info = {
                let db_guard = node.get_fold_db().await.map_err(|e| {
                    HandlerError::Internal(format!("Failed to access database: {}", e))
                })?;
                db_guard.get_backfill_tracker().get_backfill_by_hash(hash)
            };

            if let Some(info) = backfill_info {
                let progress = if info.mutations_expected > 0 {
                    info.mutations_completed as f64 / info.mutations_expected as f64
                } else {
                    0.0
                };

                // If not complete, return pending status
                if info.status
                    != crate::fold_db_core::infrastructure::backfill_tracker::BackfillStatus::Completed
                {
                    return Ok(ApiResponse::success_with_user(
                        ExecuteQueryPlanHandlerResponse {
                            status: QueryExecutionStatus::Running,
                            backfill_progress: Some(progress),
                            results: None,
                            summary: None,
                        },
                        user_hash,
                    ));
                }
            }
        }
    }

    // Execute the query
    let processor = OperationProcessor::new(node.clone());
    let results = match processor.execute_query_map(query_plan.query.clone()).await {
        Ok(result_map) => {
            let records_map = records_from_field_map(&result_map);
            records_map
                .into_iter()
                .map(|(key, record)| {
                    json!({"key": key, "fields": record.fields, "metadata": record.metadata})
                })
                .collect::<Vec<Value>>()
        }
        Err(e) => {
            return Err(HandlerError::Internal(format!(
                "Failed to execute query: {}",
                e
            )));
        }
    };

    // Store results in session
    session_manager
        .add_results(session_id, results.clone())
        .map_err(|e| HandlerError::Internal(format!("Failed to store results: {}", e)))?;

    // Get session to access original query
    let original_query = match session_manager.get_session(session_id) {
        Ok(Some(ctx)) => ctx.original_query,
        Ok(None) => {
            return Err(HandlerError::NotFound("Session not found".to_string()));
        }
        Err(e) => {
            return Err(HandlerError::Internal(format!(
                "Failed to get session: {}",
                e
            )));
        }
    };

    // Summarize results with LLM if available
    let summary = if let Some(svc) = service {
        match svc.summarize_results(&original_query, &results).await {
            Ok(s) => Some(s),
            Err(e) => {
                log::warn!("Failed to summarize results: {}", e);
                None
            }
        }
    } else {
        None
    };

    log::info!(
        "AI Query: Execution complete for session {:?}. Found {} results.",
        session_id,
        results.len()
    );

    Ok(ApiResponse::success_with_user(
        ExecuteQueryPlanHandlerResponse {
            status: QueryExecutionStatus::Complete,
            backfill_progress: Some(1.0),
            results: Some(results),
            summary,
        },
        user_hash,
    ))
}

/// Handle chat action - ask a follow-up question about query results
///
/// # Arguments
/// * `request` - The chat request
/// * `user_hash` - User identifier for isolation
/// * `service` - LLM query service
/// * `session_manager` - Session manager for tracking conversation state
/// * `node` - DataFold node instance
///
/// # Returns
/// * `HandlerResult<ChatHandlerResponse>` - Chat response with answer
pub async fn chat(
    request: ChatRequest,
    user_hash: &str,
    service: &LlmQueryService,
    session_manager: &SessionManager,
    node: &FoldNode,
) -> HandlerResult<ChatHandlerResponse> {
    log::info!(
        "AI Query Chat: received for session: {:?}, user: {}",
        request.session_id,
        user_hash
    );

    let session_id = &request.session_id;
    let question = &request.question;

    // Get session context
    let context = match session_manager.get_session(session_id) {
        Ok(Some(ctx)) => ctx,
        Ok(None) => {
            return Err(HandlerError::NotFound("Session not found".to_string()));
        }
        Err(e) => {
            return Err(HandlerError::Internal(format!(
                "Failed to get session: {}",
                e
            )));
        }
    };

    let results = match context.query_results {
        Some(ref r) => r.clone(),
        None => {
            return Err(HandlerError::BadRequest(
                "No query results available in session".to_string(),
            ));
        }
    };

    // Get schemas for analysis
    let schemas: Vec<SchemaWithState> = {
        let db_guard = node
            .get_fold_db()
            .await
            .map_err(|e| HandlerError::Internal(format!("Failed to access database: {}", e)))?;
        db_guard
            .schema_manager()
            .get_schemas_with_states()
            .map_err(|e| HandlerError::Internal(format!("Failed to get schemas: {}", e)))?
    };

    // Analyze if question needs a new query
    let analysis = service
        .analyze_followup_question(&context.original_query, &results, question, &schemas)
        .await
        .map_err(|e| HandlerError::Internal(format!("Failed to analyze followup: {}", e)))?;

    // Answer the question using existing context
    let answer = service
        .answer_question(
            &context.original_query,
            &results,
            &context.conversation_history,
            question,
        )
        .await
        .map_err(|e| HandlerError::Internal(format!("Failed to answer question: {}", e)))?;

    // Update session with conversation
    let _ = session_manager.add_message(session_id, "user".to_string(), question.clone());

    let assistant_message = if analysis.needs_query {
        format!("[Analyzed context: {}]\n\n{}", analysis.reasoning, answer)
    } else {
        answer.clone()
    };

    let _ = session_manager.add_message(
        session_id,
        "assistant".to_string(),
        assistant_message.clone(),
    );

    Ok(ApiResponse::success_with_user(
        ChatHandlerResponse {
            answer: assistant_message,
            context_used: true,
        },
        user_hash,
    ))
}

/// Analyze if a follow-up question can be answered from existing context
///
/// # Arguments
/// * `request` - The chat request containing the question
/// * `user_hash` - User identifier for isolation
/// * `service` - LLM query service
/// * `session_manager` - Session manager for tracking conversation state
/// * `node` - DataFold node instance
///
/// # Returns
/// * `HandlerResult<AnalyzeFollowupHandlerResponse>` - Analysis of whether new query is needed
pub async fn analyze_followup(
    request: ChatRequest,
    user_hash: &str,
    service: &LlmQueryService,
    session_manager: &SessionManager,
    node: &FoldNode,
) -> HandlerResult<AnalyzeFollowupHandlerResponse> {
    log::info!(
        "AI Query Analyze Followup: received for session: {:?}, user: {}",
        request.session_id,
        user_hash
    );

    let session_id = &request.session_id;
    let question = &request.question;

    // Get session context
    let context = match session_manager.get_session(session_id) {
        Ok(Some(ctx)) => ctx,
        Ok(None) => {
            return Err(HandlerError::NotFound("Session not found".to_string()));
        }
        Err(e) => {
            return Err(HandlerError::Internal(format!(
                "Failed to get session: {}",
                e
            )));
        }
    };

    let results = match context.query_results {
        Some(ref r) => r.clone(),
        None => {
            return Err(HandlerError::BadRequest(
                "No query results available in session".to_string(),
            ));
        }
    };

    // Get schemas
    let schemas: Vec<SchemaWithState> = {
        let db_guard = node
            .get_fold_db()
            .await
            .map_err(|e| HandlerError::Internal(format!("Failed to access database: {}", e)))?;
        db_guard
            .schema_manager()
            .get_schemas_with_states()
            .map_err(|e| HandlerError::Internal(format!("Failed to get schemas: {}", e)))?
    };

    // Analyze followup question
    let analysis = service
        .analyze_followup_question(&context.original_query, &results, question, &schemas)
        .await
        .map_err(|e| HandlerError::Internal(format!("Failed to analyze followup: {}", e)))?;

    Ok(ApiResponse::success_with_user(
        AnalyzeFollowupHandlerResponse {
            needs_query: analysis.needs_query,
            query: analysis.query,
            reasoning: analysis.reasoning,
        },
        user_hash,
    ))
}

/// Get backfill status for a transform
///
/// # Arguments
/// * `hash` - The backfill hash
/// * `user_hash` - User identifier for isolation
/// * `node` - DataFold node instance
///
/// # Returns
/// * `HandlerResult<BackfillStatusHandlerResponse>` - Backfill status
pub async fn get_backfill_status(
    hash: &str,
    user_hash: &str,
    node: &FoldNode,
) -> HandlerResult<BackfillStatusHandlerResponse> {
    log::info!(
        "LLM Query Backfill Status: hash={}, user: {}",
        hash,
        user_hash
    );

    let db_guard = node
        .get_fold_db()
        .await
        .map_err(|e| HandlerError::Internal(format!("Failed to access database: {}", e)))?;

    let backfill_info = db_guard.get_backfill_tracker().get_backfill_by_hash(hash);

    match backfill_info {
        Some(info) => {
            let progress = if info.mutations_expected > 0 {
                info.mutations_completed as f64 / info.mutations_expected as f64
            } else {
                0.0
            };

            Ok(ApiResponse::success_with_user(
                BackfillStatusHandlerResponse {
                    status: format!("{:?}", info.status),
                    progress,
                    total_records: info.mutations_expected,
                    processed_records: info.mutations_completed,
                    estimated_completion: None,
                },
                user_hash,
            ))
        }
        None => Err(HandlerError::NotFound("Backfill not found".to_string())),
    }
}

// ============================================================================
// Hydration Helper Functions
// ============================================================================

/// Maximum number of results to hydrate (for performance)
const MAX_HYDRATE_RESULTS: usize = 50;

/// Hydrate index results by fetching actual field values from the database
///
/// This function takes index search results (which only contain references) and
/// fetches the actual field values from the database, populating the `value` field.
///
/// # Arguments
/// * `results` - Vector of IndexResult from native index search
/// * `fold_db` - Reference to FoldDb for querying records
///
/// # Returns
/// * Vector of IndexResult with populated `value` fields
async fn hydrate_index_results(
    mut results: Vec<IndexResult>,
    fold_db: &FoldDB,
) -> Vec<IndexResult> {
    if results.is_empty() {
        return results;
    }

    // For entries sharing the same (schema, field, key_value), keep only the one
    // with the highest total molecule version.  This avoids wasting hydration
    // budget on stale index entries left over from earlier mutations.
    results = keep_highest_molecule_version(results);

    // Limit the number of results to hydrate for performance
    let hydrate_count = results.len().min(MAX_HYDRATE_RESULTS);

    log::debug!(
        "Hydrating {} of {} index results",
        hydrate_count,
        results.len()
    );

    // Group results by schema_name to batch queries
    let mut schema_groups: HashMap<String, Vec<usize>> = HashMap::new();
    for (idx, result) in results.iter().enumerate().take(hydrate_count) {
        schema_groups
            .entry(result.schema_name.clone())
            .or_default()
            .push(idx);
    }

    // For each schema, fetch all needed records in one query
    for (schema_name, indices) in schema_groups {
        // Collect unique keys for this schema
        let mut keys_to_fetch: Vec<(String, String)> = Vec::new();
        let mut key_to_indices: HashMap<String, Vec<usize>> = HashMap::new();

        for idx in &indices {
            let result = &results[*idx];
            let hash = result.key_value.hash.clone().unwrap_or_default();
            let range = result.key_value.range.clone().unwrap_or_default();

            // Create a key identifier for deduplication
            let key_id = format!("{}:{}", hash, range);

            if !key_to_indices.contains_key(&key_id) {
                keys_to_fetch.push((hash, range));
            }
            key_to_indices.entry(key_id).or_default().push(*idx);
        }

        if keys_to_fetch.is_empty() {
            continue;
        }

        // Build a query to fetch all records for this schema
        // Use HashRangeKeys filter if we have multiple keys
        let filter = if keys_to_fetch.len() == 1 {
            let (hash, range) = &keys_to_fetch[0];
            if !hash.is_empty() && !range.is_empty() {
                Some(HashRangeFilter::HashRangeKey {
                    hash: hash.clone(),
                    range: range.clone(),
                })
            } else if !hash.is_empty() {
                Some(HashRangeFilter::HashKey(hash.clone()))
            } else if !range.is_empty() {
                Some(HashRangeFilter::RangePrefix(range.clone()))
            } else {
                None
            }
        } else {
            // Use batch filter for multiple keys
            Some(HashRangeFilter::HashRangeKeys(keys_to_fetch.clone()))
        };

        // Get all field names we need to fetch
        let fields_needed: Vec<String> = indices
            .iter()
            .map(|idx| results[*idx].field.clone())
            .collect::<std::collections::HashSet<_>>()
            .into_iter()
            .collect();

        let query = Query {
            schema_name: schema_name.clone(),
            fields: fields_needed,
            filter,
            as_of: None,
            rehydrate_depth: None,
        };

        // Execute the query
        match fold_db.query_executor.query(query).await {
            Ok(field_results) => {
                // field_results is HashMap<field_name, HashMap<KeyValue, FieldValue>>
                // We need to map back to our results

                for (idx, result) in results.iter_mut().enumerate().take(hydrate_count) {
                    if result.schema_name != schema_name {
                        continue;
                    }

                    // Find the value for this result's field and key
                    if let Some(field_data) = field_results.get(&result.field) {
                        if let Some(field_value) = field_data.get(&result.key_value) {
                            // Extract the actual value from FieldValue
                            result.value = field_value.value.clone();
                            log::trace!(
                                "Hydrated result {}: schema={}, field={}, key={:?}",
                                idx,
                                result.schema_name,
                                result.field,
                                result.key_value
                            );
                        }
                    }
                }
            }
            Err(e) => {
                log::warn!(
                    "Failed to hydrate results for schema {}: {}",
                    schema_name,
                    e
                );
            }
        }
    }

    log::debug!("Hydration complete");
    results
}

/// For entries sharing the same `(schema_name, field, key_value)`, keep only
/// the one whose molecule versions sum to the highest value.
/// Entries without version info are treated as version 0 (oldest).
fn keep_highest_molecule_version(results: Vec<IndexResult>) -> Vec<IndexResult> {
    let mut best: HashMap<(String, String, crate::schema::types::KeyValue), (u64, usize)> =
        HashMap::new();

    for (idx, r) in results.iter().enumerate() {
        let key = (
            r.schema_name.clone(),
            r.field.clone(),
            r.key_value.clone(),
        );
        let highest: u64 = r
            .molecule_versions
            .as_ref()
            .and_then(|m| m.iter().max().copied())
            .unwrap_or(0);

        match best.get(&key) {
            Some(&(existing, _)) if existing >= highest => {}
            _ => {
                best.insert(key, (highest, idx));
            }
        }
    }

    let mut indices: Vec<usize> = best.into_values().map(|(_, idx)| idx).collect();
    indices.sort_unstable();

    let before = results.len();
    let kept: Vec<IndexResult> = indices.into_iter().map(|i| results[i].clone()).collect();

    if kept.len() < before {
        log::debug!(
            "keep_highest_molecule_version: {} → {} results (dropped {} stale)",
            before,
            kept.len(),
            before - kept.len()
        );
    }

    kept
}

/// Execute an AI-native index query workflow
///
/// This handler implements a three-step process:
/// 1. Search the native index for matching entries
/// 2. Hydrate results by fetching actual field values from records
/// 3. Send hydrated results to AI for interpretation
///
/// # Arguments
/// * `request` - The run query request
/// * `user_hash` - User identifier for isolation
/// * `service` - LLM query service
/// * `session_manager` - Session manager for tracking conversation state
/// * `node` - DataFold node instance
///
/// # Returns
/// * `HandlerResult<AiNativeIndexHandlerResponse>` - AI interpretation and raw results
pub async fn ai_native_index_query(
    request: RunQueryRequest,
    user_hash: &str,
    service: &LlmQueryService,
    session_manager: &SessionManager,
    node: &FoldNode,
) -> HandlerResult<AiNativeIndexHandlerResponse> {
    log::info!(
        "AI Native Index Query: received for session: {:?}, user: {}",
        request.session_id,
        user_hash
    );

    // Create or get session
    let session_id = session_manager
        .create_or_get_session(request.session_id.clone(), request.query.clone())
        .map_err(|e| HandlerError::Internal(format!("Failed to create session: {}", e)))?;

    // Get FoldDb for both schema access and hydration queries
    let db_guard = node
        .get_fold_db()
        .await
        .map_err(|e| HandlerError::Internal(format!("Failed to access database: {}", e)))?;

    // Get available schemas
    let schemas: Vec<SchemaWithState> = db_guard
        .schema_manager()
        .get_schemas_with_states()
        .map_err(|e| HandlerError::Internal(format!("Failed to get schemas: {}", e)))?;

    // Get db_ops for native index search
    let db_ops = db_guard.get_db_ops();

    // Step 1: Search the native index
    let search_results = service
        .search_native_index(&request.query, &schemas, &db_ops)
        .await
        .map_err(|e| HandlerError::Internal(format!("Native index search failed: {}", e)))?;

    log::info!(
        "AI Native Index Query: found {} results, hydrating...",
        search_results.len()
    );

    // Step 2: Hydrate results by fetching actual field values
    let hydrated_results = hydrate_index_results(search_results, &db_guard).await;

    log::info!(
        "AI Native Index Query: hydration complete, {} results ready for AI interpretation",
        hydrated_results.len()
    );

    // Step 3: Send hydrated results to AI for interpretation
    let ai_interpretation = service
        .interpret_native_index_results(&request.query, &hydrated_results)
        .await
        .map_err(|e| HandlerError::Internal(format!("AI interpretation failed: {}", e)))?;

    // Store results in session for context tracking
    let results_as_json: Vec<Value> = hydrated_results
        .into_iter()
        .map(|result| serde_json::to_value(result).unwrap_or(json!({})))
        .collect();

    if let Err(e) = session_manager.add_results(&session_id, results_as_json.clone()) {
        log::warn!("Failed to store results in session: {}", e);
    }

    // Add user message to conversation history
    if let Err(e) =
        session_manager.add_message(&session_id, "user".to_string(), request.query.clone())
    {
        log::warn!("Failed to add user message to session: {}", e);
    }

    // Add AI response to conversation history
    if let Err(e) = session_manager.add_message(
        &session_id,
        "assistant".to_string(),
        ai_interpretation.clone(),
    ) {
        log::warn!("Failed to add assistant message to session: {}", e);
    }

    log::info!("AI Native Index Query complete for session: {}", session_id);

    Ok(ApiResponse::success_with_user(
        AiNativeIndexHandlerResponse {
            ai_interpretation,
            raw_results: results_as_json,
            query: request.query,
            session_id,
        },
        user_hash,
    ))
}

/// Execute an agent query - an autonomous LLM agent that can use tools
///
/// The agent iteratively:
/// 1. Analyzes the user's query
/// 2. Calls tools (query, list_schemas, get_schema, search) as needed
/// 3. Returns a final answer when complete
///
/// # Arguments
/// * `request` - The agent query request
/// * `user_hash` - User identifier for isolation
/// * `service` - LLM query service
/// * `session_manager` - Session manager for tracking conversation state
/// * `node` - DataFold node instance
///
/// # Returns
/// * `HandlerResult<AgentQueryHandlerResponse>` - Agent response with answer and tool calls
pub async fn agent_query(
    request: AgentQueryHandlerRequest,
    user_hash: &str,
    service: &LlmQueryService,
    session_manager: &SessionManager,
    node: &FoldNode,
) -> HandlerResult<AgentQueryHandlerResponse> {
    log::info!(
        "Agent Query: received for user: {}, query: {}",
        user_hash,
        &request.query[..request.query.len().min(100)]
    );

    // Create or get session
    let session_id = session_manager
        .create_or_get_session(request.session_id.clone(), request.query.clone())
        .map_err(|e| HandlerError::Internal(format!("Failed to create session: {}", e)))?;

    // Get available schemas
    let schemas: Vec<SchemaWithState> = {
        let db_guard = node
            .get_fold_db()
            .await
            .map_err(|e| HandlerError::Internal(format!("Failed to access database: {}", e)))?;
        db_guard
            .schema_manager()
            .get_schemas_with_states()
            .map_err(|e| HandlerError::Internal(format!("Failed to get schemas: {}", e)))?
    };

    // Default max iterations
    let max_iterations = request.max_iterations.unwrap_or(10);

    // Run the agent
    let (answer, tool_calls) = service
        .run_agent_query(&request.query, &schemas, node, user_hash, max_iterations)
        .await
        .map_err(|e| HandlerError::Internal(format!("Agent query failed: {}", e)))?;

    // Store conversation in session
    if let Err(e) =
        session_manager.add_message(&session_id, "user".to_string(), request.query.clone())
    {
        log::warn!("Failed to add user message to session: {}", e);
    }

    if let Err(e) =
        session_manager.add_message(&session_id, "assistant".to_string(), answer.clone())
    {
        log::warn!("Failed to add assistant message to session: {}", e);
    }

    log::info!(
        "Agent Query complete for session: {}. Made {} tool calls.",
        session_id,
        tool_calls.len()
    );

    Ok(ApiResponse::success_with_user(
        AgentQueryHandlerResponse {
            answer,
            tool_calls,
            session_id,
        },
        user_hash,
    ))
}
