//! Security-related HTTP routes for key management and authentication

use crate::security::SecurityManager;
use crate::server::http_server::AppState;
use crate::server::routes::require_node;
use actix_web::{web, HttpResponse, Result as ActixResult};
use serde_json::json;
use std::sync::Arc;
// OpenAPI annotations are attached via #[utoipa::path(...)] on handlers

#[utoipa::path(
    get,
    path = "/api/security/system-key",
    tag = "security",
    responses((status = 200, description = "System key"), (status = 404, description = "Not found"))
)]
pub async fn get_system_public_key(data: web::Data<AppState>) -> ActixResult<HttpResponse> {
    let (_user_hash, node_arc) = match require_node(&data).await {
        Ok(res) => res,
        Err(response) => return Ok(response),
    };
    let node = node_arc.lock().await;
    let security_manager: Arc<SecurityManager> = node.get_security_manager().clone();

    match security_manager.get_system_public_key() {
        Ok(Some(key_info)) => Ok(HttpResponse::Ok().json(json!({
            "success": true,
            "key": key_info
        }))),
        Ok(None) => Ok(HttpResponse::NotFound().json(json!({
            "success": false,
            "error": "System key not found"
        }))),
        Err(e) => Ok(HttpResponse::InternalServerError().json(json!({
            "success": false,
            "error": e.to_string()
        }))),
    }
}

#[cfg(test)]
mod tests {
    #[test]
    fn minimal() {
        // Intentionally empty: compile-time smoke test for this module
    }
}
