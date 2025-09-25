use super::http_server::AppState;
use crate::log_feature;
use crate::logging::features::LogFeature;
use crate::schema::SchemaState;
use actix_web::{web, HttpResponse, Responder};
use serde_json::json;

/// Helper closure to execute schema operations with lock management
async fn with_schema_manager<F, R>(state: &web::Data<AppState>, operation: F) -> R
where
    F: FnOnce(std::sync::MutexGuard<'_, crate::fold_db_core::FoldDB>) -> R,
{
    let node_guard = state.node.lock().await;
    let db_guard = node_guard.get_fold_db().unwrap();
    let result = operation(db_guard);
    drop(node_guard);
    result
}

/// List all schemas.
pub async fn list_schemas(state: web::Data<AppState>) -> impl Responder {
    log_feature!(LogFeature::Schema, info, "Received request to list schemas");
    let result = with_schema_manager(&state, |db| db.schema_manager.get_schemas()).await;
    match result {
        Ok(schemas) => HttpResponse::Ok().json(json!({"data": schemas})),
        Err(e) => HttpResponse::InternalServerError()
            .json(json!({"error": format!("Failed to list schemas: {}", e)})),
    }
}

/// Get a schema by name.
pub async fn get_schema(path: web::Path<String>, state: web::Data<AppState>) -> impl Responder {
    let name = path.into_inner();
    let result = with_schema_manager(&state, |db| db.schema_manager.get_schema(&name)).await;
    match result {
        Ok(Some(schema)) => HttpResponse::Ok().json(schema),
        Ok(None) => HttpResponse::NotFound().json(json!({"error": "Schema not found"})),
        Err(e) => HttpResponse::InternalServerError()
            .json(json!({"error": format!("Failed to get schema: {}", e)})),
    }
}

/// List schemas by specific state
/// Approve a schema for queries and mutations
pub async fn approve_schema(path: web::Path<String>, state: web::Data<AppState>) -> impl Responder {
    let schema_name = path.into_inner();
    let result = with_schema_manager(&state, |db| db.schema_manager.set_schema_state(&schema_name, SchemaState::Approved)).await;
    match result {
        Ok(_) => HttpResponse::Ok().json(json!({"success": true})),
        Err(e) => HttpResponse::InternalServerError()
            .json(json!({"error": format!("Failed to approve schema: {}", e)})),
    }
}

/// Block a schema from queries and mutations
pub async fn block_schema(path: web::Path<String>, state: web::Data<AppState>) -> impl Responder {
    let schema_name = path.into_inner();
    let result = with_schema_manager(&state, |db| db.schema_manager.block_schema(&schema_name)).await;
    match result {
        Ok(_) => HttpResponse::Ok().json(json!({"success": true})),
        Err(e) => HttpResponse::InternalServerError()
            .json(json!({"error": format!("Failed to block schema: {}", e)})),
    }
}
