//! File upload and conversion module for ingestion

use crate::ingestion::json_processor::{convert_file_to_json_http, save_json_to_temp_file};
use crate::ingestion::routes::{get_ingestion_service, IngestionServiceState};
use crate::ingestion::{IngestionRequest, ProgressTracker};
use crate::log_feature;
use crate::logging::features::LogFeature;
use crate::server::http_server::AppState;
use crate::server::routes::require_node;
use crate::storage::UploadStorage;
use actix_multipart::Multipart;
use actix_web::{web, HttpResponse, Responder};
use futures_util::StreamExt;
use serde_json::json;
use std::path::PathBuf;
#[cfg(feature = "aws-backend")]
use tokio::fs;

// ---- Multipart form data parsing ----

/// Data extracted from multipart upload form
#[derive(Debug)]
pub struct UploadFormData {
    pub file_path: PathBuf,
    /// The unique filename as saved to disk (full SHA256 hex hash).
    /// This matches the filename in data/uploads/ directory.
    pub original_filename: String,
    pub auto_execute: bool,
    pub pub_key: String,
    /// Whether this file already existed (true = duplicate upload)
    pub already_exists: bool,
    pub progress_id: Option<String>,
    /// Full SHA256 hex hash of the uploaded file content
    pub file_hash: String,
}

/// Extract and parse multipart form data
pub async fn parse_multipart(
    mut payload: Multipart,
    upload_storage: &UploadStorage,
    encryption_key: &[u8; 32],
) -> Result<UploadFormData, HttpResponse> {
    let mut file_path: Option<PathBuf> = None;
    let mut original_filename: Option<String> = None;
    let mut file_hash: Option<String> = None;
    let mut already_exists = false;
    let mut auto_execute = true;
    let mut pub_key = "default".to_string();
    let mut progress_id = None;
    #[cfg(feature = "aws-backend")]
    let mut s3_file_path: Option<String> = None;

    while let Some(item) = payload.next().await {
        let mut field = match item {
            Ok(field) => field,
            Err(e) => {
                log_feature!(
                    LogFeature::Ingestion,
                    error,
                    "Failed to read multipart field: {}",
                    e
                );
                return Err(HttpResponse::BadRequest().json(json!({
                    "success": false,
                    "error": format!("Failed to read multipart data: {}", e)
                })));
            }
        };

        let field_name = field
            .content_disposition()
            .get_name()
            .map(|s| s.to_string());

        match field_name.as_deref() {
            Some("file") => {
                let (path, filename, exists, hash) = save_uploaded_file(field, upload_storage, encryption_key).await?;
                file_path = Some(path);
                original_filename = Some(filename);
                already_exists = exists;
                file_hash = Some(hash);
            }
            #[cfg(feature = "aws-backend")]
            Some("s3FilePath") => {
                s3_file_path = parse_field_as_string(&mut field).await;
            }
            Some("autoExecute") => {
                auto_execute = parse_field_as_bool(&mut field).await.unwrap_or(true);
            }
            Some("pubKey") => {
                pub_key = parse_field_as_string(&mut field)
                    .await
                    .unwrap_or_else(|| "default".to_string());
            }
            Some("progressId") | Some("progress_id") => {
                progress_id = parse_field_as_string(&mut field).await;
            }
            _ => {}
        }
    }

    // Handle S3 file path if provided (alternative to file upload)
    #[cfg(feature = "aws-backend")]
    if let Some(s3_path) = s3_file_path {
        if file_path.is_some() {
            log_feature!(
                LogFeature::Ingestion,
                error,
                "Both file and s3FilePath provided - only one is allowed"
            );
            return Err(HttpResponse::BadRequest().json(json!({
                "success": false,
                "error": "Cannot provide both 'file' and 's3FilePath' - use one or the other"
            })));
        }

        log_feature!(
            LogFeature::Ingestion,
            info,
            "Processing S3 file path: {}",
            s3_path
        );

        let (path, filename) = handle_s3_file_path(&s3_path, upload_storage).await?;
        file_path = Some(path);
        original_filename = Some(filename);
        already_exists = false; // S3 files are not deduplicated (already in S3)
    }

    let file_path = match file_path {
        Some(path) => path,
        None => {
            log_feature!(LogFeature::Ingestion, error, "No file provided in upload");
            return Err(HttpResponse::BadRequest().json(json!({
                "success": false,
                "error": "No file provided"
            })));
        }
    };

    let original_filename = original_filename.unwrap_or_else(|| "unknown".to_string());

    Ok(UploadFormData {
        file_path,
        original_filename,
        auto_execute,
        pub_key,
        already_exists,
        progress_id,
        file_hash: file_hash.unwrap_or_default(),
    })
}

