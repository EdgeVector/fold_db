use crate::datafold_node::OperationProcessor;
use crate::log_feature;
use crate::logging::features::LogFeature;

use crate::server::http_server::AppState;
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
    let processor = OperationProcessor::new(state.node.read().await.clone());

    match processor.list_schemas().await {
        Ok(schemas) => HttpResponse::Ok().json(schemas),
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
        (status = 200, description = "Schema", body = Schema),
        (status = 404, description = "Schema not found"),
        (status = 500, description = "Server error")
    )
)]
pub async fn get_schema(path: web::Path<String>, state: web::Data<AppState>) -> impl Responder {
    let name = path.into_inner();
    let processor = OperationProcessor::new(state.node.read().await.clone());

    match processor.get_schema(&name).await {
        Ok(Some(schema_with_state)) => HttpResponse::Ok().json(schema_with_state),
        Ok(None) => HttpResponse::NotFound().json(json!({"error": "Schema not found"})),
        Err(e) => HttpResponse::InternalServerError()
            .json(json!({"error": format!("Failed to get schema: {}", e)})),
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
    let processor = OperationProcessor::new(state.node.read().await.clone());

    match processor.approve_schema(&schema_name).await {
        Ok(backfill_hash) => HttpResponse::Ok().json(backfill_hash),
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
    responses(
        (status = 200, description = "Success status"),
        (status = 500, description = "Server error")
    )
)]
pub async fn block_schema(path: web::Path<String>, state: web::Data<AppState>) -> impl Responder {
    let schema_name = path.into_inner();
    let processor = OperationProcessor::new(state.node.read().await.clone());

    match processor.block_schema(&schema_name).await {
        Ok(_) => HttpResponse::Ok().json(SimpleSuccessResponse { success: true }),
        Err(e) => HttpResponse::InternalServerError()
            .json(json!({"error": format!("Failed to block schema: {}", e)})),
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
    let processor = OperationProcessor::new(state.node.read().await.clone());

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
    let processor = OperationProcessor::new(state.node.read().await.clone());

    match processor.load_schemas().await {
        Ok((schema_count, loaded_count, failed_schemas)) => {
            log_feature!(
                LogFeature::Schema,
                info,
                "Loaded {} of {} schemas from schema service",
                loaded_count,
                schema_count
            );

            HttpResponse::Ok().json(json!({
                "available_schemas_loaded": schema_count,
                "schemas_loaded_to_db": loaded_count,
                "failed_schemas": failed_schemas
            }))
        }
        Err(e) => {
            log_feature!(LogFeature::Schema, error, "Failed to load schemas: {}", e);
            HttpResponse::InternalServerError().json(json!({
                "error": format!("Failed to load schemas: {}", e)
            }))
        }
    }
}
