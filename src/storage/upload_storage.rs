use super::error::{StorageError, StorageResult};
use aws_sdk_s3::Client;
use std::path::PathBuf;
use tokio::fs;

/// Storage abstraction for uploaded files
#[derive(Clone)]
pub enum UploadStorage {
    /// Local filesystem storage
    Local { 
        path: PathBuf 
    },
    /// S3 storage for uploads
    S3 {
        bucket: String,
        prefix: String,
        client: Client,
    },
}

impl UploadStorage {
    /// Create local upload storage
    pub fn local(path: PathBuf) -> Self {
        Self::Local { path }
    }

    /// Create S3 upload storage
    pub fn s3(bucket: String, prefix: String, client: Client) -> Self {
        Self::S3 {
            bucket,
            prefix,
            client,
        }
    }

    /// Save a file to storage
    /// Returns the path/key where the file was saved
    pub async fn save_file(&self, filename: &str, data: &[u8]) -> StorageResult<PathBuf> {
        match self {
            Self::Local { path } => {
                // Ensure directory exists
                fs::create_dir_all(path).await?;
                
                let filepath = path.join(filename);
                
                // Write file
                fs::write(&filepath, data).await?;
                
                Ok(filepath)
            }
            Self::S3 { bucket, prefix, client } => {
                let key = if prefix.is_empty() {
                    filename.to_string()
                } else {
                    format!("{}/{}", prefix, filename)
                };
                
                client
                    .put_object()
                    .bucket(bucket)
                    .key(&key)
                    .body(data.to_vec().into())
                    .send()
                    .await
                    .map_err(|e| StorageError::UploadFailed(format!("Failed to upload to S3: {}", e)))?;
                
                // Return the S3 key as a PathBuf for consistency
                Ok(PathBuf::from(key))
            }
        }
    }

    /// Read a file from storage
    pub async fn read_file(&self, filename: &str) -> StorageResult<Vec<u8>> {
        match self {
            Self::Local { path } => {
                let filepath = path.join(filename);
                Ok(fs::read(&filepath).await?)
            }
            Self::S3 { bucket, prefix, client } => {
                let key = if prefix.is_empty() {
                    filename.to_string()
                } else {
                    format!("{}/{}", prefix, filename)
                };
                
                let response = client
                    .get_object()
                    .bucket(bucket)
                    .key(&key)
                    .send()
                    .await
                    .map_err(|e| StorageError::DownloadFailed(format!("Failed to download from S3: {}", e)))?;
                
                let data = response
                    .body
                    .collect()
                    .await
                    .map_err(|e| StorageError::DownloadFailed(format!("Failed to read S3 body: {}", e)))?
                    .into_bytes();
                
                Ok(data.to_vec())
            }
        }
    }

