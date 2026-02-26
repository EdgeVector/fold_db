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
use crate::progress::{Job, JobStatus, JobType};
use crate::server::http_server::AppState;
use crate::server::routes::{require_node, require_user_context};
use actix_web::{web, HttpResponse, Responder};
use serde::{Deserialize, Serialize};
use serde_json::json;
use std::path::Path;
use std::sync::Arc;
use tokio::sync::{Mutex, Notify};

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
    /// When true, bypass per-user file dedup so already-ingested files are reprocessed.
    #[serde(default)]
    pub force_reingest: bool,
    /// Max files to process concurrently (default: 4, clamped 1..=8).
    pub max_concurrent: Option<usize>,
}

/// Response from initiating an async scan
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SmartFolderScanStartResponse {
    pub success: bool,
    pub progress_id: String,
}

/// Scan a folder and use LLM to recommend which files contain personal data.
/// Returns a progress_id immediately; poll `/api/ingestion/progress/{id}` for steps,
/// then fetch `/api/ingestion/smart-folder/scan/{id}` for the result.
#[utoipa::path(
    post,
    path = "/api/ingestion/smart-folder/scan",
    tag = "ingestion",
    request_body = SmartFolderScanRequest,
    responses(
        (status = 202, description = "Scan started", body = SmartFolderScanStartResponse),
        (status = 400, description = "Invalid folder path"),
    )
)]
pub async fn smart_folder_scan(
    request: web::Json<SmartFolderScanRequest>,
    state: web::Data<AppState>,
    ingestion_service: web::Data<IngestionServiceState>,
    progress_tracker: web::Data<ProgressTracker>,
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

    // Resolve folder path
    let folder_path = resolve_folder_path(&request.folder_path);

    if let Err(response) = validate_folder(&folder_path) {
        return response;
    }

    let max_depth = request.max_depth.unwrap_or(10);
    let max_files = request.max_files.unwrap_or(100);

    // Create progress tracking
    let progress_id = uuid::Uuid::new_v4().to_string();
    let tracker = progress_tracker.get_ref().clone();

    // Initialize the job
    let mut job = Job::new(progress_id.clone(), JobType::Other("scan".to_string()));
    job = job.with_user(user_id.clone());
    job.message = "Starting scan...".to_string();
    job.progress_percentage = 0;
    if let Err(e) = tracker.save(&job).await {
        log::warn!("Failed to save scan progress: {}", e);
    }

    // Get shared state for the background task
    let node_arc = require_node(&state).await.ok().map(|(_uid, arc)| arc);
    let service_opt = get_ingestion_service(&ingestion_service).await;

    // Spawn the scan in the background
    let pid = progress_id.clone();
    tokio::spawn(async move {
        crate::logging::core::run_with_user(&user_id, async move {
            // Build a progress callback that writes to the tracker
            let tracker_cb = tracker.clone();
            let pid_cb = pid.clone();
            let on_progress: smart_folder::ScanProgressFn = Box::new(move |pct, msg| {
                let tracker_inner = tracker_cb.clone();
                let pid_inner = pid_cb.clone();
                // Fire-and-forget async update via a spawned task
                tokio::spawn(async move {
                    if let Ok(Some(mut job)) = tracker_inner.load(&pid_inner).await {
                        job.update_progress(pct, msg);
                        let _ = tracker_inner.save(&job).await;
                    }
                });
            });

            let result = if let Some(ref arc) = node_arc {
                let node_guard = arc.read().await;
                smart_folder::perform_smart_folder_scan_with_progress(
                    &folder_path,
                    max_depth,
                    max_files,
                    service_opt.as_deref(),
                    Some(&*node_guard),
                    Some(&on_progress),
                )
                .await
            } else {
                smart_folder::perform_smart_folder_scan_with_progress(
                    &folder_path,
                    max_depth,
                    max_files,
                    service_opt.as_deref(),
                    None,
                    Some(&on_progress),
                )
                .await
            };

            // Let any in-flight spawned progress-update tasks drain before
            // writing the final Completed/Failed status.
            tokio::task::yield_now().await;
            tokio::time::sleep(std::time::Duration::from_millis(50)).await;

            // Store result or error in the job
            if let Ok(Some(mut job)) = tracker.load(&pid).await {
                match result {
                    Ok(response) => {
                        let result_json = serde_json::to_value(&response).ok();
                        job.complete(result_json);
                    }
                    Err(e) => {
                        job.fail(e.to_string());
                    }
                }
                let _ = tracker.save(&job).await;
            }
        })
        .await
    });

    HttpResponse::Accepted().json(SmartFolderScanStartResponse {
        success: true,
        progress_id,
    })
}

