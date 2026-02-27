//! Common utilities for HTTP routes.

use crate::fold_node::FoldNode;
use crate::server::http_server::AppState;
use actix_web::{http::StatusCode, web, HttpResponse};
use serde_json::json;
use std::sync::Arc;
use tokio::sync::RwLock;

/// Convert a HandlerError to an appropriate HTTP response.
///
/// This is the centralized conversion function used by all HTTP routes
/// to convert shared handler errors to HTTP responses.
pub fn handler_error_to_response(e: crate::handlers::HandlerError) -> HttpResponse {
    let status_code = match e.status_code() {
        400 => StatusCode::BAD_REQUEST,
        401 => StatusCode::UNAUTHORIZED,
        404 => StatusCode::NOT_FOUND,
        503 => StatusCode::SERVICE_UNAVAILABLE,
        _ => StatusCode::INTERNAL_SERVER_ERROR,
    };
    HttpResponse::build(status_code).json(e.to_response())
}

/// Require user context from task-local storage.
/// Returns 401 Unauthorized error if no user context is present.
///
/// This is critical for multi-tenant isolation - all data operations
/// must have a valid user context to ensure proper data partitioning.
pub fn require_user_context() -> Result<String, HttpResponse> {
    crate::logging::core::get_current_user_id().ok_or_else(|| {
        HttpResponse::Unauthorized().json(json!({
            "ok": false,
            "error": "Authentication required. Please provide X-User-Hash header.",
            "code": "MISSING_USER_CONTEXT"
        }))
    })
}

/// Get a node for the current user from the NodeManager.
///
/// This is the key function for lazy per-user node initialization.
/// Nodes are created on first request and cached for subsequent requests.
pub async fn get_node_for_user(
    state: &web::Data<AppState>,
    user_id: &str,
) -> Result<Arc<RwLock<FoldNode>>, HttpResponse> {
    state.node_manager.get_node(user_id).await.map_err(|e| {
        log::error!("Failed to get node for user {}: {}", user_id, e);
        HttpResponse::InternalServerError().json(json!({
            "ok": false,
            "error": format!("Failed to initialize user context: {}", e),
            "code": "NODE_CREATION_FAILED"
        }))
    })
}

/// Helper macro-like pattern to get node for current user context
pub async fn require_node(
    state: &web::Data<AppState>,
) -> Result<(String, Arc<RwLock<FoldNode>>), HttpResponse> {
    let user_hash = require_user_context()?;
    let node = get_node_for_user(state, &user_hash).await?;
    Ok((user_hash, node))
}

/// Combined helper: require_node + acquire read lock.
///
/// Returns an owned read guard so the caller doesn't need `node_arc`.
pub async fn require_node_read(
    state: &web::Data<AppState>,
) -> Result<(String, tokio::sync::OwnedRwLockReadGuard<FoldNode>), HttpResponse> {
    let (user_hash, node_arc) = require_node(state).await?;
    let node = node_arc.read_owned().await;
    Ok((user_hash, node))
}
