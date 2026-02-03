//! S3 file path ingestion for programmatic use
//!
//! This module provides functions for ingesting files directly from S3 paths
//! without needing the HTTP server. This is particularly useful for:
//! - AWS Lambda functions triggered by S3 events
//! - Batch processing scripts
//! - Programmatic data pipelines

use crate::datafold_node::DataFoldNode;
use crate::ingestion::ingestion_spawner::{spawn_background_ingestion, IngestionSpawnConfig};
use crate::ingestion::json_processor::{convert_file_to_json, flatten_root_layers};
use crate::ingestion::{IngestionConfig, IngestionError, IngestionResponse, ProgressTracker};
use crate::storage::UploadStorage;
use std::path::PathBuf;
use std::sync::Arc;
use tokio::fs;
use tokio::sync::Mutex;

/// Request for S3 file ingestion
#[derive(Debug, Clone)]
pub struct S3IngestionRequest {
    /// S3 path in format: s3://bucket/key
    pub s3_path: String,
    /// Whether to auto-execute mutations
    pub auto_execute: bool,
    /// Trust distance for mutations
    pub trust_distance: u32,
    /// Public key for authentication
    pub pub_key: String,
    /// User ID owning the ingestion
    pub user_id: String,
    /// Optional ingestion configuration (if not provided, will use from_env())
    pub ingestion_config: Option<IngestionConfig>,
}

impl S3IngestionRequest {
    /// Create a new S3 ingestion request with default settings
    pub fn new(s3_path: String, user_id: String, pub_key: String) -> Self {
        Self {
            s3_path,
            auto_execute: true,
            trust_distance: 0,
            pub_key,
            user_id,
            ingestion_config: None,
        }
    }

    /// Set whether to auto-execute mutations
    pub fn with_auto_execute(mut self, auto_execute: bool) -> Self {
        self.auto_execute = auto_execute;
        self
    }

    /// Set trust distance
    pub fn with_trust_distance(mut self, trust_distance: u32) -> Self {
        self.trust_distance = trust_distance;
        self
    }

    /// Set public key
    pub fn with_pub_key(mut self, pub_key: String) -> Self {
        self.pub_key = pub_key;
        self
    }

    /// Set a complete ingestion configuration
    pub fn with_ingestion_config(mut self, config: IngestionConfig) -> Self {
        self.ingestion_config = Some(config);
        self
    }

    /// Set the OpenRouter API key directly (convenience method)
    ///
    /// This creates an ingestion config with the provided API key and default settings.
    /// If you need more control over the configuration, use `with_ingestion_config` instead.
    pub fn with_openrouter_api_key(mut self, api_key: String) -> Self {
        let mut config = IngestionConfig::default();
        config.openrouter.api_key = api_key;
        config.enabled = true;
        self.ingestion_config = Some(config);
        self
    }

    /// Set the OpenRouter configuration with custom model and base URL
    pub fn with_openrouter_config(
        mut self,
        api_key: String,
        model: String,
        base_url: String,
    ) -> Self {
        let mut config = IngestionConfig::default();
        config.openrouter.api_key = api_key;
        config.openrouter.model = model;
        config.openrouter.base_url = base_url;
        config.enabled = true;
        self.ingestion_config = Some(config);
        self
    }
}