/// Save uploaded file from multipart field with content-based hash and encryption
/// Returns (file_path, unique_filename, already_exists, file_hash) where:
/// - unique_filename is the full SHA256 hex hash (content-addressed)
/// - already_exists is true if this exact file was already uploaded
/// - file_hash is the full SHA256 hex hash string
async fn save_uploaded_file(
    mut field: actix_multipart::Field,
    upload_storage: &UploadStorage,
    encryption_key: &[u8; 32],
) -> Result<(PathBuf, String, bool, String), HttpResponse> {
    use sha2::{Digest, Sha256};

    // Read file contents and compute hash simultaneously
    let mut hasher = Sha256::new();
    let mut file_data = Vec::new();

    while let Some(chunk) = field.next().await {
        let data = match chunk {
            Ok(data) => data,
            Err(e) => {
                log_feature!(
                    LogFeature::Ingestion,
                    error,
                    "Failed to read file chunk: {}",
                    e
                );
                return Err(HttpResponse::InternalServerError().json(json!({
                    "success": false,
                    "error": format!("Failed to read file: {}", e)
                })));
            }
        };

        hasher.update(&data);
        file_data.extend_from_slice(&data);
    }

    // Use full SHA256 hex hash as content-addressed filename
    let hash_result = hasher.finalize();
    let hash_hex = format!("{:x}", hash_result);
    let unique_filename = hash_hex.clone();

    // Encrypt file data before storage
    let encrypted_data = crate::crypto::envelope::encrypt_envelope(encryption_key, &file_data)
        .map_err(|e| {
            log_feature!(LogFeature::Ingestion, error, "Failed to encrypt file: {}", e);
            HttpResponse::InternalServerError().json(json!({
                "success": false,
                "error": format!("Failed to encrypt file: {}", e)
            }))
        })?;

    // Content-addressed storage: user_id=None (same file = same hash = same object)
    let (_storage_path, already_exists) = match upload_storage
        .save_file_if_not_exists(&unique_filename, &encrypted_data, None)
        .await
    {
        Ok((path, exists)) => (path, exists),
        Err(e) => {
            log_feature!(LogFeature::Ingestion, error, "Failed to save file: {}", e);
            return Err(HttpResponse::InternalServerError().json(json!({
                "success": false,
                "error": format!("Failed to save file: {}", e)
            })));
        }
    };

    // Handle duplicate detection
    if already_exists {
        log_feature!(
            LogFeature::Ingestion,
            info,
            "File already exists (duplicate upload): {} at {}",
            unique_filename,
            upload_storage.get_display_path(&unique_filename, None)
        );
        // For processing, we need unencrypted data on a local path
        let process_path = write_unencrypted_for_processing(&unique_filename, &file_data, upload_storage).await?;
        return Ok((process_path, unique_filename, true, hash_hex));
    }

    // Storage has encrypted data; file_to_json needs unencrypted data on a local path
    let filepath = write_unencrypted_for_processing(&unique_filename, &file_data, upload_storage).await?;

    log_feature!(
        LogFeature::Ingestion,
        info,
        "File encrypted and saved to storage: {}. Unencrypted copy at {:?} for processing.",
        upload_storage.get_display_path(&unique_filename, None),
        filepath
    );

    Ok((filepath, unique_filename, false, hash_hex))
}

/// Write unencrypted file data to a temp path for processing by file_to_json.
/// Storage holds encrypted data; this provides the plaintext for conversion.
async fn write_unencrypted_for_processing(
    filename: &str,
    file_data: &[u8],
    _upload_storage: &UploadStorage,
) -> Result<PathBuf, HttpResponse> {
    let temp_path = std::env::temp_dir().join(format!("folddb_proc_{}", filename));
    tokio::fs::write(&temp_path, file_data).await.map_err(|e| {
        log_feature!(
            LogFeature::Ingestion,
            error,
            "Failed to write unencrypted file to temp for processing: {}",
            e
        );
        HttpResponse::InternalServerError().json(json!({
            "success": false,
            "error": format!("Failed to write file to temp directory: {}", e)
        }))
    })?;
    Ok(temp_path)
}

/// Parse multipart field as boolean
async fn parse_field_as_bool(field: &mut actix_multipart::Field) -> Option<bool> {
    let mut bytes = Vec::new();
    while let Some(chunk) = field.next().await {
        if let Ok(data) = chunk {
            bytes.extend_from_slice(&data);
        }
    }
    String::from_utf8(bytes).ok()?.parse().ok()
}

/// Parse multipart field as string
async fn parse_field_as_string(field: &mut actix_multipart::Field) -> Option<String> {
    let mut bytes = Vec::new();
    while let Some(chunk) = field.next().await {
        if let Ok(data) = chunk {
            bytes.extend_from_slice(&data);
        }
    }
    String::from_utf8(bytes).ok()
}

