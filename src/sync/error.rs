use thiserror::Error;

#[derive(Debug, Error)]
pub enum SyncError {
    #[error("encryption error: {0}")]
    Crypto(String),

    #[error("serialization error: {0}")]
    Serialization(String),

    #[error("IO error: {0}")]
    Io(#[from] std::io::Error),

    #[error("network error: {0}")]
    Network(String),

    #[error("auth error: {0}")]
    Auth(String),

    #[error("org membership revoked for org: {0}")]
    OrgMembershipRevoked(String),

    #[error("S3 error: {0}")]
    S3(String),

    #[error("corrupt log entry at seq {seq}: {reason}")]
    CorruptEntry { seq: u64, reason: String },

    #[error("device locked by {device_id}, expires at {expires_at}")]
    DeviceLocked {
        device_id: String,
        expires_at: String,
    },

    #[error("wrong encryption key")]
    WrongKey,

    #[error("storage error: {0}")]
    Storage(String),
}

pub type SyncResult<T> = Result<T, SyncError>;

impl From<crate::crypto::CryptoError> for SyncError {
    fn from(e: crate::crypto::CryptoError) -> Self {
        SyncError::Crypto(e.to_string())
    }
}

impl From<crate::storage::error::StorageError> for SyncError {
    fn from(e: crate::storage::error::StorageError) -> Self {
        SyncError::Storage(e.to_string())
    }
}

impl From<serde_json::Error> for SyncError {
    fn from(e: serde_json::Error) -> Self {
        SyncError::Serialization(e.to_string())
    }
}

impl From<reqwest::Error> for SyncError {
    fn from(e: reqwest::Error) -> Self {
        SyncError::Network(e.to_string())
    }
}
