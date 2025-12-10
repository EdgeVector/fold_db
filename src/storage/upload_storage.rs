use super::error::{StorageError, StorageResult};
use std::path::PathBuf;
use tokio::fs;

/// Storage abstraction for uploaded files
#[derive(Clone)]
pub enum UploadStorage {
    /// Local filesystem storage
    Local { 
        path: PathBuf 
    },
}

impl UploadStorage {
    /// Create local upload storage
    pub fn local(path: PathBuf) -> Self {
        Self::Local { path }
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
        }
    }

    /// Read a file from storage
    pub async fn read_file(&self, filename: &str) -> StorageResult<Vec<u8>> {
        match self {
            Self::Local { path } => {
                let filepath = path.join(filename);
                Ok(fs::read(&filepath).await?)
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
        }
    }

    /// Check if a file exists in storage
    pub async fn file_exists(&self, filename: &str) -> StorageResult<bool> {
        match self {
            Self::Local { path } => {
                Ok(path.join(filename).exists())
            }
        }
    }

    /// Get the display path for a file (for logging/errors)
    pub fn get_display_path(&self, filename: &str) -> String {
        match self {
            Self::Local { path } => {
                path.join(filename).display().to_string()
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

    /// Download a file from S3 (Not supported in Local mode)
    pub async fn download_from_s3_path(&self, _bucket: &str, _key: &str) -> StorageResult<Vec<u8>> {
        Err(StorageError::DownloadFailed("S3 download not supported in local mode (S3 support removed)".to_string()))
    }
}
