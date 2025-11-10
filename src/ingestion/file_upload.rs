//! File upload and conversion module for ingestion

use crate::datafold_node::http_server::AppState;
use crate::ingestion::multipart_parser::parse_multipart;
use crate::ingestion::json_processor::{
    convert_file_to_json, flatten_root_layers, save_json_to_temp_file,
};
use crate::ingestion::ingestion_spawner::{spawn_background_ingestion, IngestionSpawnConfig};
use crate::log_feature;
use crate::logging::features::LogFeature;
use actix_multipart::Multipart;
use actix_web::{web, HttpResponse, Responder};
use serde_json::json;

/// Process file upload and ingestion
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
    state: web::Data<AppState>,
) -> impl Responder {
    log_feature!(
        LogFeature::Ingestion,
        info,
        "Received file upload request"
    );

    // Extract file and form data from multipart request
    let form_data = match parse_multipart(payload).await {
        Ok(data) => data,
        Err(response) => return response,
    };

    // Convert file to JSON using file_to_json
    let json_value = match convert_file_to_json(&form_data.file_path).await {
        Ok(json) => json,
        Err(response) => return response,
    };

    // Flatten unnecessary root layers if pattern is root->array or root->root->array
    let flattened_json = flatten_root_layers(json_value);

    log_feature!(
        LogFeature::Ingestion,
        info,
        "File converted to JSON successfully, starting ingestion"
    );

    // Save JSON to a temporary file for testing/debugging
    let temp_json_path = save_json_debug_file(&flattened_json);

    // Spawn background ingestion and get progress_id
    let spawn_config = IngestionSpawnConfig {
        json_data: flattened_json,
        auto_execute: form_data.auto_execute,
        trust_distance: form_data.trust_distance,
        pub_key: form_data.pub_key,
    };

    let progress_id = spawn_background_ingestion(spawn_config, &state);

    // Return immediately with the progress_id
    log_feature!(
        LogFeature::Ingestion,
        info,
        "Returning progress_id to client for file upload: {}",
        progress_id
    );

    build_upload_response(
        progress_id,
        &form_data.file_path,
        temp_json_path,
    )
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
        "file_path": file_path.to_string_lossy().to_string()
    });

    if let Some(json_path) = temp_json_path {
        response["converted_json_path"] = json!(json_path);
    }

    HttpResponse::Accepted().json(response)
}
