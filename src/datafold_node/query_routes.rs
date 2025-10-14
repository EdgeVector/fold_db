use super::http_server::AppState;
use crate::schema::types::operations::{Query, Operation};
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
        (status = 200, description = "Array of query result records"),
        (status = 400, description = "Bad request"),
        (status = 500, description = "Server error")
    )
)]
pub async fn execute_query(query: web::Json<Query>, state: web::Data<AppState>) -> impl Responder {
    let node_arc = Arc::clone(&state.node);
    let processor = OperationProcessor::new(node_arc);

    match processor.execute_query_map(query.into_inner()).await {
        Ok(result_map) => {
            let records_map = records_from_field_map(&result_map);
            let data: Vec<Value> = records_map
                .into_iter()
                .map(|(key, record)| json!({"key": key, "fields": record.fields}))
                .collect();
            HttpResponse::Ok().json(data)
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
        (status = 200, description = "Success boolean"),
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
        Ok(result) => HttpResponse::Ok().json(result),
        Err(e) => HttpResponse::InternalServerError()
            .json(json!({"error": format!("Failed to execute mutation: {}", e)})),
    }
}

#[utoipa::path(
    get,
    path = "/api/transforms",
    tag = "query",
    responses(
        (status = 200, description = "Map of transform names to transform objects"),
        (status = 500, description = "Server error")
    )
)]
pub async fn list_transforms(state: web::Data<AppState>) -> impl Responder {
    let node = state.node.lock().await;
    match node.list_transforms() {
        Ok(map) => HttpResponse::Ok().json(map),
        Err(e) => HttpResponse::InternalServerError()
            .json(json!({"error": format!("Failed to list transforms: {}", e)})),
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
        (status = 200, description = "Transform queue information object"),
        (status = 500, description = "Server error")
    )
)]
pub async fn get_transform_queue(state: web::Data<AppState>) -> impl Responder {
    let node = state.node.lock().await;
    match node.get_transform_queue_info() {
        Ok(info) => HttpResponse::Ok().json(info),
        Err(e) => HttpResponse::InternalServerError()
            .json(json!({"error": format!("Failed to get transform queue info: {}", e)})),
    }
}

#[utoipa::path(
    get,
    path = "/api/transforms/backfills",
    tag = "query",
    responses(
        (status = 200, description = "Array of all backfill information objects"),
        (status = 500, description = "Server error")
    )
)]
pub async fn get_all_backfills(state: web::Data<AppState>) -> impl Responder {
    let node = state.node.lock().await;
    match node.get_all_backfills() {
        Ok(backfills) => HttpResponse::Ok().json(backfills),
        Err(e) => HttpResponse::InternalServerError()
            .json(json!({"error": format!("Failed to get backfills: {}", e)})),
    }
}

#[utoipa::path(
    get,
    path = "/api/transforms/backfills/active",
    tag = "query",
    responses(
        (status = 200, description = "Array of active backfill information objects"),
        (status = 500, description = "Server error")
    )
)]
pub async fn get_active_backfills(state: web::Data<AppState>) -> impl Responder {
    let node = state.node.lock().await;
    match node.get_active_backfills() {
        Ok(backfills) => HttpResponse::Ok().json(backfills),
        Err(e) => HttpResponse::InternalServerError()
            .json(json!({"error": format!("Failed to get active backfills: {}", e)})),
    }
}

#[utoipa::path(
    get,
    path = "/api/transforms/backfills/{id}",
    tag = "query",
    params(
        ("id" = String, Path, description = "Transform ID")
    ),
    responses(
        (status = 200, description = "Backfill information object"),
        (status = 404, description = "Backfill not found"),
        (status = 500, description = "Server error")
    )
)]
pub async fn get_backfill(
    path: web::Path<String>,
    state: web::Data<AppState>,
) -> impl Responder {
    let transform_id = path.into_inner();
    let node = state.node.lock().await;
    
    match node.get_backfill(&transform_id) {
        Ok(Some(backfill)) => HttpResponse::Ok().json(backfill),
        Ok(None) => HttpResponse::NotFound()
            .json(json!({"error": format!("Backfill not found for transform: {}", transform_id)})),
        Err(e) => HttpResponse::InternalServerError()
            .json(json!({"error": format!("Failed to get backfill: {}", e)})),
    }
}

#[utoipa::path(
    get,
    path = "/api/transforms/statistics",
    tag = "query",
    responses(
        (status = 200, description = "Transform statistics object"),
        (status = 500, description = "Server error")
    )
)]
pub async fn get_transform_statistics(state: web::Data<AppState>) -> impl Responder {
    let node = state.node.lock().await;
    match node.get_event_statistics() {
        Ok(stats) => HttpResponse::Ok().json(stats),
        Err(e) => HttpResponse::InternalServerError()
            .json(json!({"error": format!("Failed to get statistics: {}", e)})),
    }
}

#[utoipa::path(
    get,
    path = "/api/transforms/backfills/statistics",
    tag = "query",
    responses(
        (status = 200, description = "Aggregate backfill statistics", body = crate::fold_db_core::infrastructure::backfill_tracker::BackfillStatistics),
        (status = 500, description = "Server error")
    )
)]
pub async fn get_backfill_statistics(state: web::Data<AppState>) -> impl Responder {
    let node = state.node.lock().await;
    
    match node.get_all_backfills() {
        Ok(backfills) => {
            use crate::fold_db_core::infrastructure::backfill_tracker::{BackfillStatistics, BackfillStatus};
            
            let active_count = backfills.iter().filter(|b| b.status == BackfillStatus::InProgress).count();
            let completed_count = backfills.iter().filter(|b| b.status == BackfillStatus::Completed).count();
            let failed_count = backfills.iter().filter(|b| b.status == BackfillStatus::Failed).count();
            
            let stats = BackfillStatistics {
                total_backfills: backfills.len(),
                active_backfills: active_count,
                completed_backfills: completed_count,
                failed_backfills: failed_count,
                total_mutations_expected: backfills.iter().map(|b| b.mutations_expected).sum(),
                total_mutations_completed: backfills.iter().map(|b| b.mutations_completed).sum(),
                total_mutations_failed: backfills.iter().map(|b| b.mutations_failed).sum(),
                total_records_produced: backfills.iter().map(|b| b.records_produced).sum(),
            };
            
            HttpResponse::Ok().json(stats)
        },
        Err(e) => HttpResponse::InternalServerError()
            .json(json!({"error": format!("Failed to get backfill statistics: {}", e)})),
    }
}

#[cfg(test)]
mod tests {}
