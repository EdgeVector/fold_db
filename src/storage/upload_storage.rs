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
}

