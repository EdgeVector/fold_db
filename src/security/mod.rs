//! Security module for client key management, message signing, and encryption
//!
//! ## Encryption Architecture
//!
//! FoldDB uses **end-to-end (E2E) encryption** as its sole encryption mechanism.
//! E2E keys are derived from a local secret (`~/.fold_db/e2e.key`) via HKDF-SHA256,
//! producing an AES-256-GCM content key and an HMAC-SHA256 index key. This ensures
//! atom content and index tokens are encrypted before they ever reach storage.
//!
//! This module provides:
//! - Ed25519 key pair generation and management
//! - Message signing and verification
//! - Integration with network and permissions layers

pub mod keys;
pub mod signing;
pub mod types;
pub mod utils;

pub use keys::*;
pub use signing::*;
pub use types::{
    KeyRegistrationRequest, KeyRegistrationResponse, PublicKeyInfo,
    SignedMessage, VerificationResult,
};
pub use utils::*;

use thiserror::Error;

/// Security-related errors
#[derive(Error, Debug)]
pub enum SecurityError {
    #[error("Key generation failed: {0}")]
    KeyGenerationFailed(String),

    #[error("Signature verification failed: {0}")]
    SignatureVerificationFailed(String),

    #[error("Encryption failed: {0}")]
    EncryptionFailed(String),

    #[error("Decryption failed: {0}")]
    DecryptionFailed(String),

    #[error("Invalid public key: {0}")]
    InvalidPublicKey(String),

    #[error("Invalid signature: {0}")]
    InvalidSignature(String),

    #[error("Key not found: {0}")]
    KeyNotFound(String),

    #[error("Serialization error: {0}")]
    SerializationError(String),

    #[error("Deserialization error: {0}")]
    DeserializationError(String),

    #[error("Invalid key format: {0}")]
    InvalidKeyFormat(String),
}

pub type SecurityResult<T> = Result<T, SecurityError>;

/// Security module configuration.
///
/// Ed25519 signatures are always required on write endpoints — there is no opt-out.
/// E2E encryption is always active — there is no opt-out.
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct SecurityConfig {
    /// Whether to require TLS for all connections
    pub require_tls: bool,
}

impl Default for SecurityConfig {
    fn default() -> Self {
        Self {
            require_tls: true,
        }
    }
}

impl SecurityConfig {
    /// Load security configuration from environment variables
    pub fn from_env() -> Self {
        let mut config = Self::default();

        if let Ok(value) = std::env::var("FOLD_REQUIRE_TLS") {
            config.require_tls = value.parse().unwrap_or(true);
        }

        config
    }
}
