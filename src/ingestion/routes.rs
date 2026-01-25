//! HTTP route handlers for the ingestion API

use crate::ingestion::config::{IngestionConfig, SavedConfig};
use crate::ingestion::core::IngestionRequest;
use crate::ingestion::progress::ProgressService;
use crate::ingestion::simple_service::SimpleIngestionService;
use crate::ingestion::IngestionResponse;
use crate::ingestion::ProgressTracker;
use crate::log_feature;
use crate::logging::features::LogFeature;
use crate::server::http_server::AppState;
use actix_web::{web, HttpResponse, Responder};
use serde_json::{json, Value};

/// Process JSON ingestion request
#[utoipa::path(
    post,
    path = "/api/ingestion/process",
    tag = "ingestion",
    request_body = IngestionRequest,
    responses((status = 200, description = "Ingestion response", body = IngestionResponse))
)]
pub async fn process_json(
    request: web::Json<IngestionRequest>,
    progress_tracker: web::Data<ProgressTracker>,
    state: web::Data<AppState>,
) -> impl Responder {
    log_feature!(
        LogFeature::Ingestion,
        info,
        "Received JSON ingestion request"
    );

    // Generate a unique progress ID
    let progress_id = uuid::Uuid::new_v4().to_string();

    // Start progress tracking
    // Start progress tracking
    let user_id = match crate::logging::core::get_current_user_id() {
        Some(uid) => uid,
        None => {
            return HttpResponse::Unauthorized().json(IngestionResponse::failure(vec![
                "User not authenticated".to_string(),
            ]))
        }
    };

    let progress_service = ProgressService::new(progress_tracker.get_ref().clone());
    progress_service
        .start_progress(progress_id.clone(), user_id)
        .await;

    // Try to create a simple ingestion service
    let service = match create_simple_ingestion_service().await {
        Ok(service) => service,
        Err(e) => {
            log_feature!(
                LogFeature::Ingestion,
                error,
                "Failed to initialize ingestion service: {}",
                e
            );
            progress_service
                .fail_progress(
                    &progress_id,
                    format!("Ingestion service not available: {}", e),
                )
                .await;
            return HttpResponse::ServiceUnavailable().json(IngestionResponse::failure(vec![
                format!("Ingestion service not available: {}", e),
            ]));
        }
    };

    // Spawn ingestion as a background task and return immediately with progress_id
    let node_clone = state.node.clone();
    let request_data = request.into_inner();
    let progress_id_clone = progress_id.clone();
    let user_id_for_task =
        crate::logging::core::get_current_user_id().unwrap_or_else(|| "unknown".to_string());

    tokio::spawn(async move {
        // Wrap in run_with_user to propagate user context for progress tracking
        crate::logging::core::run_with_user(&user_id_for_task, async move {
            log_feature!(
                LogFeature::Ingestion,
                info,
                "Starting background ingestion with progress_id: {}",
                progress_id_clone
            );

            // Acquire read lock on the node
            let node_guard = node_clone.read().await;

            match service
                .process_json_with_node_and_progress(
                    request_data,
                    &node_guard,
                    &progress_service,
                    progress_id_clone.clone(),
                )
                .await
            {
                Ok(response) => {
                    if response.success {
                        log_feature!(
                            LogFeature::Ingestion,
                            info,
                            "Background ingestion completed successfully: {}",
                            progress_id_clone
                        );
                    } else {
                        log_feature!(
                            LogFeature::Ingestion,
                            error,
                            "Background ingestion failed: {:?}",
                            response.errors
                        );
                    }
                }
                Err(e) => {
                    log_feature!(
                        LogFeature::Ingestion,
                        error,
                        "Background ingestion processing failed: {}",
                        e
                    );
                    progress_service
                        .fail_progress(&progress_id_clone, format!("Processing failed: {}", e))
                        .await;
                }
            }
        })
        .await
    });

    // Return immediately with the progress_id so frontend can start polling
    log_feature!(
        LogFeature::Ingestion,
        info,
        "Returning progress_id to client: {}",
        progress_id
    );

    HttpResponse::Accepted().json(serde_json::json!({
        "success": true,
        "progress_id": progress_id,
        "message": "Ingestion started. Use progress_id to track status."
    }))
}