/// Retrieve the scan result after progress reaches 100%.
#[utoipa::path(
    get,
    path = "/api/ingestion/smart-folder/scan/{id}",
    tag = "ingestion",
    responses(
        (status = 200, description = "Scan result", body = SmartFolderScanResponse),
        (status = 404, description = "Scan not found or not complete"),
    )
)]
pub async fn get_scan_result(
    path: web::Path<String>,
    progress_tracker: web::Data<ProgressTracker>,
) -> impl Responder {
    let progress_id = path.into_inner();
    let tracker = progress_tracker.get_ref();

    match tracker.load(&progress_id).await {
        Ok(Some(job)) => {
            if !matches!(job.status, JobStatus::Completed) {
                return HttpResponse::NotFound().json(json!({
                    "success": false,
                    "error": "Scan not yet complete"
                }));
            }
            match job.result {
                Some(result) => HttpResponse::Ok().json(result),
                None => HttpResponse::NotFound().json(json!({
                    "success": false,
                    "error": "Scan result not available"
                })),
            }
        }
        _ => HttpResponse::NotFound().json(json!({
            "success": false,
            "error": "Scan not found"
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
    let force_reingest = request.force_reingest;

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
    let is_local = service.is_local_provider();
    let controller = BatchController::new(batch_id.clone(), request.spend_limit, pending_files, is_local);
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

    let max_concurrent = request.max_concurrent.unwrap_or(100).clamp(1, 100);

    // Spawn the concurrent coordinator
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
        force_reingest,
        max_concurrent,
    );

    HttpResponse::Accepted().json(BatchFolderResponse {
        success: true,
        batch_id,
        files_found: file_progress_ids.len(),
        file_progress_ids,
        message: "Smart folder ingestion started with spend tracking.".to_string(),
    })
}

/// Result of trying to pop the next file from the batch controller.
enum PopResult {
    /// A file was successfully popped and is ready for processing.
    File(PendingFile),
    /// The spend limit was hit; the batch is now paused.
    SpendLimitHit(Arc<Notify>),
    /// All pending files have been popped (queue empty).
    Done,
    /// The batch was cancelled.
    Cancelled,
}

/// Lock the controller briefly, check cancellation/empty/spend, pop a file.
async fn try_pop_file(
    map: &BatchControllerMap,
    batch_id: &str,
) -> PopResult {
    let map_guard = map.lock().await;
    let ctrl_arc = match map_guard.get(batch_id) {
        Some(c) => c.clone(),
        None => return PopResult::Cancelled,
    };
    let mut ctrl = ctrl_arc.lock().await;

    if ctrl.status == BatchStatus::Cancelled {
        return PopResult::Cancelled;
    }

    let next_file = match ctrl.pending_files.first() {
        Some(f) => f.clone(),
        None => return PopResult::Done,
    };

    if !ctrl.can_proceed(next_file.estimated_cost) {
        ctrl.pause();
        return PopResult::SpendLimitHit(ctrl.resume_notifier());
    }

    let file = ctrl.pop_next_file().expect("pending_files was non-empty");
    let name = file
        .path
        .file_name()
        .and_then(|n| n.to_str())
        .unwrap_or("unknown")
        .to_string();
    ctrl.add_in_flight(name, file.progress_id.clone());
    PopResult::File(file)
}

/// Record the result of processing a single file back into the controller.
async fn record_file_result(
    map: &BatchControllerMap,
    batch_id: &str,
    file: &PendingFile,
    result: &Result<(), String>,
    estimated_cost: f64,
    progress_service: &ProgressService,
) {
    let map_guard = map.lock().await;
    if let Some(ctrl_arc) = map_guard.get(batch_id) {
        let mut ctrl = ctrl_arc.lock().await;
        match result {
            Ok(()) => ctrl.record_completed(&file.progress_id, estimated_cost),
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
                    .fail_progress(&file.progress_id, format!("Processing failed: {}", e))
                    .await;
                ctrl.record_failed(
                    &file.progress_id,
                    file.path.display().to_string(),
                    e.clone(),
                );
            }
        }
    }
}

/// Wait for the resume notification, then check for cancellation.
/// Returns `true` if the batch was cancelled while waiting.
async fn wait_for_resume(notifier: Arc<Notify>, map: &BatchControllerMap, batch_id: &str) -> bool {
    notifier.notified().await;
    let map_guard = map.lock().await;
    if let Some(ctrl_arc) = map_guard.get(batch_id) {
        let ctrl = ctrl_arc.lock().await;
        ctrl.status == BatchStatus::Cancelled
    } else {
        true
    }
}

/// Spawn a concurrent coordinator task that processes up to `max_concurrent`
/// files in parallel, checking the spend limit before each file and pausing
/// when the limit is hit.
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
    force_reingest: bool,
    max_concurrent: usize,
) {
    let progress_tracker = progress_tracker.clone();
    let node_arc = node_arc.clone();
    let user_id = user_id.to_string();
    let map = batch_controller_map.get_ref().clone();

    tokio::spawn(async move {
        crate::logging::core::run_with_user(&user_id, async move {
            let mut join_set = tokio::task::JoinSet::new();
            let mut all_popped = false;

            loop {
                // Fill up to max_concurrent workers
                while !all_popped && join_set.len() < max_concurrent {
                    match try_pop_file(&map, &batch_id).await {
                        PopResult::File(file) => {
                            let node_arc = node_arc.clone();
                            let service = ingestion_service.clone();
                            let storage = upload_storage.clone();
                            let tracker = progress_tracker.clone();
                            let enc_key = encryption_key;
                            join_set.spawn(async move {
                                let progress_service = ProgressService::new(tracker);
                                let result = process_single_file_via_smart_folder(
                                    &file.path,
                                    &file.progress_id,
                                    &progress_service,
                                    &node_arc,
                                    auto_execute,
                                    &service,
                                    &storage,
                                    &enc_key,
                                    force_reingest,
                                )
                                .await;
                                (file, result)
                            });
                        }
                        PopResult::SpendLimitHit(notifier) => {
                            // If tasks are still in flight, let them finish
                            // before blocking on resume.
                            if join_set.is_empty() {
                                let cancelled =
                                    wait_for_resume(notifier, &map, &batch_id).await;
                                if cancelled {
                                    all_popped = true;
                                }
                            }
                            // Either way, stop filling for now — the loop will
                            // drain completions and try filling again.
                            break;
                        }
                        PopResult::Done | PopResult::Cancelled => {
                            all_popped = true;
                            break;
                        }
                    }
                }

                if join_set.is_empty() {
                    break;
                }

                // Wait for the next completion, record result, loop to fill more slots
                if let Some(Ok((file, result))) = join_set.join_next().await {
                    let estimated_cost = file.estimated_cost;
                    let progress_service = ProgressService::new(progress_tracker.clone());
                    record_file_result(
                        &map,
                        &batch_id,
                        &file,
                        &result,
                        estimated_cost,
                        &progress_service,
                    )
                    .await;
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
