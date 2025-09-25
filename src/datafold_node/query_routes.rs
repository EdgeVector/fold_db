use super::http_server::AppState;
use crate::log_feature;
use crate::logging::features::LogFeature;
use crate::schema::types::Operation;
use crate::datafold_node::OperationProcessor;
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

/// Common operation execution logic
async fn execute_operation_with_validation(
    data: web::Json<Value>,
    state: web::Data<AppState>,
    expected_operation_type: fn(&Operation) -> bool,
    log_feature: LogFeature,
    operation_name: &str,
    success_response: fn(Value) -> HttpResponse,
) -> impl Responder {
    log_feature!(
        log_feature,
        info,
        "Received {} request: {}",
        operation_name,
        serde_json::to_string(&data).unwrap_or_else(|_| "Invalid JSON".to_string())
    );

    // Parse the operation
    let web_operation = match serde_json::from_value::<Operation>(data.into_inner()) {
        Ok(op) => {
            if expected_operation_type(&op) {
                op
            } else {
                return HttpResponse::BadRequest()
                    .json(json!({"error": format!("Expected a {} operation", operation_name)}));
            }
        },
        Err(e) => {
            return HttpResponse::BadRequest()
                .json(json!({"error": format!("Failed to parse {}: {}", operation_name, e)}))
        }
    };

    // Create processor with the node
    let node_arc = Arc::clone(&state.node);
    let processor = OperationProcessor::new(node_arc);

    match processor.execute(web_operation).await {
        Ok(results) => {
            log_feature!(log_feature, info, "{} executed successfully", operation_name);
            success_response(results)
        }
        Err(e) => {
            log_feature!(log_feature, error, "{} execution failed: {}", operation_name, e);
            HttpResponse::InternalServerError()
                .json(json!({"error": format!("Failed to execute {}: {}", operation_name, e)}))
        }
    }
}

/// Execute a query.
pub async fn execute_query(query: web::Json<Value>, state: web::Data<AppState>) -> impl Responder {
    execute_operation_with_validation(
        query,
        state,
        |op| matches!(op, Operation::Query { .. }),
        LogFeature::Query,
        "query",
        |results| HttpResponse::Ok().json(json!({"data": results})),
    ).await
}

/// Execute a mutation.
pub async fn execute_mutation(
    mutation_data: web::Json<Value>,
    state: web::Data<AppState>,
) -> impl Responder {
    execute_operation_with_validation(
        mutation_data,
        state,
        |op| matches!(op, Operation::Mutation { .. }),
        LogFeature::Mutation,
        "mutation",
        |_| HttpResponse::Ok().json(json!({"success": true})),
    ).await
}

pub async fn list_transforms(state: web::Data<AppState>) -> impl Responder {
    let node = state.node.lock().await;
    match node.list_transforms() {
        Ok(map) => HttpResponse::Ok().json(json!({ "data": map })),
        Err(e) => HttpResponse::InternalServerError()
            .json(json!({ "error": format!("Failed to list transforms: {}", e) })),
    }
}

pub async fn add_to_transform_queue(
    path: web::Path<String>,
    state: web::Data<AppState>,
) -> impl Responder {
    let transform_id = path.into_inner();
    let node = state.node.lock().await;

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