/// Handle S3 file path input
/// Downloads file from S3 to /tmp for processing
/// Returns (local_path, filename)
#[cfg(feature = "aws-backend")]
async fn handle_s3_file_path(
    s3_path: &str,
    upload_storage: &UploadStorage,
) -> Result<(PathBuf, String), HttpResponse> {
    // Parse S3 path (format: s3://bucket/key or s3://bucket/prefix/key)
    if !s3_path.starts_with("s3://") {
        log_feature!(
            LogFeature::Ingestion,
            error,
            "Invalid S3 path format: {}",
            s3_path
        );
        return Err(HttpResponse::BadRequest().json(json!({
            "success": false,
            "error": format!("Invalid S3 path format. Expected 's3://bucket/key', got: {}", s3_path)
        })));
    }

    let path_without_prefix = &s3_path[5..]; // Remove "s3://"
    let parts: Vec<&str> = path_without_prefix.splitn(2, '/').collect();

    if parts.len() != 2 {
        log_feature!(
            LogFeature::Ingestion,
            error,
            "Invalid S3 path structure: {}",
            s3_path
        );
        return Err(HttpResponse::BadRequest().json(json!({
            "success": false,
            "error": format!("Invalid S3 path. Expected 's3://bucket/key', got: {}", s3_path)
        })));
    }

    let bucket = parts[0];
    let key = parts[1];

    // Extract filename from key (last path segment) and sanitize against path traversal
    let raw_filename = key.rsplit('/').next().unwrap_or(key).to_string();
    // Strip any path separators or parent-directory traversal sequences
    let filename: String = raw_filename
        .replace(['/', '\\'], "_")
        .replace("..", "_");
    if filename.is_empty() {
        return Err(HttpResponse::BadRequest().json(json!({
            "success": false,
            "error": "S3 key produced an empty filename"
        })));
    }

    log_feature!(
        LogFeature::Ingestion,
        info,
        "Downloading S3 file: bucket={}, key={}, filename={}",
        bucket,
        key,
        filename
    );

    // Download file from S3
    let file_data = match upload_storage.download_from_s3_path(bucket, key).await {
        Ok(data) => data,
        Err(e) => {
            log_feature!(
                LogFeature::Ingestion,
                error,
                "Failed to download S3 file: {}",
                e
            );
            return Err(HttpResponse::InternalServerError().json(json!({
                "success": false,
                "error": format!("Failed to download S3 file: {}", e)
            })));
        }
    };

    // Save to /tmp for processing (file_to_json needs local file)
    // Use folddb_ prefix for easy identification and cleanup
    let temp_path = std::env::temp_dir().join(format!("folddb_s3_{}", filename));
    if let Err(e) = fs::write(&temp_path, &file_data).await {
        log_feature!(
            LogFeature::Ingestion,
            error,
            "Failed to write S3 file to /tmp: {}",
            e
        );
        return Err(HttpResponse::InternalServerError().json(json!({
            "success": false,
            "error": format!("Failed to write file to temp directory: {}", e)
        })));
    }

    log_feature!(
        LogFeature::Ingestion,
        info,
        "S3 file downloaded to /tmp for processing: {:?}",
        temp_path
    );

    Ok((temp_path, filename))
}

// ---- File upload handlers ----

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

#[cfg(test)]
mod tests {
    use sha2::{Digest, Sha256};

    #[test]
    fn test_unique_filename_format() {
        // Verify the unique filename is the full SHA256 hex hash (content-addressed)
        let test_content = b"test file content";
        let mut hasher = Sha256::new();
        hasher.update(test_content);
        let hash_result = hasher.finalize();
        let hash_hex = format!("{:x}", hash_result);

        // Full hash is 64 hex chars
        assert_eq!(hash_hex.len(), 64);

        // The filename IS the hash (content-addressed)
        let unique_filename = hash_hex.clone();
        assert_eq!(unique_filename, hash_hex);
    }

    #[test]
    fn test_hash_consistency() {
        // Same content should produce same hash
        let content = b"identical content";

        let mut hasher1 = Sha256::new();
        hasher1.update(content);
        let hash1 = format!("{:x}", hasher1.finalize());

        let mut hasher2 = Sha256::new();
        hasher2.update(content);
        let hash2 = format!("{:x}", hasher2.finalize());

        assert_eq!(hash1, hash2);
    }

    #[test]
    fn test_hash_uniqueness() {
        // Different content should produce different hashes
        let content1 = b"content one";
        let content2 = b"content two";

        let mut hasher1 = Sha256::new();
        hasher1.update(content1);
        let hash1 = format!("{:x}", hasher1.finalize());

        let mut hasher2 = Sha256::new();
        hasher2.update(content2);
        let hash2 = format!("{:x}", hasher2.finalize());

        assert_ne!(hash1, hash2);
    }
}
