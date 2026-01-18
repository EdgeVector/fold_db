use thiserror::Error;

/// Comprehensive storage error type supporting multiple backends
#[derive(Debug, Error)]
pub enum StorageError {
    #[error("Item not found: {0}")]
    NotFound(String),

    #[error("Serialization error: {0}")]
    SerializationError(String),

    #[error("Storage backend error: {0}")]
    BackendError(String),

    #[error("Key already exists: {0}")]
    AlreadyExists(String),

    #[error("Invalid operation: {0}")]
    InvalidOperation(String),

    #[error("S3 error: {0}")]
    S3Error(String),

    #[error("IO error: {0}")]
    IoError(#[from] std::io::Error),

    #[error("Invalid path: {0}")]
    InvalidPath(String),

    #[error("Download failed: {0}")]
    DownloadFailed(String),

    #[error("Upload failed: {0}")]
    UploadFailed(String),

    #[error("DynamoDB error: {0}")]
    DynamoDbError(String),

    #[error("Sled error: {0}")]
    SledError(String),

    #[error("Configuration error: {0}")]
    ConfigurationError(String),
}

pub type StorageResult<T> = Result<T, StorageError>;
