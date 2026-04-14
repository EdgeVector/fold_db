use super::error::{SyncError, SyncResult};
use crate::crypto::CryptoProvider;
use base64::{engine::general_purpose::STANDARD as BASE64, Engine as _};
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::sync::Arc;
use subtle::ConstantTimeEq;

/// A single KvStore operation recorded for sync.
///
/// Each entry captures one write operation (put, delete, batch_put, batch_delete)
/// along with its sequence number for ordered replay.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LogEntry {
    /// Timestamp-based ID (nanos since epoch). Used as the R2 object key.
    /// Not sequential — just unique enough to avoid collisions.
    pub seq: u64,
    /// Client timestamp (millis since epoch). Used for LWW conflict resolution.
    pub timestamp_ms: u64,
    /// Device ID that produced this entry.
    pub device_id: String,
    /// The operation.
    pub op: LogOp,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum LogOp {
    Put {
        namespace: String,
        /// Base64-encoded key (keys are arbitrary bytes).
        key: String,
        /// Base64-encoded value.
        value: String,
    },
    Delete {
        namespace: String,
        key: String,
    },
    BatchPut {
        namespace: String,
        /// Vec of (base64 key, base64 value).
        items: Vec<(String, String)>,
    },
    BatchDelete {
        namespace: String,
        keys: Vec<String>,
    },
}

/// Serialized + encrypted log entry with integrity hash.
///
/// Wire format:
/// ```text
/// [sha256: 32 bytes] [encrypted_payload: variable]
/// ```
///
/// The SHA-256 is computed over the plaintext JSON before encryption,
/// allowing the reader to verify integrity after decryption.
pub struct SealedLogEntry {
    pub bytes: Vec<u8>,
}

const HASH_SIZE: usize = 32;

impl LogEntry {
    /// Serialize, hash, encrypt.
    pub async fn seal(&self, crypto: &Arc<dyn CryptoProvider>) -> SyncResult<SealedLogEntry> {
        let json = serde_json::to_vec(self)?;

        let mut hasher = Sha256::new();
        hasher.update(&json);
        let hash: [u8; 32] = hasher.finalize().into();

        let mut plaintext = Vec::with_capacity(HASH_SIZE + json.len());
        plaintext.extend_from_slice(&hash);
        plaintext.extend_from_slice(&json);

        let ciphertext = crypto.encrypt(&plaintext).await?;

        Ok(SealedLogEntry { bytes: ciphertext })
    }

    /// Decrypt, verify hash, deserialize.
    pub async fn unseal(sealed: &[u8], crypto: &Arc<dyn CryptoProvider>) -> SyncResult<Self> {
        let plaintext = crypto
            .decrypt(sealed)
            .await
            .map_err(|e| SyncError::Crypto(format!("failed to decrypt log entry: {e}")))?;

        if plaintext.len() < HASH_SIZE {
            return Err(SyncError::CorruptEntry {
                seq: 0,
                reason: "plaintext too short for hash".to_string(),
            });
        }

        let (stored_hash, json_bytes) = plaintext.split_at(HASH_SIZE);

        let mut hasher = Sha256::new();
        hasher.update(json_bytes);
        let computed_hash: [u8; 32] = hasher.finalize().into();

        // Constant-time compare to prevent timing oracle on integrity hash.
        // A byte-by-byte `!=` can leak the position of the first differing byte
        // via short-circuit timing, letting an attacker incrementally forge a
        // valid hash for tampered log entries. `ConstantTimeEq::ct_eq` compares
        // in time independent of the contents.
        if !bool::from(stored_hash.ct_eq(computed_hash.as_slice())) {
            return Err(SyncError::CorruptEntry {
                seq: 0,
                reason: "hash mismatch — data corrupted".to_string(),
            });
        }

        let entry: LogEntry = serde_json::from_slice(json_bytes)?;
        Ok(entry)
    }
}

impl LogOp {
    /// Returns the namespace this operation targets.
    pub fn namespace(&self) -> &str {
        match self {
            LogOp::Put { namespace, .. } => namespace,
            LogOp::Delete { namespace, .. } => namespace,
            LogOp::BatchPut { namespace, .. } => namespace,
            LogOp::BatchDelete { namespace, .. } => namespace,
        }
    }

    /// Encode key bytes to base64 for storage in the log entry.
    pub fn encode_bytes(bytes: &[u8]) -> String {
        BASE64.encode(bytes)
    }

