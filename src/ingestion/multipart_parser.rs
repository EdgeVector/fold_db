//! Multipart form data parsing for file uploads

use actix_multipart::Multipart;
use actix_web::HttpResponse;
use futures_util::StreamExt;
use serde_json::json;
use std::path::PathBuf;

use crate::log_feature;
use crate::logging::features::LogFeature;

/// Data extracted from multipart upload form
#[derive(Debug)]
pub struct UploadFormData {
    pub file_path: PathBuf,
    pub auto_execute: bool,
    pub trust_distance: u32,
    pub pub_key: String,
}

/// Extract and parse multipart form data
pub async fn parse_multipart(mut payload: Multipart) -> Result<UploadFormData, HttpResponse> {
    let mut file_path: Option<PathBuf> = None;
    let mut auto_execute = true;
    let mut trust_distance = 0;
    let mut pub_key = "default".to_string();

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

        let field_name = field.content_disposition().get_name().map(|s| s.to_string());

        match field_name.as_deref() {
            Some("file") => {
                file_path = Some(save_uploaded_file(field).await?);
            }
            Some("autoExecute") => {
                auto_execute = parse_field_as_bool(&mut field).await.unwrap_or(true);
            }
            Some("trustDistance") => {
                trust_distance = parse_field_as_u32(&mut field).await.unwrap_or(0);
            }
            Some("pubKey") => {
                pub_key = parse_field_as_string(&mut field).await.unwrap_or_else(|| "default".to_string());
            }
            _ => {}
        }
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

    Ok(UploadFormData {
        file_path,
        auto_execute,
        trust_distance,
        pub_key,
    })
}

/// Save uploaded file from multipart field
async fn save_uploaded_file(
    mut field: actix_multipart::Field,
) -> Result<PathBuf, HttpResponse> {
    use std::io::Write;
    use tokio::fs;

    let filename = field
        .content_disposition()
        .get_filename()
        .unwrap_or("uploaded_file");

    // Create uploads directory if it doesn't exist
    let uploads_dir = PathBuf::from("data/uploads");
    if let Err(e) = fs::create_dir_all(&uploads_dir).await {
        log_feature!(
            LogFeature::Ingestion,
            error,
            "Failed to create uploads directory: {}",
            e
        );
        return Err(HttpResponse::InternalServerError().json(json!({
            "success": false,
            "error": format!("Failed to create uploads directory: {}", e)
        })));
    }

    // Generate unique filename
    let unique_filename = format!("{}_{}", uuid::Uuid::new_v4(), filename);
    let filepath = uploads_dir.join(&unique_filename);

    // Save file to disk
    let mut f = match std::fs::File::create(&filepath) {
        Ok(file) => file,
        Err(e) => {
            log_feature!(
                LogFeature::Ingestion,
                error,
                "Failed to create file: {}",
                e
            );
            return Err(HttpResponse::InternalServerError().json(json!({
                "success": false,
                "error": format!("Failed to create file: {}", e)
            })));
        }
    };

    // Write file contents
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

        if let Err(e) = f.write_all(&data) {
            log_feature!(
                LogFeature::Ingestion,
                error,
                "Failed to write file chunk: {}",
                e
            );
            return Err(HttpResponse::InternalServerError().json(json!({
                "success": false,
                "error": format!("Failed to write file: {}", e)
            })));
        }
    }

    log_feature!(
        LogFeature::Ingestion,
        info,
        "File saved to: {:?}",
        filepath
    );

    Ok(filepath)
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

/// Parse multipart field as u32
async fn parse_field_as_u32(field: &mut actix_multipart::Field) -> Option<u32> {
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

