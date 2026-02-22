//! Batch folder route handlers — batch ingestion and status tracking.

use crate::ingestion::batch_controller::{BatchControllerMap, BatchStatus, BatchStatusResponse};
use crate::ingestion::progress::ProgressService;
use crate::ingestion::ProgressTracker;
use crate::log_feature;
use crate::logging::features::LogFeature;
use crate::server::http_server::AppState;
use crate::server::routes::{require_node, require_user_context};
use actix_web::{web, HttpResponse, Responder};
use serde::{Deserialize, Serialize};
use serde_json::json;

use super::routes::{
    get_ingestion_service, resolve_folder_path, spawn_file_ingestion_tasks, start_file_progress,
    validate_folder, BatchFolderResponse, IngestionServiceState,
};

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
    let file_progress_ids =
        start_file_progress(&files_to_ingest, &user_id, &progress_service).await;

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
        message: "Batch ingestion started. Use progress IDs to track individual file status."
            .to_string(),
    })
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
