//! HTTP route handlers for the ingestion API

use crate::ingestion::config::{IngestionConfig, SavedConfig};
use crate::ingestion::core::IngestionRequest;
use crate::ingestion::progress::ProgressService;
use crate::ingestion::ingestion_service::IngestionService;
use crate::ingestion::smart_folder;
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
                        match smart_folder::csv_to_json(&content) {
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

    // Delegate to shared logic
    match smart_folder::perform_smart_folder_scan(&folder_path, max_depth, max_files).await {
        Ok(response) => HttpResponse::Ok().json(response),
        Err(e) => HttpResponse::BadRequest().json(json!({
            "success": false,
            "error": e
        })),
    }
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

        tokio::spawn(async move {
            crate::logging::core::run_with_user(&user_id_clone, async move {
                let progress_service = ProgressService::new(progress_tracker_clone);

                // Process the file using shared read_file_as_json
                if let Err(e) = process_single_file_via_smart_folder(
                    &file_path,
                    &progress_id,
                    &progress_service,
                    &node_arc_clone,
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

/// Process a single file for smart ingest using shared smart_folder module
async fn process_single_file_via_smart_folder(
    file_path: &std::path::Path,
    progress_id: &str,
    progress_service: &ProgressService,
    node_arc: &std::sync::Arc<tokio::sync::Mutex<crate::datafold_node::DataFoldNode>>,
    auto_execute: bool,
) -> Result<(), String> {
    let data = smart_folder::read_file_as_json(file_path)?;

    let request = IngestionRequest {
        data,
        auto_execute: Some(auto_execute),
        trust_distance: Some(0),
        pub_key: None,
        source_file_name: file_path
            .file_name()
            .and_then(|n| n.to_str())
            .map(|s| s.to_string()),
        progress_id: Some(progress_id.to_string()),
    };

    let service = create_ingestion_service()
        .await
        .map_err(|e| e.to_string())?;

    let node = node_arc.lock().await;
    service
        .process_json_with_node_and_progress(
            request,
            &node,
            progress_service,
            progress_id.to_string(),
        )
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
        let result = smart_folder::csv_to_json(csv_content).unwrap();
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
        let result = smart_folder::csv_to_json(csv_content).unwrap();
        let parsed: Vec<serde_json::Value> = serde_json::from_str(&result).unwrap();

        assert_eq!(parsed.len(), 2);
        assert_eq!(parsed[0]["product_id"], "PROD001");
        assert_eq!(parsed[0]["name"], "Widget");
        assert_eq!(parsed[0]["price"], 19.99);
    }

    #[tokio::test]
    async fn test_csv_to_json_empty() {
        let csv_content = "name,age";
        let result = smart_folder::csv_to_json(csv_content).unwrap();
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
