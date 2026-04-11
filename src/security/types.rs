//! Security-related types and data structures

use serde::{Deserialize, Serialize};
use std::collections::HashMap;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SignedMessage {
    pub payload: String,       // Base64 encoded JSON payload
    pub public_key_id: String, // ID of the public key
    pub signature: String,     // Hex-encoded signature
    pub timestamp: i64,        // Unix timestamp
}

impl SignedMessage {
    pub fn new(payload: String, public_key_id: String, signature: String, timestamp: i64) -> Self {
        Self {
            payload,
            public_key_id,
            signature,
            timestamp,
        }
    }
}

/// Public key information stored on the backend
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PublicKeyInfo {
    /// Unique identifier for this public key
    pub id: String,
    /// Base64-encoded Ed25519 public key
    pub public_key: String,
    /// User or client ID associated with this key
    pub owner_id: String,
    /// When this key was registered
    pub created_at: i64,
    /// Optional expiration timestamp
    pub expires_at: Option<i64>,
    /// Whether this key is currently active
    pub is_active: bool,
    /// Permissions associated with this key
    pub permissions: Vec<String>,
    /// Additional metadata
    pub metadata: HashMap<String, String>,
}

impl PublicKeyInfo {
    /// Create a new public key info
    pub fn new(id: String, public_key: String, owner_id: String, permissions: Vec<String>) -> Self {
        Self {
            id,
            public_key,
            owner_id,
            created_at: chrono::Utc::now().timestamp(),
            expires_at: None,
            is_active: true,
            permissions,
            metadata: HashMap::new(),
        }
    }

    /// Check if this key is currently valid
    pub fn is_valid(&self) -> bool {
        if !self.is_active {
            return false;
        }

        if let Some(expires_at) = self.expires_at {
            return chrono::Utc::now().timestamp() < expires_at;
        }

        true
    }

    /// Add metadata to this key
    pub fn with_metadata(mut self, key: String, value: String) -> Self {
        self.metadata.insert(key, value);
        self
    }

    /// Set expiration time
    pub fn with_expiration(mut self, expires_at: i64) -> Self {
        self.expires_at = Some(expires_at);
        self
    }
}

/// Message verification result
#[derive(Debug, Clone)]
pub struct VerificationResult {
    /// Whether the signature is valid
    pub is_valid: bool,
    /// Public key info used for verification
    pub public_key_info: Option<PublicKeyInfo>,
    /// Error message if verification failed
    pub error: Option<String>,
    /// Whether the message timestamp is within acceptable range
    pub timestamp_valid: bool,
}

impl VerificationResult {
    /// Create a successful verification result
    pub fn success(public_key_info: PublicKeyInfo, timestamp_valid: bool) -> Self {
        Self {
            is_valid: true,
            public_key_info: Some(public_key_info),
            error: None,
            timestamp_valid,
        }
    }

    /// Create a failed verification result
    pub fn failure(error: String) -> Self {
        Self {
            is_valid: false,
            public_key_info: None,
            error: Some(error),
            timestamp_valid: false,
        }
    }
}
