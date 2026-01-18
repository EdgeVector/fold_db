//! HTTP route handlers for LLM query workflow.

use super::service::LlmQueryService;
use super::session::SessionManager;
use super::types::*;
use crate::datafold_node::OperationProcessor;
use crate::fold_db_core::query::records_from_field_map;
use crate::ingestion::IngestionConfig;
use crate::schema::SchemaState;
use crate::server::http_server::AppState;
use actix_web::{web, HttpResponse, Responder};
use serde_json::{json, Value};
use std::sync::Arc;

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

/// Shared state for LLM query routes
pub struct LlmQueryState {
    pub service: Option<Arc<LlmQueryService>>,
    pub session_manager: Arc<SessionManager>,
}

impl LlmQueryState {
    pub fn new() -> Self {
        let config = IngestionConfig::from_env_allow_empty();
        let service = match LlmQueryService::new(config) {
            Ok(svc) => Some(Arc::new(svc)),
            Err(e) => {
                log::warn!("LLM Query service not available: {}. LLM query endpoints will return errors until configured.", e);
                None
            }
        };
        let session_manager = Arc::new(SessionManager::new());
        Self {
            service,
            session_manager,
        }
    }
}

impl Default for LlmQueryState {
    fn default() -> Self {
        Self::new()
    }
}

/// Analyze a natural language query
#[utoipa::path(
    post,
    path = "/api/llm-query/analyze",
    tag = "llm-query",
    request_body = AnalyzeQueryRequest,
    responses(
        (status = 200, description = "Query analysis result", body = AnalyzeQueryResponse),
        (status = 400, description = "Bad request"),
        (status = 500, description = "Server error")
    )
)]
pub async fn analyze_query(
    request: web::Json<AnalyzeQueryRequest>,
    app_state: web::Data<AppState>,
    llm_state: web::Data<LlmQueryState>,
) -> impl Responder {
    // Get available schemas
    let schemas: Vec<crate::schema::SchemaWithState> = {
        let node = app_state.node.read().await;
        let db_guard = match node.get_fold_db().await {
            Ok(guard) => guard,
            Err(e) => {
                return HttpResponse::InternalServerError()
                    .json(json!({"error": format!("Failed to access database: {}", e)}));
            }
        };
        match db_guard.schema_manager.get_schemas_with_states() {
            Ok(schemas) => schemas,
            Err(e) => {
                return HttpResponse::InternalServerError()
                    .json(json!({"error": format!("Failed to get schemas: {}", e)}));
            }
        }
    };

    // Create or get session
    let session_id = match llm_state
        .session_manager
        .create_or_get_session(request.session_id.clone(), request.query.clone())
    {
        Ok(id) => id,
        Err(e) => {
            return HttpResponse::InternalServerError()
                .json(json!({"error": format!("Failed to create session: {}", e)}));
        }
    };

    // Check if LLM service is available
    let service = match &llm_state.service {
        Some(svc) => svc,
        None => {
            return HttpResponse::ServiceUnavailable()
                .json(json!({
                    "error": "LLM Query service not configured",
                    "message": "Please configure AI_PROVIDER and FOLD_OPENROUTER_API_KEY or OLLAMA_BASE_URL environment variables to use this feature"
                }));
        }
    };

    // Analyze query with LLM
    let query_plan = match service.analyze_query(&request.query, &schemas).await {
        Ok(plan) => plan,
        Err(e) => {
            return HttpResponse::InternalServerError()
                .json(json!({"error": format!("Failed to analyze query: {}", e)}));
        }
    };

    // Store the query plan in session
    if let Err(e) = llm_state.session_manager.add_message(
        &session_id,
        "assistant".to_string(),
        format!("Query plan: {}", query_plan.reasoning),
    ) {
        return HttpResponse::InternalServerError()
            .json(json!({"error": format!("Failed to update session: {}", e)}));
    }

    HttpResponse::Ok().json(AnalyzeQueryResponse {
        session_id,
        query_plan,
    })
}

