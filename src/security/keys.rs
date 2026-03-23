//! Ed25519 key generation and management

use crate::security::{SecurityError, SecurityResult};
use base64::{engine::general_purpose, Engine as _};
use ed25519_dalek::{Signature, Signer, SigningKey, Verifier, VerifyingKey};
use rand::rngs::OsRng;
use serde::{Deserialize, Serialize};

/// Ed25519 key pair for client-side use
#[derive(Debug)]
pub struct Ed25519KeyPair {
    signing_key: SigningKey,
    verifying_key: VerifyingKey,
}

impl Ed25519KeyPair {
    /// Generate a new Ed25519 key pair
    pub fn generate() -> SecurityResult<Self> {
        let signing_key = SigningKey::generate(&mut OsRng);
        let verifying_key = signing_key.verifying_key();

        Ok(Self {
            signing_key,
            verifying_key,
        })
    }

    /// Create a key pair from a secret key
    pub fn from_secret_key(secret_key: &[u8]) -> SecurityResult<Self> {
        if secret_key.len() != 32 {
            return Err(SecurityError::KeyGenerationFailed(
                "Secret key must be 32 bytes".to_string(),
            ));
        }

        let mut key_bytes = [0u8; 32];
        key_bytes.copy_from_slice(secret_key);

        let signing_key = SigningKey::from_bytes(&key_bytes);
        let verifying_key = signing_key.verifying_key();

        Ok(Self {
            signing_key,
            verifying_key,
        })
    }

    /// Get the public key as bytes
    pub fn public_key_bytes(&self) -> [u8; 32] {
        self.verifying_key.to_bytes()
    }

    /// Get the secret key as bytes
    pub fn secret_key_bytes(&self) -> [u8; 32] {
        self.signing_key.to_bytes()
    }

    /// Get the public key as base64-encoded string
    pub fn public_key_base64(&self) -> String {
        general_purpose::STANDARD.encode(self.public_key_bytes())
    }

    /// Get the secret key as base64-encoded string
    pub fn secret_key_base64(&self) -> String {
        general_purpose::STANDARD.encode(self.secret_key_bytes())
    }

    /// Sign a message with this key pair
    pub fn sign(&self, message: &[u8]) -> Signature {
        self.signing_key.sign(message)
    }

    /// Verify a signature using the public key
    pub fn verify(&self, message: &[u8], signature: &Signature) -> bool {
        self.verifying_key.verify(message, signature).is_ok()
    }
}

/// Ed25519 public key for server-side verification
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Ed25519PublicKey {
    verifying_key: VerifyingKey,
}

impl Ed25519PublicKey {
    /// Create a public key from bytes
    pub fn from_bytes(bytes: &[u8]) -> SecurityResult<Self> {
        if bytes.len() != 32 {
            return Err(SecurityError::InvalidPublicKey(
                "Public key must be 32 bytes".to_string(),
            ));
        }

        let mut key_bytes = [0u8; 32];
        key_bytes.copy_from_slice(bytes);

        let verifying_key = VerifyingKey::from_bytes(&key_bytes)
            .map_err(|e| SecurityError::InvalidPublicKey(e.to_string()))?;

        Ok(Self { verifying_key })
    }

    /// Create a public key from base64-encoded string
    pub fn from_base64(base64_key: &str) -> SecurityResult<Self> {
        let bytes = general_purpose::STANDARD
            .decode(base64_key)
            .map_err(|e| SecurityError::InvalidPublicKey(e.to_string()))?;

        Self::from_bytes(&bytes)
    }

    /// Get the public key as bytes
    pub fn to_bytes(&self) -> [u8; 32] {
        self.verifying_key.to_bytes()
    }

    /// Get the public key as base64-encoded string
    pub fn to_base64(&self) -> String {
        general_purpose::STANDARD.encode(self.to_bytes())
    }

    /// Verify a signature using this public key
    pub fn verify(&self, message: &[u8], signature: &Signature) -> bool {
        self.verifying_key.verify(message, signature).is_ok()
    }

    /// Verify a signature from raw bytes (64-byte Ed25519 signature).
    /// Returns Ok(()) on success, Err with message on failure.
    pub fn verify_raw(&self, message: &[u8], signature_bytes: &[u8]) -> SecurityResult<()> {
        if signature_bytes.len() != 64 {
            return Err(SecurityError::InvalidSignature(format!(
                "Invalid signature length: expected 64 bytes, got {}",
                signature_bytes.len()
            )));
        }
        let mut sig_array = [0u8; 64];
        sig_array.copy_from_slice(signature_bytes);
        let signature = Signature::from_bytes(&sig_array);
        if self.verifying_key.verify(message, &signature).is_ok() {
            Ok(())
        } else {
            Err(SecurityError::SignatureVerificationFailed(
                "Ed25519 signature verification failed".to_string(),
            ))
        }
    }
}

