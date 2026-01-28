//! Common utilities for HTTP routes.

use actix_web::{http::StatusCode, HttpResponse};
use serde_json::json;

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
