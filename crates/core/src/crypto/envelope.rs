use super::error::{CryptoError, CryptoResult};
use aes_gcm::{
    aead::{Aead, KeyInit, OsRng},
    Aes256Gcm, Nonce,
};
use rand::RngCore;

/// Current envelope format version.
/// Version 1: AES-256-GCM with 12-byte nonce, 16-byte tag.
pub const ENVELOPE_VERSION: u8 = 0x01;

/// Size constants for the envelope format
const NONCE_SIZE: usize = 12;
const VERSION_SIZE: usize = 1;
/// Minimum ciphertext size: version(1) + nonce(12) + tag(16)
const MIN_ENVELOPE_SIZE: usize = VERSION_SIZE + NONCE_SIZE + 16;

/// Encrypt plaintext using AES-256-GCM with a random nonce.
///
/// Returns a self-describing binary envelope:
/// ```text
/// [version: 1B] [nonce: 12B] [ciphertext+tag: variable]
/// ```
///
/// The GCM authentication tag (16 bytes) is appended to the ciphertext
/// by the `aes-gcm` crate automatically.
pub fn encrypt_envelope(key: &[u8; 32], plaintext: &[u8]) -> CryptoResult<Vec<u8>> {
    let cipher = Aes256Gcm::new_from_slice(key)
        .map_err(|e| CryptoError::EncryptionFailed(format!("Invalid key: {}", e)))?;

    // Generate a random 12-byte nonce
    let mut nonce_bytes = [0u8; NONCE_SIZE];
    OsRng.fill_bytes(&mut nonce_bytes);
    let nonce = Nonce::from_slice(&nonce_bytes);

    let ciphertext = cipher
        .encrypt(nonce, plaintext)
        .map_err(|e| CryptoError::EncryptionFailed(format!("AES-GCM encrypt: {}", e)))?;

    // Build envelope: version || nonce || ciphertext+tag
    let mut envelope = Vec::with_capacity(VERSION_SIZE + NONCE_SIZE + ciphertext.len());
    envelope.push(ENVELOPE_VERSION);
    envelope.extend_from_slice(&nonce_bytes);
    envelope.extend_from_slice(&ciphertext);

    Ok(envelope)
}

/// Decrypt an envelope produced by `encrypt_envelope`.
///
/// Parses the version byte, extracts the nonce, and decrypts the ciphertext.
pub fn decrypt_envelope(key: &[u8; 32], envelope: &[u8]) -> CryptoResult<Vec<u8>> {
    if envelope.len() < MIN_ENVELOPE_SIZE {
        return Err(CryptoError::InvalidFormat(format!(
            "Envelope too short: {} bytes (minimum {})",
            envelope.len(),
            MIN_ENVELOPE_SIZE
        )));
    }

    let version = envelope[0];
    if version != ENVELOPE_VERSION {
        return Err(CryptoError::UnsupportedVersion(version));
    }

    let nonce_bytes = &envelope[VERSION_SIZE..VERSION_SIZE + NONCE_SIZE];
    let ciphertext = &envelope[VERSION_SIZE + NONCE_SIZE..];

    let cipher = Aes256Gcm::new_from_slice(key)
        .map_err(|e| CryptoError::DecryptionFailed(format!("Invalid key: {}", e)))?;

    let nonce = Nonce::from_slice(nonce_bytes);

    cipher
        .decrypt(nonce, ciphertext)
        .map_err(|e| CryptoError::DecryptionFailed(format!("AES-GCM decrypt: {}", e)))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_roundtrip() {
        let key = [0x42u8; 32];
        let plaintext = b"Hello, FoldDB atoms!";

        let envelope = encrypt_envelope(&key, plaintext).unwrap();
        let decrypted = decrypt_envelope(&key, &envelope).unwrap();

        assert_eq!(decrypted, plaintext);
    }

    #[test]
    fn test_envelope_format() {
        let key = [0x42u8; 32];
        let plaintext = b"test";

        let envelope = encrypt_envelope(&key, plaintext).unwrap();

        // Version byte
        assert_eq!(envelope[0], ENVELOPE_VERSION);
        // Total size: 1 (version) + 12 (nonce) + 4 (plaintext) + 16 (tag)
        assert_eq!(envelope.len(), 1 + 12 + 4 + 16);
    }

    #[test]
    fn test_wrong_key_fails() {
        let key1 = [0x42u8; 32];
        let key2 = [0x43u8; 32];
        let plaintext = b"secret data";

        let envelope = encrypt_envelope(&key1, plaintext).unwrap();
        let result = decrypt_envelope(&key2, &envelope);

        assert!(result.is_err());
    }

    #[test]
    fn test_tampered_ciphertext_fails() {
        let key = [0x42u8; 32];
        let plaintext = b"don't tamper";

        let mut envelope = encrypt_envelope(&key, plaintext).unwrap();
        // Flip a bit in the ciphertext portion
        let last = envelope.len() - 1;
        envelope[last] ^= 0x01;

        let result = decrypt_envelope(&key, &envelope);
        assert!(result.is_err());
    }

    #[test]
    fn test_short_envelope_fails() {
        let key = [0x42u8; 32];
        let result = decrypt_envelope(&key, &[0x01, 0x02]);
        assert!(result.is_err());
    }

    #[test]
    fn test_unsupported_version() {
        let key = [0x42u8; 32];
        let plaintext = b"test";
        let mut envelope = encrypt_envelope(&key, plaintext).unwrap();
        envelope[0] = 0xFF;

        let result = decrypt_envelope(&key, &envelope);
        assert!(matches!(result, Err(CryptoError::UnsupportedVersion(0xFF))));
    }

    #[test]
    fn test_unique_nonces() {
        let key = [0x42u8; 32];
        let plaintext = b"same data";

        let env1 = encrypt_envelope(&key, plaintext).unwrap();
        let env2 = encrypt_envelope(&key, plaintext).unwrap();

        // Nonces should differ (bytes 1..13)
        assert_ne!(&env1[1..13], &env2[1..13]);
        // But both should decrypt to the same plaintext
        assert_eq!(
            decrypt_envelope(&key, &env1).unwrap(),
            decrypt_envelope(&key, &env2).unwrap()
        );
    }

    #[test]
    fn test_empty_plaintext() {
        let key = [0x42u8; 32];
        let plaintext = b"";

        let envelope = encrypt_envelope(&key, plaintext).unwrap();
        let decrypted = decrypt_envelope(&key, &envelope).unwrap();
        assert_eq!(decrypted, plaintext);
    }
}