    /// Decode base64 key back to bytes for replay.
    pub fn decode_bytes(encoded: &str) -> SyncResult<Vec<u8>> {
        BASE64
            .decode(encoded)
            .map_err(|e| SyncError::Serialization(format!("invalid base64: {e}")))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::crypto::provider::LocalCryptoProvider;

    fn test_crypto() -> Arc<dyn CryptoProvider> {
        Arc::new(LocalCryptoProvider::from_key([0x42u8; 32]))
    }

    fn sample_entry(seq: u64) -> LogEntry {
        LogEntry {
            seq,
            timestamp_ms: 1700000000000,
            device_id: "device-abc".to_string(),
            op: LogOp::Put {
                namespace: "main".to_string(),
                key: LogOp::encode_bytes(b"atom:123"),
                value: LogOp::encode_bytes(b"{\"name\":\"test\"}"),
            },
        }
    }

    #[tokio::test]
    async fn seal_unseal_roundtrip() {
        let crypto = test_crypto();
        let entry = sample_entry(1);

        let sealed = entry.seal(&crypto).await.unwrap();
        let unsealed = LogEntry::unseal(&sealed.bytes, &crypto).await.unwrap();

        assert_eq!(unsealed.seq, 1);
        assert_eq!(unsealed.device_id, "device-abc");
    }

    #[tokio::test]
    async fn tampered_ciphertext_fails() {
        let crypto = test_crypto();
        let entry = sample_entry(2);

        let mut sealed = entry.seal(&crypto).await.unwrap();
        let last = sealed.bytes.len() - 1;
        sealed.bytes[last] ^= 0x01;

        let result = LogEntry::unseal(&sealed.bytes, &crypto).await;
        assert!(result.is_err());
    }

    #[tokio::test]
    async fn wrong_key_fails() {
        let crypto1 = test_crypto();
        let crypto2: Arc<dyn CryptoProvider> =
            Arc::new(LocalCryptoProvider::from_key([0x99u8; 32]));

        let entry = sample_entry(3);
        let sealed = entry.seal(&crypto1).await.unwrap();

        let result = LogEntry::unseal(&sealed.bytes, &crypto2).await;
        assert!(result.is_err());
    }

    #[tokio::test]
    async fn delete_op_roundtrip() {
        let crypto = test_crypto();
        let entry = LogEntry {
            seq: 4,
            timestamp_ms: 1700000000000,
            device_id: "device-abc".to_string(),
            op: LogOp::Delete {
                namespace: "main".to_string(),
                key: LogOp::encode_bytes(b"atom:456"),
            },
        };

        let sealed = entry.seal(&crypto).await.unwrap();
        let unsealed = LogEntry::unseal(&sealed.bytes, &crypto).await.unwrap();

        assert_eq!(unsealed.seq, 4);
        assert!(matches!(unsealed.op, LogOp::Delete { .. }));
    }

    #[tokio::test]
    async fn batch_put_roundtrip() {
        let crypto = test_crypto();
        let entry = LogEntry {
            seq: 5,
            timestamp_ms: 1700000000000,
            device_id: "device-abc".to_string(),
            op: LogOp::BatchPut {
                namespace: "metadata".to_string(),
                items: vec![
                    (LogOp::encode_bytes(b"k1"), LogOp::encode_bytes(b"v1")),
                    (LogOp::encode_bytes(b"k2"), LogOp::encode_bytes(b"v2")),
                ],
            },
        };

        let sealed = entry.seal(&crypto).await.unwrap();
        let unsealed = LogEntry::unseal(&sealed.bytes, &crypto).await.unwrap();

        if let LogOp::BatchPut { items, .. } = &unsealed.op {
            assert_eq!(items.len(), 2);
        } else {
            panic!("expected BatchPut");
        }
    }

    #[tokio::test]
    async fn flipped_integrity_hash_is_rejected() {
        // Build a sealed entry by hand with a bit-flipped stored hash so that
        // the ciphertext decrypts cleanly but the integrity hash does not match.
        // This exercises the constant-time hash comparison in `unseal`.
        let crypto = test_crypto();
        let entry = sample_entry(42);
        let json = serde_json::to_vec(&entry).unwrap();

        let mut hasher = Sha256::new();
        hasher.update(&json);
        let mut hash: [u8; 32] = hasher.finalize().into();
        // Flip one bit in the stored hash.
        hash[0] ^= 0x01;

        let mut plaintext = Vec::with_capacity(HASH_SIZE + json.len());
        plaintext.extend_from_slice(&hash);
        plaintext.extend_from_slice(&json);

        let ciphertext = crypto.encrypt(&plaintext).await.unwrap();
        let result = LogEntry::unseal(&ciphertext, &crypto).await;

        match result {
            Err(SyncError::CorruptEntry { reason, .. }) => {
                assert!(
                    reason.contains("hash mismatch"),
                    "unexpected reason: {reason}"
                );
            }
            other => panic!("expected CorruptEntry hash mismatch, got {other:?}"),
        }
    }

    #[test]
    fn encode_decode_bytes() {
        let original = b"hello world";
        let encoded = LogOp::encode_bytes(original);
        let decoded = LogOp::decode_bytes(&encoded).unwrap();
        assert_eq!(decoded, original);
    }
}
