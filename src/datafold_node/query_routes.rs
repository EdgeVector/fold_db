use super::http_server::AppState;
use crate::log_feature;
use crate::logging::features::LogFeature;
use crate::schema::types::Operation;
use crate::fold_db_core::query::records_from_field_map;
use crate::datafold_node::OperationProcessor;
use actix_web::{web, HttpResponse, Responder};
use serde_json::{json, Value};
use std::sync::Arc;


/// Execute a query.
pub async fn execute_query(query: web::Json<Value>, state: web::Data<AppState>) -> impl Responder {
    let op = match serde_json::from_value::<Operation>(query.into_inner()) {
        Ok(Operation::Query { schema, fields, filter }) => (schema, fields, filter),
        Ok(_) => {
            return HttpResponse::BadRequest()
                .json(json!({"error": "Expected a query operation"}))
        }
        Err(e) => {
            return HttpResponse::BadRequest()
                .json(json!({"error": format!("Failed to parse query: {}", e)}))
        }
    };

    let (schema, fields, filter) = op;
    let node_arc = Arc::clone(&state.node);
    let processor = OperationProcessor::new(node_arc);

    match processor.execute_query_map(schema, fields, filter).await {
        Ok(result_map) => HttpResponse::Ok().json(json!({"data": records_from_field_map(&result_map)})),
        Err(e) => HttpResponse::InternalServerError()
            .json(json!({"error": format!("Failed to execute query: {}", e)})),
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

    let (schema, fields_and_values, key_value, mutation_type) = match serde_json::from_value::<Operation>(mutation_data.into_inner()) {
        Ok(Operation::Mutation { schema, fields_and_values, key_value, mutation_type }) => (schema, fields_and_values, key_value, mutation_type),
        Ok(_) => {
            return HttpResponse::BadRequest()
                .json(json!({"error": "Expected a mutation operation"}))
        }
        Err(e) => {
            return HttpResponse::BadRequest()
                .json(json!({"error": format!("Failed to parse mutation: {}", e)}))
        }
    };

    let node_arc = Arc::clone(&state.node);
    let processor = OperationProcessor::new(node_arc);

    match processor
        .execute_mutation(schema, fields_and_values, key_value, mutation_type)
        .await
    {
        Ok(result) => HttpResponse::Ok().json(json!({"data": result})),
        Err(e) => HttpResponse::InternalServerError()
            .json(json!({"error": format!("Failed to execute mutation: {}", e)})),
    }
}

// formatting is handled by fold_db_core::query::formatter

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
