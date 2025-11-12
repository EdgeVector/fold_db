use std::fmt;

#[derive(Debug)]
pub enum StorageError {
    S3Error(String),
    IoError(std::io::Error),
    InvalidPath(String),
    DownloadFailed(String),
    UploadFailed(String),
}

impl fmt::Display for StorageError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            StorageError::S3Error(msg) => write!(f, "S3 error: {}", msg),
            StorageError::IoError(err) => write!(f, "IO error: {}", err),
            StorageError::InvalidPath(msg) => write!(f, "Invalid path: {}", msg),
            StorageError::DownloadFailed(msg) => write!(f, "Download failed: {}", msg),
            StorageError::UploadFailed(msg) => write!(f, "Upload failed: {}", msg),
        }
    }
}

impl std::error::Error for StorageError {}

impl From<std::io::Error> for StorageError {
    fn from(err: std::io::Error) -> Self {
        StorageError::IoError(err)
    }
}

pub type StorageResult<T> = Result<T, StorageError>;