/// Execute a query plan
#[utoipa::path(
    post,
    path = "/api/llm-query/execute",
    tag = "llm-query",
    request_body = ExecuteQueryPlanRequest,
    responses(
        (status = 200, description = "Query execution result", body = ExecuteQueryPlanResponse),
        (status = 400, description = "Bad request"),
        (status = 500, description = "Server error")
    )
)]
pub async fn execute_query_plan(
    request: web::Json<ExecuteQueryPlanRequest>,
    app_state: web::Data<AppState>,
    llm_state: web::Data<LlmQueryState>,
) -> impl Responder {
    let session_id = &request.session_id;
    let query_plan = &request.query_plan;

    // If index schema is needed, create it
    let mut backfill_hash: Option<String> = None;
    if let Some(ref index_schema) = query_plan.index_schema {
        // Load the schema
        let schema_name = index_schema.name.clone();
        {
            let node = app_state.node.read().await;
            let db_guard = match node.get_fold_db().await {
                Ok(guard) => guard,
                Err(e) => {
                    return HttpResponse::InternalServerError()
                        .json(json!({"error": format!("Failed to access database: {}", e)}));
                }
            };

            // Interpret and load the schema from the definition
            let schema = match db_guard
                .schema_manager
                .interpret_declarative_schema(index_schema.clone())
                .await
            {
                Ok(s) => s,
                Err(e) => {
                    return HttpResponse::InternalServerError()
                        .json(json!({"error": format!("Failed to interpret schema: {}", e)}));
                }
            };

            if let Err(e) = db_guard.schema_manager.load_schema_internal(schema).await {
                return HttpResponse::InternalServerError()
                    .json(json!({"error": format!("Failed to load schema: {}", e)}));
            }

            // Check if this is a transform schema and generate backfill hash if needed
            let is_transform = match db_guard.transform_manager.transform_exists(&schema_name) {
                Ok(exists) => exists,
                Err(e) => {
                    log::warn!("Failed to check if {} is a transform: {}", schema_name, e);
                    false
                }
            };

            if is_transform {
                // Generate backfill hash for transform
                backfill_hash =
                    generate_backfill_hash_for_transform(&db_guard.transform_manager, &schema_name)
                        .await;
            }

            // Auto-approve the schema (idempotent - only approves if not already approved)
            if let Err(e) = db_guard
                .schema_manager
                .approve_with_backfill(&schema_name, backfill_hash.clone())
                .await
            {
                return HttpResponse::InternalServerError()
                    .json(json!({"error": format!("Failed to approve schema: {}", e)}));
            }
        }

        // Update session with created schema
        if let Err(e) = llm_state
            .session_manager
            .set_schema_created(session_id, schema_name)
        {
            return HttpResponse::InternalServerError()
                .json(json!({"error": format!("Failed to update session: {}", e)}));
        }

        // If we have a backfill, check its status
        if let Some(ref hash) = backfill_hash {
            let backfill_info = {
                let node = app_state.node.read().await;
                let db_guard = match node.get_fold_db().await {
                    Ok(guard) => guard,
                    Err(e) => {
                        return HttpResponse::InternalServerError()
                            .json(json!({"error": format!("Failed to access database: {}", e)}));
                    }
                };
                db_guard.get_backfill_tracker().get_backfill_by_hash(hash)
            };

            if let Some(info) = backfill_info {
                let progress = if info.mutations_expected > 0 {
                    info.mutations_completed as f64 / info.mutations_expected as f64
                } else {
                    0.0
                };

                // If not complete, return pending status
                if info.status != crate::fold_db_core::infrastructure::backfill_tracker::BackfillStatus::Completed {
                    return HttpResponse::Ok().json(ExecuteQueryPlanResponse {
                        status: QueryExecutionStatus::Running,
                        backfill_progress: Some(progress),
                        results: None,
                        summary: None,
                    });
                }
            }
        }
    }

    // Execute the query
    let node_arc = Arc::clone(&app_state.node);
    let processor = OperationProcessor::new(node_arc.read().await.clone());
    let results = match processor.execute_query_map(query_plan.query.clone()).await {
        Ok(result_map) => {
            let records_map = records_from_field_map(&result_map);
            records_map
                .into_iter()
                .map(|(key, record)| json!({"key": key, "fields": record.fields, "metadata": record.metadata}))
                .collect::<Vec<Value>>()
        }
        Err(e) => {
            return HttpResponse::InternalServerError()
                .json(json!({"error": format!("Failed to execute query: {}", e)}));
        }
    };

    // Store results in session
    if let Err(e) = llm_state
        .session_manager
        .add_results(session_id, results.clone())
    {
        return HttpResponse::InternalServerError()
            .json(json!({"error": format!("Failed to store results: {}", e)}));
    }

    // Get session to access original query
    let original_query = match llm_state.session_manager.get_session(session_id) {
        Ok(Some(ctx)) => ctx.original_query,
        Ok(None) => {
            return HttpResponse::NotFound().json(json!({"error": "Session not found"}));
        }
        Err(e) => {
            return HttpResponse::InternalServerError()
                .json(json!({"error": format!("Failed to get session: {}", e)}));
        }
    };

    // Summarize results with LLM if available
    let summary = if let Some(ref service) = llm_state.service {
        match service.summarize_results(&original_query, &results).await {
            Ok(s) => Some(s),
            Err(e) => {
                log::warn!("Failed to summarize results: {}", e);
                None
            }
        }
    } else {
        None
    };

    HttpResponse::Ok().json(ExecuteQueryPlanResponse {
        status: QueryExecutionStatus::Complete,
        backfill_progress: Some(1.0),
        results: Some(results),
        summary,
    })
}

