//! Multipart form data parsing for file uploads

use actix_multipart::Multipart;
use actix_web::HttpResponse;
use futures_util::StreamExt;
use serde_json::json;
use std::path::PathBuf;
use tokio::fs;

use crate::log_feature;
use crate::logging::features::LogFeature;
use crate::storage::UploadStorage;

/// Data extracted from multipart upload form
#[derive(Debug)]
pub struct UploadFormData {
    pub file_path: PathBuf,
    /// The unique filename as saved to disk (HASH_originalname format).
    /// This matches the filename in data/uploads/ directory.
    pub original_filename: String,
    pub auto_execute: bool,
    pub trust_distance: u32,
    pub pub_key: String,
    /// Whether this file already existed (true = duplicate upload)
    pub already_exists: bool,
}

/// Extract and parse multipart form data
pub async fn parse_multipart(
    mut payload: Multipart,
    upload_storage: &UploadStorage,
) -> Result<UploadFormData, HttpResponse> {
    let mut file_path: Option<PathBuf> = None;
    let mut original_filename: Option<String> = None;
    let mut already_exists = false;
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
                let (path, filename, exists) = save_uploaded_file(field, upload_storage).await?;
                file_path = Some(path);
                original_filename = Some(filename);
                already_exists = exists;
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

    let original_filename = original_filename.unwrap_or_else(|| "unknown".to_string());

    Ok(UploadFormData {
        file_path,
        original_filename,
        auto_execute,
        trust_distance,
        pub_key,
        already_exists,
    })
}

/// Save uploaded file from multipart field with content-based hash
/// Returns (file_path, unique_filename, already_exists) where:
/// - unique_filename has format: HASH_originalname (first 16 chars of SHA256)
/// - already_exists is true if this exact file was already uploaded
async fn save_uploaded_file(
    mut field: actix_multipart::Field,
    upload_storage: &UploadStorage,
) -> Result<(PathBuf, String, bool), HttpResponse> {
    use sha2::{Sha256, Digest};

    let filename = field
        .content_disposition()
        .get_filename()
        .unwrap_or("uploaded_file")
        .to_string();

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

    // Generate hash-based filename (use first 16 chars of hex for readability)
    let hash_result = hasher.finalize();
    let hash_hex = format!("{:x}", hash_result);
    let short_hash = &hash_hex[..16]; // First 16 characters provides plenty of uniqueness
    let unique_filename = format!("{}_{}", short_hash, &filename);

    // Atomically save file only if it doesn't exist (prevents race condition)
    let (storage_path, already_exists) = match upload_storage.save_file_if_not_exists(&unique_filename, &file_data).await {
        Ok((path, exists)) => (path, exists),
        Err(e) => {
            log_feature!(
                LogFeature::Ingestion,
                error,
                "Failed to save file: {}",
                e
            );
            return Err(HttpResponse::InternalServerError().json(json!({
                "success": false,
                "error": format!("Failed to save file: {}", e)
            })));
        }
    };

    // Handle duplicate detection
    if already_exists {
        // File already exists (duplicate upload detected atomically)
        let filepath = match upload_storage {
            UploadStorage::Local { .. } => {
                // File is already on local disk, return permanent path
                storage_path
            }
            UploadStorage::S3 { .. } => {
                // File is in S3, need to save to /tmp for processing
                let temp_path = std::env::temp_dir().join(&unique_filename);
                if let Err(e) = fs::write(&temp_path, &file_data).await {
                    log_feature!(
                        LogFeature::Ingestion,
                        error,
                        "Failed to write temp file for duplicate: {}",
                        e
                    );
                    return Err(HttpResponse::InternalServerError().json(json!({
                        "success": false,
                        "error": format!("Failed to write temp file: {}", e)
                    })));
                }
                temp_path
            }
        };
        
        log_feature!(
            LogFeature::Ingestion,
            info,
            "File already exists (duplicate upload): {} at {}",
            unique_filename,
            upload_storage.get_display_path(&unique_filename)
        );
        return Ok((filepath, unique_filename, true));
    }

    // File was newly created, determine processing path
    let filepath = match upload_storage {
        UploadStorage::Local { .. } => {
            // For local storage: file already saved, use that path for processing
            log_feature!(
                LogFeature::Ingestion,
                info,
                "File saved to local storage: {} at {}",
                unique_filename,
                upload_storage.get_display_path(&unique_filename)
            );
            storage_path
        }
        UploadStorage::S3 { .. } => {
            // For S3 storage: file already saved to S3, also save to /tmp for processing
            log_feature!(
                LogFeature::Ingestion,
                info,
                "File saved to S3: {} at {}",
                unique_filename,
                upload_storage.get_display_path(&unique_filename)
            );
            
            // Also save to /tmp for processing (file_to_json needs local file)
            let temp_path = std::env::temp_dir().join(&unique_filename);
            if let Err(e) = fs::write(&temp_path, &file_data).await {
                log_feature!(
                    LogFeature::Ingestion,
                    error,
                    "Failed to write temp file for processing: {}",
                    e
                );
                return Err(HttpResponse::InternalServerError().json(json!({
                    "success": false,
                    "error": format!("Failed to write temp file for processing: {}", e)
                })));
            }
            
            log_feature!(
                LogFeature::Ingestion,
                info,
                "File saved to /tmp for processing: {:?}",
                temp_path
            );
            
            temp_path
        }
    };

    log_feature!(
        LogFeature::Ingestion,
        info,
        "File ready for processing (new upload): {}",
        unique_filename
    );

    Ok((filepath, unique_filename, false))
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

#[cfg(test)]
mod tests {
    use sha2::{Sha256, Digest};

    #[test]
    fn test_unique_filename_format() {
        // Verify the unique filename format matches HASH_originalname pattern
        let test_content = b"test file content";
        let mut hasher = Sha256::new();
        hasher.update(test_content);
        let hash_result = hasher.finalize();
        let hash_hex = format!("{:x}", hash_result);
        let short_hash = &hash_hex[..16];
        
        let original = "tweets.js";
        let unique = format!("{}_{}", short_hash, original);
        
        // Verify format
        assert!(unique.contains('_'));
        assert!(unique.ends_with("tweets.js"));
        
        // Verify we can extract the original name if needed
        let parts: Vec<&str> = unique.splitn(2, '_').collect();
        assert_eq!(parts.len(), 2);
        assert_eq!(parts[0].len(), 16); // Short hash is 16 chars
        assert_eq!(parts[1], original);
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

