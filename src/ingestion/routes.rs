//! HTTP route handlers for the ingestion API

use crate::ingestion::batch_controller::{BatchControllerMap, BatchStatus, BatchStatusResponse};
use crate::ingestion::ingestion_service::IngestionService;
use crate::ingestion::progress::ProgressService;
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

// Re-export from sibling modules so external callers (http_server.rs) can still
// reference everything through `crate::ingestion::routes::*`.
pub use super::smart_folder_routes::*;

/// Return a 503 response when the ingestion service is unavailable.
pub(crate) fn ingestion_unavailable() -> HttpResponse {
    HttpResponse::ServiceUnavailable().json(json!({
        "success": false,
        "error": "Ingestion service not available"
    }))
}

/// Shared ingestion service state — wrapped in RwLock so config saves can reload it.
pub type IngestionServiceState = tokio::sync::RwLock<Option<Arc<IngestionService>>>;

/// Extract the user/node/ingestion triple that most ingestion handlers need.
pub(crate) async fn require_ingestion_context(
    state: &web::Data<AppState>,
    ingestion_service: &web::Data<IngestionServiceState>,
) -> Result<(String, Arc<tokio::sync::RwLock<crate::fold_node::FoldNode>>, Arc<IngestionService>), HttpResponse> {
    let (user_id, node_arc) = require_node(state).await?;
    let service = get_ingestion_service(ingestion_service)
        .await
        .ok_or_else(ingestion_unavailable)?;
    Ok((user_id, node_arc, service))
}

/// Helper to get a clone of the current IngestionService Arc from the RwLock.
pub async fn get_ingestion_service(
    state: &web::Data<IngestionServiceState>,
) -> Option<Arc<IngestionService>> {
    state.read().await.clone()
}