/// Analyze if a follow-up question can be answered from existing context
#[utoipa::path(
    post,
    path = "/api/llm-query/analyze-followup",
    tag = "llm-query",
    request_body = ChatRequest,
    responses(
        (status = 200, description = "Follow-up analysis result", body = FollowupAnalysis),
        (status = 404, description = "Session not found"),
        (status = 500, description = "Server error")
    )
)]
pub async fn analyze_followup(
    request: web::Json<ChatRequest>,
    app_state: web::Data<AppState>,
    llm_state: web::Data<LlmQueryState>,
) -> impl Responder {
    let session_id = &request.session_id;
    let question = &request.question;

    let context = match llm_state.session_manager.get_session(session_id) {
        Ok(Some(ctx)) => ctx,
        Ok(None) => {
            return HttpResponse::NotFound().json(json!({"error": "Session not found"}));
        }
        Err(e) => {
            return HttpResponse::InternalServerError()
                .json(json!({"error": format!("Failed to get session: {}", e)}));
        }
    };

    let results = match context.query_results {
        Some(ref r) => r,
        None => {
            return HttpResponse::BadRequest()
                .json(json!({"error": "No query results available in session"}));
        }
    };

    let service = match &llm_state.service {
        Some(svc) => svc,
        None => {
            return HttpResponse::ServiceUnavailable()
                .json(json!({
                    "error": "LLM Query service not configured",
                    "message": "Please configure AI_PROVIDER and FOLD_OPENROUTER_API_KEY or OLLAMA_BASE_URL environment variables to use this feature"
                }));
        }
    };

    let schemas: Vec<crate::schema::SchemaWithState> = {
        let node = app_state.node.read().await;
        let db_guard = match node.get_fold_db().await {
            Ok(guard) => guard,
            Err(e) => {
                return HttpResponse::InternalServerError()
                    .json(json!({"error": format!("Failed to access database: {}", e)}));
            }
        };
        match db_guard.schema_manager.get_schemas_with_states() {
            Ok(schemas) => schemas,
            Err(e) => {
                return HttpResponse::InternalServerError()
                    .json(json!({"error": format!("Failed to get schemas: {}", e)}));
            }
        }
    };

    let analysis = match service
        .analyze_followup_question(&context.original_query, results, question, &schemas)
        .await
    {
        Ok(a) => a,
        Err(e) => {
            return HttpResponse::InternalServerError()
                .json(json!({"error": format!("Failed to analyze followup question: {}", e)}));
        }
    };

    HttpResponse::Ok().json(analysis)
}

