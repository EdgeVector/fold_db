use crate::log_feature;
use crate::logging::features::LogFeature;
use crate::schema::{SchemaState, SchemaWithState};
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
    let node = state.node.read().await;
    match node.get_fold_db().await {
        Ok(db) => match db.schema_manager.get_schemas_with_states() {
            Ok(schemas) => HttpResponse::Ok().json(schemas),
            Err(e) => HttpResponse::InternalServerError()
                .json(json!({"error": format!("Failed to list schemas: {}", e)})),
        },
        Err(e) => HttpResponse::InternalServerError()
            .json(json!({"error": format!("Failed to access database: {}", e)})),
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
    let node = state.node.read().await;
    match node.get_fold_db().await {
        Ok(db) => {
            let mgr = &db.schema_manager;
            match mgr.get_schema(&name) {
                Ok(Some(schema)) => match mgr.get_schema_states() {
                    Ok(state) => {
                        let schema_state = state.get(&name).copied().unwrap_or_default();
                        HttpResponse::Ok().json(SchemaWithState::new(schema, schema_state))
                    }
                    Err(e) => HttpResponse::InternalServerError()
                        .json(json!({"error": format!("Failed to get schema states: {}", e)})),
                },
                Ok(None) => HttpResponse::NotFound().json(json!({"error": "Schema not found"})),
                Err(e) => HttpResponse::InternalServerError()
                    .json(json!({"error": format!("Failed to get schema: {}", e)})),
            }
        }
        Err(e) => HttpResponse::InternalServerError()
            .json(json!({"error": format!("Failed to access database: {}", e)})),
    }
}

/// Generate a backfill hash for a transform schema by looking up its source schema
/// Returns None if the schema is not a transform or if any required data is missing
async fn generate_backfill_hash_for_transform(
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

    // Look up the transform's schema from the database
    let declarative_schema = match transform_manager
        .db_ops
        .get_schema(transform.get_schema_name())
        .await
    {
        Ok(Some(s)) => s,
        Ok(None) => {
            log::warn!("Transform {} schema not found in database", schema_name);
            return None;
        }
        Err(e) => {
            log::warn!("Failed to get schema for transform {}: {}", schema_name, e);
            return None;
        }
    };

    let inputs = declarative_schema.get_inputs();
    let first_input = match inputs.first() {
        Some(i) => i,
        None => {
            log::warn!(
                "Transform {} has no inputs in declarative schema",
                schema_name
            );
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

    Some(
        crate::fold_db_core::infrastructure::backfill_tracker::BackfillTracker::generate_hash(
            schema_name,
            source_schema_name,
        ),
    )
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
    let node = state.node.read().await;

    match node.get_fold_db().await {
        Ok(db) => {
            let schema_mgr = &db.schema_manager;
            let transform_mgr = &db.transform_manager;

            // Check if schema is already approved
            match schema_mgr.get_schema_states() {
                Ok(states) => {
                    let current_state = states.get(&schema_name).copied().unwrap_or_default();
                    if current_state == SchemaState::Approved {
                        log::info!("Schema '{}' is already approved", schema_name);
                        return HttpResponse::Ok().json(Option::<String>::None);
                    }
                }
                Err(e) => {
                    return HttpResponse::InternalServerError()
                        .json(json!({"error": format!("Failed to get schema states: {}", e)}))
                }
            }

            let is_transform = transform_mgr
                .transform_exists(&schema_name)
                .unwrap_or(false);
            let backfill_hash = if is_transform {
                generate_backfill_hash_for_transform(transform_mgr, &schema_name).await
            } else {
                None
            };

            match schema_mgr
                .set_schema_state_with_backfill(
                    &schema_name,
                    SchemaState::Approved,
                    backfill_hash.clone(),
                )
                .await
            {
                Ok(_) => HttpResponse::Ok().json(backfill_hash),
                Err(e) => HttpResponse::InternalServerError()
                    .json(json!({"error": format!("Failed to approve schema: {}", e)})),
            }
        }
        Err(e) => HttpResponse::InternalServerError()
            .json(json!({"error": format!("Failed to access database: {}", e)})),
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
    let _schema_name_clone = schema_name.clone();
    let node = state.node.read().await;
    match node.get_fold_db().await {
        Ok(db) => match db.schema_manager.block_schema(&schema_name).await {
            Ok(_) => HttpResponse::Ok().json(SimpleSuccessResponse { success: true }),
            Err(e) => HttpResponse::InternalServerError()
                .json(json!({"error": format!("Failed to block schema: {}", e)})),
        },
        Err(e) => HttpResponse::InternalServerError()
            .json(json!({"error": format!("Failed to access database: {}", e)})),
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

    // Access the backfill tracker through the FoldDB
    let backfill_info = {
        let node_guard = state.node.read().await;
        if let Ok(db_guard) = node_guard.get_fold_db().await {
            db_guard
                .get_backfill_tracker()
                .get_backfill_by_hash(&backfill_hash)
        } else {
            None
        }
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
    // Fetch schemas from the schema service
    let node_guard = state.node.read().await;

    match node_guard.fetch_available_schemas().await {
        Ok(schemas) => {
            let schema_count = schemas.len();
            drop(node_guard);

            // Load each schema into the local database
            let mut loaded_count = 0;
            let mut failed_schemas = Vec::new();

            for schema in schemas {
                let schema_name = schema.name.clone();

                let result = {
                    let node = state.node.read().await;
                    match node.get_fold_db().await {
                        Ok(db) => db
                            .schema_manager
                            .load_schema_internal(schema.clone())
                            .await
                            .map_err(|e| crate::error::FoldDbError::Database(e.to_string())),
                        Err(e) => Err(crate::error::FoldDbError::Database(e.to_string())),
                    }
                };

                match result {
                    Ok(_) => {
                        loaded_count += 1;
                        log_feature!(LogFeature::Schema, debug, "Loaded schema: {}", schema_name);
                    }
                    Err(e) => {
                        log_feature!(
                            LogFeature::Schema,
                            error,
                            "Failed to load schema {}: {}",
                            schema_name,
                            e
                        );
                        failed_schemas.push(schema_name);
                    }
                }
            }

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
            log_feature!(
                LogFeature::Schema,
                error,
                "Failed to fetch schemas from schema service: {}",
                e
            );
            HttpResponse::InternalServerError().json(json!({
                "error": format!("Failed to fetch schemas from schema service: {}", e)
            }))
        }
    }
}