/// Ingest a file from S3 path with background processing
///
/// This function downloads a file from S3 and starts background ingestion.
/// It returns immediately with a progress_id that can be used to track status.
///
/// # Arguments
///
/// * `request` - S3 ingestion request with path and settings
/// * `upload_storage` - Upload storage for file management
/// * `progress_tracker` - Progress tracker for ingestion operations
/// * `node` - DataFold node for data operations
/// * `ingestion_config` - Optional ingestion configuration (if not provided, uses from_env() or request.ingestion_config)
///
/// # Returns
///
/// Returns an `IngestionResponse` with a progress_id for tracking.
///
/// # Example
///
/// ```ignore
/// use fold_db::ingestion::{ingest_from_s3_path_async, S3IngestionRequest};
/// use fold_db::storage::UploadStorage;
/// use std::sync::Arc;
///
/// #[tokio::main]
/// async fn main() -> Result<(), Box<dyn std::error::Error>> {
///     // Initialize dependencies
///     let upload_storage = UploadStorage::local("uploads".into());
///     let progress_tracker = Arc::new(std::sync::Mutex::new(std::collections::HashMap::new()));
///     let node = Arc::new(tokio::sync::Mutex::new(/* initialize DataFoldNode */));
///     
///     // Option 1: Pass API key directly in request
///     let request = S3IngestionRequest::new("s3://my-bucket/data.json".to_string())
///         .with_openrouter_api_key("your-api-key".to_string());
///     let response = ingest_from_s3_path_async(&request, &upload_storage, &progress_tracker, node.clone(), None).await?;
///     
///     // Option 2: Pass config explicitly
///     let ingestion_config = IngestionConfig::from_env()?;
///     let request = S3IngestionRequest::new("s3://my-bucket/data.json".to_string());
///     let response = ingest_from_s3_path_async(&request, &upload_storage, &progress_tracker, node, Some(&ingestion_config)).await?;
///     
///     println!("Started ingestion: {}", response.progress_id.unwrap());
///     
///     Ok(())
/// }
/// ```
pub async fn ingest_from_s3_path_async(
    request: &S3IngestionRequest,
    upload_storage: &UploadStorage,
    progress_tracker: &ProgressTracker,
    node: Arc<Mutex<DataFoldNode>>,
    ingestion_config: Option<&IngestionConfig>,
) -> Result<IngestionResponse, IngestionError> {
    // Determine which config to use: passed in, from request, or from env
    let config = if let Some(config) = ingestion_config {
        config.clone()
    } else if let Some(config) = &request.ingestion_config {
        config.clone()
    } else {
        IngestionConfig::from_env()?
    };

    // Parse and download S3 file
    let (file_path, filename) = download_s3_file(&request.s3_path, upload_storage).await?;

    // Convert file to JSON
    let json_value = convert_file_to_json(&file_path)
        .await
        .map_err(|_| IngestionError::FileConversionFailed)?;

    // Flatten unnecessary root layers
    let flattened_json = flatten_root_layers(json_value);

    // Spawn background ingestion
    let spawn_config = IngestionSpawnConfig {
        json_data: flattened_json,
        auto_execute: request.auto_execute,
        trust_distance: request.trust_distance,
        pub_key: request.pub_key.clone(),
        source_file_name: Some(filename),
        ingestion_config: config,
    };

    let progress_id = spawn_background_ingestion(
        spawn_config,
        progress_tracker,
        node,
        request.user_id.clone(),
    )
    .await;

    Ok(IngestionResponse {
        success: true,
        progress_id: Some(progress_id),
        schema_used: None,
        new_schema_created: false,
        mutations_generated: 0,
        mutations_executed: 0,
        errors: Vec::new(),
    })
}

/// Ingest a file from S3 path synchronously (waits for completion)
///
/// This function downloads a file from S3, starts ingestion, and polls
/// until completion before returning the final results.
///
/// # Arguments
///
/// * `request` - S3 ingestion request with path and settings
/// * `upload_storage` - Upload storage for file management
/// * `progress_tracker` - Progress tracker for ingestion operations
/// * `node` - DataFold node for data operations
/// * `ingestion_config` - Optional ingestion configuration (if not provided, uses from_env() or request.ingestion_config)
///
/// # Returns
///
/// Returns a complete `IngestionResponse` with results.
///
/// # Example
///
/// ```ignore
/// use fold_db::ingestion::{ingest_from_s3_path_sync, S3IngestionRequest};
/// use fold_db::storage::UploadStorage;
/// use std::sync::Arc;
///
/// #[tokio::main]
/// async fn main() -> Result<(), Box<dyn std::error::Error>> {
///     // Initialize dependencies
///     let upload_storage = UploadStorage::local("uploads".into());
///     let progress_tracker = Arc::new(std::sync::Mutex::new(std::collections::HashMap::new()));
///     let node = Arc::new(tokio::sync::Mutex::new(/* initialize DataFoldNode */));
///     
///     // Pass API key directly in request
///     let request = S3IngestionRequest::new("s3://my-bucket/data.json".to_string())
///         .with_auto_execute(true)
///         .with_openrouter_api_key("your-api-key".to_string());
///     
///     let response = ingest_from_s3_path_sync(&request, &upload_storage, &progress_tracker, node, None).await?;
///     println!("Ingestion complete: {} mutations executed",
///              response.mutations_executed);
///     
///     Ok(())
/// }
/// ```
pub async fn ingest_from_s3_path_sync(
    request: &S3IngestionRequest,
    upload_storage: &UploadStorage,
    progress_tracker: &ProgressTracker,
    node: Arc<Mutex<DataFoldNode>>,
    ingestion_config: Option<&IngestionConfig>,
) -> Result<IngestionResponse, IngestionError> {
    // Start async ingestion
    let async_response = ingest_from_s3_path_async(
        request,
        upload_storage,
        progress_tracker,
        node,
        ingestion_config,
    )
    .await?;

    let progress_id = async_response
        .progress_id
        .ok_or_else(|| IngestionError::InvalidInput("No progress_id returned".to_string()))?;

    // Poll for completion
    loop {
        tokio::time::sleep(tokio::time::Duration::from_millis(200)).await;

        let progress = progress_tracker
            .load(&progress_id)
            .await
            .map_err(|e| IngestionError::InvalidInput(format!("Failed to load progress: {}", e)))?;

        if let Some(progress) = progress {
            if progress.is_complete {
                // Return complete response with results
                return Ok(IngestionResponse {
                    success: true,
                    progress_id: Some(progress_id),
                    schema_used: progress.results.as_ref().map(|r| r.schema_name.clone()),
                    new_schema_created: progress
                        .results
                        .as_ref()
                        .is_some_and(|r| r.new_schema_created),
                    mutations_generated: progress
                        .results
                        .as_ref()
                        .map_or(0, |r| r.mutations_generated),
                    mutations_executed: progress
                        .results
                        .as_ref()
                        .map_or(0, |r| r.mutations_executed),
                    errors: progress.error_message.into_iter().collect(),
                });
            }
        } else {
            return Err(IngestionError::InvalidInput(format!(
                "Progress tracking lost for {}",
                progress_id
            )));
        }
    }
}

