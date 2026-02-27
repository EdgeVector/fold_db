//! Shared LLM Query Handlers
//!
//! Framework-agnostic handlers for LLM query operations.
//! These can be called by both HTTP server routes and Lambda handlers.

use crate::fold_node::llm_query::{conversation_store, types::*, LlmQueryService, SessionManager};
use crate::fold_node::node::FoldNode;
use crate::fold_node::OperationProcessor;
use crate::fold_db_core::query::records_from_field_map;
use crate::handlers::response::{ApiResponse, HandlerError, HandlerResult};
use crate::schema::SchemaWithState;
use serde_json::{json, Value};

use super::llm_hydration::{generate_backfill_hash_for_transform, hydrate_index_results};
pub use super::llm_types::*;

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
/// * `node` - FoldDB node instance
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
/// * `node` - FoldDB node instance
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
/// * `node` - FoldDB node instance
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
/// * `node` - FoldDB node instance
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
/// * `node` - FoldDB node instance
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
/// * `node` - FoldDB node instance
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
/// * `node` - FoldDB node instance
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

    // Persist conversation turn to FoldDB in the background
    let save_node = node.clone();
    let save_session = session_id.clone();
    let save_query = request.query.clone();
    let save_answer = answer.clone();
    let save_tools = tool_calls.clone();
    let save_user_hash = user_hash.to_string();
    tokio::spawn(async move {
        crate::logging::core::run_with_user(&save_user_hash, async move {
            conversation_store::save_conversation_turn(
                &save_node,
                save_session,
                save_query,
                save_answer,
                save_tools,
            )
            .await;
        }).await
    });

    Ok(ApiResponse::success_with_user(
        AgentQueryHandlerResponse {
            answer,
            tool_calls,
            session_id,
        },
        user_hash,
    ))
}
