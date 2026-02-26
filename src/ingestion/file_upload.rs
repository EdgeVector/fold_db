//! File upload and conversion module for ingestion

use crate::ingestion::json_processor::{convert_file_to_json_http, save_json_to_temp_file};
use crate::ingestion::multipart_parser::parse_multipart;
use crate::ingestion::routes::{get_ingestion_service, IngestionServiceState};
use crate::ingestion::{IngestionRequest, ProgressTracker};
use crate::log_feature;
use crate::logging::features::LogFeature;
use crate::server::http_server::AppState;
use crate::server::routes::require_node;
use crate::storage::UploadStorage;
use actix_multipart::Multipart;
use actix_web::{web, HttpResponse, Responder};
use serde_json::json;

/// Process file upload and ingestion
///
/// Accepts multipart/form-data with either:
/// - file: Binary file to upload (traditional upload)
/// - s3FilePath: S3 path (e.g., "s3://bucket/path/to/file.json") for files already in S3
///
/// Additional optional fields:
/// - autoExecute: Boolean (default: true)
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
    ingestion_service: web::Data<IngestionServiceState>,
) -> impl Responder {
    log_feature!(LogFeature::Ingestion, info, "Received file upload request");

    // Get node first (for encryption key)
    let (user_id, node_arc) = match require_node(&state).await {
        Ok(res) => res,
        Err(response) => return response,
    };
    let encryption_key = {
        let node = node_arc.read().await;
        node.get_encryption_key()
    };

    // Extract file and form data from multipart request (encrypts before save)
    let form_data = match parse_multipart(payload, &upload_storage, &encryption_key).await {
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

    // Check per-user file dedup — skip entire pipeline if this user already ingested this file
    {
        let node = node_arc.read().await;
        let pub_key = node.get_node_public_key().to_string();
        if let Some(record) = node.is_file_ingested(&pub_key, &form_data.file_hash).await {
            log_feature!(
                LogFeature::Ingestion,
                info,
                "File already ingested by this user (at {}), skipping: {}",
                record.ingested_at,
                form_data.original_filename
            );
            return HttpResponse::Ok().json(json!({
                "success": true,
                "message": "File already ingested",
                "duplicate": true,
                "ingested_at": record.ingested_at,
                "source_folder": record.source_folder,
            }));
        }
    }

    // Convert file to JSON using file_to_json
    let mut json_value = match convert_file_to_json_http(&form_data.file_path).await {
        Ok(json) => json,
        Err(response) => return response,
    };

    // Enrich image JSON with image_type and created_at for HashRange schema support
    let image_descriptive_name = if crate::ingestion::is_image_file(&form_data.original_filename) {
        crate::ingestion::json_processor::enrich_image_json(
            &mut json_value,
            &form_data.file_path,
            Some(&form_data.original_filename),
        )
    } else {
        None
    };

    // Clean up the unencrypted temp file now that conversion is complete.
    // The encrypted copy is already stored; leaving plaintext on disk is a data leak.
    if let Err(e) = tokio::fs::remove_file(&form_data.file_path).await {
        log_feature!(
            LogFeature::Ingestion,
            warn,
            "Failed to clean up temp processing file {:?}: {}",
            form_data.file_path,
            e
        );
    }

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

    // Build ingestion request and delegate to the shared handler
    let request = IngestionRequest {
        data: json_value,
        auto_execute: form_data.auto_execute,
        pub_key: form_data.pub_key,
        source_file_name: Some(form_data.original_filename.clone()),
        progress_id: Some(progress_id),
        file_hash: Some(form_data.file_hash.clone()),
        source_folder: None,
        image_descriptive_name,
    };

    // Extract ingestion service
    let service = match get_ingestion_service(&ingestion_service).await {
        Some(s) => s,
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

/// Serve an uploaded file by its content hash.
///
/// Reads the encrypted file from upload storage, decrypts it, and
/// returns the raw bytes with an appropriate Content-Type header
/// derived from the optional `name` query parameter.
#[utoipa::path(
    get,
    path = "/api/file/{hash}",
    tag = "ingestion",
    params(
        ("hash" = String, Path, description = "SHA256 content hash of the file"),
        ("name" = Option<String>, Query, description = "Original filename for Content-Type detection")
    ),
    responses(
        (status = 200, description = "File content"),
        (status = 404, description = "File not found"),
        (status = 500, description = "Internal server error")
    )
)]
pub async fn serve_file(
    path: web::Path<String>,
    query: web::Query<std::collections::HashMap<String, String>>,
    upload_storage: web::Data<UploadStorage>,
    state: web::Data<AppState>,
) -> impl Responder {
    let file_hash = path.into_inner();

    // Get encryption key from node
    let (_user_id, node_arc) = match require_node(&state).await {
        Ok(res) => res,
        Err(response) => return response,
    };
    let encryption_key = {
        let node = node_arc.read().await;
        node.get_encryption_key()
    };

    // Read encrypted file (content-addressed, user_id=None)
    let encrypted_data = match upload_storage.read_file(&file_hash, None).await {
        Ok(data) => data,
        Err(_) => {
            return HttpResponse::NotFound().json(json!({
                "error": "File not found"
            }));
        }
    };

    // Decrypt
    let decrypted = match crate::crypto::envelope::decrypt_envelope(&encryption_key, &encrypted_data) {
        Ok(data) => data,
        Err(e) => {
            log_feature!(LogFeature::Ingestion, error, "Failed to decrypt file {}: {}", file_hash, e);
            return HttpResponse::InternalServerError().json(json!({
                "error": "Failed to decrypt file"
            }));
        }
    };

    // Determine content type from optional name query param
    let content_type = query.get("name")
        .and_then(|name| {
            let lower = name.to_lowercase();
            if lower.ends_with(".jpg") || lower.ends_with(".jpeg") { Some("image/jpeg") }
            else if lower.ends_with(".png") { Some("image/png") }
            else if lower.ends_with(".gif") { Some("image/gif") }
            else if lower.ends_with(".webp") { Some("image/webp") }
            else if lower.ends_with(".svg") { Some("image/svg+xml") }
            else if lower.ends_with(".pdf") { Some("application/pdf") }
            else if lower.ends_with(".json") { Some("application/json") }
            else if lower.ends_with(".csv") { Some("text/csv") }
            else if lower.ends_with(".txt") { Some("text/plain") }
            else { None }
        })
        .unwrap_or("application/octet-stream");

    HttpResponse::Ok()
        .content_type(content_type)
        .body(decrypted)
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
