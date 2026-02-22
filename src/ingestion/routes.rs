//! HTTP route handlers for the ingestion API

use crate::ingestion::batch_controller::{
    BatchController, BatchControllerMap, BatchStatus, BatchStatusResponse, PendingFile,
};
use crate::ingestion::config::SavedConfig;
use crate::ingestion::ingestion_service::IngestionService;
use crate::ingestion::progress::ProgressService;
use crate::ingestion::smart_folder;
use crate::ingestion::IngestionRequest;
use crate::ingestion::ProgressTracker;
use crate::log_feature;
use crate::logging::features::LogFeature;
use crate::server::http_server::AppState;
use crate::server::routes::{handler_error_to_response, require_node, require_user_context};
use actix_web::{web, HttpResponse, Responder};
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
use std::path::{Path, PathBuf};
use std::sync::Arc;
use tokio::sync::Mutex;

/// Shared ingestion service state — wrapped in RwLock so config saves can reload it.
pub type IngestionServiceState = tokio::sync::RwLock<Option<Arc<IngestionService>>>;

/// Helper to get a clone of the current IngestionService Arc from the RwLock.
pub async fn get_ingestion_service(
    state: &web::Data<IngestionServiceState>,
) -> Option<Arc<IngestionService>> {
    state.read().await.clone()
}

