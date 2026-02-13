use super::error::{CryptoError, CryptoResult};
use base64::{engine::general_purpose::URL_SAFE_NO_PAD, Engine as _};
use hkdf::Hkdf;
use hmac::{Hmac, Mac};
use sha2::Sha256;
use std::path::Path;
use tokio::fs;

const HKDF_SALT: &[u8] = b"fold:e2e:v1";
const CONTENT_KEY_INFO: &[u8] = b"fold:content-key";
const INDEX_KEY_INFO: &[u8] = b"fold:index-key";

/// Holds the two E2E encryption keys derived from a single passkey secret:
/// - `encryption_key`: AES-256-GCM key for atom content (used by `LocalCryptoProvider`)
/// - `index_key`: HMAC-SHA256 key for blind index tokens
#[derive(Clone)]
pub struct E2eKeys {
    encryption_key: [u8; 32],
    index_key: [u8; 32],
}

impl E2eKeys {
    /// Derive both keys from a 32-byte secret using HKDF-SHA256.
    pub fn from_secret(secret: &[u8; 32]) -> Self {
        let hk = Hkdf::<Sha256>::new(Some(HKDF_SALT), secret);

        let mut encryption_key = [0u8; 32];
        hk.expand(CONTENT_KEY_INFO, &mut encryption_key)
            .expect("32 bytes is a valid HKDF-SHA256 output length");

        let mut index_key = [0u8; 32];
        hk.expand(INDEX_KEY_INFO, &mut index_key)
            .expect("32 bytes is a valid HKDF-SHA256 output length");

        Self {
            encryption_key,
            index_key,
        }
    }

    /// Load a 32-byte secret from `key_path`, or generate a random one if the
    /// file does not exist. Then derive both E2E keys via [`from_secret`].
    pub async fn load_or_generate(key_path: &Path) -> CryptoResult<Self> {
        let secret = if key_path.exists() {
            let bytes = fs::read(key_path)
                .await
                .map_err(|e| CryptoError::KeyError(format!("Failed to read E2E key file: {}", e)))?;

            if bytes.len() != 32 {
                return Err(CryptoError::KeyError(format!(
                    "E2E key file has invalid length: {} (expected 32)",
                    bytes.len()
                )));
            }

            let mut secret = [0u8; 32];
            secret.copy_from_slice(&bytes);
            secret
        } else {
            let mut secret = [0u8; 32];
            use rand::RngCore;
            rand::rngs::OsRng.fill_bytes(&mut secret);

            if let Some(parent) = key_path.parent() {
                fs::create_dir_all(parent).await.map_err(|e| {
                    CryptoError::KeyError(format!("Failed to create E2E key directory: {}", e))
                })?;
            }

            fs::write(key_path, &secret)
                .await
                .map_err(|e| CryptoError::KeyError(format!("Failed to write E2E key file: {}", e)))?;

            log::info!(
                "Generated new E2E key at {}",
                key_path.display()
            );
            log::warn!(
                "Back up your E2E key! Without it, encrypted data cannot be recovered."
            );

            secret
        };

        Ok(Self::from_secret(&secret))
    }

    /// AES-256-GCM key for atom content encryption.
    pub fn encryption_key(&self) -> [u8; 32] {
        self.encryption_key
    }

    /// HMAC-SHA256 key for blind index tokens.
    pub fn index_key(&self) -> [u8; 32] {
        self.index_key
    }

    /// Compute a blind index token: HMAC-SHA256(index_key, term), truncated to
    /// 16 bytes and base64url-encoded (no padding).
    pub fn blind_token(index_key: &[u8; 32], term: &str) -> String {
        let mut mac =
            Hmac::<Sha256>::new_from_slice(index_key).expect("HMAC accepts any key length");
        mac.update(term.as_bytes());
        let result = mac.finalize().into_bytes();
        URL_SAFE_NO_PAD.encode(&result[..16])
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::crypto::provider::LocalCryptoProvider;
    use crate::crypto::CryptoProvider;
    use tempfile::tempdir;

    #[test]
    fn test_derivation_determinism() {
        let secret = [0x42u8; 32];
        let a = E2eKeys::from_secret(&secret);
        let b = E2eKeys::from_secret(&secret);

        assert_eq!(a.encryption_key(), b.encryption_key());
        assert_eq!(a.index_key(), b.index_key());
        // The two derived keys must be different from each other
        assert_ne!(a.encryption_key(), a.index_key());
    }

    #[test]
    fn test_blind_token_determinism() {
        let key = [0xABu8; 32];
        let t1 = E2eKeys::blind_token(&key, "hello");
        let t2 = E2eKeys::blind_token(&key, "hello");
        assert_eq!(t1, t2);

        // Different input → different token
        let t3 = E2eKeys::blind_token(&key, "world");
        assert_ne!(t1, t3);
    }

    #[tokio::test]
    async fn test_roundtrip_with_local_crypto_provider() {
        let secret = [0x99u8; 32];
        let keys = E2eKeys::from_secret(&secret);

        let provider = LocalCryptoProvider::from_key(keys.encryption_key());
        let plaintext = b"sensitive atom content";

        let ciphertext = provider.encrypt(plaintext).await.unwrap();
        assert_ne!(&ciphertext[..], plaintext);

        let decrypted = provider.decrypt(&ciphertext).await.unwrap();
        assert_eq!(&decrypted[..], plaintext);
    }

    #[tokio::test]
    async fn test_load_or_generate_creates_and_reloads() {
        let tmp = tempdir().unwrap();
        let key_path = tmp.path().join(".fold_db").join("e2e.key");

        let keys1 = E2eKeys::load_or_generate(&key_path).await.unwrap();
        assert!(key_path.exists());

        let keys2 = E2eKeys::load_or_generate(&key_path).await.unwrap();
        assert_eq!(keys1.encryption_key(), keys2.encryption_key());
        assert_eq!(keys1.index_key(), keys2.index_key());
    }

    #[tokio::test]
    async fn test_load_or_generate_rejects_bad_length() {
        let tmp = tempdir().unwrap();
        let key_path = tmp.path().join("bad_e2e.key");
        tokio::fs::write(&key_path, b"short").await.unwrap();

        let result = E2eKeys::load_or_generate(&key_path).await;
        assert!(result.is_err());
    }
}
