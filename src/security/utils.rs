//! Security utility functions and helpers

use crate::{
    constants::SINGLE_PUBLIC_KEY_ID,
    security::{
        ConditionalEncryption, Ed25519PublicKey,
        KeyRegistrationRequest, KeyRegistrationResponse, MessageVerifier, PublicKeyInfo,
        SecurityError, SecurityResult, SignedMessage,
    },
};
use serde_json::Value;
use std::sync::Arc;

/// Security manager that combines all security functionality
pub struct SecurityManager {
    /// Message verifier for signature verification
    pub verifier: Arc<MessageVerifier>,
    /// Conditional encryption for data at rest
    pub encryption: Arc<ConditionalEncryption>,
    /// Security configuration
    pub config: crate::security::SecurityConfig,
}

impl SecurityManager {
    /// Create a new security manager without persistence
    pub fn new(config: crate::security::SecurityConfig) -> SecurityResult<Self> {
        let verifier = Arc::new(MessageVerifier::new(300)); // 5 minute timestamp drift

        let encryption = Arc::new(ConditionalEncryption::new(
            config.encrypt_at_rest,
            config.master_key,
        )?);

        Ok(Self {
            verifier,
            encryption,
            config,
        })
    }

    /// Create a new security manager with database persistence
    pub async fn new_with_persistence(
        config: crate::security::SecurityConfig,
        db_ops: Arc<crate::db_operations::DbOperations>,
    ) -> SecurityResult<Self> {
        let verifier = Arc::new(MessageVerifier::new_with_persistence(300, db_ops).await?);

        let encryption = Arc::new(ConditionalEncryption::new(
            config.encrypt_at_rest,
            config.master_key,
        )?);

        Ok(Self {
            verifier,
            encryption,
            config,
        })
    }

    /// Register the system-wide public key
    pub async fn register_system_public_key(
        &self,
        request: KeyRegistrationRequest,
    ) -> SecurityResult<KeyRegistrationResponse> {
        // Validate the public key format
        let public_key = Ed25519PublicKey::from_base64(&request.public_key)
            .map_err(|e| SecurityError::InvalidPublicKey(e.to_string()))?;

        // Create public key info, using the validated and re-encoded key
        let mut key_info = PublicKeyInfo::new(
            SINGLE_PUBLIC_KEY_ID.to_string(),
            public_key.to_base64(), // Use the validated key, re-encoded
            request.owner_id,
            request.permissions,
        );

        // Add metadata
        for (k, v) in request.metadata {
            key_info = key_info.with_metadata(k, v);
        }

        // Set expiration if provided
        if let Some(expires_at) = request.expires_at {
            key_info = key_info.with_expiration(expires_at);
        }

        // Register with the verifier
        self.verifier
            .register_system_public_key(key_info.clone())
            .await?;

        Ok(KeyRegistrationResponse {
            success: true,
            public_key_id: Some(SINGLE_PUBLIC_KEY_ID.to_string()),
            key: Some(key_info),
            error: None,
        })
    }

    /// Verify a signed message
    pub fn verify_message(
        &self,
        signed_message: &SignedMessage,
    ) -> SecurityResult<crate::security::VerificationResult> {
        if !self.config.require_signatures {
            // If signatures are not required, create a mock successful result
            return Ok(crate::security::VerificationResult {
                is_valid: true,
                public_key_info: None,
                error: None,
                timestamp_valid: true,
            });
        }

        self.verifier.verify_message(signed_message)
    }

    /// Verify a message with required permissions
    pub fn verify_message_with_permissions(
        &self,
        signed_message: &SignedMessage,
        required_permissions: &[String],
    ) -> SecurityResult<crate::security::VerificationResult> {
        if !self.config.require_signatures {
            // If signatures are not required, create a mock successful result
            return Ok(crate::security::VerificationResult {
                is_valid: true,
                public_key_info: None,
                error: None,
                timestamp_valid: true,
            });
        }

        self.verifier
            .verify_message_with_permissions(signed_message, required_permissions)
    }

    /// Encrypt data if encryption is enabled
    pub fn encrypt_data(
        &self,
        data: &[u8],
    ) -> SecurityResult<Option<crate::security::EncryptedData>> {
        self.encryption.maybe_encrypt(data)
    }

    /// Encrypt JSON data if encryption is enabled
    pub fn encrypt_json(
        &self,
        json_data: &Value,
    ) -> SecurityResult<Option<crate::security::EncryptedData>> {
        self.encryption.maybe_encrypt_json(json_data)
    }

    /// Decrypt data
    pub fn decrypt_data(
        &self,
        encrypted_data: &crate::security::EncryptedData,
    ) -> SecurityResult<Vec<u8>> {
        self.encryption.maybe_decrypt(encrypted_data)
    }

    /// Decrypt JSON data
    pub fn decrypt_json(
        &self,
        encrypted_data: &crate::security::EncryptedData,
    ) -> SecurityResult<Value> {
        self.encryption.maybe_decrypt_json(encrypted_data)
    }

    /// Check if encryption is enabled
    pub fn is_encryption_enabled(&self) -> bool {
        self.encryption.is_encryption_enabled()
    }

    /// Get the system public key if it exists.
    pub fn get_system_public_key(&self) -> SecurityResult<Option<PublicKeyInfo>> {
        self.verifier.get_system_public_key()
    }

    /// Remove the system public key
    pub async fn remove_system_public_key(&self) -> SecurityResult<()> {
        self.verifier.remove_system_public_key().await
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::security::Ed25519KeyPair;

    #[tokio::test]
    async fn test_security_manager() {
        // Test with default config (no encryption)
        let config = crate::security::SecurityConfig {
            require_tls: true,
            require_signatures: true,
            encrypt_at_rest: true,
            master_key: Some([0; 32]),
        };
        let manager = SecurityManager::new(config).unwrap();

        // Generate a client keypair
        let keypair = Ed25519KeyPair::generate().unwrap();

        // Register the public key
        let registration_request = crate::security::KeyRegistrationRequest {
            public_key: keypair.public_key_base64(),
            owner_id: "test_user".to_string(),
            permissions: vec!["read".to_string()],
            metadata: std::collections::HashMap::new(),
            expires_at: None,
        };

        let response = manager
            .register_system_public_key(registration_request)
            .await
            .unwrap();
        assert!(response.success);
        assert!(response.public_key_id.is_some());
    }
}
