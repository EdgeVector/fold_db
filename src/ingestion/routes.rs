//! HTTP route handlers for the ingestion API

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
pub use super::batch_routes::*;
pub use super::smart_folder_routes::*;

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
    request: web::Json<crate::ingestion::config::SavedConfig>,
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
    let (data, file_hash, raw_bytes) = match crate::ingestion::smart_folder::read_file_with_hash(
        file_path,
    ) {
        Ok(result) => result,
        Err(_) => {
            let raw_bytes = std::fs::read(file_path)
                .map_err(|e| format!("Failed to read file: {}", e))?;
            let hash_hex = {
                use sha2::{Digest, Sha256};
                format!("{:x}", Sha256::digest(&raw_bytes))
            };
            let data =
                crate::ingestion::json_processor::convert_file_to_json(&file_path.to_path_buf())
                    .await
                    .map_err(|e| e.to_string())?;
            (data, hash_hex, raw_bytes)
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
        trust_distance: 0,
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
