use super::http_server::AppState;
use crate::log_feature;
use crate::logging::features::LogFeature;
use crate::schema::Schema;
use actix_web::{web, HttpResponse, Responder};
use serde_json::json;

/// List all schemas.
pub async fn list_schemas(state: web::Data<AppState>) -> impl Responder {
    log_feature!(LogFeature::Schema, info, "Received request to list schemas");
    let node_guard = state.node.lock().await;

    match node_guard.list_schemas_with_state() {
        Ok(schemas) => {
            log_feature!(
                LogFeature::Schema,
                info,
                "Successfully loaded {} schemas with states: {:?}",
                schemas.len(),
                schemas
            );
            HttpResponse::Ok().json(json!({"data": schemas}))
        }
        Err(e) => {
            log_feature!(LogFeature::Schema, error, "Failed to list schemas: {}", e);
            HttpResponse::InternalServerError()
                .json(json!({"error": format!("Failed to list schemas: {}", e)}))
        }
    }
}

/// Get a schema by name.
pub async fn get_schema(path: web::Path<String>, state: web::Data<AppState>) -> impl Responder {
    let name = path.into_inner();
    let node_guard = state.node.lock().await;

    match node_guard.get_schema(&name) {
        Ok(Some(schema)) => HttpResponse::Ok().json(schema),
        Ok(None) => {
            HttpResponse::NotFound().json(json!({"error": format!("Schema '{}' not found", name)}))
        }
        Err(e) => HttpResponse::InternalServerError()
            .json(json!({"error": format!("Failed to get schema: {}", e)})),
    }
}

/// Create a new schema.
pub async fn create_schema(
    schema: web::Json<Schema>,
    state: web::Data<AppState>,
) -> impl Responder {
    let mut node_guard = state.node.lock().await;

    match node_guard.load_schema(schema.into_inner()) {
        Ok(_) => HttpResponse::Created().json(json!({"success": true})),
        Err(e) => HttpResponse::InternalServerError()
            .json(json!({"error": format!("Failed to create schema: {}", e)})),
    }
}

/// Unload a schema so it is no longer active.
pub async fn unload_schema_route(
    path: web::Path<String>,
    state: web::Data<AppState>,
) -> impl Responder {
    let name = path.into_inner();
    let node_guard = state.node.lock().await;

    match node_guard.unload_schema(&name) {
        Ok(_) => HttpResponse::Ok().json(json!({"success": true})),
        Err(e) => HttpResponse::InternalServerError()
            .json(json!({"error": format!("Failed to unload schema: {}", e)})),
    }
}

/// Load a schema that exists but is not currently loaded.
pub async fn load_schema_route(
    path: web::Path<String>,
    state: web::Data<AppState>,
) -> impl Responder {
    let name = path.into_inner();
    let mut node_guard = state.node.lock().await;

    // First check if schema exists but is unloaded
    match node_guard.get_schema(&name) {
        Ok(Some(schema)) => {
            // Schema exists, try to load it
            match node_guard.load_schema(schema) {
                Ok(_) => HttpResponse::Ok().json(json!({"success": true, "message": format!("Schema '{}' loaded successfully", name)})),
                Err(e) => HttpResponse::InternalServerError().json(json!({"error": format!("Failed to load schema '{}': {}", name, e)})),
            }
        }
        Ok(None) => {
            HttpResponse::NotFound().json(json!({"error": format!("Schema '{}' not found", name)}))
        }
        Err(e) => HttpResponse::InternalServerError()
            .json(json!({"error": format!("Failed to get schema '{}': {}", name, e)})),
    }
}

/// List all available schemas (any state)
pub async fn list_available_schemas(state: web::Data<AppState>) -> impl Responder {
    log_feature!(
        LogFeature::Schema,
        info,
        "Received request to list available schemas"
    );
    let node_guard = state.node.lock().await;

    match node_guard.list_available_schemas() {
        Ok(schemas) => {
            log_feature!(
                LogFeature::Schema,
                info,
                "Successfully retrieved {} available schemas",
                schemas.len()
            );
            HttpResponse::Ok().json(json!({"data": schemas}))
        }
        Err(e) => {
            log_feature!(
                LogFeature::Schema,
                error,
                "Failed to list available schemas: {}",
                e
            );
            HttpResponse::InternalServerError()
                .json(json!({"error": format!("Failed to list available schemas: {}", e)}))
        }
    }
}

