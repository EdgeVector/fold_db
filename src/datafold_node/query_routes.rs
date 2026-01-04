use super::http_server::AppState;
use crate::datafold_node::OperationProcessor;
use crate::fold_db_core::query::records_from_field_map;
use crate::schema::types::operations::{Operation, Query};
use actix_web::{web, HttpResponse, Responder};
use log::{debug, error, info, warn};
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
use std::sync::Arc;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SuccessResponse {
    pub success: bool,
    pub message: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, utoipa::ToSchema)]
pub struct MutationResponse {
    pub mutation_id: String,
}

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
    let query_inner = query.into_inner();
    log::info!(
        "🔍 execute_query: schema={}, fields={:?}, filter={:?}",
        query_inner.schema_name,
        query_inner.fields,
        query_inner.filter
    );

    let node_arc = Arc::clone(&state.node);
    let processor = OperationProcessor::new(node_arc);

    match processor.execute_query_map(query_inner).await {
        Ok(result_map) => {
            log::debug!("✅ Query returned {} fields", result_map.len());
            let records_map = records_from_field_map(&result_map);
            let data: Vec<Value> = records_map
                .into_iter()
                .map(|(key, record)| json!({"key": key, "fields": record.fields, "metadata": record.metadata}))
                .collect();
            log::info!("✅ Query completed: {} records returned", data.len());
            HttpResponse::Ok().json(data)
        }
        Err(e) => {
            log::error!("❌ Query failed: {}", e);
            HttpResponse::InternalServerError()
                .json(json!({"error": format!("Failed to execute query: {}", e)}))
        }
    }
}

/// Execute a mutation.
#[utoipa::path(
    post,
    path = "/api/mutation",
    tag = "query",
    request_body = serde_json::Value,
    responses(
        (status = 200, description = "Mutation accepted", body = MutationResponse),
        (status = 400, description = "Bad request"),
        (status = 500, description = "Server error")
    )
)]
pub async fn execute_mutation(
    mutation_data: web::Json<Value>,
    state: web::Data<AppState>,
) -> impl Responder {
    log::info!("📥 Received mutation request");

    let (schema, fields_and_values, key_value, mutation_type) =
        match serde_json::from_value::<Operation>(mutation_data.into_inner()) {
            Ok(Operation::Mutation {
                schema,
                fields_and_values,
                key_value,
                mutation_type,
                source_file_name: _,
            }) => {
                log::info!(
                    "✅ Parsed mutation: schema={}, type={:?}, fields={}",
                    schema,
                    mutation_type,
                    fields_and_values.len()
                );
                (schema, fields_and_values, key_value, mutation_type)
            }
            Err(e) => {
                log::error!("❌ Failed to parse mutation: {}", e);
                return HttpResponse::BadRequest()
                    .json(json!({"error": format!("Failed to parse mutation: {}", e)}));
            }
        };

    let node_arc = Arc::clone(&state.node);
    let processor = OperationProcessor::new(node_arc);

    log::info!("🚀 Executing mutation via OperationProcessor");
    match processor
        .execute_mutation(schema, fields_and_values, key_value, mutation_type)
        .await
    {
        Ok(mutation_id) => {
            log::info!("✅ Mutation executed successfully: {}", mutation_id);
            HttpResponse::Ok().json(json!({"mutation_id": mutation_id, "success": true}))
        }
        Err(e) => {
            log::error!("❌ Mutation execution failed: {}", e);
            HttpResponse::InternalServerError().json(
                json!({"error": format!("Failed to execute mutation: {}", e), "success": false}),
            )
        }
    }
}

