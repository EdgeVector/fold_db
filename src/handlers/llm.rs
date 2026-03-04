//! Shared LLM Query Handlers
//!
//! Framework-agnostic handlers for LLM query operations.
//! These can be called by both HTTP server routes and Lambda handlers.

use crate::fold_node::llm_query::{conversation_store, types::*, LlmQueryService, SessionManager};
use crate::fold_node::node::FoldNode;
use crate::handlers::response::{get_db_guard, ApiResponse, HandlerError, HandlerResult, IntoHandlerError};
use crate::schema::SchemaWithState;
use serde_json::{json, Value};

use super::llm_hydration::hydrate_index_results;
pub use super::llm_types::*;

// ============================================================================
// Shared Helpers
// ============================================================================

/// Retrieve a session context and its query results, or return an appropriate error.
fn get_session_with_results(
    session_manager: &SessionManager,
    session_id: &str,
) -> Result<(SessionContext, Vec<Value>), HandlerError> {
    let context = match session_manager.get_session(session_id) {
        Ok(Some(ctx)) => ctx,
        Ok(None) => return Err(HandlerError::NotFound("Session not found".to_string())),
        Err(e) => return Err(HandlerError::Internal(format!("Failed to get session: {}", e))),
    };

    let results = context
        .query_results
        .clone()
        .ok_or_else(|| HandlerError::BadRequest("No query results available in session".to_string()))?;

    Ok((context, results))
}

/// Fetch all schemas with states from the database.
async fn get_schemas(node: &FoldNode) -> Result<Vec<SchemaWithState>, HandlerError> {
    let db_guard = get_db_guard(node).await?;
    db_guard
        .schema_manager()
        .get_schemas_with_states()
        .handler_err("get schemas")
}

// ============================================================================
// Handler Functions
// ============================================================================

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

    let (context, results) = get_session_with_results(session_manager, session_id)?;
    let schemas = get_schemas(node).await?;

    // Analyze if question needs a new query
    let analysis = service
        .analyze_followup_question(&context.original_query, &results, question, &schemas)
        .await
        .handler_err("analyze followup")?;

    // Answer the question using existing context
    let answer = service
        .answer_question(
            &context.original_query,
            &results,
            &context.conversation_history,
            question,
        )
        .await
        .handler_err("answer question")?;

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

    let (context, results) = get_session_with_results(session_manager, session_id)?;
    let schemas = get_schemas(node).await?;

    // Analyze followup question
    let analysis = service
        .analyze_followup_question(&context.original_query, &results, question, &schemas)
        .await
        .handler_err("analyze followup")?;

    Ok(ApiResponse::success_with_user(
        AnalyzeFollowupHandlerResponse {
            needs_query: analysis.needs_query,
            query: analysis.query,
            reasoning: analysis.reasoning,
        },
        user_hash,
    ))
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
        .handler_err("create session")?;

    // Get FoldDb for both schema access and hydration queries
    let db_guard = get_db_guard(node).await?;

    // Get available schemas
    let schemas: Vec<SchemaWithState> = db_guard
        .schema_manager()
        .get_schemas_with_states()
        .handler_err("get schemas")?;

    let db_ops = db_guard.get_db_ops();

    // Step 1: Search the native index
    let search_results = service
        .search_native_index(&request.query, &schemas, &db_ops)
        .await
        .handler_err("search native index")?;

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
        .handler_err("interpret results")?;

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
        .handler_err("create session")?;

    let schemas = get_schemas(node).await?;

    // Default max iterations
    let max_iterations = request.max_iterations.unwrap_or(10);

    // Run the agent
    let (answer, tool_calls) = service
        .run_agent_query(&request.query, &schemas, node, user_hash, max_iterations)
        .await
        .handler_err("run agent query")?;

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
