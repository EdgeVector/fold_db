//! File upload and conversion module for ingestion

use crate::ingestion::ingestion_service::IngestionService;
use crate::ingestion::json_processor::{convert_file_to_json_http, save_json_to_temp_file};
use crate::ingestion::multipart_parser::parse_multipart;
use crate::ingestion::{IngestionRequest, ProgressTracker};
use crate::log_feature;
use crate::logging::features::LogFeature;
use crate::server::http_server::AppState;
use crate::server::routes::require_node;
use crate::storage::UploadStorage;
use actix_multipart::Multipart;
use actix_web::{web, HttpResponse, Responder};
use serde_json::json;
use std::sync::Arc;

/// Process file upload and ingestion
///
/// Accepts multipart/form-data with either:
/// - file: Binary file to upload (traditional upload)
/// - s3FilePath: S3 path (e.g., "s3://bucket/path/to/file.json") for files already in S3
///
/// Additional optional fields:
/// - autoExecute: Boolean (default: true)
/// - trustDistance: Number (default: 0)
/// - pubKey: String (default: "default")
///
/// Note: Provide either 'file' OR 's3FilePath', not both.
/// If s3FilePath is used, the file is downloaded from S3 for processing but not re-uploaded.
#[utoipa::path(
    post,
    path = "/api/ingestion/upload",
    tag = "ingestion",
    responses(
        (status = 202, description = "Upload accepted and processing started", body = Value),
        (status = 400, description = "Bad request - invalid file or data", body = Value),
        (status = 500, description = "Internal server error", body = Value)
    )
)]
pub async fn upload_file(
    payload: Multipart,
    upload_storage: web::Data<UploadStorage>,
    progress_tracker: web::Data<ProgressTracker>,
    state: web::Data<AppState>,
    ingestion_service: web::Data<Option<Arc<IngestionService>>>,
) -> impl Responder {
    log_feature!(LogFeature::Ingestion, info, "Received file upload request");

    // Extract file and form data from multipart request
    let form_data = match parse_multipart(payload, &upload_storage).await {
        Ok(data) => data,
        Err(response) => return response,
    };

    // Check if file already exists (duplicate upload) - Log it but proceed with ingestion!
    if form_data.already_exists {
        log_feature!(
            LogFeature::Ingestion,
            info,
            "File already exists (duplicate upload): {}. Proceeding with re-ingestion.",
            form_data.original_filename
        );
    }

    // Convert file to JSON using file_to_json
    let json_value = match convert_file_to_json_http(&form_data.file_path).await {
        Ok(json) => json,
        Err(response) => return response,
    };

    log_feature!(
        LogFeature::Ingestion,
        info,
        "File converted to JSON successfully, starting ingestion"
    );

    // Save JSON to a temporary file for testing/debugging
    let temp_json_path = save_json_debug_file(&json_value);

    log_feature!(
        LogFeature::Ingestion,
        info,
        "Creating mutations with source_file_name: {}",
        form_data.original_filename
    );

    // Use client-provided progress_id if available, otherwise generate one
    let progress_id = form_data
        .progress_id
        .clone()
        .unwrap_or_else(|| uuid::Uuid::new_v4().to_string());

    let (user_id, node_arc) = match require_node(&state).await {
        Ok(res) => res,
        Err(response) => return response,
    };

    // Build ingestion request and delegate to the shared handler
    let request = IngestionRequest {
        data: json_value,
        auto_execute: form_data.auto_execute,
        trust_distance: form_data.trust_distance,
        pub_key: form_data.pub_key,
        source_file_name: Some(form_data.original_filename.clone()),
        progress_id: Some(progress_id),
    };

    // Extract ingestion service
    let service = match ingestion_service.get_ref() {
        Some(s) => s.clone(),
        None => {
            return HttpResponse::ServiceUnavailable().json(json!({
                "error": "Ingestion service not available"
            }));
        }
    };

    // Lock briefly — the handler clones the node and spawns a background task
    let node = node_arc.read().await;

    match crate::handlers::ingestion::process_json(
        request,
        &user_id,
        progress_tracker.get_ref(),
        &node,
        service,
    )
    .await
    {
        Ok(api_response) => {
            let progress_id = api_response
                .data
                .as_ref()
                .map(|d| d.progress_id.clone())
                .unwrap_or_default();

            log_feature!(
                LogFeature::Ingestion,
                info,
                "Returning progress_id to client for file upload: {}",
                progress_id
            );

            build_upload_response(progress_id, &form_data.file_path, temp_json_path)
        }
        Err(e) => {
            let status_code = match e.status_code() {
                400 => actix_web::http::StatusCode::BAD_REQUEST,
                503 => actix_web::http::StatusCode::SERVICE_UNAVAILABLE,
                _ => actix_web::http::StatusCode::INTERNAL_SERVER_ERROR,
            };
            HttpResponse::build(status_code).json(e.to_response())
        }
    }
}

/// Save JSON to debug file and return path
fn save_json_debug_file(json: &serde_json::Value) -> Option<String> {
    match save_json_to_temp_file(json) {
        Ok(path) => {
            log_feature!(
                LogFeature::Ingestion,
                info,
                "Converted JSON saved to temporary file for testing: {}",
                path
            );
            Some(path)
        }
        Err(e) => {
            log_feature!(
                LogFeature::Ingestion,
                warn,
                "Failed to save JSON to temp file (non-critical): {}",
                e
            );
            None
        }
    }
}

/// Build the HTTP response for file upload
fn build_upload_response(
    progress_id: String,
    file_path: &std::path::Path,
    temp_json_path: Option<String>,
) -> HttpResponse {
    let mut response = json!({
        "success": true,
        "progress_id": progress_id,
        "message": "File upload and ingestion started. Use progress_id to track status.",
        "file_path": file_path.to_string_lossy().to_string(),
        "duplicate": false
    });

    if let Some(json_path) = temp_json_path {
        response["converted_json_path"] = json!(json_path);
    }

    HttpResponse::Accepted().json(response)
}