/// Ask a follow-up question about query results
#[utoipa::path(
    post,
    path = "/api/llm-query/chat",
    tag = "llm-query",
    request_body = ChatRequest,
    responses(
        (status = 200, description = "Answer to question", body = ChatResponse),
        (status = 404, description = "Session not found"),
        (status = 500, description = "Server error")
    )
)]
pub async fn chat(
    request: web::Json<ChatRequest>,
    app_state: web::Data<AppState>,
    llm_state: web::Data<LlmQueryState>,
) -> impl Responder {
    let session_id = &request.session_id;
    let question = &request.question;

    let context = match llm_state.session_manager.get_session(session_id) {
        Ok(Some(ctx)) => ctx,
        Ok(None) => {
            return HttpResponse::NotFound().json(json!({"error": "Session not found"}));
        }
        Err(e) => {
            return HttpResponse::InternalServerError()
                .json(json!({"error": format!("Failed to get session: {}", e)}));
        }
    };

    let results = match context.query_results {
        Some(ref r) => r,
        None => {
            return HttpResponse::BadRequest()
                .json(json!({"error": "No query results available in session"}));
        }
    };

    let service = match &llm_state.service {
        Some(svc) => svc,
        None => {
            return HttpResponse::ServiceUnavailable()
                .json(json!({
                    "error": "LLM Query service not configured",
                    "message": "Please configure AI_PROVIDER and FOLD_OPENROUTER_API_KEY or OLLAMA_BASE_URL environment variables to use this feature"
                }));
        }
    };

    let schemas: Vec<crate::schema::SchemaWithState> = {
        let node = app_state.node.read().await;
        let db_guard = match node.get_fold_db().await {
            Ok(guard) => guard,
            Err(e) => {
                return HttpResponse::InternalServerError()
                    .json(json!({"error": format!("Failed to access database: {}", e)}));
            }
        };
        match db_guard.schema_manager.get_schemas_with_states() {
            Ok(schemas) => schemas,
            Err(e) => {
                return HttpResponse::InternalServerError()
                    .json(json!({"error": format!("Failed to get schemas: {}", e)}));
            }
        }
    };

    let analysis = match service
        .analyze_followup_question(&context.original_query, results, question, &schemas)
        .await
    {
        Ok(a) => a,
        Err(e) => {
            return HttpResponse::InternalServerError()
                .json(json!({"error": format!("Failed to analyze followup question: {}", e)}));
        }
    };

    let mut combined_results = results.clone();
    let mut executed_query = false;
    let mut retry_info: Option<String> = None;

    if analysis.needs_query {
        if let Some(ref initial_query) = analysis.query {
            executed_query = true;

            let mut current_query = initial_query.clone();
            let mut attempts: Vec<String> = Vec::new();
            const MAX_FOLLOWUP_ATTEMPTS: usize = 3;

            for attempt in 0..MAX_FOLLOWUP_ATTEMPTS {
                let node_arc = Arc::clone(&app_state.node);
                let processor = OperationProcessor::new(node_arc.read().await.clone());
                match processor.execute_query_map(current_query.clone()).await {
                    Ok(result_map) => {
                        let records_map = records_from_field_map(&result_map);
                        let new_results: Vec<Value> = records_map
                            .into_iter()
                            .map(|(key, record)| json!({"key": key, "fields": record.fields, "metadata": record.metadata}))
                            .collect();

                        if !new_results.is_empty() {
                            if attempt > 0 {
                                retry_info = Some(format!(
                                    "Found results using alternative strategy after {} attempts",
                                    attempt + 1
                                ));
                            }
                            combined_results = new_results;
                            break;
                        }

                        attempts.push(format!(
                            "Schema: {}, Filter: {:?}",
                            current_query.schema_name, current_query.filter
                        ));

                        if attempt < MAX_FOLLOWUP_ATTEMPTS - 1 {
                            match service
                                .suggest_alternative_query(
                                    question,
                                    &current_query,
                                    &schemas,
                                    &attempts,
                                )
                                .await
                            {
                                Ok(Some(alternative_plan)) => {
                                    current_query = alternative_plan.query;
                                }
                                Ok(None) => {
                                    retry_info = Some(format!(
                                        "No results found after trying {} approaches",
                                        attempt + 1
                                    ));
                                    break;
                                }
                                Err(e) => {
                                    log::warn!(
                                        "Failed to generate alternative for follow-up: {}",
                                        e
                                    );
                                    break;
                                }
                            }
                        } else {
                            retry_info = Some(format!(
                                "No results found after trying {} approaches",
                                MAX_FOLLOWUP_ATTEMPTS
                            ));
                        }
                    }
                    Err(e) => {
                        log::warn!("Failed to execute followup query: {}", e);
                        break;
                    }
                }
            }
        }
    }

    let answer = match service
        .answer_question(
            &context.original_query,
            &combined_results,
            &context.conversation_history,
            question,
        )
        .await
    {
        Ok(a) => a,
        Err(e) => {
            return HttpResponse::InternalServerError()
                .json(json!({"error": format!("Failed to get answer: {}", e)}));
        }
    };

    if let Err(e) =
        llm_state
            .session_manager
            .add_message(session_id, "user".to_string(), question.clone())
    {
        return HttpResponse::InternalServerError()
            .json(json!({"error": format!("Failed to update session: {}", e)}));
    }

    let mut assistant_message = if executed_query {
        format!("[Executed new query: {}]\n\n{}", analysis.reasoning, answer)
    } else {
        answer.clone()
    };

    if let Some(info) = retry_info {
        assistant_message.push_str(&format!("\n\n[Note: {}]", info));
    }

    if let Err(e) = llm_state.session_manager.add_message(
        session_id,
        "assistant".to_string(),
        assistant_message.clone(),
    ) {
        return HttpResponse::InternalServerError()
            .json(json!({"error": format!("Failed to update session: {}", e)}));
    }

    HttpResponse::Ok().json(ChatResponse {
        answer: assistant_message,
        context_used: true,
    })
}