/// List schemas by specific state
pub async fn list_schemas_by_state(
    path: web::Path<String>,
    state: web::Data<AppState>,
) -> impl Responder {
    let state_str = path.into_inner();
    log_feature!(
        LogFeature::Schema,
        info,
        "Received request to list schemas by state: {}",
        state_str
    );

    let schema_state = match state_str.as_str() {
        "available" => crate::schema::SchemaState::Available,
        "approved" => crate::schema::SchemaState::Approved,
        "blocked" => crate::schema::SchemaState::Blocked,
        _ => {
            return HttpResponse::BadRequest().json(json!({
                "error": "Invalid state. Use: available, approved, or blocked"
            }));
        }
    };

    let node_guard = state.node.lock().await;
    match node_guard.list_schemas_by_state(schema_state) {
        Ok(schemas) => HttpResponse::Ok().json(json!({
            "data": schemas,
            "state": state_str
        })),
        Err(e) => {
            log_feature!(
                LogFeature::Schema,
                error,
                "Failed to list schemas by state '{}': {}",
                state_str,
                e
            );
            HttpResponse::InternalServerError().json(json!({
                "error": format!("Failed to list schemas by state: {}", e)
            }))
        }
    }
}

/// Approve a schema for queries and mutations
pub async fn approve_schema(path: web::Path<String>, state: web::Data<AppState>) -> impl Responder {
    let schema_name = path.into_inner();
    log_feature!(
        LogFeature::Schema,
        info,
        "Received request to approve schema: {}",
        schema_name
    );

    let mut node_guard = state.node.lock().await;
    match node_guard.approve_schema(&schema_name) {
        Ok(()) => {
            log_feature!(
                LogFeature::Schema,
                info,
                "Schema '{}' approved successfully",
                schema_name
            );
            HttpResponse::Ok().json(json!({
                "message": format!("Schema '{}' approved successfully", schema_name),
                "schema": schema_name,
                "state": "approved"
            }))
        }
        Err(e) => {
            log_feature!(
                LogFeature::Schema,
                error,
                "Failed to approve schema '{}': {}",
                schema_name,
                e
            );
            HttpResponse::BadRequest().json(json!({
                "error": format!("Failed to approve schema: {}", e)
            }))
        }
    }
}

/// Block a schema from queries and mutations
pub async fn block_schema(path: web::Path<String>, state: web::Data<AppState>) -> impl Responder {
    let schema_name = path.into_inner();
    log_feature!(
        LogFeature::Schema,
        info,
        "Received request to block schema: {}",
        schema_name
    );

    let mut node_guard = state.node.lock().await;
    match node_guard.block_schema(&schema_name) {
        Ok(()) => {
            log_feature!(
                LogFeature::Schema,
                info,
                "Schema '{}' blocked successfully",
                schema_name
            );
            HttpResponse::Ok().json(json!({
                "message": format!("Schema '{}' blocked successfully", schema_name),
                "schema": schema_name,
                "state": "blocked"
            }))
        }
        Err(e) => {
            log_feature!(
                LogFeature::Schema,
                error,
                "Failed to block schema '{}': {}",
                schema_name,
                e
            );
            HttpResponse::BadRequest().json(json!({
                "error": format!("Failed to block schema: {}", e)
            }))
        }
    }
}

