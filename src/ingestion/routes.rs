//! HTTP route handlers for the ingestion API

use crate::ingestion::config::SavedConfig;
use crate::ingestion::IngestionRequest;
use crate::ingestion::progress::ProgressService;
use crate::ingestion::ingestion_service::IngestionService;
use crate::ingestion::smart_folder;
use crate::ingestion::ProgressTracker;
use crate::log_feature;
use crate::logging::features::LogFeature;
use crate::server::http_server::AppState;
use crate::server::routes::{require_node, require_user_context};
use actix_web::{web, HttpResponse, Responder};
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
use std::path::{Path, PathBuf};
use std::sync::Arc;

/// Resolve a folder path — absolute paths pass through, relative paths
/// are resolved against the current working directory.
fn resolve_folder_path(path: &str) -> PathBuf {
    if Path::new(path).is_absolute() {
        PathBuf::from(path)
    } else {
        std::env::current_dir().unwrap_or_default().join(path)
    }
}

/// Initialize progress tracking for a list of files, returning a FileProgressInfo per file.
async fn start_file_progress(
    files: &[std::path::PathBuf],
    user_id: &str,
    progress_service: &ProgressService,
) -> Vec<FileProgressInfo> {
    let mut infos = Vec::with_capacity(files.len());
    for file_path in files {
        let progress_id = uuid::Uuid::new_v4().to_string();
        let file_name = file_path
            .file_name()
            .and_then(|n| n.to_str())
            .unwrap_or("unknown")
            .to_string();

        progress_service
            .start_progress(progress_id.clone(), user_id.to_string())
            .await;

        infos.push(FileProgressInfo {
            file_name,
            progress_id,
        });
    }
    infos
}

/// Validate that a path exists and is a directory, returning an error HttpResponse if not.
fn validate_folder(path: &Path) -> Result<(), HttpResponse> {
    if !path.exists() {
        return Err(HttpResponse::BadRequest().json(json!({
            "success": false,
            "error": format!("Folder not found: {}", path.display())
        })));
    }
    if !path.is_dir() {
        return Err(HttpResponse::BadRequest().json(json!({
            "success": false,
            "error": format!("Path is not a directory: {}", path.display())
        })));
    }
    Ok(())
}

