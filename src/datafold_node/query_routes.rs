use super::http_server::AppState;
use crate::schema::types::operations::Query;
use crate::schema::types::operations::Operation;
use crate::fold_db_core::query::records_from_field_map;
use crate::datafold_node::OperationProcessor;
use actix_web::{web, HttpResponse, Responder};
use serde_json::{json, Value};
use std::sync::Arc;


/// Execute a query.
#[utoipa::path(
    post,
    path = "/api/query",
    tag = "query",
    request_body = serde_json::Value,
    responses(
        (status = 200, description = "Query result", body = serde_json::Value),
        (status = 400, description = "Bad request"),
        (status = 500, description = "Server error")
    )
)]
pub async fn execute_query(query: web::Json<Query>, state: web::Data<AppState>) -> impl Responder {
    let node_arc = Arc::clone(&state.node);
    let processor = OperationProcessor::new(node_arc);

    match processor.execute_query_map(query.clone()).await {
        Ok(result_map) => {
            // Return results as array of { key: KeyValue, fields: {...} }
            let records_map = records_from_field_map(&result_map);
            let data: Vec<Value> = records_map
                .into_iter()
                .map(|(key, record)| json!({ "key": key, "fields": record.fields }))
                .collect();
            HttpResponse::Ok().json(json!({"data": data}))
        },
        Err(e) => HttpResponse::InternalServerError()
            .json(json!({"error": format!("Failed to execute query: {}", e)})),
    }
}

/// Execute a mutation.
#[utoipa::path(
    post,
    path = "/api/mutation",
    tag = "query",
    request_body = serde_json::Value,
    responses(
        (status = 200, description = "Mutation result", body = serde_json::Value),
        (status = 400, description = "Bad request"),
        (status = 500, description = "Server error")
    )
)]
pub async fn execute_mutation(
    mutation_data: web::Json<Value>,
    state: web::Data<AppState>,
) -> impl Responder {

    let (schema, fields_and_values, key_value, mutation_type) = match serde_json::from_value::<Operation>(mutation_data.into_inner()) {
        Ok(Operation::Mutation { schema, fields_and_values, key_value, mutation_type }) => (schema, fields_and_values, key_value, mutation_type),
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

#[utoipa::path(
    get,
    path = "/api/transforms",
    tag = "query",
    responses(
        (status = 200, description = "Transforms list", body = serde_json::Value),
        (status = 500, description = "Server error")
    )
)]
pub async fn list_transforms(state: web::Data<AppState>) -> impl Responder {
    let node = state.node.lock().await;
    match node.list_transforms() {
        Ok(map) => HttpResponse::Ok().json(json!({ "data": map })),
        Err(e) => HttpResponse::InternalServerError()
            .json(json!({ "error": format!("Failed to list transforms: {}", e) })),
    }
}

#[utoipa::path(
    post,
    path = "/api/transforms/queue/{id}",
    tag = "query",
    params(
        ("id" = String, Path, description = "Transform id")
    ),
    responses(
        (status = 200, description = "Queued"),
        (status = 500, description = "Server error")
    )
)]
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

#[utoipa::path(
    get,
    path = "/api/transforms/queue",
    tag = "query",
    responses(
        (status = 200, description = "Queue info", body = serde_json::Value),
        (status = 500, description = "Server error")
    )
)]
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
