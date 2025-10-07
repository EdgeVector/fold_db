//! HTTP route handlers for the ingestion API

use crate::datafold_node::http_server::AppState;
use crate::ingestion::config::{IngestionConfig, SavedConfig};
use crate::ingestion::core::IngestionRequest;
use crate::ingestion::simple_service::SimpleIngestionService;
use crate::ingestion::IngestionResponse;
use crate::log_feature;
use crate::logging::features::LogFeature;
use actix_web::{web, HttpResponse, Responder};
use serde_json::json;
use std::fs;
use std::path::Path;
use std::sync::Arc;

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
    state: web::Data<AppState>,
) -> impl Responder {
    log_feature!(
        LogFeature::Ingestion,
        info,
        "Received JSON ingestion request"
    );

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
            return HttpResponse::ServiceUnavailable().json(IngestionResponse::failure(vec![
                format!("Ingestion service not available: {}", e),
            ]));
        }
    };

    // Process the ingestion request
    match service
        .process_json_with_node(request.into_inner(), Arc::clone(&state.node))
        .await
    {
        Ok(response) => {
            if response.success {
                log_feature!(
                    LogFeature::Ingestion,
                    info,
                    "Ingestion completed successfully"
                );
                HttpResponse::Ok().json(response)
            } else {
                log_feature!(
                    LogFeature::Ingestion,
                    error,
                    "Ingestion failed: {:?}",
                    response.errors
                );
                HttpResponse::InternalServerError().json(response)
            }
        }
        Err(e) => {
            log_feature!(
                LogFeature::Ingestion,
                error,
                "Ingestion processing failed: {}",
                e
            );
            HttpResponse::InternalServerError().json(IngestionResponse::failure(vec![format!(
                "Processing failed: {}",
                e
            )]))
        }
    }
}

