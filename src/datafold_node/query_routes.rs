use super::http_server::AppState;
use super::OperationProcessor;
use crate::log_feature;
use crate::logging::features::LogFeature;
use crate::schema::types::Operation;
use actix_web::{web, HttpResponse, Responder};
use serde::Deserialize;
use serde_json::{json, Value};
use std::sync::Arc;

/// Execute an operation (query or mutation).
#[derive(Deserialize)]
pub struct OperationRequest {
    operation: String,
}

pub async fn execute_operation(
    request: web::Json<OperationRequest>,
    state: web::Data<AppState>,
) -> impl Responder {
    let operation_str = &request.operation;

    let operation: Operation = match serde_json::from_str(operation_str) {
        Ok(op) => op,
        Err(e) => {
            return HttpResponse::BadRequest()
                .json(json!({"error": format!("Failed to parse operation: {}", e)}));
        }
    };

    // Create processor with the node
    let node_arc = Arc::clone(&state.node);
    let processor = OperationProcessor::new(node_arc);

    match processor.execute(operation).await {
        Ok(result) => HttpResponse::Ok().json(json!({"data": result})),
        Err(e) => HttpResponse::InternalServerError()
            .json(json!({"error": format!("Failed to execute operation: {}", e)})),
    }
}

/// Execute a query.
pub async fn execute_query(query: web::Json<Value>, state: web::Data<AppState>) -> impl Responder {
    let query_value = query.into_inner();
    log_feature!(
        LogFeature::Query,
        info,
        "Received query request: {}",
        serde_json::to_string(&query_value).unwrap_or_else(|_| "Invalid JSON".to_string())
    );

    // Parse the simple web UI operation
    let web_operation = match serde_json::from_value::<Operation>(query_value) {
        Ok(op) => match op {
            Operation::Query { .. } => op,
            _ => {
                return HttpResponse::BadRequest()
                    .json(json!({"error": "Expected a query operation"}))
            }
        },
        Err(e) => {
            return HttpResponse::BadRequest()
                .json(json!({"error": format!("Failed to parse query: {}", e)}))
        }
    };

    // Create processor with the node
    let node_arc = Arc::clone(&state.node);
    let processor = OperationProcessor::new(node_arc);

    match processor.execute(web_operation).await {
        Ok(results) => {
            log_feature!(LogFeature::Query, info, "Query executed successfully");
            HttpResponse::Ok().json(json!({"data": results}))
        }
        Err(e) => {
            log_feature!(LogFeature::Query, error, "Query execution failed: {}", e);
            HttpResponse::InternalServerError()
                .json(json!({"error": format!("Failed to execute query: {}", e)}))
        }
    }
}

/// Execute a mutation.
pub async fn execute_mutation(
    mutation_data: web::Json<Value>,
    state: web::Data<AppState>,
) -> impl Responder {
    log_feature!(
        LogFeature::Mutation,
        info,
        "Received mutation request: {}",
        serde_json::to_string(&mutation_data).unwrap_or_else(|_| "Invalid JSON".to_string())
    );

    // Parse the mutation operation
    let web_operation = match serde_json::from_value::<Operation>(mutation_data.into_inner()) {
        Ok(op) => match op {
            Operation::Mutation { .. } => op,
            _ => {
                return HttpResponse::BadRequest()
                    .json(json!({"error": "Expected a mutation operation"}))
            }
        },
        Err(e) => {
            return HttpResponse::BadRequest()
                .json(json!({"error": format!("Failed to parse mutation: {}", e)}))
        }
    };

    // Create processor with the node
    let node_arc = Arc::clone(&state.node);
    let processor = OperationProcessor::new(node_arc);

    match processor.execute(web_operation).await {
        Ok(_) => {
            log_feature!(LogFeature::Mutation, info, "Mutation executed successfully");
            HttpResponse::Ok().json(json!({"success": true}))
        }
        Err(e) => {
            log_feature!(
                LogFeature::Mutation,
                error,
                "Mutation execution failed: {}",
                e
            );
            HttpResponse::InternalServerError()
                .json(json!({"error": format!("Failed to execute mutation: {}", e)}))
        }
    }
}

pub async fn list_transforms(state: web::Data<AppState>) -> impl Responder {
    let node = state.node.lock().await;
    match node.list_transforms() {
        Ok(map) => HttpResponse::Ok().json(json!({ "data": map })),
        Err(e) => HttpResponse::InternalServerError()
            .json(json!({ "error": format!("Failed to list transforms: {}", e) })),
    }
}

pub async fn run_transform(path: web::Path<String>, state: web::Data<AppState>) -> impl Responder {
    let id = path.into_inner();
    let mut node = state.node.lock().await;
    match node.run_transform(&id) {
        Ok(val) => HttpResponse::Ok().json(json!({ "data": val })),
        Err(e) => HttpResponse::InternalServerError()
            .json(json!({ "error": format!("Failed to run transform: {}", e) })),
    }
}

pub async fn add_to_transform_queue(
    path: web::Path<String>,
    state: web::Data<AppState>,
) -> impl Responder {
    let transform_id = path.into_inner();
    let node = state.node.lock().await;

    match node.list_transforms() {
        Ok(transforms) => {
            if !transforms.contains_key(&transform_id) {
                return HttpResponse::NotFound().json(json!({"error": format!("Transform '{}' not found. Available transforms: {:?}", transform_id, transforms.keys().collect::<Vec<_>>())}));
            }
        }
        Err(e) => {
            return HttpResponse::InternalServerError()
                .json(json!({"error": format!("Failed to verify transform: {}", e)}))
        }
    }

    match node.add_transform_to_queue(&transform_id) {
        Ok(_) => HttpResponse::Ok().json(json!({"success": true, "message": format!("Transform '{}' added to queue", transform_id)})),
        Err(e) => HttpResponse::InternalServerError().json(json!({"error": format!("Failed to add transform to queue: {}", e)})),
    }
}

pub async fn get_transform_queue(state: web::Data<AppState>) -> impl Responder {
    let node = state.node.lock().await;
    match node.get_transform_queue_info() {
        Ok(info) => HttpResponse::Ok().json(info),
        Err(e) => HttpResponse::InternalServerError()
            .json(json!({ "error": format!("Failed to get transform queue info: {}", e) })),
    }
}

#[cfg(test)]
mod tests {}
