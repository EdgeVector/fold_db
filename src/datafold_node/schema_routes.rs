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
        (status = 200, description = "Array of schemas with states"),
        (status = 500, description = "Server error")
    )
)]
pub async fn list_schemas(state: web::Data<AppState>) -> impl Responder {
    log_feature!(LogFeature::Schema, info, "Received request to list schemas");
    let result =
        with_schema_manager(&state, |db| db.schema_manager.get_schemas_with_states()).await;
    match result {
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

/// Generate a backfill hash for a transform schema by looking up its source schema
/// Returns None if the schema is not a transform or if any required data is missing
fn generate_backfill_hash_for_transform(
    transform_manager: &crate::transform::manager::TransformManager,
    schema_name: &str,
) -> Option<String> {
    let transforms = match transform_manager.list_transforms() {
        Ok(t) => t,
        Err(e) => {
            log::warn!("Failed to list transforms for {}: {}", schema_name, e);
            return None;
        }
    };
    
    let transform = match transforms.get(schema_name) {
        Some(t) => t,
        None => {
            log::debug!("Transform {} not found in transform list", schema_name);
            return None;
        }
    };
    
    let declarative_schema = match transform.get_declarative_schema() {
        Some(s) => s,
        None => {
            log::warn!("Transform {} has no declarative schema", schema_name);
            return None;
        }
    };
    
    let inputs = declarative_schema.get_inputs();
    let first_input = match inputs.first() {
        Some(i) => i,
        None => {
            log::warn!("Transform {} has no inputs in declarative schema", schema_name);
            return None;
        }
    };
    
    let source_schema_name = match first_input.split('.').next() {
        Some(s) => s,
        None => {
            log::warn!("Failed to parse source schema from input: {}", first_input);
            return None;
        }
    };
    
    Some(crate::fold_db_core::infrastructure::backfill_tracker::BackfillTracker::generate_hash(
        schema_name,
        source_schema_name,
    ))
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
    let result: Result<Option<String>, SchemaError> = with_schema_manager(&state, |db| {
        // Check if this is a transform schema and generate backfill hash if needed
        let is_transform = match db.transform_manager.transform_exists(&schema_name) {
            Ok(exists) => exists,
            Err(e) => {
                log::warn!("Failed to check if {} is a transform, assuming false: {}", schema_name, e);
                false
            }
        };
        
        let backfill_hash = if is_transform {
            generate_backfill_hash_for_transform(&db.transform_manager, &schema_name)
        } else {
            None
        };
        
        // Approve the schema with the backfill hash
        db.schema_manager.set_schema_state_with_backfill(&schema_name, SchemaState::Approved, backfill_hash.clone())?;
        
        Ok(backfill_hash)
    }).await;
    
    match result {
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
    let result = with_schema_manager(&state, |db| db.schema_manager.block_schema(&schema_name)).await;
    match result {
        Ok(_) => HttpResponse::Ok().json(json!({"success": true})),
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
pub async fn get_backfill_status(path: web::Path<String>, state: web::Data<AppState>) -> impl Responder {
    let backfill_hash = path.into_inner();
    
    // Access the backfill tracker through the FoldDB
    let backfill_info = {
        let node_guard = state.node.lock().await;
        let db_guard = node_guard.get_fold_db().unwrap();
        db_guard.get_backfill_tracker().get_backfill_by_hash(&backfill_hash)
    };
    
    match backfill_info {
        Some(info) => HttpResponse::Ok().json(info),
        None => HttpResponse::NotFound().json(json!({"error": "Backfill not found"})),
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
    log_feature!(LogFeature::Schema, info, "Received request to load schemas from directories");
    let result: Result<(usize, usize), crate::error::FoldDbError> = with_schema_manager(&state, |db| {
        // Try available_schemas and data/schemas
        let available_loaded = db
            .schema_manager
            .load_schemas_from_directory("available_schemas")
            .map_err(|e| {
                log_feature!(LogFeature::Schema, error, "Failed to load schemas from available_schemas directory: {}", e);
                e
            })?;
        let data_loaded = db
            .schema_manager
            .load_schemas_from_directory("data/schemas")
            .map_err(|e| {
                log_feature!(LogFeature::Schema, error, "Failed to load schemas from data/schemas directory: {}", e);
                e
            })?;
        Ok((available_loaded, data_loaded))
    })
    .await;

    match result {
        Ok((available_loaded, data_loaded)) => {
            HttpResponse::Ok().json(json!({
                "available_schemas_loaded": available_loaded,
                "data_schemas_loaded": data_loaded
            }))
        }
        Err(e) => {
            HttpResponse::InternalServerError().json(json!({"error": format!("Failed to load schemas: {}", e)}))
        }
    }
}
