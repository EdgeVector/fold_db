use super::envelope::{decrypt_envelope, encrypt_envelope};
use super::error::{CryptoError, CryptoResult};
use async_trait::async_trait;
use std::path::Path;
use tokio::fs;

/// Trait for providing encryption/decryption of byte payloads.
///
/// Implementations handle key management internally. Callers
/// only see `encrypt(plaintext) -> ciphertext` and `decrypt(ciphertext) -> plaintext`.
#[async_trait]
pub trait CryptoProvider: Send + Sync {
    /// Encrypt plaintext bytes.
    ///
    /// Returns a self-describing ciphertext envelope that includes
    /// all metadata needed for decryption (nonce, version, etc.).
    async fn encrypt(&self, plaintext: &[u8]) -> CryptoResult<Vec<u8>>;

    /// Decrypt a ciphertext envelope back to plaintext.
    async fn decrypt(&self, ciphertext: &[u8]) -> CryptoResult<Vec<u8>>;
}

// ---------------------------------------------------------------------------
// NoOpCryptoProvider — passthrough, no encryption (for tests)
// ---------------------------------------------------------------------------

/// A no-op provider that passes data through without encryption.
/// Used for unit tests and migration scenarios.
pub struct NoOpCryptoProvider;

#[async_trait]
impl CryptoProvider for NoOpCryptoProvider {
    async fn encrypt(&self, plaintext: &[u8]) -> CryptoResult<Vec<u8>> {
        Ok(plaintext.to_vec())
    }

    async fn decrypt(&self, ciphertext: &[u8]) -> CryptoResult<Vec<u8>> {
        Ok(ciphertext.to_vec())
    }
}

// ---------------------------------------------------------------------------
// LocalCryptoProvider — file-based key, AES-256-GCM (for standalone dev)
// ---------------------------------------------------------------------------

/// A local crypto provider that derives its key from a file on disk.
///
/// On first use, if no key file exists, a random 256-bit key is generated
/// and written to `~/.fold_db/encryption.key`. Subsequent uses load
/// the same key.
///
/// This gives local dev parity with the cloud envelope encryption
/// pattern without requiring AWS KMS.
pub struct LocalCryptoProvider {
    key: [u8; 32],
}

impl LocalCryptoProvider {
    /// Load or generate a local encryption key.
    ///
    /// The key is stored at the given path. If the file does not exist,
    /// a new random key is generated and written.
    pub async fn load_or_generate(key_path: &Path) -> CryptoResult<Self> {
        if key_path.exists() {
            let bytes = fs::read(key_path)
                .await
                .map_err(|e| CryptoError::KeyError(format!("Failed to read key file: {}", e)))?;

            if bytes.len() != 32 {
                return Err(CryptoError::KeyError(format!(
                    "Key file has invalid length: {} (expected 32)",
                    bytes.len()
                )));
            }

            let mut key = [0u8; 32];
            key.copy_from_slice(&bytes);
            Ok(Self { key })
        } else {
            // Generate a new key
            let mut key = [0u8; 32];
            use rand::RngCore;
            rand::rngs::OsRng.fill_bytes(&mut key);

            // Ensure parent directory exists
            if let Some(parent) = key_path.parent() {
                fs::create_dir_all(parent).await.map_err(|e| {
                    CryptoError::KeyError(format!("Failed to create key directory: {}", e))
                })?;
            }

            fs::write(key_path, &key)
                .await
                .map_err(|e| CryptoError::KeyError(format!("Failed to write key file: {}", e)))?;

            // Restrict key file permissions to owner-only (Unix)
            #[cfg(unix)]
            {
                use std::os::unix::fs::PermissionsExt;
                let perms = std::fs::Permissions::from_mode(0o600);
                std::fs::set_permissions(key_path, perms).map_err(|e| {
                    CryptoError::KeyError(format!("Failed to set key file permissions: {}", e))
                })?;
            }

            tracing::info!(
                "Generated new local encryption key at {}",
                key_path.display()
            );
            tracing::warn!(
                "⚠️  Back up your encryption key! Without it, encrypted data cannot be recovered."
            );

            Ok(Self { key })
        }
    }

    /// Create a provider from an explicit key (useful for tests).
    pub fn from_key(key: [u8; 32]) -> Self {
        Self { key }
    }
}

#[async_trait]
impl CryptoProvider for LocalCryptoProvider {
    async fn encrypt(&self, plaintext: &[u8]) -> CryptoResult<Vec<u8>> {
        encrypt_envelope(&self.key, plaintext)
    }

    async fn decrypt(&self, ciphertext: &[u8]) -> CryptoResult<Vec<u8>> {
        decrypt_envelope(&self.key, ciphertext)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::tempdir;

    #[tokio::test]
    async fn test_noop_provider() {
        let provider = NoOpCryptoProvider;
        let data = b"hello world";

        let encrypted = provider.encrypt(data).await.unwrap();
        assert_eq!(encrypted, data);

        let decrypted = provider.decrypt(&encrypted).await.unwrap();
        assert_eq!(decrypted, data);
    }

    #[tokio::test]
    async fn test_local_provider_roundtrip() {
        let provider = LocalCryptoProvider::from_key([0x42u8; 32]);
        let data = b"atom content: {\"name\": \"test\"}";

        let encrypted = provider.encrypt(data).await.unwrap();
        assert_ne!(encrypted, data); // Should be different (encrypted)

        let decrypted = provider.decrypt(&encrypted).await.unwrap();
        assert_eq!(decrypted, data);
    }

    #[tokio::test]
    async fn test_local_provider_generate_and_load() {
        let tmp = tempdir().unwrap();
        let key_path = tmp.path().join(".fold_db").join("encryption.key");

        // First call: generates key file
        let provider1 = LocalCryptoProvider::load_or_generate(&key_path)
            .await
            .unwrap();
        assert!(key_path.exists());

        // Second call: loads existing key
        let provider2 = LocalCryptoProvider::load_or_generate(&key_path)
            .await
            .unwrap();

        // Both should use the same key
        assert_eq!(provider1.key, provider2.key);

        // Data encrypted by one should be decryptable by the other
        let data = b"persistence test";
        let encrypted = provider1.encrypt(data).await.unwrap();
        let decrypted = provider2.decrypt(&encrypted).await.unwrap();
        assert_eq!(decrypted, data);
    }

    #[tokio::test]
    async fn test_local_provider_invalid_key_file() {
        let tmp = tempdir().unwrap();
        let key_path = tmp.path().join("bad.key");

        // Write wrong-length key
        tokio::fs::write(&key_path, b"too_short").await.unwrap();

        let result = LocalCryptoProvider::load_or_generate(&key_path).await;
        assert!(result.is_err());
    }
}