/// Spawn background ingestion tasks for a list of files, each tracked by its progress ID.
fn spawn_file_ingestion_tasks(
    files_with_progress: impl IntoIterator<Item = (std::path::PathBuf, String)>,
    progress_tracker: &ProgressTracker,
    node_arc: &std::sync::Arc<tokio::sync::RwLock<crate::datafold_node::DataFoldNode>>,
    user_id: &str,
    auto_execute: bool,
    ingestion_service: Arc<IngestionService>,
) {
    for (file_path, progress_id) in files_with_progress {
        let progress_tracker_clone = progress_tracker.clone();
        let node_arc_clone = node_arc.clone();
        let user_id_clone = user_id.to_string();
        let service_clone = ingestion_service.clone();

        tokio::spawn(async move {
            crate::logging::core::run_with_user(&user_id_clone, async move {
                let progress_service = ProgressService::new(progress_tracker_clone);

                if let Err(e) = process_single_file_via_smart_folder(
                    &file_path,
                    &progress_id,
                    &progress_service,
                    &node_arc_clone,
                    auto_execute,
                    &service_clone,
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
}

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

/// Convert a HandlerError to an HttpResponse, mapping status codes.
fn handler_error_to_response(e: crate::handlers::response::HandlerError) -> HttpResponse {
    let status_code = match e.status_code() {
        400 => actix_web::http::StatusCode::BAD_REQUEST,
        401 => actix_web::http::StatusCode::UNAUTHORIZED,
        404 => actix_web::http::StatusCode::NOT_FOUND,
        503 => actix_web::http::StatusCode::SERVICE_UNAVAILABLE,
        _ => actix_web::http::StatusCode::INTERNAL_SERVER_ERROR,
    };
    HttpResponse::build(status_code).json(e.to_response())
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
    ingestion_service: web::Data<Option<Arc<IngestionService>>>,
) -> impl Responder {
    log_feature!(
        LogFeature::Ingestion,
        info,
        "Received JSON ingestion request"
    );

    let user_id = match require_user_context() {
        Ok(hash) => hash,
        Err(response) => return response,
    };

    let (_, node_arc) = match require_node(&state).await {
        Ok(res) => res,
        Err(response) => return response,
    };

    let service = match ingestion_service.get_ref() {
        Some(s) => s.clone(),
        None => {
            return HttpResponse::ServiceUnavailable().json(json!({
                "error": "Ingestion service not available"
            }));
        }
    };

    // Lock briefly — handler clones the node and spawns a background task
    let node = node_arc.read().await;

    match crate::handlers::ingestion::process_json(
        request.into_inner(),
        &user_id,
        progress_tracker.get_ref(),
        &node,
        service,
    )
    .await
    {
        Ok(api_response) => HttpResponse::Accepted().json(api_response.data),
        Err(e) => handler_error_to_response(e),
    }
}

/// Get ingestion status
#[utoipa::path(
    get,
    path = "/api/ingestion/status",
    tag = "ingestion",
    responses((status = 200, description = "Ingestion status", body = crate::ingestion::IngestionStatus))
)]
pub async fn get_status(
    ingestion_service: web::Data<Option<Arc<IngestionService>>>,
) -> impl Responder {
    log_feature!(
        LogFeature::Ingestion,
        debug,
        "Received ingestion status request"
    );

    match ingestion_service.get_ref() {
        Some(service) => match service.get_status() {
            Ok(status) => HttpResponse::Ok().json(status),
            Err(e) => HttpResponse::InternalServerError().json(json!({
                "error": format!("Failed to get status: {}", e)
            })),
        },
        None => HttpResponse::ServiceUnavailable().json(json!({
            "error": "Ingestion service not available",
            "enabled": false,
            "configured": false
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
pub async fn validate_json(
    request: web::Json<Value>,
    ingestion_service: web::Data<Option<Arc<IngestionService>>>,
) -> impl Responder {
    log_feature!(
        LogFeature::Ingestion,
        info,
        "Received JSON validation request"
    );

    match ingestion_service.get_ref() {
        Some(service) => match service.validate_input(&request.into_inner()) {
            Ok(()) => HttpResponse::Ok().json(json!({
                "valid": true,
                "message": "JSON data is valid for ingestion"
            })),
            Err(e) => HttpResponse::BadRequest().json(json!({
                "valid": false,
                "error": format!("Validation failed: {}", e)
            })),
        },
        None => HttpResponse::ServiceUnavailable().json(json!({
            "valid": false,
            "error": "Ingestion service not available"
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

    let config = crate::ingestion::config::IngestionConfig::from_env_allow_empty();
    HttpResponse::Ok().json(config.redacted())
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

    match crate::ingestion::config::IngestionConfig::save_to_file(&request.into_inner()) {
        Ok(()) => HttpResponse::Ok().json(json!({
            "success": true,
            "message": "Configuration saved successfully"
        })),
        Err(e) => HttpResponse::InternalServerError().json(json!({
            "success": false,
            "error": format!("Failed to save configuration: {}", e)
        })),
    }
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

    let user_hash = match require_user_context() {
        Ok(hash) => hash,
        Err(response) => return response,
    };

    match crate::handlers::ingestion::get_progress(&id, &user_hash, progress_tracker.get_ref())
        .await
    {
        Ok(api_response) => HttpResponse::Ok().json(api_response.data),
        Err(e) => handler_error_to_response(e),
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
        Err(e) => handler_error_to_response(e),
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
    ingestion_service: web::Data<Option<Arc<IngestionService>>>,
) -> impl Responder {
    log_feature!(
        LogFeature::Ingestion,
        info,
        "Received batch folder ingestion request for: {}",
        request.folder_path
    );

    // Get user context
    let user_id = match require_user_context() {
        Ok(hash) => hash,
        Err(response) => return response,
    };

    // Resolve folder path - support both absolute and relative paths
    let folder_path = resolve_folder_path(&request.folder_path);

    if let Err(response) = validate_folder(&folder_path) {
        return response;
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
    let file_progress_ids = start_file_progress(&files_to_ingest, &user_id, &progress_service).await;

    // Validate ingestion service is available before spawning tasks
    let service = match ingestion_service.get_ref() {
        Some(s) => s.clone(),
        None => {
            log_feature!(
                LogFeature::Ingestion,
                error,
                "Ingestion service not available"
            );
            for info in &file_progress_ids {
                progress_service
                    .fail_progress(&info.progress_id, "Ingestion service not available".to_string())
                    .await;
            }
            return HttpResponse::ServiceUnavailable().json(json!({
                "success": false,
                "error": "Ingestion service not available"
            }));
        }
    };

    // Get node for processing
    let (user_id_for_task, node_arc) = match require_node(&state).await {
        Ok(res) => res,
        Err(response) => return response,
    };

    let auto_execute = request.auto_execute.unwrap_or(true);

    spawn_file_ingestion_tasks(
        files_to_ingest
            .into_iter()
            .zip(file_progress_ids.iter())
            .map(|(path, info)| (path, info.progress_id.clone())),
        progress_tracker.get_ref(),
        &node_arc,
        &user_id_for_task,
        auto_execute,
        service,
    );

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
    ingestion_service: web::Data<Option<Arc<IngestionService>>>,
) -> impl Responder {
    log_feature!(
        LogFeature::Ingestion,
        info,
        "Smart folder scan requested for: {}",
        request.folder_path
    );

    // Get user context
    let _user_id = match require_user_context() {
        Ok(hash) => hash,
        Err(response) => return response,
    };

    // Resolve folder path
    let folder_path = resolve_folder_path(&request.folder_path);

    if let Err(response) = validate_folder(&folder_path) {
        return response;
    }

    let max_depth = request.max_depth.unwrap_or(5);
    let max_files = request.max_files.unwrap_or(500);

    // Delegate to shared logic
    let service_ref = ingestion_service.get_ref().as_deref();
    match smart_folder::perform_smart_folder_scan(&folder_path, max_depth, max_files, service_ref).await {
        Ok(response) => HttpResponse::Ok().json(response),
        Err(e) => HttpResponse::BadRequest().json(json!({
            "success": false,
            "error": e.to_string()
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
    ingestion_service: web::Data<Option<Arc<IngestionService>>>,
) -> impl Responder {
    log_feature!(
        LogFeature::Ingestion,
        info,
        "Smart folder ingest requested for {} files",
        request.files_to_ingest.len()
    );

    // Convert to batch folder request format
    // We'll process each approved file
    let folder_path = resolve_folder_path(&request.folder_path);

    // Get user context
    let user_id = match require_user_context() {
        Ok(hash) => hash,
        Err(response) => return response,
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

    // Validate ingestion service is available
    let service = match ingestion_service.get_ref() {
        Some(s) => s.clone(),
        None => {
            return HttpResponse::ServiceUnavailable().json(json!({
                "success": false,
                "error": "Ingestion service not available"
            }));
        }
    };

    // Generate batch ID
    let batch_id = uuid::Uuid::new_v4().to_string();

    // Create progress tracking for each file
    let progress_service = ProgressService::new(progress_tracker.get_ref().clone());
    let file_progress_ids = start_file_progress(&files_to_process, &user_id, &progress_service).await;

    // Get node for processing
    let (user_id_task, node_arc) = match require_node(&state).await {
        Ok(res) => res,
        Err(response) => return response,
    };

    let auto_execute = request.auto_execute.unwrap_or(true);

    spawn_file_ingestion_tasks(
        files_to_process
            .into_iter()
            .enumerate()
            .map(|(i, path)| (path, file_progress_ids[i].progress_id.clone())),
        progress_tracker.get_ref(),
        &node_arc,
        &user_id_task,
        auto_execute,
        service,
    );

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
    node_arc: &std::sync::Arc<tokio::sync::RwLock<crate::datafold_node::DataFoldNode>>,
    auto_execute: bool,
    service: &IngestionService,
) -> Result<(), String> {
    let data = smart_folder::read_file_as_json(file_path)
        .map_err(|e| e.to_string())?;

    let node = node_arc.read().await;
    let pub_key = node.get_node_public_key().to_string();

    let request = IngestionRequest {
        data,
        auto_execute,
        trust_distance: 0,
        pub_key,
        source_file_name: file_path
            .file_name()
            .and_then(|n| n.to_str())
            .map(|s| s.to_string()),
        progress_id: Some(progress_id.to_string()),
    };

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
        let ingestion_service: Option<Arc<IngestionService>> = None;
        let app = test::init_service(
            App::new()
                .app_data(web::Data::new(ingestion_service))
                .route("/status", web::get().to(get_status)),
        )
        .await;

        let req = test::TestRequest::get().uri("/status").to_request();
        let resp = test::call_service(&app, req).await;
        // Should return service unavailable if not configured
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