/// Get backfill status for a transform
#[utoipa::path(
    get,
    path = "/api/llm-query/backfill/{hash}",
    tag = "llm-query",
    params(
        ("hash" = String, Path, description = "Backfill hash")
    ),
    responses(
        (status = 200, description = "Backfill status", body = BackfillStatusResponse),
        (status = 404, description = "Backfill not found"),
        (status = 500, description = "Server error")
    )
)]
pub async fn get_backfill_status(
    path: web::Path<String>,
    app_state: web::Data<AppState>,
) -> impl Responder {
    let backfill_hash = path.into_inner();

    let backfill_info = {
        let node = app_state.node.read().await;
        let db_guard = match node.get_fold_db().await {
            Ok(guard) => guard,
            Err(e) => {
                return HttpResponse::InternalServerError()
                    .json(json!({"error": format!("Failed to access database: {}", e)}));
            }
        };
        db_guard
            .get_backfill_tracker()
            .get_backfill_by_hash(&backfill_hash)
    };

    match backfill_info {
        Some(info) => {
            let progress = if info.mutations_expected > 0 {
                info.mutations_completed as f64 / info.mutations_expected as f64
            } else {
                0.0
            };

            HttpResponse::Ok().json(BackfillStatusResponse {
                status: format!("{:?}", info.status),
                progress,
                total_records: info.mutations_expected,
                processed_records: info.mutations_completed,
                estimated_completion: None,
            })
        }
        None => HttpResponse::NotFound().json(json!({"error": "Backfill not found"})),
    }
}

