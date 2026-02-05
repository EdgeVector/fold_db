//! HTTP route handlers for the ingestion API

use crate::ingestion::config::{IngestionConfig, SavedConfig};
use crate::ingestion::core::IngestionRequest;
use crate::ingestion::progress::ProgressService;
use crate::ingestion::ingestion_service::IngestionService;
use crate::ingestion::IngestionResponse;
use crate::ingestion::ProgressTracker;
use crate::log_feature;
use crate::logging::features::LogFeature;
use crate::server::http_server::AppState;
use crate::server::routes::require_node;
use actix_web::{web, HttpResponse, Responder};
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
use std::path::Path;

/// Request for batch folder ingestion
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BatchFolderRequest {
    /// Path to the folder (relative to project root or absolute)
    pub folder_path: String,
    /// Optional schema hint for all files
    pub schema_hint: Option<String>,
    /// Whether to auto-execute mutations (default: true)
    pub auto_execute: Option<bool>,
}

/// Response for batch folder ingestion
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BatchFolderResponse {
    pub success: bool,
    pub batch_id: String,
    pub files_found: usize,
    pub file_progress_ids: Vec<FileProgressInfo>,
    pub message: String,
}

/// Progress info for a single file in a batch
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FileProgressInfo {
    pub file_name: String,
    pub progress_id: String,
}

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

    // Use client-provided progress_id if available, otherwise generate one
    let progress_id = request
        .progress_id
        .clone()
        .unwrap_or_else(|| uuid::Uuid::new_v4().to_string());

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
    let service = match create_ingestion_service().await {
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

    // Get user and node from NodeManager
    let (user_id_for_task, node_arc) = match require_node(&state).await {
        Ok(res) => res,
        Err(response) => return response,
    };

    let request_data = request.into_inner();
    let progress_id_clone = progress_id.clone();

    tokio::spawn(async move {
        // Wrap in run_with_user to propagate user context for progress tracking
        crate::logging::core::run_with_user(&user_id_for_task, async move {
            log_feature!(
                LogFeature::Ingestion,
                info,
                "Starting background ingestion with progress_id: {}",
                progress_id_clone
            );

            // Acquire lock on the node
            let node_guard = node_arc.lock().await;

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

    match create_ingestion_service().await {
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
    match create_ingestion_service().await {
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

    match create_ingestion_service().await {
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
async fn create_ingestion_service(
) -> Result<IngestionService, crate::ingestion::IngestionError> {
    let config = IngestionConfig::from_env()?;
    IngestionService::new(config)
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

    // Get user from context - required for multi-tenancy
    let user_hash = match crate::server::routes::require_user_context() {
        Ok(hash) => hash,
        Err(response) => return response,
    };

    // Use shared handler
    match crate::handlers::ingestion::get_all_progress(&user_hash, progress_tracker.get_ref()).await
    {
        Ok(response) => HttpResponse::Ok().json(response),
        Err(e) => {
            let status_code = match e.status_code() {
                400 => actix_web::http::StatusCode::BAD_REQUEST,
                401 => actix_web::http::StatusCode::UNAUTHORIZED,
                404 => actix_web::http::StatusCode::NOT_FOUND,
                503 => actix_web::http::StatusCode::SERVICE_UNAVAILABLE,
                _ => actix_web::http::StatusCode::INTERNAL_SERVER_ERROR,
            };
            HttpResponse::build(status_code).json(e.to_response())
        }
    }
}

/// Batch ingest all files from a folder
#[utoipa::path(
    post,
    path = "/api/ingestion/batch-folder",
    tag = "ingestion",
    request_body = BatchFolderRequest,
    responses((status = 202, description = "Batch ingestion started", body = BatchFolderResponse), (status = 400, description = "Invalid folder path"))
)]
pub async fn batch_folder_ingest(
    request: web::Json<BatchFolderRequest>,
    progress_tracker: web::Data<ProgressTracker>,
    state: web::Data<AppState>,
) -> impl Responder {
    log_feature!(
        LogFeature::Ingestion,
        info,
        "Received batch folder ingestion request for: {}",
        request.folder_path
    );

    // Get user context
    let user_id = match crate::logging::core::get_current_user_id() {
        Some(uid) => uid,
        None => {
            return HttpResponse::Unauthorized().json(json!({
                "success": false,
                "error": "User not authenticated"
            }))
        }
    };

    // Resolve folder path - support both absolute and relative paths
    let folder_path = if Path::new(&request.folder_path).is_absolute() {
        std::path::PathBuf::from(&request.folder_path)
    } else {
        // Relative to project root
        std::env::current_dir()
            .unwrap_or_default()
            .join(&request.folder_path)
    };

    // Validate folder exists
    if !folder_path.exists() {
        return HttpResponse::BadRequest().json(json!({
            "success": false,
            "error": format!("Folder not found: {}", folder_path.display())
        }));
    }

    if !folder_path.is_dir() {
        return HttpResponse::BadRequest().json(json!({
            "success": false,
            "error": format!("Path is not a directory: {}", folder_path.display())
        }));
    }

    // List supported files in the folder
    let supported_extensions = ["json", "csv", "txt", "md"];
    let mut files_to_ingest: Vec<std::path::PathBuf> = Vec::new();

    match std::fs::read_dir(&folder_path) {
        Ok(entries) => {
            for entry in entries.flatten() {
                let path = entry.path();
                if path.is_file() {
                    if let Some(ext) = path.extension().and_then(|e| e.to_str()) {
                        if supported_extensions.contains(&ext.to_lowercase().as_str()) {
                            files_to_ingest.push(path);
                        }
                    }
                }
            }
        }
        Err(e) => {
            return HttpResponse::BadRequest().json(json!({
                "success": false,
                "error": format!("Failed to read folder: {}", e)
            }));
        }
    }

    if files_to_ingest.is_empty() {
        return HttpResponse::BadRequest().json(json!({
            "success": false,
            "error": "No supported files found in folder (supported: .json, .csv, .txt, .md)"
        }));
    }

    // Generate batch ID
    let batch_id = uuid::Uuid::new_v4().to_string();

    // Create progress tracking for each file
    let progress_service = ProgressService::new(progress_tracker.get_ref().clone());
    let mut file_progress_ids: Vec<FileProgressInfo> = Vec::new();

    for file_path in &files_to_ingest {
        let progress_id = uuid::Uuid::new_v4().to_string();
        let file_name = file_path
            .file_name()
            .and_then(|n| n.to_str())
            .unwrap_or("unknown")
            .to_string();

        progress_service
            .start_progress(progress_id.clone(), user_id.clone())
            .await;

        file_progress_ids.push(FileProgressInfo {
            file_name,
            progress_id,
        });
    }

    // Try to create ingestion service
    let service = match create_ingestion_service().await {
        Ok(service) => service,
        Err(e) => {
            log_feature!(
                LogFeature::Ingestion,
                error,
                "Failed to initialize ingestion service: {}",
                e
            );
            // Mark all progress as failed
            for info in &file_progress_ids {
                progress_service
                    .fail_progress(&info.progress_id, format!("Ingestion service not available: {}", e))
                    .await;
            }
            return HttpResponse::ServiceUnavailable().json(json!({
                "success": false,
                "error": format!("Ingestion service not available: {}", e)
            }));
        }
    };

    // Get node for processing
    let (user_id_for_task, node_arc) = match require_node(&state).await {
        Ok(res) => res,
        Err(response) => return response,
    };

    // Get the node's public key for mutation generation
    let node_public_key = {
        let node_guard = node_arc.lock().await;
        node_guard.get_node_public_key().to_string()
    };

    let auto_execute = request.auto_execute.unwrap_or(true);

    // Wrap service in Arc for sharing across tasks
    let service = std::sync::Arc::new(service);

    // Spawn background tasks for each file
    for (file_path, file_info) in files_to_ingest.into_iter().zip(file_progress_ids.iter()) {
        let service = service.clone();
        let progress_service = progress_service.clone();
        let progress_id = file_info.progress_id.clone();
        let user_id_clone = user_id_for_task.clone();
        let node_arc_clone = node_arc.clone();
        let source_file_name = file_info.file_name.clone();
        let pub_key = node_public_key.clone();

        tokio::spawn(async move {
            crate::logging::core::run_with_user(&user_id_clone, async move {
                log_feature!(
                    LogFeature::Ingestion,
                    info,
                    "Starting ingestion for file: {} with progress_id: {}",
                    file_path.display(),
                    progress_id
                );

                // Read file content
                let content = match std::fs::read_to_string(&file_path) {
                    Ok(c) => c,
                    Err(e) => {
                        progress_service
                            .fail_progress(&progress_id, format!("Failed to read file: {}", e))
                            .await;
                        return;
                    }
                };

                // Determine file type and convert to JSON if needed
                let ext = file_path
                    .extension()
                    .and_then(|e| e.to_str())
                    .unwrap_or("")
                    .to_lowercase();

                let json_content = match ext.as_str() {
                    "json" => content,
                    "csv" => {
                        // Convert CSV to JSON array
                        match csv_to_json(&content) {
                            Ok(json) => json,
                            Err(e) => {
                                progress_service
                                    .fail_progress(&progress_id, format!("Failed to parse CSV: {}", e))
                                    .await;
                                return;
                            }
                        }
                    }
                    "txt" | "md" => {
                        // Wrap text content in a JSON structure
                        let file_name = file_path
                            .file_name()
                            .and_then(|n| n.to_str())
                            .unwrap_or("unknown");
                        serde_json::json!({
                            "content": content,
                            "source_file": file_name,
                            "file_type": ext
                        })
                        .to_string()
                    }
                    _ => {
                        progress_service
                            .fail_progress(&progress_id, format!("Unsupported file type: {}", ext))
                            .await;
                        return;
                    }
                };

                // Parse JSON content
                let json_data: Value = match serde_json::from_str(&json_content) {
                    Ok(v) => v,
                    Err(e) => {
                        progress_service
                            .fail_progress(&progress_id, format!("Invalid JSON: {}", e))
                            .await;
                        return;
                    }
                };

                // Create ingestion request
                let ingestion_request = IngestionRequest {
                    data: json_data,
                    auto_execute: Some(auto_execute),
                    trust_distance: Some(0),
                    pub_key: Some(pub_key),
                    source_file_name: Some(source_file_name),
                    progress_id: Some(progress_id.clone()),
                };

                // Acquire lock and process
                let node_guard = node_arc_clone.lock().await;

                match service
                    .process_json_with_node_and_progress(
                        ingestion_request,
                        &node_guard,
                        &progress_service,
                        progress_id.clone(),
                    )
                    .await
                {
                    Ok(response) => {
                        if response.success {
                            log_feature!(
                                LogFeature::Ingestion,
                                info,
                                "Successfully ingested file with progress_id: {}",
                                progress_id
                            );
                        } else {
                            log_feature!(
                                LogFeature::Ingestion,
                                error,
                                "Ingestion failed for progress_id {}: {:?}",
                                progress_id,
                                response.errors
                            );
                        }
                    }
                    Err(e) => {
                        log_feature!(
                            LogFeature::Ingestion,
                            error,
                            "Ingestion processing failed for progress_id {}: {}",
                            progress_id,
                            e
                        );
                        progress_service
                            .fail_progress(&progress_id, format!("Processing failed: {}", e))
                            .await;
                    }
                }
            })
            .await
        });
    }

    // Return immediately with batch info
    HttpResponse::Accepted().json(BatchFolderResponse {
        success: true,
        batch_id,
        files_found: file_progress_ids.len(),
        file_progress_ids,
        message: "Batch ingestion started. Use progress IDs to track individual file status.".to_string(),
    })
}

/// Convert CSV content to JSON array
fn csv_to_json(csv_content: &str) -> Result<String, String> {
    let mut reader = csv::ReaderBuilder::new()
        .has_headers(true)
        .from_reader(csv_content.as_bytes());

    let headers: Vec<String> = reader
        .headers()
        .map_err(|e| format!("Failed to read CSV headers: {}", e))?
        .iter()
        .map(|h| h.to_string())
        .collect();

    let mut records: Vec<Value> = Vec::new();

    for result in reader.records() {
        let record = result.map_err(|e| format!("Failed to read CSV record: {}", e))?;
        let mut obj = serde_json::Map::new();

        for (i, field) in record.iter().enumerate() {
            if let Some(header) = headers.get(i) {
                // Try to parse as number or boolean, otherwise keep as string
                let value = if let Ok(n) = field.parse::<f64>() {
                    Value::Number(serde_json::Number::from_f64(n).unwrap_or_else(|| serde_json::Number::from(0)))
                } else if field == "true" {
                    Value::Bool(true)
                } else if field == "false" {
                    Value::Bool(false)
                } else {
                    Value::String(field.to_string())
                };
                obj.insert(header.clone(), value);
            }
        }

        records.push(Value::Object(obj));
    }

    serde_json::to_string(&records).map_err(|e| format!("Failed to serialize JSON: {}", e))
}

// ============================================================================
// Smart Folder Ingestion - LLM-powered file filtering
// ============================================================================

/// Request for smart folder scanning
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SmartFolderScanRequest {
    /// Path to the folder to scan
    pub folder_path: String,
    /// Maximum depth to scan (default: 5)
    pub max_depth: Option<usize>,
    /// Maximum files to analyze (default: 500)
    pub max_files: Option<usize>,
}

/// A file recommendation from the LLM
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FileRecommendation {
    /// File path relative to the scanned folder
    pub path: String,
    /// Whether the file should be ingested
    pub should_ingest: bool,
    /// Category: "personal_data", "media", "config", "website_scaffolding", "work", "unknown"
    pub category: String,
    /// Brief reason for the recommendation
    pub reason: String,
}

/// Response from smart folder scanning
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SmartFolderScanResponse {
    pub success: bool,
    /// Total files scanned
    pub total_files: usize,
    /// Files recommended for ingestion
    pub recommended_files: Vec<FileRecommendation>,
    /// Files recommended to skip
    pub skipped_files: Vec<FileRecommendation>,
    /// Summary statistics
    pub summary: SmartFolderSummary,
}

/// Summary of smart folder scan
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SmartFolderSummary {
    pub personal_data_count: usize,
    pub media_count: usize,
    pub config_count: usize,
    pub website_scaffolding_count: usize,
    pub work_count: usize,
    pub unknown_count: usize,
}

/// Request for smart folder ingestion (after user approval)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SmartFolderIngestRequest {
    /// Base folder path
    pub folder_path: String,
    /// List of file paths (relative to folder) to ingest
    pub files_to_ingest: Vec<String>,
    /// Whether to auto-execute mutations (default: true)
    pub auto_execute: Option<bool>,
}

/// Scan a folder and use LLM to recommend which files contain personal data
#[utoipa::path(
    post,
    path = "/api/ingestion/smart-folder/scan",
    tag = "ingestion",
    request_body = SmartFolderScanRequest,
    responses(
        (status = 200, description = "Scan complete with recommendations", body = SmartFolderScanResponse),
        (status = 400, description = "Invalid folder path"),
        (status = 503, description = "AI service not available")
    )
)]
pub async fn smart_folder_scan(
    request: web::Json<SmartFolderScanRequest>,
    _state: web::Data<AppState>,
) -> impl Responder {
    log_feature!(
        LogFeature::Ingestion,
        info,
        "Smart folder scan requested for: {}",
        request.folder_path
    );

    // Get user context
    let _user_id = match crate::logging::core::get_current_user_id() {
        Some(uid) => uid,
        None => {
            return HttpResponse::Unauthorized().json(json!({
                "success": false,
                "error": "User not authenticated"
            }))
        }
    };

    // Resolve folder path
    let folder_path = if Path::new(&request.folder_path).is_absolute() {
        std::path::PathBuf::from(&request.folder_path)
    } else {
        std::env::current_dir()
            .unwrap_or_default()
            .join(&request.folder_path)
    };

    // Validate folder exists
    if !folder_path.exists() {
        return HttpResponse::BadRequest().json(json!({
            "success": false,
            "error": format!("Folder not found: {}", folder_path.display())
        }));
    }

    if !folder_path.is_dir() {
        return HttpResponse::BadRequest().json(json!({
            "success": false,
            "error": format!("Path is not a directory: {}", folder_path.display())
        }));
    }

    let max_depth = request.max_depth.unwrap_or(5);
    let max_files = request.max_files.unwrap_or(500);

    // Recursively scan directory tree
    let file_tree = match scan_directory_tree(&folder_path, max_depth, max_files) {
        Ok(tree) => tree,
        Err(e) => {
            return HttpResponse::BadRequest().json(json!({
                "success": false,
                "error": format!("Failed to scan directory: {}", e)
            }));
        }
    };

    if file_tree.is_empty() {
        return HttpResponse::Ok().json(SmartFolderScanResponse {
            success: true,
            total_files: 0,
            recommended_files: vec![],
            skipped_files: vec![],
            summary: SmartFolderSummary {
                personal_data_count: 0,
                media_count: 0,
                config_count: 0,
                website_scaffolding_count: 0,
                work_count: 0,
                unknown_count: 0,
            },
        });
    }

    // Create the LLM prompt with the file tree
    let prompt = create_smart_folder_prompt(&file_tree);

    // Call the LLM
    let service = match create_ingestion_service().await {
        Ok(s) => s,
        Err(e) => {
            return HttpResponse::ServiceUnavailable().json(json!({
                "success": false,
                "error": format!("AI service not available: {}", e)
            }));
        }
    };

    let llm_response = match call_llm_for_file_analysis(&service, &prompt).await {
        Ok(response) => response,
        Err(e) => {
            return HttpResponse::ServiceUnavailable().json(json!({
                "success": false,
                "error": format!("LLM call failed: {}", e)
            }));
        }
    };

    // Parse LLM response
    let recommendations = match parse_llm_file_recommendations(&llm_response, &file_tree) {
        Ok(recs) => recs,
        Err(e) => {
            log_feature!(
                LogFeature::Ingestion,
                warn,
                "Failed to parse LLM response, using heuristics: {}",
                e
            );
            // Fall back to heuristic-based filtering
            apply_heuristic_filtering(&file_tree)
        }
    };

    // Split into recommended and skipped
    let mut recommended_files = Vec::new();
    let mut skipped_files = Vec::new();
    let mut summary = SmartFolderSummary {
        personal_data_count: 0,
        media_count: 0,
        config_count: 0,
        website_scaffolding_count: 0,
        work_count: 0,
        unknown_count: 0,
    };

    for rec in recommendations {
        match rec.category.as_str() {
            "personal_data" => summary.personal_data_count += 1,
            "media" => summary.media_count += 1,
            "config" => summary.config_count += 1,
            "website_scaffolding" => summary.website_scaffolding_count += 1,
            "work" => summary.work_count += 1,
            _ => summary.unknown_count += 1,
        }

        if rec.should_ingest {
            recommended_files.push(rec);
        } else {
            skipped_files.push(rec);
        }
    }

    HttpResponse::Ok().json(SmartFolderScanResponse {
        success: true,
        total_files: file_tree.len(),
        recommended_files,
        skipped_files,
        summary,
    })
}

/// Ingest files from a smart folder scan (after user approval)
#[utoipa::path(
    post,
    path = "/api/ingestion/smart-folder/ingest",
    tag = "ingestion",
    request_body = SmartFolderIngestRequest,
    responses(
        (status = 202, description = "Batch ingestion started", body = BatchFolderResponse),
        (status = 400, description = "Invalid request")
    )
)]
pub async fn smart_folder_ingest(
    request: web::Json<SmartFolderIngestRequest>,
    progress_tracker: web::Data<ProgressTracker>,
    state: web::Data<AppState>,
) -> impl Responder {
    log_feature!(
        LogFeature::Ingestion,
        info,
        "Smart folder ingest requested for {} files",
        request.files_to_ingest.len()
    );

    // Convert to batch folder request format
    // We'll process each approved file
    let folder_path = if Path::new(&request.folder_path).is_absolute() {
        std::path::PathBuf::from(&request.folder_path)
    } else {
        std::env::current_dir()
            .unwrap_or_default()
            .join(&request.folder_path)
    };

    // Get user context
    let user_id = match crate::logging::core::get_current_user_id() {
        Some(uid) => uid,
        None => {
            return HttpResponse::Unauthorized().json(json!({
                "success": false,
                "error": "User not authenticated"
            }))
        }
    };

    // Validate files exist and build full paths
    let mut files_to_process: Vec<std::path::PathBuf> = Vec::new();
    for relative_path in &request.files_to_ingest {
        let full_path = folder_path.join(relative_path);
        if full_path.exists() && full_path.is_file() {
            files_to_process.push(full_path);
        } else {
            log_feature!(
                LogFeature::Ingestion,
                warn,
                "Skipping non-existent file: {}",
                full_path.display()
            );
        }
    }

    if files_to_process.is_empty() {
        return HttpResponse::BadRequest().json(json!({
            "success": false,
            "error": "No valid files to ingest"
        }));
    }

    // Generate batch ID
    let batch_id = uuid::Uuid::new_v4().to_string();

    // Create progress tracking for each file
    let progress_service = ProgressService::new(progress_tracker.get_ref().clone());
    let mut file_progress_ids: Vec<FileProgressInfo> = Vec::new();

    for file_path in &files_to_process {
        let progress_id = uuid::Uuid::new_v4().to_string();
        let file_name = file_path
            .file_name()
            .and_then(|n| n.to_str())
            .unwrap_or("unknown")
            .to_string();

        progress_service
            .start_progress(progress_id.clone(), user_id.clone())
            .await;

        file_progress_ids.push(FileProgressInfo {
            file_name,
            progress_id,
        });
    }

    // Get node for processing
    let (user_id_task, node_arc) = match require_node(&state).await {
        Ok(res) => res,
        Err(response) => return response,
    };

    let auto_execute = request.auto_execute.unwrap_or(true);

    // Spawn background tasks for each file
    for (i, file_path) in files_to_process.into_iter().enumerate() {
        let progress_id = file_progress_ids[i].progress_id.clone();
        let progress_tracker_clone = progress_tracker.get_ref().clone();
        let node_arc_clone = node_arc.clone();
        let user_id_clone = user_id_task.clone();
        let user_id_inner = user_id_task.clone();

        tokio::spawn(async move {
            crate::logging::core::run_with_user(&user_id_clone, async move {
                let progress_service = ProgressService::new(progress_tracker_clone);

                // Process the file (reuse existing logic)
                if let Err(e) = process_single_file_for_smart_ingest(
                    &file_path,
                    &progress_id,
                    &progress_service,
                    &node_arc_clone,
                    &user_id_inner,
                    auto_execute,
                )
                .await
                {
                    log_feature!(
                        LogFeature::Ingestion,
                        error,
                        "Failed to process file {}: {}",
                        file_path.display(),
                        e
                    );
                    progress_service
                        .fail_progress(&progress_id, format!("Processing failed: {}", e))
                        .await;
                }
            })
            .await
        });
    }

    HttpResponse::Accepted().json(BatchFolderResponse {
        success: true,
        batch_id,
        files_found: file_progress_ids.len(),
        file_progress_ids,
        message: "Smart folder ingestion started. Use progress IDs to track individual file status.".to_string(),
    })
}

/// Recursively scan a directory tree up to max_depth
fn scan_directory_tree(
    root: &std::path::Path,
    max_depth: usize,
    max_files: usize,
) -> Result<Vec<String>, String> {
    let mut files = Vec::new();
    scan_directory_recursive(root, root, 0, max_depth, max_files, &mut files)?;
    Ok(files)
}

fn scan_directory_recursive(
    root: &std::path::Path,
    current: &std::path::Path,
    depth: usize,
    max_depth: usize,
    max_files: usize,
    files: &mut Vec<String>,
) -> Result<(), String> {
    if depth > max_depth || files.len() >= max_files {
        return Ok(());
    }

    let entries = std::fs::read_dir(current)
        .map_err(|e| format!("Failed to read directory {}: {}", current.display(), e))?;

    for entry in entries.flatten() {
        if files.len() >= max_files {
            break;
        }

        let path = entry.path();
        let file_name = path.file_name().and_then(|n| n.to_str()).unwrap_or("");

        // Skip hidden files and common skip patterns
        if file_name.starts_with('.') {
            continue;
        }

        // Skip common non-data directories
        let skip_dirs = [
            "node_modules",
            "__pycache__",
            ".git",
            ".svn",
            "target",
            "build",
            "dist",
            ".cache",
            "venv",
            ".venv",
        ];
        if path.is_dir() && skip_dirs.contains(&file_name) {
            continue;
        }

        if path.is_dir() {
            scan_directory_recursive(root, &path, depth + 1, max_depth, max_files, files)?;
        } else if path.is_file() {
            // Get relative path from root
            if let Ok(relative) = path.strip_prefix(root) {
                files.push(relative.to_string_lossy().to_string());
            }
        }
    }

    Ok(())
}

/// Create the LLM prompt for file analysis
fn create_smart_folder_prompt(file_tree: &[String]) -> String {
    let files_list = file_tree.join("\n");

    format!(
        r#"Analyze this directory listing and categorize each file for personal data ingestion.

DIRECTORY LISTING:
{}

For each file, determine:
1. Should it be ingested into a personal database?
2. What category does it belong to?

CATEGORIES:
- personal_data: Personal documents, notes, journals, photos, messages, financial records, health data, creative work, personal projects
- media: Images, videos, audio (user-created content, not UI assets)
- config: Application configs, settings files, dotfiles
- website_scaffolding: HTML templates, CSS, JS bundles, emoji assets, fonts, node_modules contents
- work: Work/corporate files, professional documents
- unknown: Cannot determine

SKIP CRITERIA (should_ingest = false):
- Application scaffolding (runtime.js, modules.js, twemoji/, fonts/)
- Config files (.config, .env, settings.json unless personal)
- Cache and temporary files
- Binary executables
- Downloaded installers/archives
- Work/corporate documents (if identifiable)

INGEST CRITERIA (should_ingest = true):
- Personal documents (letters, notes, journals)
- Photos and videos (user-created, not UI assets)
- Messages and chat logs
- Financial records (statements, budgets)
- Health data
- Creative work (writing, art, music)
- Data exports from services (Twitter, Facebook, etc.)

Respond with a JSON array of objects:
```json
[
  {{"path": "file/path.ext", "should_ingest": true, "category": "personal_data", "reason": "Brief reason"}},
  ...
]
```

Only return the JSON array, no other text."#,
        files_list
    )
}

/// Call the LLM for file analysis
async fn call_llm_for_file_analysis(
    _service: &IngestionService,
    prompt: &str,
) -> Result<String, String> {
    // Access the underlying service to make a raw LLM call
    let config = IngestionConfig::from_env().map_err(|e| e.to_string())?;

    match config.provider {
        crate::ingestion::config::AIProvider::OpenRouter => {
            let openrouter = crate::ingestion::openrouter_service::OpenRouterService::new(
                config.openrouter,
                config.timeout_seconds,
                config.max_retries,
            )
            .map_err(|e| e.to_string())?;

            openrouter
                .call_openrouter_api(prompt)
                .await
                .map_err(|e| e.to_string())
        }
        crate::ingestion::config::AIProvider::Ollama => {
            let ollama = crate::ingestion::ollama_service::OllamaService::new(
                config.ollama,
                config.timeout_seconds,
                config.max_retries,
            )
            .map_err(|e| e.to_string())?;

            ollama
                .call_ollama_api(prompt)
                .await
                .map_err(|e| e.to_string())
        }
    }
}

/// Parse LLM response into file recommendations
fn parse_llm_file_recommendations(
    response: &str,
    file_tree: &[String],
) -> Result<Vec<FileRecommendation>, String> {
    // Try to extract JSON from the response
    let json_str = extract_json_from_response(response)?;

    let parsed: Vec<FileRecommendation> =
        serde_json::from_str(&json_str).map_err(|e| format!("Failed to parse JSON: {}", e))?;

    // Validate that paths exist in our file tree
    let file_set: std::collections::HashSet<&str> = file_tree.iter().map(|s| s.as_str()).collect();

    let valid_recs: Vec<FileRecommendation> = parsed
        .into_iter()
        .filter(|rec| file_set.contains(rec.path.as_str()))
        .collect();

    Ok(valid_recs)
}

/// Extract JSON array from LLM response (may have markdown code blocks)
fn extract_json_from_response(response: &str) -> Result<String, String> {
    // Try to find JSON array in the response
    let trimmed = response.trim();

    // Check if it starts with [ directly
    if trimmed.starts_with('[') {
        if let Some(end) = trimmed.rfind(']') {
            return Ok(trimmed[..=end].to_string());
        }
    }

    // Try to extract from markdown code block
    if let Some(start) = trimmed.find("```json") {
        let after_marker = &trimmed[start + 7..];
        if let Some(end) = after_marker.find("```") {
            return Ok(after_marker[..end].trim().to_string());
        }
    }

    // Try to extract from generic code block
    if let Some(start) = trimmed.find("```") {
        let after_marker = &trimmed[start + 3..];
        // Skip language identifier if present
        let content_start = after_marker.find('\n').unwrap_or(0);
        let content = &after_marker[content_start..];
        if let Some(end) = content.find("```") {
            return Ok(content[..end].trim().to_string());
        }
    }

    // Try to find [ and ] anywhere
    if let Some(start) = trimmed.find('[') {
        if let Some(end) = trimmed.rfind(']') {
            return Ok(trimmed[start..=end].to_string());
        }
    }

    Err("Could not extract JSON from response".to_string())
}

/// Apply heuristic-based filtering when LLM fails
fn apply_heuristic_filtering(file_tree: &[String]) -> Vec<FileRecommendation> {
    file_tree
        .iter()
        .map(|path| {
            let lower = path.to_lowercase();
            let ext = std::path::Path::new(path)
                .extension()
                .and_then(|e| e.to_str())
                .unwrap_or("")
                .to_lowercase();

            // Website scaffolding patterns
            let is_scaffolding = lower.contains("node_modules")
                || lower.contains("twemoji")
                || lower.contains("/assets/")
                || lower.contains("runtime.")
                || lower.contains("modules.")
                || ext == "woff"
                || ext == "woff2"
                || ext == "eot"
                || ext == "ttf"
                || (ext == "svg" && lower.contains("emoji"));

            // Config patterns
            let is_config = lower.starts_with(".")
                || lower.contains(".config")
                || lower.contains("config/")
                || ext == "env"
                || ext == "ini"
                || ext == "yaml"
                || ext == "yml";

            // Personal data patterns
            let is_personal = ext == "json"
                || ext == "csv"
                || ext == "txt"
                || ext == "md"
                || ext == "doc"
                || ext == "docx"
                || ext == "pdf"
                || lower.contains("data/")
                || lower.contains("export")
                || lower.contains("backup");

            // Media patterns
            let is_media = ext == "jpg"
                || ext == "jpeg"
                || ext == "png"
                || ext == "gif"
                || ext == "mp4"
                || ext == "mp3"
                || ext == "wav";

            let (should_ingest, category, reason) = if is_scaffolding {
                (false, "website_scaffolding", "Appears to be website/app scaffolding")
            } else if is_config {
                (false, "config", "Appears to be configuration file")
            } else if is_media && !lower.contains("twemoji") && !lower.contains("/assets/") {
                (true, "media", "User media file")
            } else if is_personal {
                (true, "personal_data", "Potential personal data file")
            } else {
                (false, "unknown", "Unknown file type")
            };

            FileRecommendation {
                path: path.clone(),
                should_ingest,
                category: category.to_string(),
                reason: reason.to_string(),
            }
        })
        .collect()
}

/// Process a single file for smart ingest (reuses existing logic)
async fn process_single_file_for_smart_ingest(
    file_path: &std::path::Path,
    progress_id: &str,
    progress_service: &ProgressService,
    node_arc: &std::sync::Arc<tokio::sync::Mutex<crate::datafold_node::DataFoldNode>>,
    _user_id: &str,
    auto_execute: bool,
) -> Result<(), String> {
    // Read file content
    let content = std::fs::read_to_string(file_path)
        .map_err(|e| format!("Failed to read file: {}", e))?;

    let ext = file_path
        .extension()
        .and_then(|e| e.to_str())
        .unwrap_or("")
        .to_lowercase();

    // Convert to JSON based on file type
    let json_content = match ext.as_str() {
        "json" => content,
        "csv" => csv_to_json(&content)?,
        "txt" | "md" => {
            let file_name = file_path
                .file_name()
                .and_then(|n| n.to_str())
                .unwrap_or("unknown");
            serde_json::to_string(&json!({
                "content": content,
                "source_file": file_name,
                "file_type": ext
            }))
            .map_err(|e| format!("Failed to wrap text content: {}", e))?
        }
        _ => return Err(format!("Unsupported file type: {}", ext)),
    };

    // Parse JSON
    let data: Value = serde_json::from_str(&json_content)
        .map_err(|e| format!("Failed to parse JSON: {}", e))?;

    // Create ingestion request
    let request = crate::ingestion::core::IngestionRequest {
        data,
        auto_execute: Some(auto_execute),
        trust_distance: Some(0),
        pub_key: None,
        source_file_name: file_path.file_name().and_then(|n| n.to_str()).map(|s| s.to_string()),
        progress_id: Some(progress_id.to_string()),
    };

    // Create service and process
    let service = create_ingestion_service()
        .await
        .map_err(|e| e.to_string())?;

    let node = node_arc.lock().await;
    service
        .process_json_with_node_and_progress(request, &node, progress_service, progress_id.to_string())
        .await
        .map_err(|e| e.to_string())?;

    Ok(())
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

    #[tokio::test]
    async fn test_csv_to_json_basic() {
        let csv_content = "name,age,active\nAlice,30,true\nBob,25,false";
        let result = csv_to_json(csv_content).unwrap();
        let parsed: Vec<serde_json::Value> = serde_json::from_str(&result).unwrap();

        assert_eq!(parsed.len(), 2);
        assert_eq!(parsed[0]["name"], "Alice");
        assert_eq!(parsed[0]["age"], 30.0);
        assert_eq!(parsed[0]["active"], true);
        assert_eq!(parsed[1]["name"], "Bob");
        assert_eq!(parsed[1]["age"], 25.0);
        assert_eq!(parsed[1]["active"], false);
    }

    #[tokio::test]
    async fn test_csv_to_json_with_strings() {
        let csv_content = "product_id,name,price\nPROD001,Widget,19.99\nPROD002,Gadget,29.99";
        let result = csv_to_json(csv_content).unwrap();
        let parsed: Vec<serde_json::Value> = serde_json::from_str(&result).unwrap();

        assert_eq!(parsed.len(), 2);
        assert_eq!(parsed[0]["product_id"], "PROD001");
        assert_eq!(parsed[0]["name"], "Widget");
        assert_eq!(parsed[0]["price"], 19.99);
    }

    #[tokio::test]
    async fn test_csv_to_json_empty() {
        let csv_content = "name,age";
        let result = csv_to_json(csv_content).unwrap();
        let parsed: Vec<serde_json::Value> = serde_json::from_str(&result).unwrap();

        assert_eq!(parsed.len(), 0);
    }

    #[tokio::test]
    async fn test_batch_folder_request_serialization() {
        let request = BatchFolderRequest {
            folder_path: "sample_data".to_string(),
            schema_hint: Some("TestSchema".to_string()),
            auto_execute: Some(true),
        };

        let json = serde_json::to_string(&request).unwrap();
        let parsed: BatchFolderRequest = serde_json::from_str(&json).unwrap();

        assert_eq!(parsed.folder_path, "sample_data");
        assert_eq!(parsed.schema_hint, Some("TestSchema".to_string()));
        assert_eq!(parsed.auto_execute, Some(true));
    }

    #[tokio::test]
    async fn test_batch_folder_response_serialization() {
        let response = BatchFolderResponse {
            success: true,
            batch_id: "test-batch-id".to_string(),
            files_found: 3,
            file_progress_ids: vec![
                FileProgressInfo {
                    file_name: "file1.json".to_string(),
                    progress_id: "prog-1".to_string(),
                },
                FileProgressInfo {
                    file_name: "file2.csv".to_string(),
                    progress_id: "prog-2".to_string(),
                },
            ],
            message: "Batch started".to_string(),
        };

        let json = serde_json::to_string(&response).unwrap();
        let parsed: BatchFolderResponse = serde_json::from_str(&json).unwrap();

        assert!(parsed.success);
        assert_eq!(parsed.batch_id, "test-batch-id");
        assert_eq!(parsed.files_found, 3);
        assert_eq!(parsed.file_progress_ids.len(), 2);
        assert_eq!(parsed.file_progress_ids[0].file_name, "file1.json");
    }
}
