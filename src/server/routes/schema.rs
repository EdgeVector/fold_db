use crate::handlers::schema as schema_handlers;
use crate::log_feature;
use crate::logging::features::LogFeature;
use crate::server::http_server::AppState;
use crate::server::routes::{handler_error_to_response, require_node};
use actix_web::{web, HttpResponse, Responder};
use serde::{Deserialize, Serialize};
use serde_json::json;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SimpleSuccessResponse {
    pub success: bool,
}

#[utoipa::path(
    get,
    path = "/api/schemas",
    tag = "schemas",
    responses(
        (status = 200, description = "Array of schemas with states"),
        (status = 500, description = "Server error")
    )
)]
pub async fn list_schemas(state: web::Data<AppState>) -> impl Responder {
    let (user_hash, node_arc) = match require_node(&state).await {
        Ok(res) => res,
        Err(response) => return response,
    };

    let node = node_arc.read().await;

    // Use shared handler
    match schema_handlers::list_schemas(&user_hash, &node).await {
        Ok(response) => HttpResponse::Ok().json(response),
        Err(e) => handler_error_to_response(e),
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
        (status = 200, description = "Schema", body = Schema),
        (status = 404, description = "Schema not found"),
        (status = 500, description = "Server error")
    )
)]
pub async fn get_schema(path: web::Path<String>, state: web::Data<AppState>) -> impl Responder {
    let name = path.into_inner();
    let (user_hash, node_arc) = match require_node(&state).await {
        Ok(res) => res,
        Err(response) => return response,
    };

    let node = node_arc.read().await;

    // Use shared handler
    match schema_handlers::get_schema(&name, &user_hash, &node).await {
        Ok(response) => HttpResponse::Ok().json(response),
        Err(e) => handler_error_to_response(e),
    }
}

/// Approve a schema for queries and mutations
#[utoipa::path(
    post,
    path = "/api/schema/{name}/approve",
    tag = "schemas",
    params(
        ("name" = String, Path, description = "Schema name")
    ),
    responses(
        (status = 200, description = "Backfill hash if transform, null otherwise"),
        (status = 500, description = "Server error")
    )
)]
pub async fn approve_schema(path: web::Path<String>, state: web::Data<AppState>) -> impl Responder {
    let schema_name = path.into_inner();
    let (user_hash, node_arc) = match require_node(&state).await {
        Ok(res) => res,
        Err(response) => return response,
    };

    let node = node_arc.read().await;

    // Use shared handler
    match schema_handlers::approve_schema(&schema_name, &user_hash, &node).await {
        Ok(response) => HttpResponse::Ok().json(response),
        Err(e) => handler_error_to_response(e),
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
    responses(
        (status = 200, description = "Success status"),
        (status = 500, description = "Server error")
    )
)]
pub async fn block_schema(path: web::Path<String>, state: web::Data<AppState>) -> impl Responder {
    let schema_name = path.into_inner();
    let (user_hash, node_arc) = match require_node(&state).await {
        Ok(res) => res,
        Err(response) => return response,
    };

    let node = node_arc.read().await;

    // Use shared handler
    match schema_handlers::block_schema(&schema_name, &user_hash, &node).await {
        Ok(response) => HttpResponse::Ok().json(response),
        Err(e) => handler_error_to_response(e),
    }
}

/// Get backfill status by hash
#[utoipa::path(
    get,
    path = "/api/backfill/{hash}",
    tag = "backfill",
    params(
        ("hash" = String, Path, description = "Backfill hash")
    ),
    responses(
        (status = 200, description = "Backfill information object"),
        (status = 404, description = "Backfill not found"),
        (status = 500, description = "Server error")
    )
)]
pub async fn get_backfill_status(
    path: web::Path<String>,
    state: web::Data<AppState>,
) -> impl Responder {
    let backfill_hash = path.into_inner();
    let (user_hash, node_arc) = match require_node(&state).await {
        Ok(res) => res,
        Err(response) => return response,
    };
    let _user_hash = user_hash; // For logging/context if needed
    let node = node_arc.read().await;
    let processor = crate::datafold_node::OperationProcessor::new((*node).clone());

    match processor.get_backfill(&backfill_hash).await {
        Ok(Some(info)) => HttpResponse::Ok().json(info),
        Ok(None) => HttpResponse::NotFound().json(json!({"error": "Backfill not found"})),
        Err(e) => HttpResponse::InternalServerError()
            .json(json!({"error": format!("Failed to get backfill status: {}", e)})),
    }
}

/// Load schemas from standard directories into memory as Available
#[utoipa::path(
    post,
    path = "/api/schemas/load",
    tag = "schemas",
    responses(
        (status = 200, description = "Load counts for available and data schemas"),
        (status = 500, description = "Server error")
    )
)]
pub async fn load_schemas(state: web::Data<AppState>) -> impl Responder {
    let (user_hash, node_arc) = match require_node(&state).await {
        Ok(res) => res,
        Err(response) => return response,
    };

    let node = node_arc.read().await;

    // Use shared handler
    match schema_handlers::load_schemas(&user_hash, &node).await {
        Ok(response) => {
            if let Some(ref data) = response.data {
                log_feature!(
                    LogFeature::Schema,
                    info,
                    "Loaded {} of {} schemas from schema service",
                    data.schemas_loaded_to_db,
                    data.available_schemas_loaded
                );
            }
            HttpResponse::Ok().json(response)
        }
        Err(e) => {
            log_feature!(LogFeature::Schema, error, "Failed to load schemas: {}", e);
            handler_error_to_response(e)
        }
    }
}