/// Get ingestion status
#[utoipa::path(
    get,
    path = "/api/ingestion/status",
    tag = "ingestion",
    responses((status = 200, description = "Ingestion status", body = crate::ingestion::IngestionStatus))
)]
pub async fn get_status() -> impl Responder {
    log_feature!(
        LogFeature::Ingestion,
        debug,
        "Received ingestion status request"
    );

    match create_simple_ingestion_service().await {
        Ok(service) => match service.get_status() {
            Ok(status) => HttpResponse::Ok().json(status),
            Err(e) => {
                log_feature!(
                    LogFeature::Ingestion,
                    error,
                    "Failed to get ingestion status: {}",
                    e
                );
                HttpResponse::InternalServerError().json(json!({
                    "error": format!("Failed to get status: {}", e)
                }))
            }
        },
        Err(e) => {
            log_feature!(
                LogFeature::Ingestion,
                warn,
                "Ingestion service not available: {}",
                e
            );
            HttpResponse::ServiceUnavailable().json(json!({
                "error": format!("Ingestion service not available: {}", e),
                "enabled": false,
                "configured": false
            }))
        }
    }
}

/// Health check endpoint for ingestion service
#[utoipa::path(
    get,
    path = "/api/ingestion/health",
    tag = "ingestion",
    responses((status = 200, description = "Health OK", body = Value), (status = 503, description = "Health not OK", body = Value))
)]
pub async fn health_check() -> impl Responder {
    match create_simple_ingestion_service().await {
        Ok(service) => {
            let status = service.get_status();

            match status {
                Ok(ingestion_status) => {
                    let is_healthy = ingestion_status.enabled && ingestion_status.configured;

                    if is_healthy {
                        HttpResponse::Ok().json(json!({
                            "status": "healthy",
                            "service": "ingestion",
                            "details": ingestion_status
                        }))
                    } else {
                        HttpResponse::ServiceUnavailable().json(json!({
                            "status": "unhealthy",
                            "service": "ingestion",
                            "details": ingestion_status
                        }))
                    }
                }
                Err(e) => HttpResponse::ServiceUnavailable().json(json!({
                    "status": "error",
                    "service": "ingestion",
                    "error": e.to_string()
                })),
            }
        }
        Err(e) => HttpResponse::ServiceUnavailable().json(json!({
            "status": "unavailable",
            "service": "ingestion",
            "error": e.to_string()
        })),
    }
}

/// Validate JSON data without processing
#[utoipa::path(
    post,
    path = "/api/ingestion/validate",
    tag = "ingestion",
    request_body = Value,
    responses((status = 200, description = "Validation result", body = Value), (status = 400, description = "Invalid"))
)]
pub async fn validate_json(request: web::Json<Value>) -> impl Responder {
    log_feature!(
        LogFeature::Ingestion,
        info,
        "Received JSON validation request"
    );

    match create_simple_ingestion_service().await {
        Ok(service) => match service.validate_input(&request.into_inner()) {
            Ok(()) => HttpResponse::Ok().json(json!({
                "valid": true,
                "message": "JSON data is valid for ingestion"
            })),
            Err(e) => HttpResponse::BadRequest().json(json!({
                "valid": false,
                "error": format!("Validation failed: {}", e)
            })),
        },
        Err(e) => HttpResponse::ServiceUnavailable().json(json!({
            "valid": false,
            "error": format!("Ingestion service not available: {}", e)
        })),
    }
}

/// Get Ingestion configuration
#[utoipa::path(
    get,
    path = "/api/ingestion/config",
    tag = "ingestion",
    responses((status = 200, description = "Ingestion config", body = IngestionConfig))
)]
pub async fn get_ingestion_config() -> impl Responder {
    log_feature!(
        LogFeature::Ingestion,
        debug,
        "Received ingestion config request"
    );

    let mut config = IngestionConfig::from_env_allow_empty();

    // Don't return the actual API key for security, just indicate if it's set
    if !config.openrouter.api_key.is_empty() {
        config.openrouter.api_key = "***configured***".to_string();
    }

    HttpResponse::Ok().json(config)
}

