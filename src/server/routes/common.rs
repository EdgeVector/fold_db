//! Common utilities for HTTP routes.

use actix_web::{http::StatusCode, HttpResponse};

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
