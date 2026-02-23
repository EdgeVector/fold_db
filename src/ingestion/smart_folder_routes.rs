//! Smart folder route handlers — LLM-powered file filtering and ingestion.

use crate::ingestion::batch_controller::{
    BatchController, BatchControllerMap, BatchStatus, PendingFile,
};
use crate::ingestion::ingestion_service::IngestionService;
use crate::ingestion::progress::ProgressService;
use crate::ingestion::smart_folder;
use crate::ingestion::ProgressTracker;
use crate::log_feature;
use crate::logging::features::LogFeature;
use crate::server::http_server::AppState;
use crate::server::routes::{require_node, require_user_context};
use actix_web::{web, HttpResponse, Responder};
use serde::{Deserialize, Serialize};
use serde_json::json;
use std::path::Path;
use std::sync::Arc;
use tokio::sync::Mutex;

use super::routes::{
    get_ingestion_service, process_single_file_via_smart_folder, resolve_folder_path,
    start_file_progress, validate_folder, BatchFolderResponse, IngestionServiceState,
};

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
    state: web::Data<AppState>,
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

    // Get node for dedup checking (best-effort — scan still works without it)
    let node_arc = require_node(&state).await.ok().map(|(_uid, arc)| arc);

    let service_opt = get_ingestion_service(&ingestion_service).await;
    let service_ref = service_opt.as_deref();

    let result = if let Some(ref arc) = node_arc {
        let node_guard = arc.read().await;
        smart_folder::perform_smart_folder_scan(
            &folder_path,
            max_depth,
            max_files,
            service_ref,
            Some(&*node_guard),
        )
        .await
    } else {
        smart_folder::perform_smart_folder_scan(
            &folder_path,
            max_depth,
            max_files,
            service_ref,
            None,
        )
        .await
    };

    match result {
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
    let controller = BatchController::new(batch_id.clone(), request.spend_limit, pending_files);
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

            // Clean up the controller after a short delay so final status
            // polls can still read it before it's removed.
            let map_cleanup = map.clone();
            let batch_id_cleanup = batch_id.clone();
            tokio::spawn(async move {
                tokio::time::sleep(std::time::Duration::from_secs(300)).await;
                let mut map_guard = map_cleanup.lock().await;
                map_guard.remove(&batch_id_cleanup);
            });
        })
        .await
    });
}
