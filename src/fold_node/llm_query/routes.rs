//! HTTP route handlers for LLM query workflow.
//!
//! These are Actix-web route handlers that delegate to the shared handler layer.
//! They handle HTTP-specific concerns (request/response extraction) and OpenAPI documentation.

use super::service::LlmQueryService;
use super::session::SessionManager;
use super::types::*;
use crate::handlers::llm as shared_handlers;
use crate::handlers::llm::AgentQueryHandlerRequest;
use crate::ingestion::IngestionConfig;
use crate::server::http_server::AppState;
use crate::server::routes::{handler_error_to_response, require_node};
use actix_web::{web, HttpResponse, Responder};
use serde_json::json;
use std::sync::Arc;
use tokio::sync::RwLock;

/// Shared state for LLM query routes
pub struct LlmQueryState {
    pub service: RwLock<Option<Arc<LlmQueryService>>>,
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
            service: RwLock::new(service),
            session_manager,
        }
    }

    /// Reload the LLM query service with fresh config
    pub async fn reload(&self) {
        let config = IngestionConfig::from_env_allow_empty();
        match LlmQueryService::new(config) {
            Ok(svc) => {
                let mut guard = self.service.write().await;
                *guard = Some(Arc::new(svc));
                log::info!("LlmQueryService reloaded with new configuration");
            }
            Err(e) => {
                log::warn!("Failed to reload LlmQueryService: {}", e);
            }
        }
    }
}

impl Default for LlmQueryState {
    fn default() -> Self {
        Self::new()
    }
}

/// Helper to require LLM service or return error response
async fn require_service(llm_state: &LlmQueryState) -> Result<Arc<LlmQueryService>, HttpResponse> {
    let guard = llm_state.service.read().await;
    guard.clone().ok_or_else(|| {
        HttpResponse::ServiceUnavailable().json(json!({
            "error": "LLM Query service not configured",
            "message": "Please configure AI_PROVIDER and FOLD_OPENROUTER_API_KEY or OLLAMA_BASE_URL environment variables to use this feature"
        }))
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
    let service = match require_service(&llm_state).await {
        Ok(svc) => svc,
        Err(response) => return response,
    };

    let (user_hash, node_arc) = match require_node(&app_state).await {
        Ok(res) => res,
        Err(response) => return response,
    };
    let node = node_arc.read().await;

    match shared_handlers::analyze_followup(
        request.into_inner(),
        &user_hash,
        service.as_ref(),
        llm_state.session_manager.as_ref(),
        &node,
    )
    .await
    {
        Ok(response) => {
            if let Some(data) = response.data {
                HttpResponse::Ok().json(FollowupAnalysis {
                    needs_query: data.needs_query,
                    query: data.query,
                    reasoning: data.reasoning,
                })
            } else {
                HttpResponse::InternalServerError().json(json!({"error": "Missing response data"}))
            }
        }
        Err(e) => handler_error_to_response(e),
    }
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
    let service = match require_service(&llm_state).await {
        Ok(svc) => svc,
        Err(response) => return response,
    };

    let (user_hash, node_arc) = match require_node(&app_state).await {
        Ok(res) => res,
        Err(response) => return response,
    };
    let node = node_arc.read().await;

    match shared_handlers::chat(
        request.into_inner(),
        &user_hash,
        service.as_ref(),
        llm_state.session_manager.as_ref(),
        &node,
    )
    .await
    {
        Ok(response) => {
            if let Some(data) = response.data {
                HttpResponse::Ok().json(ChatResponse {
                    answer: data.answer,
                    context_used: data.context_used,
                })
            } else {
                HttpResponse::InternalServerError().json(json!({"error": "Missing response data"}))
            }
        }
        Err(e) => handler_error_to_response(e),
    }
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
    let service = match require_service(&llm_state).await {
        Ok(svc) => svc,
        Err(response) => return response,
    };

    let (user_hash, node_arc) = match require_node(&app_state).await {
        Ok(res) => res,
        Err(response) => return response,
    };
    let node = node_arc.read().await;

    match shared_handlers::ai_native_index_query(
        request.into_inner(),
        &user_hash,
        service.as_ref(),
        llm_state.session_manager.as_ref(),
        &node,
    )
    .await
    {
        Ok(response) => {
            if let Some(data) = response.data {
                HttpResponse::Ok().json(json!({
                    "ai_interpretation": data.ai_interpretation,
                    "raw_results": data.raw_results,
                    "query": data.query,
                    "session_id": data.session_id
                }))
            } else {
                HttpResponse::InternalServerError().json(json!({"error": "Missing response data"}))
            }
        }
        Err(e) => handler_error_to_response(e),
    }
}

/// Execute an agent query - an autonomous LLM agent that can use tools
#[utoipa::path(
    post,
    path = "/api/llm-query/agent",
    tag = "llm-query",
    request_body = AgentQueryRequest,
    responses(
        (status = 200, description = "Agent query result", body = AgentQueryResponse),
        (status = 400, description = "Bad request"),
        (status = 500, description = "Server error")
    )
)]
pub async fn agent_query(
    request: web::Json<AgentQueryRequest>,
    app_state: web::Data<AppState>,
    llm_state: web::Data<LlmQueryState>,
) -> impl Responder {
    let service = match require_service(&llm_state).await {
        Ok(svc) => svc,
        Err(response) => return response,
    };

    let (user_hash, node_arc) = match require_node(&app_state).await {
        Ok(res) => res,
        Err(response) => return response,
    };
    let node = node_arc.read().await;

    // Convert the request to handler request type
    let handler_request = AgentQueryHandlerRequest {
        query: request.query.clone(),
        session_id: request.session_id.clone(),
        max_iterations: request.max_iterations,
    };

    match shared_handlers::agent_query(
        handler_request,
        &user_hash,
        service.as_ref(),
        llm_state.session_manager.as_ref(),
        &node,
    )
    .await
    {
        Ok(response) => {
            if let Some(data) = response.data {
                HttpResponse::Ok().json(AgentQueryResponse {
                    answer: data.answer,
                    tool_calls: data.tool_calls,
                    session_id: data.session_id,
                })
            } else {
                HttpResponse::InternalServerError().json(json!({"error": "Missing response data"}))
            }
        }
        Err(e) => handler_error_to_response(e),
    }
}