/// Resolve a folder path — expands `~` to the home directory, absolute paths
/// pass through, relative paths are resolved against the current working directory.
pub(crate) fn resolve_folder_path(path: &str) -> PathBuf {
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
pub(crate) async fn start_file_progress(
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
pub(crate) fn validate_folder(path: &Path) -> Result<(), HttpResponse> {
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
pub(crate) fn spawn_file_ingestion_tasks(
    files_with_progress: impl IntoIterator<Item = (std::path::PathBuf, String)>,
    progress_tracker: &ProgressTracker,
    node_arc: &std::sync::Arc<tokio::sync::RwLock<crate::fold_node::FoldNode>>,
    user_id: &str,
    auto_execute: bool,
    ingestion_service: Arc<IngestionService>,
    upload_storage: crate::storage::UploadStorage,
    encryption_key: [u8; 32],
    force_reingest: bool,
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
                    force_reingest,
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

    let (user_id, node_arc, service) = match require_ingestion_context(&state, &ingestion_service).await {
        Ok(ctx) => ctx,
        Err(response) => return response,
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
    request: web::Json<crate::ingestion::config::SavedConfig>,
    ingestion_service: web::Data<IngestionServiceState>,
    llm_state: web::Data<crate::fold_node::llm_query::LlmQueryState>,
) -> impl Responder {
    log_feature!(
        LogFeature::Ingestion,
        info,
        "Received ingestion config save request"
    );

    match crate::ingestion::config::IngestionConfig::save_to_file(&request.into_inner()) {
        Ok(()) => {
            // Reload the IngestionService so the new config takes effect immediately.
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
            // Also reload the LLM query service so model changes take effect
            llm_state.reload().await;

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

/// Process a single file for smart ingest using shared smart_folder module.
/// Reads the file, computes its SHA256 hash, encrypts and stores in upload storage,
/// then ingests the JSON content with file_hash metadata.
#[allow(clippy::too_many_arguments)]
pub(crate) async fn process_single_file_via_smart_folder(
    file_path: &std::path::Path,
    progress_id: &str,
    progress_service: &ProgressService,
    node_arc: &std::sync::Arc<tokio::sync::RwLock<crate::fold_node::FoldNode>>,
    auto_execute: bool,
    service: &IngestionService,
    upload_storage: &crate::storage::UploadStorage,
    encryption_key: &[u8; 32],
    force_reingest: bool,
) -> Result<(), String> {
    // Try native parser first (handles json, js/Twitter, csv, txt, md),
    // fall back to file_to_json for unsupported types (images, PDFs, etc.)
    let (data, file_hash, raw_bytes, image_descriptive_name) = match crate::ingestion::smart_folder::read_file_with_hash(
        file_path,
    ) {
        Ok(result) => {
            let (data, hash, bytes) = result;
            (data, hash, bytes, None)
        }
        Err(_) => {
            let raw_bytes = std::fs::read(file_path)
                .map_err(|e| format!("Failed to read file: {}", e))?;
            let hash_hex = {
                use sha2::{Digest, Sha256};
                format!("{:x}", Sha256::digest(&raw_bytes))
            };
            let mut data =
                crate::ingestion::json_processor::convert_file_to_json(&file_path.to_path_buf())
                    .await
                    .map_err(|e| e.to_string())?;
            // Enrich image JSON with image_type and created_at for HashRange schema support
            let image_descriptive_name = file_path
                .file_name()
                .and_then(|n| n.to_str())
                .filter(|name| crate::ingestion::is_image_file(name))
                .and_then(|name| {
                    crate::ingestion::json_processor::enrich_image_json(
                        &mut data,
                        &file_path.to_path_buf(),
                        Some(name),
                    )
                });
            (data, hash_hex, raw_bytes, image_descriptive_name)
        }
    };

    // Encrypt and store the raw file in upload storage (content-addressed)
    let encrypted_data = crate::crypto::envelope::encrypt_envelope(encryption_key, &raw_bytes)
        .map_err(|e| format!("Failed to encrypt file: {}", e))?;
    // Content-addressed: user_id=None (same file = same hash = same object)
    upload_storage
        .save_file_if_not_exists(&file_hash, &encrypted_data, None)
        .await
        .map_err(|e| format!("Failed to store encrypted file: {}", e))?;

    let node = node_arc.read().await;
    let pub_key = node.get_node_public_key().to_string();

    // Check per-user file dedup — skip entire pipeline if this user already ingested this file
    if !force_reingest {
        if let Some(record) = node.is_file_ingested(&pub_key, &file_hash).await {
            log_feature!(
                LogFeature::Ingestion,
                info,
                "File already ingested by this user (at {}), skipping: {}",
                record.ingested_at,
                file_path.display()
            );
            progress_service
                .update_progress(
                    progress_id,
                    crate::ingestion::IngestionStep::Completed,
                    format!("Skipped (already ingested at {})", record.ingested_at),
                )
                .await;
            return Ok(());
        }
    }

    let request = IngestionRequest {
        data,
        auto_execute,
        pub_key,
        source_file_name: file_path
            .file_name()
            .and_then(|n| n.to_str())
            .map(|s| s.to_string()),
        progress_id: Some(progress_id.to_string()),
        file_hash: Some(file_hash),
        source_folder: file_path
            .parent()
            .map(|p| p.to_string_lossy().to_string()),
        image_descriptive_name,
    };

    service
        .process_json_with_node_and_progress(
            request,
            &node,
            progress_service,
            progress_id.to_string(),
        )
        .await
        .map_err(|e| e.user_message())?;

    Ok(())
}

/// Query parameters for the Ollama models endpoint.
#[derive(Debug, Deserialize)]
pub struct OllamaModelsQuery {
    pub base_url: String,
}

/// A single model entry returned by the Ollama `/api/tags` endpoint.
#[derive(Debug, Serialize, Deserialize)]
pub struct OllamaModelInfo {
    pub name: String,
    pub size: u64,
}

/// Return a 200 response with an empty models list and an error message.
fn ollama_models_error(msg: String) -> HttpResponse {
    HttpResponse::Ok().json(json!({ "models": [], "error": msg }))
}

/// List models available on a remote Ollama instance.
///
/// Proxies `GET {base_url}/api/tags` and returns the model list.
/// Short timeout (5 s) to avoid hanging on unreachable servers.
pub async fn list_ollama_models(query: web::Query<OllamaModelsQuery>) -> impl Responder {
    let base_url = query.base_url.trim_end_matches('/');

    let url = format!("{}/api/tags", base_url);

    let client = match reqwest::Client::builder()
        .timeout(std::time::Duration::from_secs(5))
        .build()
    {
        Ok(c) => c,
        Err(e) => return ollama_models_error(format!("Failed to create HTTP client: {}", e)),
    };

    match client.get(&url).send().await {
        Ok(resp) => {
            if !resp.status().is_success() {
                return ollama_models_error(format!("Ollama returned status {}", resp.status()));
            }
            match resp.json::<serde_json::Value>().await {
                Ok(body) => {
                    let models: Vec<OllamaModelInfo> = body
                        .get("models")
                        .and_then(|m| m.as_array())
                        .map(|arr| {
                            arr.iter()
                                .filter_map(|v| {
                                    let name = v.get("name")?.as_str()?.to_string();
                                    let size = v
                                        .get("size")
                                        .and_then(|s| s.as_u64())
                                        .unwrap_or(0);
                                    Some(OllamaModelInfo { name, size })
                                })
                                .collect()
                        })
                        .unwrap_or_default();
                    HttpResponse::Ok().json(json!({ "models": models }))
                }
                Err(e) => ollama_models_error(format!("Failed to parse Ollama response: {}", e)),
            }
        }
        Err(e) => ollama_models_error(format!("Failed to connect to Ollama at {}: {}", base_url, e)),
    }
}

// ---- Batch folder route handlers ----

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

    // Extract user, node, and ingestion service up front
    let (user_id, node_arc, service) = match require_ingestion_context(&state, &ingestion_service).await {
        Ok(ctx) => ctx,
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
    let file_progress_ids =
        start_file_progress(&files_to_ingest, &user_id, &progress_service).await;

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
        &user_id,
        auto_execute,
        service,
        upload_storage.get_ref().clone(),
        encryption_key,
        false,
    );

    // Return immediately with batch info
    HttpResponse::Accepted().json(BatchFolderResponse {
        success: true,
        batch_id,
        files_found: file_progress_ids.len(),
        file_progress_ids,
        message: "Batch ingestion started. Use progress IDs to track individual file status."
            .to_string(),
    })
}

/// Get batch status
pub async fn get_batch_status(
    path: web::Path<String>,
    batch_controller_map: web::Data<BatchControllerMap>,
    progress_tracker: web::Data<ProgressTracker>,
) -> impl Responder {
    let batch_id = path.into_inner();
    let map_guard = batch_controller_map.lock().await;

    match map_guard.get(&batch_id) {
        Some(ctrl_arc) => {
            let ctrl = ctrl_arc.lock().await;
            let mut resp = BatchStatusResponse::from_controller(&ctrl);
            let progress_id = ctrl.in_flight_files.first().map(|f| f.progress_id.clone());
            // Drop the lock before the async progress lookup
            drop(ctrl);
            drop(map_guard);

            // Enrich with per-file progress from the tracker
            if let Some(pid) = progress_id {
                match progress_tracker.load(&pid).await {
                    Ok(Some(job)) => {
                        resp.current_file_step = Some(job.message);
                        resp.current_file_progress = Some(job.progress_percentage);
                    }
                    _ => {
                        resp.current_file_step = Some("Processing...".to_string());
                        resp.current_file_progress = Some(0);
                    }
                }
            }
            HttpResponse::Ok().json(resp)
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
