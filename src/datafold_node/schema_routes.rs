use super::http_server::AppState;
use crate::log_feature;
use crate::logging::features::LogFeature;
use crate::schema::{SchemaError, SchemaState, SchemaWithState};
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
#[utoipa::path(
    get,
    path = "/api/schemas",
    tag = "schemas",
    responses(
        (status = 200, description = "List of schemas"),
        (status = 500, description = "Server error")
    )
)]
pub async fn list_schemas(state: web::Data<AppState>) -> impl Responder {
    log_feature!(LogFeature::Schema, info, "Received request to list schemas");
    let result =
        with_schema_manager(&state, |db| db.schema_manager.get_schemas_with_states()).await;
    match result {
        Ok(schemas) => HttpResponse::Ok().json(json!({"data": schemas})),
        Err(e) => HttpResponse::InternalServerError()
            .json(json!({"error": format!("Failed to list schemas: {}", e)})),
    }
}

/// Get a schema by name.
#[utoipa::path(
    get,
    path = "/api/schema/{name}",
    tag = "schemas",
    params(
        ("name" = String, Path, description = "Schema name")
    ),
    responses(
        (status = 200, description = "Schema", body = crate::schema::types::schema::Schema),
        (status = 404, description = "Schema not found"),
        (status = 500, description = "Server error")
    )
)]
pub async fn get_schema(path: web::Path<String>, state: web::Data<AppState>) -> impl Responder {
    let name = path.into_inner();
    let result: Result<Option<SchemaWithState>, SchemaError> =
        with_schema_manager(&state, |db| {
            let schema = db.schema_manager.get_schema(&name)?;
            if let Some(schema) = schema {
                let state = db.schema_manager.get_schema_states()?;
                let schema_state = state.get(&name).copied().unwrap_or_default();
                Ok(Some(SchemaWithState::new(schema, schema_state)))
            } else {
                Ok(None)
            }
        })
        .await;

    match result {
        Ok(Some(schema)) => HttpResponse::Ok().json(schema),
        Ok(None) => HttpResponse::NotFound().json(json!({"error": "Schema not found"})),
        Err(e) => HttpResponse::InternalServerError()
            .json(json!({"error": format!("Failed to get schema: {}", e)})),
    }
}

/// List schemas by specific state
/// Approve a schema for queries and mutations
#[utoipa::path(
    post,
    path = "/api/schema/{name}/approve",
    tag = "schemas",
    params(
        ("name" = String, Path, description = "Schema name")
    ),
    responses((status = 200, description = "Approved"), (status = 500, description = "Server error"))
)]
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
#[utoipa::path(
    post,
    path = "/api/schema/{name}/block",
    tag = "schemas",
    params(
        ("name" = String, Path, description = "Schema name")
    ),
    responses((status = 200, description = "Blocked"), (status = 500, description = "Server error"))
)]
pub async fn block_schema(path: web::Path<String>, state: web::Data<AppState>) -> impl Responder {
    let schema_name = path.into_inner();
    let result = with_schema_manager(&state, |db| db.schema_manager.block_schema(&schema_name)).await;
    match result {
        Ok(_) => HttpResponse::Ok().json(json!({"success": true})),
        Err(e) => HttpResponse::InternalServerError()
            .json(json!({"error": format!("Failed to block schema: {}", e)})),
    }
}

/// Load schemas from standard directories into memory as Available
#[utoipa::path(
    post,
    path = "/api/schemas/load",
    tag = "schemas",
    responses(
        (status = 200, description = "Load attempt summary"),
        (status = 500, description = "Server error")
    )
)]
pub async fn load_schemas(state: web::Data<AppState>) -> impl Responder {
    log_feature!(LogFeature::Schema, info, "Received request to load schemas from directories");
    let result = with_schema_manager(&state, |db| {
        // Try available_schemas and data/schemas
        let available_loaded = db
            .schema_manager
            .load_schemas_from_directory("available_schemas")
            .unwrap_or(0);
        let data_loaded = db
            .schema_manager
            .load_schemas_from_directory("data/schemas")
            .unwrap_or(0);
        (available_loaded, data_loaded)
    })
    .await;

    HttpResponse::Ok().json(json!({
        "data": {
            "available_schemas_loaded": result.0,
            "data_schemas_loaded": result.1
        }
    }))
}