/// Resolve a folder path — expands `~` to the home directory, absolute paths
/// pass through, relative paths are resolved against the current working directory.
fn resolve_folder_path(path: &str) -> PathBuf {
    let expanded = if path == "~" {
        dirs::home_dir().unwrap_or_else(|| PathBuf::from(path))
    } else if let Some(rest) = path.strip_prefix("~/") {
        if let Some(home) = dirs::home_dir() {
            home.join(rest)
        } else {
            PathBuf::from(path)
        }
    } else {
        PathBuf::from(path)
    };

    if expanded.is_absolute() {
        expanded
    } else {
        std::env::current_dir().unwrap_or_default().join(expanded)
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
#[allow(clippy::too_many_arguments)]
fn spawn_file_ingestion_tasks(
    files_with_progress: impl IntoIterator<Item = (std::path::PathBuf, String)>,
    progress_tracker: &ProgressTracker,
    node_arc: &std::sync::Arc<tokio::sync::RwLock<crate::fold_node::FoldNode>>,
    user_id: &str,
    auto_execute: bool,
    ingestion_service: Arc<IngestionService>,
    upload_storage: crate::storage::UploadStorage,
    encryption_key: [u8; 32],
) {
    for (file_path, progress_id) in files_with_progress {
        let progress_tracker_clone = progress_tracker.clone();
        let node_arc_clone = node_arc.clone();
        let user_id_clone = user_id.to_string();
        let service_clone = ingestion_service.clone();
        let upload_storage_clone = upload_storage.clone();
        let enc_key = encryption_key;

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
                    &upload_storage_clone,
                    &enc_key,
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
    ingestion_service: web::Data<IngestionServiceState>,
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

    let service = match get_ingestion_service(&ingestion_service).await {
        Some(s) => s,
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
    ingestion_service: web::Data<IngestionServiceState>,
) -> impl Responder {
    log_feature!(
        LogFeature::Ingestion,
        debug,
        "Received ingestion status request"
    );

    match get_ingestion_service(&ingestion_service).await {
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
    ingestion_service: web::Data<IngestionServiceState>,
) -> impl Responder {
    log_feature!(
        LogFeature::Ingestion,
        info,
        "Received JSON validation request"
    );

    match get_ingestion_service(&ingestion_service).await {
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
pub async fn save_ingestion_config(
    request: web::Json<SavedConfig>,
    ingestion_service: web::Data<IngestionServiceState>,
) -> impl Responder {
    log_feature!(
        LogFeature::Ingestion,
        info,
        "Received ingestion config save request"
    );

    match crate::ingestion::config::IngestionConfig::save_to_file(&request.into_inner()) {
        Ok(()) => {
            // Reload the IngestionService so the new config takes effect immediately.
            // Use from_env_allow_empty() to skip strict validation — the user just
            // saved this config through the UI, so honour it even if e.g. an API key
            // is missing (the status endpoint will reflect configured=false).
            let reload_config = crate::ingestion::config::IngestionConfig::from_env_allow_empty();
            match IngestionService::new(reload_config) {
                Ok(new_service) => {
                    let mut guard = ingestion_service.write().await;
                    *guard = Some(Arc::new(new_service));
                    log_feature!(
                        LogFeature::Ingestion,
                        info,
                        "IngestionService reloaded with new configuration"
                    );
                }
                Err(e) => {
                    log_feature!(
                        LogFeature::Ingestion,
                        warn,
                        "Config saved but failed to reload IngestionService: {}. Service may be unavailable until restart.",
                        e
                    );
                }
            }
            HttpResponse::Ok().json(json!({
                "success": true,
                "message": "Configuration saved successfully"
            }))
        }
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
    ingestion_service: web::Data<IngestionServiceState>,
    upload_storage: web::Data<crate::storage::UploadStorage>,
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
    let service = match get_ingestion_service(&ingestion_service).await {
        Some(s) => s,
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
    let encryption_key = {
        let node = node_arc.read().await;
        node.get_encryption_key()
    };

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
        upload_storage.get_ref().clone(),
        encryption_key,
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
    /// Optional spend limit in USD. None = no cap.
    pub spend_limit: Option<f64>,
    /// Per-file estimated costs (parallel to files_to_ingest). Used for spend tracking.
    pub file_costs: Option<Vec<f64>>,
}

/// Request to resume a paused batch
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BatchResumeRequest {
    pub batch_id: String,
    pub new_spend_limit: f64,
}

/// Request to cancel a batch
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BatchCancelRequest {
    pub batch_id: String,
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
    ingestion_service: web::Data<IngestionServiceState>,
) -> impl Responder {
    log_feature!(
        LogFeature::Ingestion,
        info,
        "Smart folder scan requested for: {}",
        request.folder_path
    );

    // Get user context
    let user_id = match require_user_context() {
        Ok(hash) => hash,
        Err(response) => return response,
    };

    log_feature!(
        LogFeature::Ingestion,
        info,
        "Smart folder scan for user: {}",
        user_id
    );

    // Resolve folder path
    let folder_path = resolve_folder_path(&request.folder_path);

    if let Err(response) = validate_folder(&folder_path) {
        return response;
    }

    let max_depth = request.max_depth.unwrap_or(10);
    let max_files = request.max_files.unwrap_or(100);

    // Delegate to shared logic
    let service_opt = get_ingestion_service(&ingestion_service).await;
    let service_ref = service_opt.as_deref();
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
    ingestion_service: web::Data<IngestionServiceState>,
    batch_controller_map: web::Data<BatchControllerMap>,
    upload_storage: web::Data<crate::storage::UploadStorage>,
) -> impl Responder {
    log_feature!(
        LogFeature::Ingestion,
        info,
        "Smart folder ingest requested for {} files (spend_limit: {:?})",
        request.files_to_ingest.len(),
        request.spend_limit
    );

    let folder_path = resolve_folder_path(&request.folder_path);

    // Get user context
    let user_id = match require_user_context() {
        Ok(hash) => hash,
        Err(response) => return response,
    };

    // Validate files exist and build full paths with costs
    let file_costs = request.file_costs.as_deref();
    let mut files_to_process: Vec<std::path::PathBuf> = Vec::new();
    let mut costs: Vec<f64> = Vec::new();
    for (i, relative_path) in request.files_to_ingest.iter().enumerate() {
        let full_path = folder_path.join(relative_path);
        if full_path.exists() && full_path.is_file() {
            let cost = file_costs
                .and_then(|c| c.get(i).copied())
                .unwrap_or_else(|| {
                    smart_folder::estimate_file_cost(Path::new(relative_path), &folder_path)
                });
            files_to_process.push(full_path);
            costs.push(cost);
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
    let service = match get_ingestion_service(&ingestion_service).await {
        Some(s) => s,
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
    let file_progress_ids =
        start_file_progress(&files_to_process, &user_id, &progress_service).await;

    // Get node for processing
    let (user_id_task, node_arc) = match require_node(&state).await {
        Ok(res) => res,
        Err(response) => return response,
    };

    let auto_execute = request.auto_execute.unwrap_or(true);

    // Build pending files for the batch controller
    let pending_files: Vec<PendingFile> = files_to_process
        .iter()
        .zip(file_progress_ids.iter())
        .zip(costs.iter())
        .map(|((path, info), &cost)| PendingFile {
            path: path.clone(),
            progress_id: info.progress_id.clone(),
            estimated_cost: cost,
        })
        .collect();

    // Create the batch controller
    let controller = BatchController::new(
        batch_id.clone(),
        request.spend_limit,
        pending_files,
    );
    let ctrl_arc = Arc::new(Mutex::new(controller));

    // Register in the global map
    {
        let mut map_guard = batch_controller_map.lock().await;
        map_guard.insert(batch_id.clone(), ctrl_arc);
    }

    let encryption_key = {
        let node = node_arc.read().await;
        node.get_encryption_key()
    };

    // Spawn the sequential coordinator
    spawn_batch_coordinator(
        batch_id.clone(),
        batch_controller_map,
        progress_tracker.get_ref(),
        &node_arc,
        &user_id_task,
        auto_execute,
        service,
        upload_storage.get_ref().clone(),
        encryption_key,
    );

    HttpResponse::Accepted().json(BatchFolderResponse {
        success: true,
        batch_id,
        files_found: file_progress_ids.len(),
        file_progress_ids,
        message: "Smart folder ingestion started with spend tracking.".to_string(),
    })
}

/// Spawn a sequential coordinator task that processes files one at a time,
/// checking the spend limit before each file and pausing when the limit is hit.
#[allow(clippy::too_many_arguments)]
fn spawn_batch_coordinator(
    batch_id: String,
    batch_controller_map: web::Data<BatchControllerMap>,
    progress_tracker: &ProgressTracker,
    node_arc: &Arc<tokio::sync::RwLock<crate::fold_node::FoldNode>>,
    user_id: &str,
    auto_execute: bool,
    ingestion_service: Arc<IngestionService>,
    upload_storage: crate::storage::UploadStorage,
    encryption_key: [u8; 32],
) {
    let progress_tracker = progress_tracker.clone();
    let node_arc = node_arc.clone();
    let user_id = user_id.to_string();
    let map = batch_controller_map.get_ref().clone();

    tokio::spawn(async move {
        crate::logging::core::run_with_user(&user_id, async move {
            loop {
                // Lock the controller briefly to check state and pop next file
                let (file, resume_notifier) = {
                    let map_guard = map.lock().await;
                    let ctrl_arc = match map_guard.get(&batch_id) {
                        Some(c) => c.clone(),
                        None => break,
                    };
                    let mut ctrl = ctrl_arc.lock().await;

                    // Check for cancellation
                    if ctrl.status == BatchStatus::Cancelled {
                        break;
                    }

                    // Check if any files remain
                    let next_file = match ctrl.pending_files.first() {
                        Some(f) => f.clone(),
                        None => {
                            // All files processed
                            ctrl.status = BatchStatus::Completed;
                            break;
                        }
                    };

                    // Check spend limit
                    if !ctrl.can_proceed(next_file.estimated_cost) {
                        ctrl.pause();
                        (None, Some(ctrl.resume_notifier()))
                    } else {
                        let file = ctrl.pop_next_file();
                        (file, None)
                    }
                };

                // If paused, wait for the resume notification
                if let Some(notifier) = resume_notifier {
                    notifier.notified().await;

                    // After waking, re-check status (might have been cancelled)
                    let map_guard = map.lock().await;
                    if let Some(ctrl_arc) = map_guard.get(&batch_id) {
                        let ctrl = ctrl_arc.lock().await;
                        if ctrl.status == BatchStatus::Cancelled {
                            break;
                        }
                    } else {
                        break;
                    }
                    continue;
                }

                // Process the file (outside any lock)
                let file = match file {
                    Some(f) => f,
                    None => continue,
                };

                let progress_service = ProgressService::new(progress_tracker.clone());
                let service_clone = ingestion_service.clone();
                let estimated_cost = file.estimated_cost;

                let result = process_single_file_via_smart_folder(
                    &file.path,
                    &file.progress_id,
                    &progress_service,
                    &node_arc,
                    auto_execute,
                    &service_clone,
                    &upload_storage,
                    &encryption_key,
                )
                .await;

                // Update controller with result
                {
                    let map_guard = map.lock().await;
                    if let Some(ctrl_arc) = map_guard.get(&batch_id) {
                        let mut ctrl = ctrl_arc.lock().await;
                        match &result {
                            Ok(()) => ctrl.record_completed(estimated_cost),
                            Err(e) => {
                                log_feature!(
                                    LogFeature::Ingestion,
                                    error,
                                    "Batch {}: file {} failed: {}",
                                    batch_id,
                                    file.path.display(),
                                    e
                                );
                                progress_service
                                    .fail_progress(
                                        &file.progress_id,
                                        format!("Processing failed: {}", e),
                                    )
                                    .await;
                                ctrl.record_failed();
                            }
                        }
                    }
                }
            }

            // Mark completed if not already cancelled/failed
            {
                let map_guard = map.lock().await;
                if let Some(ctrl_arc) = map_guard.get(&batch_id) {
                    let mut ctrl = ctrl_arc.lock().await;
                    if ctrl.status == BatchStatus::Running {
                        ctrl.status = BatchStatus::Completed;
                    }
                }
            }
        })
        .await
    });
}

/// Get batch status
pub async fn get_batch_status(
    path: web::Path<String>,
    batch_controller_map: web::Data<BatchControllerMap>,
) -> impl Responder {
    let batch_id = path.into_inner();
    let map_guard = batch_controller_map.lock().await;

    match map_guard.get(&batch_id) {
        Some(ctrl_arc) => {
            let ctrl = ctrl_arc.lock().await;
            HttpResponse::Ok().json(BatchStatusResponse::from_controller(&ctrl))
        }
        None => HttpResponse::NotFound().json(json!({
            "error": format!("Batch {} not found", batch_id)
        })),
    }
}

/// Resume a paused batch with a new spend limit
pub async fn resume_batch(
    request: web::Json<BatchResumeRequest>,
    batch_controller_map: web::Data<BatchControllerMap>,
) -> impl Responder {
    let map_guard = batch_controller_map.lock().await;

    match map_guard.get(&request.batch_id) {
        Some(ctrl_arc) => {
            let mut ctrl = ctrl_arc.lock().await;
            if ctrl.status != BatchStatus::Paused {
                return HttpResponse::BadRequest().json(json!({
                    "error": format!("Batch is not paused (status: {})", ctrl.status)
                }));
            }
            ctrl.resume(Some(request.new_spend_limit));
            HttpResponse::Ok().json(BatchStatusResponse::from_controller(&ctrl))
        }
        None => HttpResponse::NotFound().json(json!({
            "error": format!("Batch {} not found", request.batch_id)
        })),
    }
}

/// Cancel a batch
pub async fn cancel_batch(
    request: web::Json<BatchCancelRequest>,
    batch_controller_map: web::Data<BatchControllerMap>,
) -> impl Responder {
    let map_guard = batch_controller_map.lock().await;

    match map_guard.get(&request.batch_id) {
        Some(ctrl_arc) => {
            let mut ctrl = ctrl_arc.lock().await;
            ctrl.cancel();
            HttpResponse::Ok().json(BatchStatusResponse::from_controller(&ctrl))
        }
        None => HttpResponse::NotFound().json(json!({
            "error": format!("Batch {} not found", request.batch_id)
        })),
    }
}

/// Process a single file for smart ingest using shared smart_folder module.
/// Reads the file, computes its SHA256 hash, encrypts and stores in upload storage,
/// then ingests the JSON content with file_hash metadata.
#[allow(clippy::too_many_arguments)]
async fn process_single_file_via_smart_folder(
    file_path: &std::path::Path,
    progress_id: &str,
    progress_service: &ProgressService,
    node_arc: &std::sync::Arc<tokio::sync::RwLock<crate::fold_node::FoldNode>>,
    auto_execute: bool,
    service: &IngestionService,
    upload_storage: &crate::storage::UploadStorage,
    encryption_key: &[u8; 32],
) -> Result<(), String> {
    let (data, file_hash) = smart_folder::read_file_with_hash(file_path)
        .map_err(|e| e.to_string())?;

    // Encrypt and store the raw file in upload storage (content-addressed)
    let raw_bytes = std::fs::read(file_path)
        .map_err(|e| format!("Failed to read file for encryption: {}", e))?;
    let encrypted_data = crate::crypto::envelope::encrypt_envelope(encryption_key, &raw_bytes)
        .map_err(|e| format!("Failed to encrypt file: {}", e))?;
    // Content-addressed: user_id=None (same file = same hash = same object)
    upload_storage
        .save_file_if_not_exists(&file_hash, &encrypted_data, None)
        .await
        .map_err(|e| format!("Failed to store encrypted file: {}", e))?;

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
        file_hash: Some(file_hash),
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
        let ingestion_service: IngestionServiceState = tokio::sync::RwLock::new(None);
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

    #[tokio::test]
    async fn test_resolve_folder_path_tilde() {
        let result = resolve_folder_path("~/Documents");
        let home = dirs::home_dir().expect("home_dir must exist for this test");
        assert_eq!(result, home.join("Documents"));
    }

    #[tokio::test]
    async fn test_resolve_folder_path_tilde_only() {
        let result = resolve_folder_path("~");
        let home = dirs::home_dir().expect("home_dir must exist for this test");
        assert_eq!(result, home);
    }

    #[tokio::test]
    async fn test_resolve_folder_path_absolute() {
        let result = resolve_folder_path("/tmp/test");
        assert_eq!(result, PathBuf::from("/tmp/test"));
    }
}