/// Utility functions for key management
pub struct KeyUtils;

impl KeyUtils {
    /// Generate a unique key ID from a public key
    pub fn generate_key_id(public_key: &Ed25519PublicKey) -> String {
        use sha2::{Digest, Sha256};

        let mut hasher = Sha256::new();
        hasher.update(public_key.to_bytes());
        let hash = hasher.finalize();

        // Use first 16 bytes of SHA256 hash as key ID
        general_purpose::STANDARD.encode(&hash[..16])
    }

    /// Parse a signature from base64-encoded string
    pub fn signature_from_base64(base64_sig: &str) -> SecurityResult<Signature> {
        let bytes = general_purpose::STANDARD
            .decode(base64_sig)
            .map_err(|e| SecurityError::InvalidSignature(e.to_string()))?;

        if bytes.len() != 64 {
            return Err(SecurityError::InvalidSignature(
                "Signature must be 64 bytes".to_string(),
            ));
        }

        let mut sig_bytes = [0u8; 64];
        sig_bytes.copy_from_slice(&bytes);

        Ok(Signature::from_bytes(&sig_bytes))
    }

    /// Convert a signature to base64-encoded string
    pub fn signature_to_base64(signature: &Signature) -> String {
        general_purpose::STANDARD.encode(signature.to_bytes())
    }

    /// Generate a random nonce
    pub fn generate_nonce() -> String {
        use rand::RngCore;

        let mut nonce = [0u8; 16];
        OsRng.fill_bytes(&mut nonce);
        general_purpose::STANDARD.encode(nonce)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_key_generation() {
        let keypair = Ed25519KeyPair::generate().unwrap();

        // Test that we can get public and secret keys
        let public_bytes = keypair.public_key_bytes();
        let secret_bytes = keypair.secret_key_bytes();

        assert_eq!(public_bytes.len(), 32);
        assert_eq!(secret_bytes.len(), 32);

        // Test base64 encoding
        let public_b64 = keypair.public_key_base64();
        let secret_b64 = keypair.secret_key_base64();

        assert!(!public_b64.is_empty());
        assert!(!secret_b64.is_empty());
    }

    #[test]
    fn test_signing_and_verification() {
        let keypair = Ed25519KeyPair::generate().unwrap();
        let message = b"Hello, world!";

        // Sign the message
        let signature = keypair.sign(message);

        // Verify the signature
        assert!(keypair.verify(message, &signature));

        // Verify with wrong message should fail
        let wrong_message = b"Hello, world?";
        assert!(!keypair.verify(wrong_message, &signature));
    }

    #[test]
    fn test_public_key_operations() {
        let keypair = Ed25519KeyPair::generate().unwrap();
        let public_bytes = keypair.public_key_bytes();

        // Create public key from bytes
        let public_key = Ed25519PublicKey::from_bytes(&public_bytes).unwrap();

        // Test base64 conversion
        let base64_key = public_key.to_base64();
        let public_key2 = Ed25519PublicKey::from_base64(&base64_key).unwrap();

        assert_eq!(public_key.to_bytes(), public_key2.to_bytes());
    }

    #[test]
    fn test_key_id_generation() {
        let keypair = Ed25519KeyPair::generate().unwrap();
        let public_key = Ed25519PublicKey::from_bytes(&keypair.public_key_bytes()).unwrap();

        let key_id = KeyUtils::generate_key_id(&public_key);
        assert!(!key_id.is_empty());

        // Same key should generate same ID
        let key_id2 = KeyUtils::generate_key_id(&public_key);
        assert_eq!(key_id, key_id2);
    }

    #[test]
    fn test_signature_base64_conversion() {
        let keypair = Ed25519KeyPair::generate().unwrap();
        let message = b"Test message";
        let signature = keypair.sign(message);

        // Convert to base64 and back
        let base64_sig = KeyUtils::signature_to_base64(&signature);
        let parsed_sig = KeyUtils::signature_from_base64(&base64_sig).unwrap();

        assert_eq!(signature.to_bytes(), parsed_sig.to_bytes());
    }

    #[test]
    fn test_nonce_generation() {
        let nonce1 = KeyUtils::generate_nonce();
        let nonce2 = KeyUtils::generate_nonce();

        assert!(!nonce1.is_empty());
        assert!(!nonce2.is_empty());
        assert_ne!(nonce1, nonce2); // Should be different
    }
}