/// Download a file from S3 path
///
/// Internal helper function that parses S3 path and downloads file to /tmp
async fn download_s3_file(
    s3_path: &str,
    upload_storage: &UploadStorage,
) -> Result<(PathBuf, String), IngestionError> {
    // Parse S3 path (format: s3://bucket/key)
    if !s3_path.starts_with("s3://") {
        return Err(IngestionError::InvalidInput(format!(
            "Invalid S3 path format. Expected 's3://bucket/key', got: {}",
            s3_path
        )));
    }

    let path_without_prefix = &s3_path[5..]; // Remove "s3://"
    let parts: Vec<&str> = path_without_prefix.splitn(2, '/').collect();

    if parts.len() != 2 {
        return Err(IngestionError::InvalidInput(format!(
            "Invalid S3 path structure. Expected 's3://bucket/key', got: {}",
            s3_path
        )));
    }

    let bucket = parts[0];
    let key = parts[1];

    // Extract filename from key (last part of the path)
    let filename = key.rsplit('/').next().unwrap_or(key).to_string();

    // Download file from S3
    let file_data = upload_storage
        .download_from_s3_path(bucket, key)
        .await
        .map_err(|e| IngestionError::StorageError(e.to_string()))?;

    // Save to /tmp for processing (file_to_json needs local file)
    let temp_path = std::env::temp_dir().join(&filename);
    fs::write(&temp_path, &file_data)
        .await
        .map_err(|e| IngestionError::StorageError(format!("Failed to write temp file: {}", e)))?;

    Ok((temp_path, filename))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_s3_ingestion_request_builder() {
        let request = S3IngestionRequest::new(
            "s3://bucket/file.json".to_string(),
            "user1".to_string(),
            "default".to_string(),
        )
        .with_auto_execute(false)
        .with_trust_distance(5)
        .with_pub_key("custom".to_string());

        assert_eq!(request.s3_path, "s3://bucket/file.json");
        assert!(!request.auto_execute);
        assert_eq!(request.trust_distance, 5);
        assert_eq!(request.pub_key, "custom");
        assert_eq!(request.user_id, "user1");
        assert!(request.ingestion_config.is_none());
    }

    #[test]
    fn test_s3_ingestion_request_with_api_key() {
        let request = S3IngestionRequest::new(
            "s3://bucket/file.json".to_string(),
            "user1".to_string(),
            "key".to_string(),
        )
        .with_openrouter_api_key("test-key".to_string());

        assert_eq!(request.s3_path, "s3://bucket/file.json");
        assert!(request.ingestion_config.is_some());

        let config = request.ingestion_config.unwrap();
        assert_eq!(config.openrouter.api_key, "test-key");
        assert!(config.enabled);
    }

    #[test]
    fn test_s3_ingestion_request_with_config() {
        let mut custom_config = IngestionConfig::default();
        custom_config.openrouter.api_key = "custom-key".to_string();
        custom_config.openrouter.model = "custom-model".to_string();
        custom_config.enabled = true;

        let request = S3IngestionRequest::new(
            "s3://bucket/file.json".to_string(),
            "user1".to_string(),
            "key".to_string(),
        )
        .with_ingestion_config(custom_config);

        assert!(request.ingestion_config.is_some());
        let config = request.ingestion_config.unwrap();
        assert_eq!(config.openrouter.api_key, "custom-key");
        assert_eq!(config.openrouter.model, "custom-model");
    }

    #[test]
    fn test_s3_path_parsing() {
        let path = "s3://my-bucket/path/to/file.json";
        assert!(path.starts_with("s3://"));

        let path_without_prefix = &path[5..];
        let parts: Vec<&str> = path_without_prefix.splitn(2, '/').collect();

        assert_eq!(parts.len(), 2);
        assert_eq!(parts[0], "my-bucket");
        assert_eq!(parts[1], "path/to/file.json");
    }

    #[test]
    fn test_filename_extraction() {
        let key = "path/to/file.json";
        let filename = key.rsplit('/').next().unwrap_or(key);
        assert_eq!(filename, "file.json");

        let key2 = "file.json";
        let filename2 = key2.rsplit('/').next().unwrap_or(key2);
        assert_eq!(filename2, "file.json");
    }
}
