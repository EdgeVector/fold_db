use thiserror::Error;

/// Errors that can occur during cryptographic operations
#[derive(Debug, Error)]
pub enum CryptoError {
    #[error("Encryption failed: {0}")]
    EncryptionFailed(String),

    #[error("Decryption failed: {0}")]
    DecryptionFailed(String),

    #[error("Invalid ciphertext format: {0}")]
    InvalidFormat(String),

    #[error("Key management error: {0}")]
    KeyError(String),

    #[error("Unsupported envelope version: {0}")]
    UnsupportedVersion(u8),
}

pub type CryptoResult<T> = Result<T, CryptoError>;
