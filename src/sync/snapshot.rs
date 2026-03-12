use super::error::{SyncError, SyncResult};
use crate::crypto::CryptoProvider;
use crate::storage::traits::NamespacedStore;
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::sync::Arc;

/// A serialized snapshot of the entire database.
///
/// The snapshot format is backend-agnostic: a list of namespaces,
/// each containing a list of key-value pairs. Keys and values are
/// base64-encoded bytes.
///
/// For streaming, the snapshot is written namespace-by-namespace to
/// a temp file, then encrypted and uploaded.
#[derive(Debug, Serialize, Deserialize)]
pub struct Snapshot {
    /// Format version for forward compatibility.
    pub version: u32,
    /// Timestamp when the snapshot was created (millis since epoch).
    pub created_at_ms: u64,
    /// Device ID that created this snapshot.
    pub device_id: String,
    /// The log sequence number this snapshot covers up to (inclusive).
    pub last_seq: u64,
    /// All namespaces and their key-value pairs.
    pub namespaces: Vec<NamespaceData>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct NamespaceData {
    pub name: String,
    /// Key-value pairs, both base64-encoded.
    pub entries: Vec<SnapshotEntry>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct SnapshotEntry {
    pub key: String,
    pub value: String,
}

const SNAPSHOT_VERSION: u32 = 1;
const HASH_SIZE: usize = 32;

impl Snapshot {
    /// Create a snapshot from a NamespacedStore by iterating all namespaces and keys.
    ///
    /// This loads one namespace at a time to limit memory usage.
    pub async fn create(
        store: &dyn NamespacedStore,
        device_id: &str,
        last_seq: u64,
    ) -> SyncResult<Self> {
        use base64::{engine::general_purpose::STANDARD as BASE64, Engine as _};

        let ns_names = store.list_namespaces().await?;
        let mut namespaces = Vec::with_capacity(ns_names.len());

        for ns_name in &ns_names {
            // Skip internal sled namespace
            if ns_name == "__sled__default" {
                continue;
            }

            let kv = store.open_namespace(ns_name).await?;
            let pairs = kv.scan_prefix(&[]).await?;

            let entries: Vec<SnapshotEntry> = pairs
                .into_iter()
                .map(|(k, v)| SnapshotEntry {
                    key: BASE64.encode(&k),
                    value: BASE64.encode(&v),
                })
                .collect();

            namespaces.push(NamespaceData {
                name: ns_name.clone(),
                entries,
            });
        }

        let now = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap_or_default()
            .as_millis() as u64;

        Ok(Snapshot {
            version: SNAPSHOT_VERSION,
            created_at_ms: now,
            device_id: device_id.to_string(),
            last_seq,
            namespaces,
        })
    }

    /// Serialize the snapshot to a temp file to avoid holding everything in memory,
    /// then encrypt the file contents.
    ///
    /// Returns the encrypted bytes (hash + ciphertext).
    pub async fn seal(
        &self,
        crypto: &Arc<dyn CryptoProvider>,
    ) -> SyncResult<Vec<u8>> {
        // Serialize to a temp file for large snapshots
        let json = tokio::task::spawn_blocking({
            let snapshot = serde_json::to_vec(self)
                .map_err(|e| SyncError::Serialization(e.to_string()));
            move || snapshot
        })
        .await
        .map_err(|e| SyncError::Serialization(e.to_string()))??;

        // Hash the plaintext for integrity verification
        let mut hasher = Sha256::new();
        hasher.update(&json);
        let hash: [u8; 32] = hasher.finalize().into();

        let mut plaintext = Vec::with_capacity(HASH_SIZE + json.len());
        plaintext.extend_from_slice(&hash);
        plaintext.extend_from_slice(&json);

        let ciphertext = crypto.encrypt(&plaintext).await?;
        Ok(ciphertext)
    }

    /// Decrypt and deserialize a snapshot.
    pub async fn unseal(
        data: &[u8],
        crypto: &Arc<dyn CryptoProvider>,
    ) -> SyncResult<Self> {
        let plaintext = crypto.decrypt(data).await.map_err(|_| SyncError::WrongKey)?;

        if plaintext.len() < HASH_SIZE {
            return Err(SyncError::Crypto("snapshot too short for hash".to_string()));
        }

        let (stored_hash, json_bytes) = plaintext.split_at(HASH_SIZE);

        let mut hasher = Sha256::new();
        hasher.update(json_bytes);
        let computed_hash: [u8; 32] = hasher.finalize().into();

        if stored_hash != computed_hash.as_slice() {
            return Err(SyncError::Crypto("snapshot hash mismatch — data corrupted".to_string()));
        }

        let snapshot: Snapshot = serde_json::from_slice(json_bytes)?;

        if snapshot.version != SNAPSHOT_VERSION {
            return Err(SyncError::Crypto(format!(
                "unsupported snapshot version: {} (expected {})",
                snapshot.version, SNAPSHOT_VERSION
            )));
        }

        Ok(snapshot)
    }

    /// Restore a snapshot into a NamespacedStore.
    ///
    /// This clears existing namespaces and writes the snapshot data.
    pub async fn restore(
        &self,
        store: &dyn NamespacedStore,
    ) -> SyncResult<()> {
        use base64::{engine::general_purpose::STANDARD as BASE64, Engine as _};

        for ns_data in &self.namespaces {
            let kv = store.open_namespace(&ns_data.name).await?;

            // Clear existing keys in this namespace before writing snapshot data
            let existing = kv.scan_prefix(&[]).await?;
            if !existing.is_empty() {
                let keys: Vec<Vec<u8>> = existing.into_iter().map(|(k, _)| k).collect();
                kv.batch_delete(keys).await?;
            }

            // Write entries in batches
            const BATCH_SIZE: usize = 25;
            for chunk in ns_data.entries.chunks(BATCH_SIZE) {
                let items: Vec<(Vec<u8>, Vec<u8>)> = chunk
                    .iter()
                    .map(|entry| {
                        let key = BASE64.decode(&entry.key).map_err(|e| {
                            SyncError::Serialization(format!("invalid key base64: {e}"))
                        });
                        let value = BASE64.decode(&entry.value).map_err(|e| {
                            SyncError::Serialization(format!("invalid value base64: {e}"))
                        });
                        Ok((key?, value?))
                    })
                    .collect::<SyncResult<Vec<_>>>()?;

                kv.batch_put(items).await?;
            }
        }

        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::crypto::provider::LocalCryptoProvider;
    use crate::storage::inmemory_backend::InMemoryNamespacedStore;
    use crate::storage::traits::NamespacedStore;

    fn test_crypto() -> Arc<dyn CryptoProvider> {
        Arc::new(LocalCryptoProvider::from_key([0x42u8; 32]))
    }

    async fn populated_store() -> InMemoryNamespacedStore {
        let store = InMemoryNamespacedStore::new();

        let main = store.open_namespace("main").await.unwrap();
        main.put(b"atom:1", b"value1".to_vec()).await.unwrap();
        main.put(b"atom:2", b"value2".to_vec()).await.unwrap();

        let meta = store.open_namespace("metadata").await.unwrap();
        meta.put(b"schema:foo", b"schema_data".to_vec()).await.unwrap();

        store
    }

    #[tokio::test]
    async fn create_snapshot_from_store() {
        let store = populated_store().await;
        let snapshot = Snapshot::create(&store, "device-1", 10).await.unwrap();

        assert_eq!(snapshot.version, SNAPSHOT_VERSION);
        assert_eq!(snapshot.device_id, "device-1");
        assert_eq!(snapshot.last_seq, 10);
        assert_eq!(snapshot.namespaces.len(), 2);

        let main_ns = snapshot.namespaces.iter().find(|n| n.name == "main").unwrap();
        assert_eq!(main_ns.entries.len(), 2);
    }

    #[tokio::test]
    async fn seal_unseal_roundtrip() {
        let store = populated_store().await;
        let crypto = test_crypto();

        let snapshot = Snapshot::create(&store, "device-1", 10).await.unwrap();
        let sealed = snapshot.seal(&crypto).await.unwrap();
        let unsealed = Snapshot::unseal(&sealed, &crypto).await.unwrap();

        assert_eq!(unsealed.device_id, "device-1");
        assert_eq!(unsealed.last_seq, 10);
        assert_eq!(unsealed.namespaces.len(), 2);
    }

    #[tokio::test]
    async fn wrong_key_fails() {
        let store = populated_store().await;
        let crypto1 = test_crypto();
        let crypto2: Arc<dyn CryptoProvider> =
            Arc::new(LocalCryptoProvider::from_key([0x99u8; 32]));

        let snapshot = Snapshot::create(&store, "device-1", 10).await.unwrap();
        let sealed = snapshot.seal(&crypto1).await.unwrap();

        let result = Snapshot::unseal(&sealed, &crypto2).await;
        assert!(matches!(result, Err(SyncError::WrongKey)));
    }

    #[tokio::test]
    async fn restore_snapshot_to_empty_store() {
        let source = populated_store().await;
        let crypto = test_crypto();

        let snapshot = Snapshot::create(&source, "device-1", 10).await.unwrap();
        let sealed = snapshot.seal(&crypto).await.unwrap();
        let restored_snapshot = Snapshot::unseal(&sealed, &crypto).await.unwrap();

        let target = InMemoryNamespacedStore::new();
        restored_snapshot.restore(&target).await.unwrap();

        // Verify data was restored
        let main = target.open_namespace("main").await.unwrap();
        let val = main.get(b"atom:1").await.unwrap();
        assert_eq!(val, Some(b"value1".to_vec()));

        let val2 = main.get(b"atom:2").await.unwrap();
        assert_eq!(val2, Some(b"value2".to_vec()));

        let meta = target.open_namespace("metadata").await.unwrap();
        let schema = meta.get(b"schema:foo").await.unwrap();
        assert_eq!(schema, Some(b"schema_data".to_vec()));
    }

    #[tokio::test]
    async fn empty_store_snapshot() {
        let store = InMemoryNamespacedStore::new();
        let crypto = test_crypto();

        let snapshot = Snapshot::create(&store, "device-1", 0).await.unwrap();
        assert!(snapshot.namespaces.is_empty());

        let sealed = snapshot.seal(&crypto).await.unwrap();
        let unsealed = Snapshot::unseal(&sealed, &crypto).await.unwrap();
        assert!(unsealed.namespaces.is_empty());
    }

    #[tokio::test]
    async fn tampered_snapshot_fails() {
        let store = populated_store().await;
        let crypto = test_crypto();

        let snapshot = Snapshot::create(&store, "device-1", 10).await.unwrap();
        let mut sealed = snapshot.seal(&crypto).await.unwrap();

        // Tamper with the ciphertext
        let last = sealed.len() - 1;
        sealed[last] ^= 0x01;

        let result = Snapshot::unseal(&sealed, &crypto).await;
        assert!(result.is_err());
    }
}
