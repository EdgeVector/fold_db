use crate::logging::LoggingSystem;
use crate::web_logger;
use actix_web::{web, HttpResponse, Responder, Result};
use futures_util::stream::StreamExt;
use serde::{Deserialize, Serialize};
use tokio_stream::wrappers::BroadcastStream; // Keep for backward compatibility

#[derive(Serialize, Deserialize, utoipa::ToSchema)]
pub struct LogLevelUpdate {
    pub feature: String,
    pub level: String,
}

#[derive(Serialize, Deserialize, utoipa::ToSchema)]
pub struct LogConfigResponse {
    pub message: String,
    pub current_level: String,
}

/// List current logs (backward compatibility)
#[utoipa::path(
    get,
    path = "/api/logs",
    tag = "logs",
    responses((status = 200, description = "List logs", body = serde_json::Value))
)]
pub async fn list_logs() -> impl Responder {
    let logs = web_logger::get_logs();
    HttpResponse::Ok().json(serde_json::json!({
        "logs": logs,
        "count": logs.len(),
        "timestamp": std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap_or_default()
            .as_secs()
    }))
}

/// Stream logs via Server-Sent Events (backward compatibility)
#[utoipa::path(
    get,
    path = "/api/logs/stream",
    tag = "logs",
    responses((status = 200, description = "Stream logs"))
)]
pub async fn stream_logs() -> impl Responder {
    let rx = match web_logger::subscribe() {
        Some(r) => r,
        None => return HttpResponse::InternalServerError().finish(),
    };
    let stream = BroadcastStream::new(rx).filter_map(|msg| async move {
        match msg {
            Ok(line) => Some(Ok::<web::Bytes, actix_web::Error>(web::Bytes::from(
                format!("data: {}\n\n", line),
            ))),
            Err(_) => None,
        }
    });
    HttpResponse::Ok()
        .insert_header(("Content-Type", "text/event-stream"))
        .streaming(stream)
}

/// Get current logging configuration
#[utoipa::path(
    get,
    path = "/api/logs/config",
    tag = "logs",
    responses((status = 200, description = "Logging configuration", body = LogConfigResponse))
)]
pub async fn get_config() -> Result<impl Responder> {
    if let Some(config) = LoggingSystem::get_config().await {
        Ok(HttpResponse::Ok().json(serde_json::json!({
            "config": config
        })))
    } else {
        let current_level = log::max_level().to_string();
        Ok(HttpResponse::Ok().json(LogConfigResponse {
            message: "Basic logging configuration".to_string(),
            current_level,
        }))
    }
}

/// Update feature-specific log level at runtime
#[utoipa::path(
    put,
    path = "/api/logs/level",
    tag = "logs",
    request_body = LogLevelUpdate,
    responses(
        (status = 200, description = "Updated"),
        (status = 400, description = "Bad request"),
        (status = 500, description = "Server error")
    )
)]
pub async fn update_feature_level(
    level_update: web::Json<LogLevelUpdate>,
) -> Result<impl Responder> {
    let valid_levels = ["TRACE", "DEBUG", "INFO", "WARN", "ERROR"];
    if !valid_levels.contains(&level_update.level.as_str()) {
        return Ok(HttpResponse::BadRequest().json(serde_json::json!({
            "error": format!("Invalid log level: {}", level_update.level)
        })));
    }

    match LoggingSystem::update_feature_level(&level_update.feature, &level_update.level).await {
        Ok(_) => {
            Ok(HttpResponse::Ok().json(serde_json::json!({
                "success": true,
                "message": format!("Updated {} log level to {}", level_update.feature, level_update.level)
            })))
        }
        Err(e) => {
            Ok(HttpResponse::InternalServerError().json(serde_json::json!({
                "error": format!("Failed to update log level: {}", e)
            })))
        }
    }
}

/// Reload logging configuration from file
#[utoipa::path(
    post,
    path = "/api/logs/config/reload",
    tag = "logs",
    responses((status = 200, description = "Reloaded"), (status = 400, description = "Bad request"))
)]
pub async fn reload_config() -> Result<impl Responder> {
    match LoggingSystem::reload_config_from_file("config/logging.toml").await {
        Ok(_) => Ok(HttpResponse::Ok().json(serde_json::json!({
            "success": true,
            "message": "Configuration reloaded successfully"
        }))),
        Err(e) => Ok(HttpResponse::BadRequest().json(serde_json::json!({
            "error": format!("Failed to reload configuration: {}", e)
        }))),
    }
}

/// Get available log features and their current levels
#[utoipa::path(
    get,
    path = "/api/logs/features",
    tag = "logs",
    responses((status = 200, description = "Features", body = serde_json::Value))
)]
pub async fn get_features() -> Result<impl Responder> {
    if let Some(features) = LoggingSystem::get_features().await {
        Ok(HttpResponse::Ok().json(serde_json::json!({
            "features": features,
            "available_levels": ["TRACE", "DEBUG", "INFO", "WARN", "ERROR"]
        })))
    } else {
        let current_level = log::max_level().to_string();
        Ok(HttpResponse::Ok().json(serde_json::json!({
            "features": {
                "transform": current_level,
                "network": current_level,
                "database": current_level,
                "schema": current_level,
                "query": current_level,
                "mutation": current_level,
                "permissions": current_level,
                "http_server": current_level,
                "tcp_server": current_level,
                "ingestion": current_level
            },
            "available_levels": ["TRACE", "DEBUG", "INFO", "WARN", "ERROR"]
        })))
    }
}