/// Single-step query execution: analyze, create index, wait for backfill, execute, and summarize
#[utoipa::path(
    post,
    path = "/api/llm-query/run",
    tag = "llm-query",
    request_body = RunQueryRequest,
    responses(
        (status = 200, description = "Query execution complete", body = RunQueryResponse),
        (status = 400, description = "Bad request"),
        (status = 500, description = "Server error")
    )
)]
pub async fn run_query(
    request: web::Json<RunQueryRequest>,
    app_state: web::Data<AppState>,
    llm_state: web::Data<LlmQueryState>,
) -> impl Responder {
    let service = match &llm_state.service {
        Some(svc) => svc,
        None => {
            return HttpResponse::ServiceUnavailable()
                .json(json!({
                    "error": "LLM Query service not configured",
                    "message": "Please configure AI_PROVIDER and FOLD_OPENROUTER_API_KEY or OLLAMA_BASE_URL environment variables to use this feature"
                }));
        }
    };

    let schemas: Vec<crate::schema::SchemaWithState> = {
        let node = app_state.node.read().await;
        let db_guard = match node.get_fold_db().await {
            Ok(guard) => guard,
            Err(e) => {
                return HttpResponse::InternalServerError()
                    .json(json!({"error": format!("Failed to access database: {}", e)}));
            }
        };
        match db_guard.schema_manager.get_schemas_with_states() {
            Ok(schemas) => schemas,
            Err(e) => {
                return HttpResponse::InternalServerError()
                    .json(json!({"error": format!("Failed to get schemas: {}", e)}));
            }
        }
    };

    let session_id = match llm_state
        .session_manager
        .create_or_get_session(request.session_id.clone(), request.query.clone())
    {
        Ok(id) => id,
        Err(e) => {
            return HttpResponse::InternalServerError()
                .json(json!({"error": format!("Failed to create session: {}", e)}));
        }
    };

    let query_plan = match service.analyze_query(&request.query, &schemas).await {
        Ok(plan) => plan,
        Err(e) => {
            return HttpResponse::InternalServerError()
                .json(json!({"error": format!("Failed to analyze query: {}", e)}));
        }
    };

    if let Err(e) = llm_state.session_manager.add_message(
        &session_id,
        "assistant".to_string(),
        format!("Query plan: {}", query_plan.reasoning),
    ) {
        return HttpResponse::InternalServerError()
            .json(json!({"error": format!("Failed to update session: {}", e)}));
    }

    let mut backfill_hash: Option<String> = None;
    if let Some(ref index_schema) = query_plan.index_schema {
        let schema_name = index_schema.name.clone();
        {
            let node = app_state.node.read().await;
            let db_guard = match node.get_fold_db().await {
                Ok(guard) => guard,
                Err(e) => {
                    return HttpResponse::InternalServerError()
                        .json(json!({"error": format!("Failed to access database: {}", e)}));
                }
            };

            let schema = match db_guard
                .schema_manager
                .interpret_declarative_schema(index_schema.clone())
                .await
            {
                Ok(s) => s,
                Err(e) => {
                    return HttpResponse::InternalServerError()
                        .json(json!({"error": format!("Failed to interpret schema: {}", e)}));
                }
            };

            if let Err(e) = db_guard.schema_manager.load_schema_internal(schema).await {
                return HttpResponse::InternalServerError()
                    .json(json!({"error": format!("Failed to load schema: {}", e)}));
            }

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

            let current_state = match db_guard.schema_manager.get_schema_states() {
                Ok(states) => states.get(&schema_name).copied().unwrap_or_default(),
                Err(e) => {
                    return HttpResponse::InternalServerError()
                        .json(json!({"error": format!("Failed to get schema states: {}", e)}));
                }
            };

            if current_state != SchemaState::Approved {
                if let Err(e) = db_guard
                    .schema_manager
                    .set_schema_state_with_backfill(
                        &schema_name,
                        SchemaState::Approved,
                        backfill_hash.clone(),
                    )
                    .await
                {
                    return HttpResponse::InternalServerError()
                        .json(json!({"error": format!("Failed to approve schema: {}", e)}));
                }
            }
        }

        if let Err(e) = llm_state
            .session_manager
            .set_schema_created(&session_id, schema_name)
        {
            return HttpResponse::InternalServerError()
                .json(json!({"error": format!("Failed to update session: {}", e)}));
        }

        if let Some(ref hash) = backfill_hash {
            loop {
                let backfill_info = {
                    let node = app_state.node.read().await;
                    let db_guard = match node.get_fold_db().await {
                        Ok(guard) => guard,
                        Err(e) => {
                            return HttpResponse::InternalServerError().json(
                                json!({"error": format!("Failed to access database: {}", e)}),
                            );
                        }
                    };
                    db_guard.get_backfill_tracker().get_backfill_by_hash(hash)
                };

                if let Some(info) = backfill_info {
                    if info.status == crate::fold_db_core::infrastructure::backfill_tracker::BackfillStatus::Completed {
                        break;
                    }
                    if info.status == crate::fold_db_core::infrastructure::backfill_tracker::BackfillStatus::Failed {
                        return HttpResponse::InternalServerError()
                            .json(json!({"error": "Backfill failed"}));
                    }
                } else {
                    break;
                }

                tokio::time::sleep(tokio::time::Duration::from_millis(500)).await;
            }
        }
    }

    let mut current_query_plan = query_plan.clone();
    let mut results: Vec<Value> = Vec::new();
    let mut attempts: Vec<String> = Vec::new();
    const MAX_ATTEMPTS: usize = 5;

    for attempt in 0..MAX_ATTEMPTS {
        let node_arc = Arc::clone(&app_state.node);
        let processor = OperationProcessor::new(node_arc.read().await.clone());
        match processor
            .execute_query_map(current_query_plan.query.clone())
            .await
        {
            Ok(result_map) => {
                let records_map = records_from_field_map(&result_map);
                results = records_map
                    .into_iter()
                    .map(|(key, record)| json!({"key": key, "fields": record.fields, "metadata": record.metadata}))
                    .collect();

                if !results.is_empty() {
                    break;
                }

                attempts.push(format!(
                    "Schema: {}, Filter: {:?} - {}",
                    current_query_plan.query.schema_name,
                    current_query_plan.query.filter,
                    current_query_plan.reasoning
                ));

                if attempt < MAX_ATTEMPTS - 1 {
                    match service
                        .suggest_alternative_query(
                            &request.query,
                            &current_query_plan.query,
                            &schemas,
                            &attempts,
                        )
                        .await
                    {
                        Ok(Some(alternative_plan)) => {
                            current_query_plan = alternative_plan;
                        }
                        Ok(None) => {
                            break;
                        }
                        Err(e) => {
                            log::warn!("Failed to generate alternative query: {}", e);
                            break;
                        }
                    }
                }
            }
            Err(e) => {
                return HttpResponse::InternalServerError()
                    .json(json!({"error": format!("Failed to execute query: {}", e)}));
            }
        }
    }

    if let Err(e) = llm_state
        .session_manager
        .add_results(&session_id, results.clone())
    {
        return HttpResponse::InternalServerError()
            .json(json!({"error": format!("Failed to store results: {}", e)}));
    }

    let original_query = match llm_state.session_manager.get_session(&session_id) {
        Ok(Some(ctx)) => ctx.original_query,
        Ok(None) => {
            return HttpResponse::NotFound().json(json!({"error": "Session not found"}));
        }
        Err(e) => {
            return HttpResponse::InternalServerError()
                .json(json!({"error": format!("Failed to get session: {}", e)}));
        }
    };

    let mut summary_text = match service.summarize_results(&original_query, &results).await {
        Ok(s) => s,
        Err(e) => {
            log::warn!("Failed to summarize results: {}", e);
            String::new()
        }
    };

    if !attempts.is_empty() {
        let retry_info = if results.is_empty() {
            format!(
                "\n\n[Note: No results found after trying {} different approaches: {}]",
                attempts.len() + 1,
                attempts.join("; ")
            )
        } else {
            format!(
                "\n\n[Note: Found results using alternative strategy after {} attempts. Final approach: {}]",
                attempts.len() + 1,
                current_query_plan.reasoning
            )
        };
        summary_text.push_str(&retry_info);
    }

    let final_summary = if summary_text.is_empty() {
        None
    } else {
        Some(summary_text)
    };

    HttpResponse::Ok().json(RunQueryResponse {
        session_id,
        query_plan: current_query_plan,
        results,
        summary: final_summary,
    })
}