/// Execute multiple mutations in a batch for improved performance.
#[utoipa::path(
    post,
    path = "/api/mutations/batch",
    tag = "query",
    request_body = Vec<serde_json::Value>,
    responses(
        (status = 200, description = "Array of mutation IDs"),
        (status = 400, description = "Bad request"),
        (status = 500, description = "Server error")
    )
)]
pub async fn execute_mutations_batch(
    mutations_data: web::Json<Vec<Value>>,
    state: web::Data<AppState>,
) -> impl Responder {
    let node_arc = Arc::clone(&state.node);
    let processor = OperationProcessor::new(node_arc);

    match processor
        .execute_mutations_batch(mutations_data.into_inner())
        .await
    {
        Ok(mutation_ids) => HttpResponse::Ok().json(mutation_ids),
        Err(e) => HttpResponse::InternalServerError()
            .json(json!({"error": format!("Failed to execute batch mutations: {}", e)})),
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
    match node.list_transforms().await {
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

    match node.add_transform_to_queue(&transform_id).await {
        Ok(_) => HttpResponse::Ok().json(SuccessResponse {
            success: true,
            message: format!("Transform '{}' added to queue", transform_id),
        }),
        Err(e) => HttpResponse::InternalServerError()
            .json(json!({"error": format!("Failed to add transform to queue: {}", e)})),
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
    match node.get_transform_queue_info().await {
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
    match node.get_all_backfills().await {
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
    match node.get_active_backfills().await {
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
pub async fn get_backfill(path: web::Path<String>, state: web::Data<AppState>) -> impl Responder {
    let transform_id = path.into_inner();
    let node = state.node.lock().await;

    match node.get_backfill(&transform_id).await {
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
    match node.get_event_statistics().await {
        Ok(stats) => HttpResponse::Ok().json(stats),
        Err(e) => HttpResponse::InternalServerError()
            .json(json!({"error": format!("Failed to get statistics: {}", e)})),
    }
}

/// Search the native word index for a term.
#[utoipa::path(
    get,
    path = "/api/native-index/search",
    tag = "query",
    params(
        ("term" = String, Query, description = "Search term for native word index")
    ),
    responses(
        (status = 200, description = "Array of native index results", body = [crate::db_operations::IndexResult]),
        (status = 400, description = "Bad request"),
        (status = 500, description = "Server error")
    )
)]
pub async fn native_index_search(
    query: web::Query<std::collections::HashMap<String, String>>,
    state: web::Data<AppState>,
) -> impl Responder {
    info!("API: native_index_search endpoint called");

    let term = match query.get("term") {
        Some(t) if !t.trim().is_empty() => t.trim().to_string(),
        _ => {
            warn!("API: Missing or empty term parameter");
            return HttpResponse::BadRequest()
                .json(json!({"error": "Missing required 'term' query parameter"}));
        }
    };

    info!("API: Searching for term: '{}'", term);

    // Acquire FoldDB and perform search
    let node_arc = Arc::clone(&state.node);
    let node_guard = node_arc.lock().await;
    let fold_db = match node_guard.get_fold_db().await {
        Ok(guard) => guard,
        Err(e) => {
            error!("API: Failed to acquire database: {}", e);
            return HttpResponse::InternalServerError()
                .json(json!({"error": format!("Failed to acquire database: {}", e)}));
        }
    };

    debug!("API: Acquired database, calling native_search_all_classifications");
    match fold_db.native_search_all_classifications(&term).await {
        Ok(results) => {
            info!("API: Search completed, found {} results", results.len());
            HttpResponse::Ok().json(results)
        }
        Err(e) => {
            error!("API: Search failed: {}", e);
            HttpResponse::InternalServerError()
                .json(json!({"error": format!("Native index search failed: {}", e)}))
        }
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

    match node.get_all_backfills().await {
        Ok(backfills) => {
            use crate::fold_db_core::infrastructure::backfill_tracker::{
                BackfillStatistics, BackfillStatus,
            };

            let active_count = backfills
                .iter()
                .filter(|b| b.status == BackfillStatus::InProgress)
                .count();
            let completed_count = backfills
                .iter()
                .filter(|b| b.status == BackfillStatus::Completed)
                .count();
            let failed_count = backfills
                .iter()
                .filter(|b| b.status == BackfillStatus::Failed)
                .count();

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
        }
        Err(e) => HttpResponse::InternalServerError()
            .json(json!({"error": format!("Failed to get backfill statistics: {}", e)})),
    }
}

/// Get indexing status
#[utoipa::path(
    get,
    path = "/api/indexing/status",
    tag = "system",
    responses(
        (status = 200, description = "Current indexing status", body = IndexingStatus),
        (status = 500, description = "Server error")
    )
)]
pub async fn get_indexing_status(state: web::Data<AppState>) -> impl Responder {
    let node = state.node.lock().await;
    let status = node.get_indexing_status().await;
    HttpResponse::Ok().json(status)
}

#[cfg(test)]
mod tests {}
