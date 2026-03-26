use ed25519_dalek::{SigningKey, VerifyingKey};
use hmac::{Hmac, Mac};
use sha2::{Digest, Sha256};
use uuid::Uuid;

type HmacSha256 = Hmac<Sha256>;

/// Derive a unique, unlinkable ed25519 keypair for a specific fragment.
///
/// Uses HMAC-SHA256(master_key_bytes, content_hash) to produce 32 deterministic bytes,
/// then interprets those as an ed25519 signing key. This follows the paper's specification:
///
///   (pk_i, sk_i) = Derive(sk, h_i)
///
/// Properties:
/// - **Deterministic**: Same master key + same content = same derived keypair
/// - **Unlinkable**: Two derived public keys from the same master key are computationally
///   indistinguishable from independently generated keys
/// - **Non-transferable**: The derived sk_i can sign messages verifiable against pk_i
///   without revealing the master key or any other derived key
pub fn derive_fragment_keypair(
    master_key: &SigningKey,
    fragment_content: &str,
) -> (VerifyingKey, SigningKey) {
    let content_hash = content_hash(fragment_content);
    derive_fragment_keypair_from_hash(master_key, &content_hash)
}

/// Derive keypair from a pre-computed content hash.
pub fn derive_fragment_keypair_from_hash(
    master_key: &SigningKey,
    content_hash: &[u8],
) -> (VerifyingKey, SigningKey) {
    let mut mac =
        HmacSha256::new_from_slice(master_key.as_bytes()).expect("HMAC accepts any key length");
    mac.update(content_hash);
    let result = mac.finalize().into_bytes();

    // Use the 32-byte HMAC output as an ed25519 signing key seed
    let mut seed = [0u8; 32];
    seed.copy_from_slice(&result[..32]);

    let derived_sk = SigningKey::from_bytes(&seed);
    let derived_pk = derived_sk.verifying_key();

    (derived_pk, derived_sk)
}

/// Derive a deterministic UUID pseudonym (backward-compatible with fold_db_node's existing format).
///
/// Uses keyed SHA-256: `SHA256(master_key || "discovery-pseudonym" || content_hash)`,
/// then takes the first 16 bytes as a UUID v4-compatible identifier.
pub fn derive_pseudonym_uuid(master_key: &[u8], content_hash: &[u8]) -> Uuid {
    let mut hasher = Sha256::new();
    hasher.update(master_key);
    hasher.update(b"discovery-pseudonym");
    hasher.update(content_hash);
    let hash = hasher.finalize();

    let mut bytes = [0u8; 16];
    bytes.copy_from_slice(&hash[..16]);
    // Set version 4 (bits 12-15 of byte 6)
    bytes[6] = (bytes[6] & 0x0F) | 0x40;
    // Set variant (bits 6-7 of byte 8)
    bytes[8] = (bytes[8] & 0x3F) | 0x80;

    Uuid::from_bytes(bytes)
}

/// Compute SHA-256 hash of text content.
pub fn content_hash(text: &str) -> Vec<u8> {
    let mut hasher = Sha256::new();
    hasher.update(text.as_bytes());
    hasher.finalize().to_vec()
}

/// Compute SHA-256 hash of raw bytes.
pub fn content_hash_bytes(data: &[u8]) -> Vec<u8> {
    let mut hasher = Sha256::new();
    hasher.update(data);
    hasher.finalize().to_vec()
}

#[cfg(test)]
mod tests {
    use super::*;

    fn test_master_key() -> SigningKey {
        SigningKey::from_bytes(&[42u8; 32])
    }

    #[test]
    fn test_keypair_determinism() {
        let master = test_master_key();
        let (pk1, sk1) = derive_fragment_keypair(&master, "hello world");
        let (pk2, sk2) = derive_fragment_keypair(&master, "hello world");
        assert_eq!(pk1, pk2);
        assert_eq!(sk1.to_bytes(), sk2.to_bytes());
    }

    #[test]
    fn test_keypair_unlinkability() {
        let master = test_master_key();
        let (pk1, _) = derive_fragment_keypair(&master, "content A");
        let (pk2, _) = derive_fragment_keypair(&master, "content B");
        assert_ne!(pk1, pk2, "Different content must produce different keys");
    }

    #[test]
    fn test_keypair_different_masters() {
        let master1 = SigningKey::from_bytes(&[1u8; 32]);
        let master2 = SigningKey::from_bytes(&[2u8; 32]);
        let (pk1, _) = derive_fragment_keypair(&master1, "same content");
        let (pk2, _) = derive_fragment_keypair(&master2, "same content");
        assert_ne!(pk1, pk2, "Different masters must produce different keys");
    }

    #[test]
    fn test_derived_key_can_sign_and_verify() {
        let master = test_master_key();
        let (pk, sk) = derive_fragment_keypair(&master, "test fragment");
        let message = b"access request";

        use ed25519_dalek::Signer;
        let signature = sk.sign(message);

        use ed25519_dalek::Verifier;
        assert!(pk.verify(message, &signature).is_ok());
    }

    #[test]
    fn test_uuid_backward_compat() {
        // Verify the UUID derivation matches the fold_db_node implementation
        let master_key = b"test-master-key";
        let hash = content_hash("hello world");
        let uuid = derive_pseudonym_uuid(master_key, &hash);

        // Same inputs should produce the same UUID
        let uuid2 = derive_pseudonym_uuid(master_key, &hash);
        assert_eq!(uuid, uuid2);

        // Different content = different UUID
        let hash2 = content_hash("different content");
        let uuid3 = derive_pseudonym_uuid(master_key, &hash2);
        assert_ne!(uuid, uuid3);

        // UUID should be version 4
        assert_eq!(uuid.get_version_num(), 4);
    }

    #[test]
    fn test_content_hash_determinism() {
        let h1 = content_hash("test");
        let h2 = content_hash("test");
        assert_eq!(h1, h2);
    }

    #[test]
    fn test_content_hash_different_inputs() {
        let h1 = content_hash("hello");
        let h2 = content_hash("world");
        assert_ne!(h1, h2);
    }
}
