use super::error::{StorageError, StorageResult};
use std::path::PathBuf;
use tokio::fs;

/// Storage abstraction for uploaded files
#[derive(Clone)]
pub enum UploadStorage {
    /// Local filesystem storage
    Local { path: PathBuf },
}

impl UploadStorage {
    /// Create local upload storage
    pub fn local(path: PathBuf) -> Self {
        Self::Local { path }
    }

    /// Save a file to storage
    ///
    /// # Arguments
    /// * `filename` - Name of the file to save
    /// * `data` - File content as bytes
    /// * `user_id` - Optional user ID for multi-tenant S3 isolation (files stored at {prefix}{user_id}/{filename})
    ///
    /// Returns the path/key where the file was saved
    pub async fn save_file(
        &self,
        filename: &str,
        data: &[u8],
        user_id: Option<&str>,
    ) -> StorageResult<PathBuf> {
        match self {
            Self::Local { path } => {
                // For local storage, optionally use user_id subdirectory
                let target_path = match user_id {
                    Some(uid) => path.join(uid),
                    None => path.clone(),
                };

                // Ensure directory exists
                fs::create_dir_all(&target_path).await?;

                let filepath = target_path.join(filename);

                // Write file
                fs::write(&filepath, data).await?;

                Ok(filepath)
            }
        }
    }

    /// Read a file from storage
    ///
    /// # Arguments
    /// * `filename` - Name of the file to read
    /// * `user_id` - Optional user ID for multi-tenant S3 isolation
    pub async fn read_file(&self, filename: &str, user_id: Option<&str>) -> StorageResult<Vec<u8>> {
        match self {
            Self::Local { path } => {
                let target_path = match user_id {
                    Some(uid) => path.join(uid),
                    None => path.clone(),
                };
                let filepath = target_path.join(filename);
                Ok(fs::read(&filepath).await?)
            }
        }
    }

