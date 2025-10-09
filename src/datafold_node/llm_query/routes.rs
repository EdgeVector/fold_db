//! HTTP route handlers for LLM query workflow.

use super::service::LlmQueryService;
use super::session::SessionManager;
use super::types::*;
use crate::datafold_node::http_server::AppState;
use crate::datafold_node::OperationProcessor;
use crate::fold_db_core::query::records_from_field_map;
use crate::ingestion::IngestionConfig;
use crate::schema::SchemaState;
use actix_web::{web, HttpResponse, Responder};
use serde_json::{json, Value};
use std::sync::Arc;

/// Generate a backfill hash for a transform schema
fn generate_backfill_hash_for_transform(
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
    
    let declarative_schema = match transform.get_declarative_schema() {
        Some(s) => s,
        None => {
            log::warn!("Transform {} has no declarative schema", schema_name);
            return None;
        }
    };
    
    let inputs = declarative_schema.get_inputs();
    let first_input = match inputs.first() {
        Some(i) => i,
        None => {
            log::warn!("Transform {} has no inputs in declarative schema", schema_name);
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
    
    Some(crate::fold_db_core::infrastructure::backfill_tracker::BackfillTracker::generate_hash(
        schema_name,
        source_schema_name,
    ))
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
            Ok(svc) => {
                log::info!("LLM Query service initialized successfully");
                Some(Arc::new(svc))
            }
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
    let schemas = {
        let node = app_state.node.lock().await;
        let db_guard = match node.get_fold_db() {
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
    let session_id = match llm_state.session_manager.create_or_get_session(
        request.session_id.clone(),
        request.query.clone(),
    ) {
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
            let node = app_state.node.lock().await;
            let db_guard = match node.get_fold_db() {
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
            {
                Ok(s) => s,
                Err(e) => {
                    return HttpResponse::InternalServerError()
                        .json(json!({"error": format!("Failed to interpret schema: {}", e)}));
                }
            };

            if let Err(e) = db_guard.schema_manager.load_schema_internal(schema) {
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
                backfill_hash = generate_backfill_hash_for_transform(&db_guard.transform_manager, &schema_name);
            }

            // Approve the schema to trigger backfill
            if let Err(e) = db_guard.schema_manager.set_schema_state_with_backfill(
                &schema_name,
                SchemaState::Approved,
                backfill_hash.clone(),
            ) {
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
                let node = app_state.node.lock().await;
                let db_guard = match node.get_fold_db() {
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
    let processor = OperationProcessor::new(node_arc);
    let results = match processor.execute_query_map(query_plan.query.clone()).await {
        Ok(result_map) => {
            let records_map = records_from_field_map(&result_map);
            records_map
                .into_iter()
                .map(|(key, record)| json!({"key": key, "fields": record.fields}))
                .collect::<Vec<Value>>()
        }
        Err(e) => {
            return HttpResponse::InternalServerError()
                .json(json!({"error": format!("Failed to execute query: {}", e)}));
        }
    };

    // Store results in session
    if let Err(e) = llm_state.session_manager.add_results(session_id, results.clone()) {
        return HttpResponse::InternalServerError()
            .json(json!({"error": format!("Failed to store results: {}", e)}));
    }

    // Get session to access original query
    let original_query = match llm_state.session_manager.get_session(session_id) {
        Ok(Some(ctx)) => ctx.original_query,
        Ok(None) => {
            return HttpResponse::NotFound()
                .json(json!({"error": "Session not found"}));
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
    llm_state: web::Data<LlmQueryState>,
) -> impl Responder {
    let session_id = &request.session_id;
    let question = &request.question;

    // Get session context
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

    // Check if we have results
    let results = match context.query_results {
        Some(ref r) => r,
        None => {
            return HttpResponse::BadRequest()
                .json(json!({"error": "No query results available in session"}));
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

    // Get answer from LLM
    let answer = match service
        .answer_question(
            &context.original_query,
            results,
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

    // Add messages to conversation history
    if let Err(e) = llm_state
        .session_manager
        .add_message(session_id, "user".to_string(), question.clone())
    {
        return HttpResponse::InternalServerError()
            .json(json!({"error": format!("Failed to update session: {}", e)}));
    }

    if let Err(e) = llm_state
        .session_manager
        .add_message(session_id, "assistant".to_string(), answer.clone())
    {
        return HttpResponse::InternalServerError()
            .json(json!({"error": format!("Failed to update session: {}", e)}));
    }

    HttpResponse::Ok().json(ChatResponse {
        answer,
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
        let node = app_state.node.lock().await;
        let db_guard = match node.get_fold_db() {
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
                estimated_completion: None, // TODO: Calculate based on rate
            })
        }
        None => HttpResponse::NotFound().json(json!({"error": "Backfill not found"})),
    }
}