/// Execute an AI-native index query workflow
#[utoipa::path(
    post,
    path = "/api/llm-query/native-index",
    tag = "llm-query",
    request_body = RunQueryRequest,
    responses(
        (status = 200, description = "AI-native index query result", body = String),
        (status = 400, description = "Bad request"),
        (status = 500, description = "Server error")
    )
)]
pub async fn ai_native_index_query(
    request: web::Json<RunQueryRequest>,
    app_state: web::Data<AppState>,
    llm_state: web::Data<LlmQueryState>,
) -> impl Responder {
    let service = match &llm_state.service {
        Some(svc) => svc,
        None => {
            return HttpResponse::ServiceUnavailable()
                .json(json!({
                    "error": "LLM Query service not configured",
                    "message": "Please configure AI_PROVIDER and FOLD_OPENROUTER_API_KEY or OLLAMA_BASE_URL environment variables to use this feature"
                }));
        }
    };

    // Create or get session to maintain conversation context
    let session_id = match llm_state
        .session_manager
        .create_or_get_session(request.session_id.clone(), request.query.clone())
    {
        Ok(id) => id,
        Err(e) => {
            return HttpResponse::InternalServerError()
                .json(json!({"error": format!("Failed to create session: {}", e)}));
        }
    };

    // Get available schemas
    let schemas = {
        let node = app_state.node.read().await;
        let db_guard = match node.get_fold_db().await {
            Ok(guard) => guard,
            Err(e) => {
                return HttpResponse::InternalServerError()
                    .json(json!({"error": format!("Failed to access database: {}", e)}));
            }
        };
        match db_guard.schema_manager.get_schemas_with_states() {
            Ok(schemas) => schemas,
            Err(e) => {
                return HttpResponse::InternalServerError()
                    .json(json!({"error": format!("Failed to get schemas: {}", e)}));
            }
        }
    };

    // Execute AI-native index query workflow
    let result = async {
        let node = app_state.node.read().await;
        let db_ops = match node.get_fold_db().await {
            Ok(guard) => guard.get_db_ops(),
            Err(e) => {
                return Err(format!("Failed to access database: {}", e));
            }
        };
        drop(node); // Drop the mutex guard before await

        // Get both AI interpretation and raw results
        service
            .execute_ai_native_index_query_with_results(&request.query, &schemas, &db_ops)
            .await
    }
    .await;

    match result {
        Ok((ai_interpretation, raw_results)) => {
            // Store results in session for context tracking
            let results_as_json: Vec<serde_json::Value> = raw_results
                .into_iter()
                .map(|result| serde_json::to_value(result).unwrap_or(json!({})))
                .collect();

            if let Err(e) = llm_state
                .session_manager
                .add_results(&session_id, results_as_json.clone())
            {
                log::warn!("Failed to store results in session: {}", e);
            }

            // Add user message to conversation history
            if let Err(e) = llm_state.session_manager.add_message(
                &session_id,
                "user".to_string(),
                request.query.clone(),
            ) {
                log::warn!("Failed to add user message to session: {}", e);
            }

            // Add AI response to conversation history
            if let Err(e) = llm_state.session_manager.add_message(
                &session_id,
                "assistant".to_string(),
                ai_interpretation.clone(),
            ) {
                log::warn!("Failed to add assistant message to session: {}", e);
            }

            HttpResponse::Ok().json(json!({
                "ai_interpretation": ai_interpretation,
                "raw_results": results_as_json,
                "query": request.query,
                "session_id": session_id
            }))
        }
        Err(e) => HttpResponse::InternalServerError()
            .json(json!({"error": format!("AI-native index query failed: {}", e)})),
    }
}