    /// Atomically save a file only if it doesn't already exist.
    ///
    /// # Arguments
    /// * `filename` - Name of the file to save
    /// * `data` - File content as bytes
    /// * `user_id` - Optional user ID for multi-tenant S3 isolation
    ///
    /// Returns (PathBuf, bool) where:
    /// - PathBuf is the path/key where the file was (or would be) saved
    /// - bool is true if file already existed (duplicate), false if newly created
    pub async fn save_file_if_not_exists(
        &self,
        filename: &str,
        data: &[u8],
        user_id: Option<&str>,
    ) -> StorageResult<(PathBuf, bool)> {
        match self {
            Self::Local { path } => {
                let target_path = match user_id {
                    Some(uid) => path.join(uid),
                    None => path.clone(),
                };

                // Ensure directory exists
                fs::create_dir_all(&target_path).await?;

                let filepath = target_path.join(filename);

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
                })
                .await
                {
                    Ok(Ok(())) => {
                        // File created successfully (new file)
                        Ok((filepath, false))
                    }
                    Ok(Err(e)) if e.kind() == std::io::ErrorKind::AlreadyExists => {
                        // File already exists (duplicate upload detected atomically)
                        Ok((filepath, true))
                    }
                    Ok(Err(e)) => Err(StorageError::UploadFailed(format!(
                        "Failed to write file: {}",
                        e
                    ))),
                    Err(e) => Err(StorageError::UploadFailed(format!(
                        "Task join error: {}",
                        e
                    ))),
                }
            }
        }
    }

    /// Check if a file exists in storage
    ///
    /// # Arguments
    /// * `filename` - Name of the file to check
    /// * `user_id` - Optional user ID for multi-tenant S3 isolation
    pub async fn file_exists(&self, filename: &str, user_id: Option<&str>) -> StorageResult<bool> {
        match self {
            Self::Local { path } => {
                let target_path = match user_id {
                    Some(uid) => path.join(uid),
                    None => path.clone(),
                };
                Ok(target_path.join(filename).exists())
            }
        }
    }

    /// Get the display path for a file (for logging/errors)
    ///
    /// # Arguments
    /// * `filename` - Name of the file
    /// * `user_id` - Optional user ID for multi-tenant S3 isolation
    pub fn get_display_path(&self, filename: &str, user_id: Option<&str>) -> String {
        match self {
            Self::Local { path } => {
                let target_path = match user_id {
                    Some(uid) => path.join(uid),
                    None => path.clone(),
                };
                target_path.join(filename).display().to_string()
            }
        }
    }

    /// Returns true if this is local storage
    pub fn is_local(&self) -> bool {
        matches!(self, Self::Local { .. })
    }

    /// Returns true if this is S3 storage
    pub fn is_s3(&self) -> bool {
        false
    }

    /// Download a file from an external S3 path (bucket/key)
    /// This is used for S3 event-triggered ingestion where files come from external buckets
    pub async fn download_from_s3_path(&self, _bucket: &str, _key: &str) -> StorageResult<Vec<u8>> {
        match self {
            Self::Local { .. } => {
                Err(StorageError::DownloadFailed(
                    "S3 download not supported in local mode. Configure S3 storage to enable S3 downloads.".to_string()
                ))
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::tempdir;

    #[tokio::test]
    async fn test_local_save_and_read() {
        let temp_dir = tempdir().unwrap();
        let storage = UploadStorage::local(temp_dir.path().to_path_buf());

        let data = b"test content";
        let path = storage.save_file("test.txt", data, None).await.unwrap();

        assert!(path.exists());

        let read_data = storage.read_file("test.txt", None).await.unwrap();
        assert_eq!(read_data, data.to_vec());
    }

    #[tokio::test]
    async fn test_local_save_and_read_with_user_id() {
        let temp_dir = tempdir().unwrap();
        let storage = UploadStorage::local(temp_dir.path().to_path_buf());

        let data = b"user specific content";
        let user_id = Some("user_123");
        let path = storage.save_file("test.txt", data, user_id).await.unwrap();

        assert!(path.exists());
        assert!(path.to_string_lossy().contains("user_123"));

        let read_data = storage.read_file("test.txt", user_id).await.unwrap();
        assert_eq!(read_data, data.to_vec());
    }

    #[tokio::test]
    async fn test_local_file_exists() {
        let temp_dir = tempdir().unwrap();
        let storage = UploadStorage::local(temp_dir.path().to_path_buf());

        assert!(!storage.file_exists("nonexistent.txt", None).await.unwrap());

        storage
            .save_file("exists.txt", b"data", None)
            .await
            .unwrap();
        assert!(storage.file_exists("exists.txt", None).await.unwrap());
    }

    #[tokio::test]
    async fn test_local_save_if_not_exists() {
        let temp_dir = tempdir().unwrap();
        let storage = UploadStorage::local(temp_dir.path().to_path_buf());

        // First save should succeed
        let (path, existed) = storage
            .save_file_if_not_exists("unique.txt", b"original", None)
            .await
            .unwrap();
        assert!(!existed);
        assert!(path.exists());

        // Second save should detect duplicate
        let (_, existed) = storage
            .save_file_if_not_exists("unique.txt", b"duplicate", None)
            .await
            .unwrap();
        assert!(existed);

        // Original content should be preserved
        let content = storage.read_file("unique.txt", None).await.unwrap();
        assert_eq!(content, b"original".to_vec());
    }

    #[test]
    fn test_is_local_and_is_s3() {
        let local = UploadStorage::Local {
            path: PathBuf::from("/tmp"),
        };
        assert!(local.is_local());
        assert!(!local.is_s3());
    }

    #[test]
    fn test_get_display_path_local() {
        let storage = UploadStorage::Local {
            path: PathBuf::from("/tmp/uploads"),
        };
        assert_eq!(
            storage.get_display_path("file.txt", None),
            "/tmp/uploads/file.txt"
        );
        assert_eq!(
            storage.get_display_path("file.txt", Some("user_123")),
            "/tmp/uploads/user_123/file.txt"
        );
    }

    #[test]
    fn test_multi_tenant_isolation() {
        // Different users should have different paths
        let storage = UploadStorage::Local {
            path: PathBuf::from("/tmp/uploads"),
        };

        let path1 = storage.get_display_path("data.json", Some("user_1"));
        let path2 = storage.get_display_path("data.json", Some("user_2"));

        assert_ne!(path1, path2);
        assert!(path1.contains("user_1"));
        assert!(path2.contains("user_2"));
    }
}
