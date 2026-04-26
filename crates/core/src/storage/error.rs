use thiserror::Error;

/// Comprehensive storage error type supporting multiple backends
#[derive(Debug, Error)]
pub enum StorageError {
    #[error("Serialization error: {0}")]
    SerializationError(String),

    #[error("Storage backend error: {0}")]
    BackendError(String),

    #[error("Invalid operation: {0}")]
    InvalidOperation(String),

    #[error("IO error: {0}")]
    IoError(#[from] std::io::Error),

    #[error("Download failed: {0}")]
    DownloadFailed(String),

    #[error("Upload failed: {0}")]
    UploadFailed(String),

    #[error("Sled error: {0}")]
    SledError(String),

    #[error("Configuration error: {0}")]
    ConfigurationError(String),

    #[error("Encryption error: {0}")]
    EncryptionError(String),
}

pub type StorageResult<T> = Result<T, StorageError>;
