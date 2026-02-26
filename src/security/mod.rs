//! Security module for client key management, message signing, and encryption
//!
//! ## Encryption Architecture
//!
//! FoldDB uses **end-to-end (E2E) encryption** as its primary encryption mechanism.
//! E2E keys are derived from a local secret (`~/.fold_db/e2e.key`) via HKDF-SHA256,
//! producing an AES-256-GCM content key and an HMAC-SHA256 index key. This ensures
//! atom content and index tokens are encrypted before they ever reach storage.
//!
//! **Encrypt-at-rest** (`SecurityConfig::encrypt_at_rest`) is an optional, separate
//! layer that can be enabled when E2E encryption is not in use (e.g., legacy setups
//! or specific deployment scenarios). It is disabled by default.
//!
//! This module provides:
//! - Ed25519 key pair generation and management
//! - Message signing and verification
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
    /// Whether to encrypt sensitive data at rest (optional fallback).
    /// Not needed when E2E encryption is active — E2E encrypts content before
    /// it reaches the storage layer. Enable this only if E2E is disabled.
    pub encrypt_at_rest: bool,
    /// Master key for at-rest encryption (only used when encrypt_at_rest is true).
    /// Not needed when E2E encryption handles content encryption.
    #[serde(skip)]
    pub master_key: Option<[u8; 32]>,
}

impl Default for SecurityConfig {
    fn default() -> Self {
        Self {
            require_tls: true,
            require_signatures: true,
            encrypt_at_rest: false, // Not needed when E2E encryption is active (default)
            master_key: None,
        }
    }
}

impl SecurityConfig {
    /// Load security configuration from environment variables
    pub fn from_env() -> Self {
        let mut config = Self::default();

        // Load from environment variables
        if let Ok(value) = std::env::var("FOLD_REQUIRE_TLS") {
            config.require_tls = value.parse().unwrap_or(true);
        }

        if let Ok(value) = std::env::var("FOLD_REQUIRE_SIGNATURES") {
            config.require_signatures = value.parse().unwrap_or(true);
        }

        if let Ok(value) = std::env::var("FOLD_ENCRYPT_AT_REST") {
            config.encrypt_at_rest = value.parse().unwrap_or(true);
        }

        // Load master key from environment (base64 encoded)
        if let Ok(key_base64) = std::env::var("FOLD_MASTER_KEY") {
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
}
