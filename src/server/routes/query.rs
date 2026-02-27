use crate::handlers::query as query_handlers;
use crate::schema::types::operations::{Operation, Query};
use crate::server::http_server::AppState;
use crate::server::routes::{handler_error_to_response, require_node_read};
use actix_web::{web, HttpResponse, Responder};
use log::{debug, error, info, warn};
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};

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

    let (user_hash, node) = match require_node_read(&state).await {
        Ok(res) => res,
        Err(response) => return response,
    };

    // Use shared handler
    match query_handlers::execute_query(query_inner, &user_hash, &node).await {
        Ok(response) => {
            if let Some(ref data) = response.data {
                if let serde_json::Value::Array(ref arr) = data.results {
                    log::info!("✅ Query completed: {} records returned", arr.len());
                }
            }
            HttpResponse::Ok().json(response)
        }
        Err(e) => {
            log::error!("❌ Query failed: {}", e);
            handler_error_to_response(e)
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

    let (user_hash, node) = match require_node_read(&state).await {
        Ok(res) => res,
        Err(response) => return response,
    };

    log::info!("🚀 Executing mutation via shared handler");
    match crate::handlers::mutation::execute_mutation_from_components(
        schema,
        fields_and_values,
        key_value,
        mutation_type,
        &user_hash,
        &node,
    )
    .await
    {
        Ok(response) => {
            log::info!("✅ Mutation executed successfully");
            HttpResponse::Ok().json(response)
        }
        Err(e) => {
            log::error!("❌ Mutation execution failed: {}", e);
            handler_error_to_response(e)
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
    let (user_hash, node) = match require_node_read(&state).await {
        Ok(res) => res,
        Err(response) => return response,
    };

    match crate::handlers::mutation::execute_mutations_batch_from_json(
        mutations_data.into_inner(),
        &user_hash,
        &node,
    )
    .await
    {
        Ok(response) => HttpResponse::Ok().json(response),
        Err(e) => handler_error_to_response(e),
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
    let (user_hash, node) = match require_node_read(&state).await {
        Ok(res) => res,
        Err(response) => return response,
    };

    match crate::handlers::transform::list_transforms(&user_hash, &node).await {
        Ok(response) => {
            HttpResponse::Ok().json(response.data.map(|d| d.transforms).unwrap_or(json!({})))
        }
        Err(e) => handler_error_to_response(e),
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
    let (user_hash, node) = match require_node_read(&state).await {
        Ok(res) => res,
        Err(response) => return response,
    };

    match crate::handlers::transform::add_to_transform_queue(&transform_id, &user_hash, &node).await
    {
        Ok(response) => HttpResponse::Ok().json(SuccessResponse {
            success: response.data.as_ref().map(|d| d.success).unwrap_or(false),
            message: response
                .data
                .as_ref()
                .map(|d| d.message.clone())
                .unwrap_or_default(),
        }),
        Err(e) => handler_error_to_response(e),
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
    let (user_hash, node) = match require_node_read(&state).await {
        Ok(res) => res,
        Err(response) => return response,
    };

    match crate::handlers::transform::get_transform_queue(&user_hash, &node).await {
        Ok(response) => HttpResponse::Ok().json(json!({
            "length": response.data.as_ref().map(|d| d.length).unwrap_or(0),
            "queued_transforms": response.data.as_ref().map(|d| &d.queued_transforms).unwrap_or(&vec![])
        })),
        Err(e) => handler_error_to_response(e),
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
    let (user_hash, node) = match require_node_read(&state).await {
        Ok(res) => res,
        Err(response) => return response,
    };

    match crate::handlers::transform::get_all_backfills(&user_hash, &node).await {
        Ok(response) => {
            HttpResponse::Ok().json(response.data.map(|d| d.backfills).unwrap_or(json!([])))
        }
        Err(e) => handler_error_to_response(e),
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
    let (user_hash, node) = match require_node_read(&state).await {
        Ok(res) => res,
        Err(response) => return response,
    };

    match crate::handlers::transform::get_active_backfills(&user_hash, &node).await {
        Ok(response) => {
            HttpResponse::Ok().json(response.data.map(|d| d.backfills).unwrap_or(json!([])))
        }
        Err(e) => handler_error_to_response(e),
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
    let (user_hash, node) = match require_node_read(&state).await {
        Ok(res) => res,
        Err(response) => return response,
    };

    match crate::handlers::transform::get_backfill(&transform_id, &user_hash, &node).await {
        Ok(response) => {
            HttpResponse::Ok().json(response.data.map(|d| d.backfill).unwrap_or(json!(null)))
        }
        Err(e) => handler_error_to_response(e),
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
    let (user_hash, node) = match require_node_read(&state).await {
        Ok(res) => res,
        Err(response) => return response,
    };

    match crate::handlers::transform::get_transform_statistics(&user_hash, &node).await {
        Ok(response) => {
            HttpResponse::Ok().json(response.data.map(|d| d.stats).unwrap_or(json!(null)))
        }
        Err(e) => handler_error_to_response(e),
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

    let (user_hash, node) = match require_node_read(&state).await {
        Ok(res) => res,
        Err(response) => return response,
    };

    info!(
        "API: Searching native index for term: '{}', user_hash: '{}'",
        term, user_hash
    );

    // Use shared handler
    debug!("API: Acquired database, calling native_index_search via shared handler");
    match query_handlers::native_index_search(&term, &user_hash, &node).await {
        Ok(response) => {
            if let Some(ref data) = response.data {
                if let serde_json::Value::Array(ref arr) = data.results {
                    info!("API: Search completed, found {} results", arr.len());
                }
            }
            HttpResponse::Ok().json(response)
        }
        Err(e) => {
            error!("API: Search failed: {}", e);
            handler_error_to_response(e)
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
    let (user_hash, node) = match require_node_read(&state).await {
        Ok(res) => res,
        Err(response) => return response,
    };

    match crate::handlers::transform::get_backfill_statistics(&user_hash, &node).await {
        Ok(response) => {
            HttpResponse::Ok().json(response.data.map(|d| d.stats).unwrap_or(json!(null)))
        }
        Err(e) => handler_error_to_response(e),
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
    let (user_hash, node) = match require_node_read(&state).await {
        Ok(res) => res,
        Err(response) => return response,
    };

    match crate::handlers::system::get_indexing_status(&user_hash, &node).await {
        Ok(response) => {
            HttpResponse::Ok().json(response.data.map(|d| d.status).unwrap_or(json!(null)))
        }
        Err(e) => handler_error_to_response(e),
    }
}

/// Get mutation history for a molecule.
#[utoipa::path(
    get,
    path = "/api/history/{molecule_uuid}",
    tag = "query",
    params(
        ("molecule_uuid" = String, Path, description = "Molecule UUID")
    ),
    responses(
        (status = 200, description = "Molecule mutation history"),
        (status = 500, description = "Server error")
    )
)]
pub async fn get_molecule_history(
    path: web::Path<String>,
    state: web::Data<AppState>,
) -> impl Responder {
    let molecule_uuid = path.into_inner();
    let (user_hash, node) = match require_node_read(&state).await {
        Ok(res) => res,
        Err(response) => return response,
    };

    match query_handlers::get_molecule_history(&molecule_uuid, &user_hash, &node).await {
        Ok(response) => HttpResponse::Ok().json(response),
        Err(e) => handler_error_to_response(e),
    }
}

/// Get atom content by UUID.
#[utoipa::path(
    get,
    path = "/api/atom/{atom_uuid}",
    tag = "query",
    params(
        ("atom_uuid" = String, Path, description = "Atom UUID")
    ),
    responses(
        (status = 200, description = "Atom content"),
        (status = 404, description = "Atom not found"),
        (status = 500, description = "Server error")
    )
)]
pub async fn get_atom_content(
    path: web::Path<String>,
    state: web::Data<AppState>,
) -> impl Responder {
    let atom_uuid = path.into_inner();
    let (user_hash, node) = match require_node_read(&state).await {
        Ok(res) => res,
        Err(response) => return response,
    };

    match query_handlers::get_atom_content(&atom_uuid, &user_hash, &node).await {
        Ok(response) => HttpResponse::Ok().json(response),
        Err(e) => handler_error_to_response(e),
    }
}

/// Get process results for a progress_id (actual stored keys from ingestion mutations).
pub async fn get_process_results(
    path: web::Path<String>,
    state: web::Data<AppState>,
) -> impl Responder {
    let progress_id = path.into_inner();
    let (user_hash, node) = match require_node_read(&state).await {
        Ok(res) => res,
        Err(response) => return response,
    };

    match query_handlers::get_process_results(&progress_id, &user_hash, &node).await {
        Ok(response) => HttpResponse::Ok().json(response),
        Err(e) => handler_error_to_response(e),
    }
}

#[cfg(test)]
mod tests {}