/// Save Ingestion configuration
#[utoipa::path(
    post,
    path = "/api/ingestion/config",
    tag = "ingestion",
    request_body = SavedConfig,
    responses((status = 200, description = "Saved"), (status = 500, description = "Failed"))
)]
pub async fn save_ingestion_config(request: web::Json<SavedConfig>) -> impl Responder {
    log_feature!(
        LogFeature::Ingestion,
        info,
        "Received ingestion config save request"
    );

    let config = request.into_inner();

    match IngestionConfig::save_to_file(&config) {
        Ok(()) => {
            log_feature!(
                LogFeature::Ingestion,
                info,
                "Ingestion configuration saved successfully"
            );
            HttpResponse::Ok().json(json!({
                "success": true,
                "message": "Configuration saved successfully"
            }))
        }
        Err(e) => {
            log_feature!(
                LogFeature::Ingestion,
                error,
                "Failed to save ingestion config: {}",
                e
            );
            HttpResponse::InternalServerError().json(json!({
                "success": false,
                "error": format!("Failed to save configuration: {}", e)
            }))
        }
    }
}

/// Create a simple ingestion service with potentially updated config
async fn create_simple_ingestion_service(
) -> Result<SimpleIngestionService, crate::ingestion::IngestionError> {
    let config = IngestionConfig::from_env()?;
    SimpleIngestionService::new(config)
}

/// Get ingestion progress by ID
#[utoipa::path(
    get,
    path = "/api/ingestion/progress/{id}",
    tag = "ingestion",
    responses((status = 200, description = "Progress information", body = IngestionProgress), (status = 404, description = "Progress not found"))
)]
pub async fn get_progress(
    path: web::Path<String>,
    progress_tracker: web::Data<ProgressTracker>,
) -> impl Responder {
    let id = path.into_inner();

    log_feature!(
        LogFeature::Ingestion,
        debug,
        "Received progress request for ID: {}",
        id
    );

    // Get progress tracker from data
    let progress_service = ProgressService::new(progress_tracker.get_ref().clone());

    match progress_service.get_progress(&id).await {
        Some(progress) => HttpResponse::Ok().json(progress),
        None => {
            log_feature!(
                LogFeature::Ingestion,
                warn,
                "Progress not found for ID: {}",
                id
            );
            HttpResponse::NotFound().json(json!({
                "error": "Progress not found",
                "id": id
            }))
        }
    }
}

/// Get all active ingestion progress
#[utoipa::path(
    get,
    path = "/api/ingestion/progress",
    tag = "ingestion",
    responses((status = 200, description = "All active progress", body = Vec<IngestionProgress>))
)]
pub async fn get_all_progress(progress_tracker: web::Data<ProgressTracker>) -> impl Responder {
    log_feature!(
        LogFeature::Ingestion,
        debug,
        "Received request for all progress"
    );

    let progress_service = ProgressService::new(progress_tracker.get_ref().clone());
    let all_progress = progress_service.get_all_progress().await;

    HttpResponse::Ok().json(all_progress)
}

#[cfg(test)]
mod tests {
    use super::*;
    use actix_web::{test, App};

    #[actix_web::test]
    async fn test_get_status() {
        let app = test::init_service(App::new().route("/status", web::get().to(get_status))).await;

        let req = test::TestRequest::get().uri("/status").to_request();
        let resp = test::call_service(&app, req).await;
        // Should return service unavailable if not configured
        assert!(resp.status().is_server_error() || resp.status().is_success());
    }

    #[actix_web::test]
    async fn test_health_check() {
        let app =
            test::init_service(App::new().route("/health", web::get().to(health_check))).await;

        let req = test::TestRequest::get().uri("/health").to_request();
        let resp = test::call_service(&app, req).await;
        assert!(resp.status().is_server_error() || resp.status().is_success());
    }

    #[actix_web::test]
    async fn test_get_ingestion_config() {
        let app =
            test::init_service(App::new().route("/config", web::get().to(get_ingestion_config)))
                .await;

        let req = test::TestRequest::get().uri("/config").to_request();
        let resp = test::call_service(&app, req).await;
        assert!(resp.status().is_success());
    }
}