/// Get ingestion status
#[utoipa::path(
    get,
    path = "/api/ingestion/status",
    tag = "ingestion",
    responses((status = 200, description = "Ingestion status", body = serde_json::Value))
)]
pub async fn get_status(_state: web::Data<AppState>) -> impl Responder {
    log_feature!(
        LogFeature::Ingestion,
        info,
        "Received ingestion status request"
    );

    match create_simple_ingestion_service().await {
        Ok(service) => match service.get_status() {
            Ok(status) => {
                log_feature!(LogFeature::Ingestion, info, "Returning ingestion status");
                HttpResponse::Ok().json(status)
            }
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
    responses((status = 200, description = "Health OK", body = serde_json::Value), (status = 503, description = "Health not OK", body = serde_json::Value))
)]
pub async fn health_check(_state: web::Data<AppState>) -> impl Responder {
    match create_simple_ingestion_service().await {
        Ok(service) => {
            let status = service.get_status().unwrap_or_else(|_| {
                json!({
                    "enabled": false,
                    "configured": false
                })
            });

            let is_healthy = status
                .get("enabled")
                .and_then(|v| v.as_bool())
                .unwrap_or(false)
                && status
                    .get("configured")
                    .and_then(|v| v.as_bool())
                    .unwrap_or(false);

            if is_healthy {
                HttpResponse::Ok().json(json!({
                    "status": "healthy",
                    "service": "ingestion",
                    "details": status
                }))
            } else {
                HttpResponse::ServiceUnavailable().json(json!({
                    "status": "unhealthy",
                    "service": "ingestion",
                    "details": status
                }))
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
    request_body = serde_json::Value,
    responses((status = 200, description = "Validation result", body = serde_json::Value), (status = 400, description = "Invalid"))
)]
pub async fn validate_json(
    request: web::Json<serde_json::Value>,
    _state: web::Data<AppState>,
) -> impl Responder {
    log_feature!(
        LogFeature::Ingestion,
        info,
        "Received JSON validation request"
    );

    match create_simple_ingestion_service().await {
        Ok(service) => match service.validate_input(&request.into_inner()) {
            Ok(()) => {
                log_feature!(LogFeature::Ingestion, info, "JSON validation successful");
                HttpResponse::Ok().json(json!({
                    "valid": true,
                    "message": "JSON data is valid for ingestion"
                }))
            }
            Err(e) => {
                log_feature!(LogFeature::Ingestion, info, "JSON validation failed: {}", e);
                HttpResponse::BadRequest().json(json!({
                    "valid": false,
                    "error": format!("Validation failed: {}", e)
                }))
            }
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
pub async fn get_ingestion_config(_state: web::Data<AppState>) -> impl Responder {
    log_feature!(
        LogFeature::Ingestion,
        info,
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
pub async fn save_ingestion_config(
    request: web::Json<SavedConfig>,
    _state: web::Data<AppState>,
) -> impl Responder {
    log_feature!(
        LogFeature::Ingestion,
        info,
        "Received ingestion config save request"
    );

    let config = request.into_inner();

    match save_config_to_file(&config) {
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

/// Save Ingestion configuration to file
fn save_config_to_file(config: &SavedConfig) -> Result<(), Box<dyn std::error::Error>> {
    let config_path = get_config_file_path();

    // Create directory if it doesn't exist
    if let Some(parent) = config_path.parent() {
        fs::create_dir_all(parent)?;
    }

    let content = serde_json::to_string_pretty(config)?;
    fs::write(&config_path, content)?;

    log_feature!(
        LogFeature::Ingestion,
        info,
        "Ingestion config saved to: {:?}",
        config_path
    );
    Ok(())
}

/// Get the path to the ingestion configuration file
fn get_config_file_path() -> std::path::PathBuf {
    let config_dir =
        std::env::var("DATAFOLD_CONFIG_DIR").unwrap_or_else(|_| "./config".to_string());

    Path::new(&config_dir).join("ingestion_config.json")
}

/// Create a simple ingestion service with potentially updated config
async fn create_simple_ingestion_service(
) -> Result<SimpleIngestionService, crate::ingestion::IngestionError> {
    let config = IngestionConfig::from_env()?;
    SimpleIngestionService::new(config)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::datafold_node::{DataFoldNode, NodeConfig};
    use actix_web::{test, App};
    use std::sync::Arc;
    use tempfile::tempdir;

    async fn create_test_app_state() -> web::Data<AppState> {
        let temp_dir = tempdir().unwrap();
        let config = NodeConfig::new(temp_dir.path().to_path_buf());
        let node = DataFoldNode::new(config).unwrap();

        web::Data::new(AppState {
            node: Arc::new(tokio::sync::Mutex::new(node)),
        })
    }

    #[actix_web::test]
    async fn test_get_status() {
        let app_state = create_test_app_state().await;
        let app = test::init_service(
            App::new()
                .app_data(app_state)
                .route("/status", web::get().to(get_status)),
        )
        .await;

        let req = test::TestRequest::get().uri("/status").to_request();
        let resp = test::call_service(&app, req).await;
        // Should return service unavailable if not configured
        assert!(resp.status().is_server_error() || resp.status().is_success());
    }

    #[actix_web::test]
    async fn test_health_check() {
        let app_state = create_test_app_state().await;
        let app = test::init_service(
            App::new()
                .app_data(app_state)
                .route("/health", web::get().to(health_check)),
        )
        .await;

        let req = test::TestRequest::get().uri("/health").to_request();
        let resp = test::call_service(&app, req).await;
        assert!(resp.status().is_server_error() || resp.status().is_success());
    }

    #[actix_web::test]
    async fn test_get_ingestion_config() {
        let app_state = create_test_app_state().await;
        let app = test::init_service(
            App::new()
                .app_data(app_state)
                .route("/config", web::get().to(get_ingestion_config)),
        )
        .await;

        let req = test::TestRequest::get().uri("/config").to_request();
        let resp = test::call_service(&app, req).await;
        assert!(resp.status().is_success());
    }
}