    /// Atomically save a file only if it doesn't already exist.
    /// Returns (PathBuf, bool) where:
    /// - PathBuf is the path/key where the file was (or would be) saved
    /// - bool is true if file already existed (duplicate), false if newly created
    pub async fn save_file_if_not_exists(&self, filename: &str, data: &[u8]) -> StorageResult<(PathBuf, bool)> {
        match self {
            Self::Local { path } => {
                // Ensure directory exists
                fs::create_dir_all(path).await?;
                
                let filepath = path.join(filename);
                
                // Atomically create file only if it doesn't exist (prevents race condition)
                match tokio::task::spawn_blocking({
                    let filepath = filepath.clone();
                    let data = data.to_vec();
                    move || {
                        use std::io::Write;
                        std::fs::OpenOptions::new()
                            .write(true)
                            .create_new(true)
                            .open(&filepath)
                            .and_then(|mut f| f.write_all(&data))
                    }
                }).await {
                    Ok(Ok(())) => {
                        // File created successfully (new file)
                        Ok((filepath, false))
                    }
                    Ok(Err(e)) if e.kind() == std::io::ErrorKind::AlreadyExists => {
                        // File already exists (duplicate upload detected atomically)
                        Ok((filepath, true))
                    }
                    Ok(Err(e)) => {
                        Err(StorageError::UploadFailed(format!("Failed to write file: {}", e)))
                    }
                    Err(e) => {
                        Err(StorageError::UploadFailed(format!("Task join error: {}", e)))
                    }
                }
            }
            Self::S3 { bucket, prefix, client } => {
                let key = if prefix.is_empty() {
                    filename.to_string()
                } else {
                    format!("{}/{}", prefix, filename)
                };
                
                // Use conditional PUT with if-none-match: * to only create if doesn't exist
                let result = client
                    .put_object()
                    .bucket(bucket)
                    .key(&key)
                    .body(data.to_vec().into())
                    .if_none_match("*")
                    .send()
                    .await;
                
                match result {
                    Ok(_) => {
                        // File created successfully (new file)
                        Ok((PathBuf::from(key), false))
                    }
                    Err(e) => {
                        let error_msg = format!("{}", e);
                        // S3 returns PreconditionFailed when if-none-match fails (file exists)
                        if error_msg.contains("PreconditionFailed") || error_msg.contains("412") {
                            // File already exists (duplicate upload detected atomically)
                            Ok((PathBuf::from(key), true))
                        } else {
                            Err(StorageError::UploadFailed(format!("Failed to upload to S3: {}", e)))
                        }
                    }
                }
            }
        }
    }

    /// Check if a file exists in storage
    pub async fn file_exists(&self, filename: &str) -> StorageResult<bool> {
        match self {
            Self::Local { path } => {
                Ok(path.join(filename).exists())
            }
            Self::S3 { bucket, prefix, client } => {
                let key = if prefix.is_empty() {
                    filename.to_string()
                } else {
                    format!("{}/{}", prefix, filename)
                };
                
                match client
                    .head_object()
                    .bucket(bucket)
                    .key(&key)
                    .send()
                    .await
                {
                    Ok(_) => Ok(true),
                    Err(_) => Ok(false),
                }
            }
        }
    }

    /// Get the display path for a file (for logging/errors)
    pub fn get_display_path(&self, filename: &str) -> String {
        match self {
            Self::Local { path } => {
                path.join(filename).display().to_string()
            }
            Self::S3 { bucket, prefix, .. } => {
                if prefix.is_empty() {
                    format!("s3://{}/{}", bucket, filename)
                } else {
                    format!("s3://{}/{}/{}", bucket, prefix, filename)
                }
            }
        }
    }

    /// Returns true if this is local storage
    pub fn is_local(&self) -> bool {
        matches!(self, Self::Local { .. })
    }

    /// Returns true if this is S3 storage
    pub fn is_s3(&self) -> bool {
        matches!(self, Self::S3 { .. })
    }

