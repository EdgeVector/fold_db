//! Security module for client key management, message signing, and encryption
//!
//! This module provides:
//! - Ed25519 key pair generation and management
//! - Message signing and verification
//! - AES-GCM encryption/decryption for data at rest
//! - Integration with network and permissions layers

use base64::{engine::general_purpose, Engine as _};

pub mod encryption;
pub mod keys;
pub mod signing;
pub mod types;
pub mod utils;

pub use encryption::*;
pub use keys::*;
pub use signing::*;
pub use types::{
    EncryptedData, KeyRegistrationRequest, KeyRegistrationResponse, PublicKeyInfo,
    PublicKeyRegistration, SignedMessage, VerificationResult,
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

/// Security module configuration
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct SecurityConfig {
    /// Whether to require TLS for all connections
    pub require_tls: bool,
    /// Whether to require signatures on all messages
    pub require_signatures: bool,
    /// Whether to encrypt sensitive data at rest
    pub encrypt_at_rest: bool,
    /// Master key for at-rest encryption (should be securely managed)
    #[serde(skip)]
    pub master_key: Option<[u8; 32]>,
}

impl Default for SecurityConfig {
    fn default() -> Self {
        Self {
            require_tls: true,
            require_signatures: true,
            encrypt_at_rest: false, // Disabled by default (enable with a master_key)
            master_key: None,
        }
    }
}

impl SecurityConfig {
    /// Create a new security config with default settings
    pub fn new() -> Self {
        Self::default()
    }

    /// Load security configuration from environment variables
    pub fn from_env() -> Self {
        let mut config = Self::default();

        // Load from environment variables
        if let Ok(value) = std::env::var("DATAFOLD_REQUIRE_TLS") {
            config.require_tls = value.parse().unwrap_or(true);
        }

        if let Ok(value) = std::env::var("DATAFOLD_REQUIRE_SIGNATURES") {
            config.require_signatures = value.parse().unwrap_or(true);
        }

        if let Ok(value) = std::env::var("DATAFOLD_ENCRYPT_AT_REST") {
            config.encrypt_at_rest = value.parse().unwrap_or(true);
        }

        // Load master key from environment (base64 encoded)
        if let Ok(key_base64) = std::env::var("DATAFOLD_MASTER_KEY") {
            if let Ok(key_bytes) = general_purpose::STANDARD.decode(&key_base64) {
                if key_bytes.len() == 32 {
                    let mut key = [0u8; 32];
                    key.copy_from_slice(&key_bytes);
                    config.master_key = Some(key);
                }
            }
        }

        config
    }

    /// Enable or disable TLS requirement
    pub fn with_tls(mut self, require_tls: bool) -> Self {
        self.require_tls = require_tls;
        self
    }

    /// Enable or disable signature requirement
    pub fn with_signatures(mut self, require_signatures: bool) -> Self {
        self.require_signatures = require_signatures;
        self
    }

    /// Enable or disable at-rest encryption
    pub fn with_encryption(mut self, encrypt_at_rest: bool) -> Self {
        self.encrypt_at_rest = encrypt_at_rest;
        self
    }

    /// Set the master key for at-rest encryption
    pub fn with_master_key(mut self, key: [u8; 32]) -> Self {
        self.master_key = Some(key);
        self
    }
}