/// Get the current state of a schema
pub async fn get_schema_state(
    path: web::Path<String>,
    state: web::Data<AppState>,
) -> impl Responder {
    let schema_name = path.into_inner();
    log_feature!(
        LogFeature::Schema,
        info,
        "Received request to get state for schema: {}",
        schema_name
    );

    let node_guard = state.node.lock().await;
    match node_guard.get_schema_state(&schema_name) {
        Ok(schema_state) => {
            let state_str = match schema_state {
                crate::schema::SchemaState::Available => "available",
                crate::schema::SchemaState::Approved => "approved",
                crate::schema::SchemaState::Blocked => "blocked",
            };
            HttpResponse::Ok().json(json!({
                "schema": schema_name,
                "state": state_str
            }))
        }
        Err(e) => {
            log_feature!(
                LogFeature::Schema,
                error,
                "Failed to get state for schema '{}': {}",
                schema_name,
                e
            );
            HttpResponse::NotFound().json(json!({
                "error": format!("Failed to get schema state: {}", e)
            }))
        }
    }
}

/// Get comprehensive schema status (NEW UNIFIED ENDPOINT)
pub async fn get_schema_status(state: web::Data<AppState>) -> impl Responder {
    log_feature!(
        LogFeature::Schema,
        info,
        "Received request to get comprehensive schema status"
    );
    let node_guard = state.node.lock().await;

    match node_guard.get_schema_status() {
        Ok(report) => HttpResponse::Ok().json(json!({"data": report})),
        Err(e) => {
            log_feature!(
                LogFeature::Schema,
                error,
                "Failed to get schema status: {}",
                e
            );
            HttpResponse::InternalServerError()
                .json(json!({"error": format!("Failed to get schema status: {}", e)}))
        }
    }
}

/// Refresh schemas from all sources (NEW UNIFIED ENDPOINT)
pub async fn refresh_schemas(state: web::Data<AppState>) -> impl Responder {
    log_feature!(
        LogFeature::Schema,
        info,
        "Received request to refresh schemas from all sources"
    );
    let node_guard = state.node.lock().await;

    match node_guard.refresh_schemas() {
        Ok(report) => {
            log_feature!(
                LogFeature::Schema,
                info,
                "Schema refresh completed: {} discovered, {} loaded, {} failed",
                report.discovered_schemas.len(),
                report.loaded_schemas.len(),
                report.failed_schemas.len()
            );
            HttpResponse::Ok().json(json!({"data": report}))
        }
        Err(e) => {
            log_feature!(
                LogFeature::Schema,
                error,
                "Failed to refresh schemas: {}",
                e
            );
            HttpResponse::InternalServerError()
                .json(json!({"error": format!("Failed to refresh schemas: {}", e)}))
        }
    }
}

/// Add a new schema to the available_schemas directory with validation
pub async fn add_schema_to_available(
    schema_data: web::Json<serde_json::Value>,
    state: web::Data<AppState>,
) -> impl Responder {
    log_feature!(
        LogFeature::Schema,
        info,
        "Received request to add schema to available_schemas directory"
    );
    let node_guard = state.node.lock().await;

    // Extract optional custom name from query parameters or JSON
    let custom_name = schema_data
        .get("custom_name")
        .and_then(|v| v.as_str())
        .map(|s| s.to_string());

    // Convert JSON value to string for processing
    let json_content = match serde_json::to_string(&*schema_data) {
        Ok(content) => content,
        Err(e) => {
            log_feature!(LogFeature::Schema, error, "Failed to serialize JSON: {}", e);
            return HttpResponse::BadRequest().json(json!({
                "error": format!("Invalid JSON format: {}", e)
            }));
        }
    };

    match node_guard.add_schema_to_available_directory(&json_content, custom_name) {
        Ok(schema_name) => {
            log_feature!(
                LogFeature::Schema,
                info,
                "Successfully added schema '{}' to available_schemas directory",
                schema_name
            );
            HttpResponse::Created().json(json!({
                "success": true,
                "schema_name": schema_name,
                "message": format!("Schema '{}' added to available_schemas directory and is ready for approval", schema_name)
            }))
        }
        Err(e) => {
            log_feature!(
                LogFeature::Schema,
                error,
                "Failed to add schema to available_schemas: {}",
                e
            );
            HttpResponse::BadRequest().json(json!({
                "error": format!("Failed to add schema: {}", e)
            }))
        }
    }
}