    /// Download a file from S3 using a full S3 path (bucket and key)
    /// This allows downloading from any S3 location, not just the configured upload storage
    pub async fn download_from_s3_path(&self, bucket: &str, key: &str) -> StorageResult<Vec<u8>> {
        // Determine which client to use
        let client = match self {
            Self::S3 { client, .. } => client.clone(),
            Self::Local { .. } => {
                // For local storage, try to create a temporary client from environment
                // This enables downloading S3 files (e.g. imports) even when using local storage
                
                // First, create a client without region to query the bucket location
                let config_without_region = aws_config::defaults(aws_config::BehaviorVersion::latest())
                    .region(aws_sdk_s3::config::Region::new("us-east-1")) // Use us-east-1 for bucket location query
                    .load()
                    .await;
                let temp_client = Client::new(&config_without_region);
                
                // Get the bucket's region
                let bucket_location = temp_client
                    .get_bucket_location()
                    .bucket(bucket)
                    .send()
                    .await
                    .map_err(|e| StorageError::DownloadFailed(format!("Failed to get bucket location: {}", e)))?;
                
                // LocationConstraint is None for us-east-1, otherwise it's the region name
                let region = match bucket_location.location_constraint() {
                    Some(constraint) => constraint.as_str().to_string(),
                    None => "us-east-1".to_string(),
                };
                
                // Create the actual client with the correct region
                let config = aws_config::defaults(aws_config::BehaviorVersion::latest())
                    .region(aws_sdk_s3::config::Region::new(region))
                    .load()
                    .await;
                Client::new(&config)
            }
        };

        // Use the client to download from the specified bucket/key
        let response = client
            .get_object()
            .bucket(bucket)
            .key(key)
            .send()
            .await
            .map_err(|e| StorageError::DownloadFailed(format!("Failed to download from S3: {}", e)))?;
        
        let data = response
            .body
            .collect()
            .await
            .map_err(|e| StorageError::DownloadFailed(format!("Failed to read S3 body: {}", e)))?
            .into_bytes();
        
        Ok(data.to_vec())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;

    #[tokio::test]
    async fn test_local_save_and_read() {
        let temp_dir = TempDir::new().unwrap();
        let storage = UploadStorage::local(temp_dir.path().to_path_buf());

        let data = b"test file content";
        let filename = "test.txt";

        // Save file
        let path = storage.save_file(filename, data).await.unwrap();
        assert!(path.exists());

        // Read file back
        let read_data = storage.read_file(filename).await.unwrap();
        assert_eq!(read_data, data);

        // Check exists
        assert!(storage.file_exists(filename).await.unwrap());
        assert!(!storage.file_exists("nonexistent.txt").await.unwrap());
    }

    #[test]
    fn test_display_path() {
        let local = UploadStorage::local(PathBuf::from("/tmp/uploads"));
        assert_eq!(local.get_display_path("test.txt"), "/tmp/uploads/test.txt");
        assert!(local.is_local());
        assert!(!local.is_s3());
    }

    #[tokio::test]
    async fn test_atomic_save_prevents_race_condition() {
        let temp_dir = TempDir::new().unwrap();
        let storage = UploadStorage::local(temp_dir.path().to_path_buf());

        let data = b"test file content";
        let filename = "race_test.txt";

        // First save should succeed and return false (not duplicate)
        let (path1, exists1) = storage.save_file_if_not_exists(filename, data).await.unwrap();
        assert!(!exists1, "First save should not be a duplicate");
        assert!(path1.exists());

        // Second save should detect duplicate and return true
        let (path2, exists2) = storage.save_file_if_not_exists(filename, data).await.unwrap();
        assert!(exists2, "Second save should be detected as duplicate");
        assert_eq!(path1, path2);

        // File should only contain one copy of the data
        let file_data = std::fs::read(&path1).unwrap();
        assert_eq!(file_data, data);
    }

    #[tokio::test]
    async fn test_concurrent_save_race_protection() {
        let temp_dir = TempDir::new().unwrap();
        let storage = UploadStorage::local(temp_dir.path().to_path_buf());

        let data = b"concurrent test data";
        let filename = "concurrent_test.txt";

        // Simulate concurrent uploads
        let storage1 = storage.clone();
        let storage2 = storage.clone();
        
        let handle1 = tokio::spawn(async move {
            storage1.save_file_if_not_exists(filename, data).await
        });
        
        let handle2 = tokio::spawn(async move {
            storage2.save_file_if_not_exists(filename, data).await
        });

        let result1 = handle1.await.unwrap().unwrap();
        let result2 = handle2.await.unwrap().unwrap();

        // Exactly one should be new (false), one should be duplicate (true)
        let new_count = [result1.1, result2.1].iter().filter(|&&x| !x).count();
        let dup_count = [result1.1, result2.1].iter().filter(|&&x| x).count();
        
        assert_eq!(new_count, 1, "Exactly one save should succeed as new");
        assert_eq!(dup_count, 1, "Exactly one save should be detected as duplicate");

        // File should only contain one copy of the data
        let file_data = std::fs::read(&result1.0).unwrap();
        assert_eq!(file_data, data);
    }
}

