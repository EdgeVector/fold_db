use actix_web::{error::JsonPayloadError, HttpRequest, HttpResponse};
use serde_json::json;

/// Custom error handler for JSON deserialization errors.
///
/// This function is registered with Actix Web to handle errors that occur
/// when deserializing JSON payloads from requests. It logs the specific
/// error and returns a user-friendly JSON response.
pub fn json_error_handler(err: JsonPayloadError, req: &HttpRequest) -> actix_web::Error {
    let detail = err.to_string();
    let path = req.path().to_string();
    let method = req.method().to_string();

    log::error!(
        "JSON payload error: \"{}\" for {} request to {}",
        detail,
        method,
        path
    );

    let response = match &err {
        JsonPayloadError::Deserialize(serde_err) => HttpResponse::BadRequest().json(json!({
            "success": false,
            "error": "Invalid JSON format",
            "detail": serde_err.to_string()
        })),
        _ => HttpResponse::BadRequest().json(json!({
            "success": false,
            "error": "Invalid request payload",
            "detail": detail
        })),
    };

    actix_web::error::InternalError::from_response(err, response).into()
}
