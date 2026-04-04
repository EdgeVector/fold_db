use super::error::{StorageError, StorageResult};
use super::traits::{ExecutionModel, FlushBehavior, KvStore};
use crate::crypto::CryptoProvider;
use crate::sync::org_sync::strip_org_prefix;
use async_trait::async_trait;
use base64::{engine::general_purpose::STANDARD as B64, Engine};
use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::RwLock;

/// Prefix marker for encrypted values.
/// On write: `ENC:` + base64(ciphertext) → valid UTF-8 string for DynamoDB `S` attributes.
/// On read: detect prefix → strip → base64-decode → decrypt.
const ENCRYPTED_PREFIX: &str = "ENC:";

/// A decorator that transparently encrypts values on write and decrypts on read.
///
/// This wraps any `KvStore` implementation, inserting an encryption layer
/// between the `TypedKvStore` serialization boundary and the actual backend.
///
/// ```text
/// TypedKvStore (JSON bytes)
///       ↓
/// EncryptingKvStore (encrypt → base64 + prefix → valid UTF-8 string)
///       ↓
/// DynamoDbKvStore / SledKvStore (stores as DynamoDB S attribute)
/// ```
///
/// Keys are NOT encrypted — only values. This preserves indexing and scan_prefix.
///
/// Encrypted values are stored as `ENC:<base64(ciphertext)>` so they remain valid
/// UTF-8 strings and survive DynamoDB's `S` attribute type, which requires UTF-8.
///
/// During migration (dual-read mode), values without the `ENC:` prefix are
/// treated as pre-migration plaintext and returned as-is.
pub struct EncryptingKvStore {
    inner: Arc<dyn KvStore>,
    crypto: Arc<dyn CryptoProvider>,
    /// When true, if decryption fails, assume data is plaintext and return as-is.
    /// This enables gradual migration from unencrypted to encrypted storage.
    migration_mode: bool,
    /// Per-org crypto providers. When a key starts with `{org_hash}:`, the
    /// corresponding provider is used instead of the default `crypto`.
    org_crypto: Arc<RwLock<HashMap<String, Arc<dyn CryptoProvider>>>>,
}

impl EncryptingKvStore {
    /// Create a new encrypting store wrapping the given inner store.
    ///
    /// - `migration_mode`: When `true`, failed decryption returns raw bytes
    ///   (assumes plaintext pre-migration data).
    pub fn new(
        inner: Arc<dyn KvStore>,
        crypto: Arc<dyn CryptoProvider>,
        migration_mode: bool,
    ) -> Self {
        Self {
            inner,
            crypto,
            migration_mode,
            org_crypto: Arc::new(RwLock::new(HashMap::new())),
        }
    }

    /// Create with org crypto routing support.
    pub fn with_org_crypto(
        inner: Arc<dyn KvStore>,
        crypto: Arc<dyn CryptoProvider>,
        migration_mode: bool,
        org_crypto: Arc<RwLock<HashMap<String, Arc<dyn CryptoProvider>>>>,
    ) -> Self {
        Self {
            inner,
            crypto,
            migration_mode,
            org_crypto,
        }
    }

    /// Select the right crypto provider for a key.
    ///
    /// If the key starts with a 64-char hex org_hash followed by `:`,
    /// and we have a registered provider for that org, use it.
    /// Otherwise fall back to the default (personal) provider.
    async fn select_crypto(&self, key: &[u8]) -> Arc<dyn CryptoProvider> {
        if let Ok(key_str) = std::str::from_utf8(key) {
            if let Some((org_hash, _)) = strip_org_prefix(key_str) {
                let org_map = self.org_crypto.read().await;
                if let Some(provider) = org_map.get(org_hash) {
                    return Arc::clone(provider);
                }
            }
        }
        Arc::clone(&self.crypto)
    }

    /// Encode ciphertext bytes into a UTF-8-safe string with the `ENC:` prefix.
    fn encode_ciphertext(ciphertext: &[u8]) -> Vec<u8> {
        let encoded = format!("{}{}", ENCRYPTED_PREFIX, B64.encode(ciphertext));
        encoded.into_bytes()
    }

    /// Attempt to decrypt stored data using the given crypto provider.
    /// If data has the `ENC:` prefix, decode and decrypt.
    /// Otherwise fall back to plaintext if in migration mode.
    async fn decrypt_or_passthrough(
        &self,
        data: Vec<u8>,
        crypto: &dyn CryptoProvider,
    ) -> StorageResult<Vec<u8>> {
        // Check for the ENC: prefix
        if data.starts_with(ENCRYPTED_PREFIX.as_bytes()) {
            let b64_part = &data[ENCRYPTED_PREFIX.len()..];
            let ciphertext = B64.decode(b64_part).map_err(|e| {
                StorageError::EncryptionError(format!("Base64 decode failed: {}", e))
            })?;
            return crypto
                .decrypt(&ciphertext)
                .await
                .map_err(|e| StorageError::EncryptionError(e.to_string()));
        }

        // No ENC: prefix — this is either plaintext or raw binary
        if self.migration_mode {
            log::debug!("No ENC: prefix in migration mode, returning raw data as plaintext");
            Ok(data)
        } else {
            Err(StorageError::EncryptionError(
                "Data is not encrypted (missing ENC: prefix) and migration mode is off".to_string(),
            ))
        }
    }
}

#[async_trait]
impl KvStore for EncryptingKvStore {
    async fn get(&self, key: &[u8]) -> StorageResult<Option<Vec<u8>>> {
        match self.inner.get(key).await? {
            Some(stored) => {
                let crypto = self.select_crypto(key).await;
                let plaintext = self.decrypt_or_passthrough(stored, crypto.as_ref()).await?;
                Ok(Some(plaintext))
            }
            None => Ok(None),
        }
    }

    async fn put(&self, key: &[u8], value: Vec<u8>) -> StorageResult<()> {
        let crypto = self.select_crypto(key).await;
        let ciphertext = crypto
            .encrypt(&value)
            .await
            .map_err(|e| StorageError::EncryptionError(e.to_string()))?;
        let encoded = Self::encode_ciphertext(&ciphertext);
        self.inner.put(key, encoded).await
    }

    async fn delete(&self, key: &[u8]) -> StorageResult<bool> {
        self.inner.delete(key).await
    }

    async fn exists(&self, key: &[u8]) -> StorageResult<bool> {
        self.inner.exists(key).await
    }

    async fn scan_prefix(&self, prefix: &[u8]) -> StorageResult<Vec<(Vec<u8>, Vec<u8>)>> {
        let results = self.inner.scan_prefix(prefix).await?;
        let mut decrypted_results = Vec::with_capacity(results.len());

        for (key, stored) in results {
            let crypto = self.select_crypto(&key).await;
            let plaintext = self.decrypt_or_passthrough(stored, crypto.as_ref()).await?;
            decrypted_results.push((key, plaintext));
        }

        Ok(decrypted_results)
    }

    async fn batch_put(&self, items: Vec<(Vec<u8>, Vec<u8>)>) -> StorageResult<()> {
        let mut encrypted_items = Vec::with_capacity(items.len());

        for (key, value) in items {
            let crypto = self.select_crypto(&key).await;
            let ciphertext = crypto
                .encrypt(&value)
                .await
                .map_err(|e| StorageError::EncryptionError(e.to_string()))?;
            encrypted_items.push((key, Self::encode_ciphertext(&ciphertext)));
        }

        self.inner.batch_put(encrypted_items).await
    }

    async fn batch_delete(&self, keys: Vec<Vec<u8>>) -> StorageResult<()> {
        self.inner.batch_delete(keys).await
    }

    async fn flush(&self) -> StorageResult<()> {
        self.inner.flush().await
    }

    fn backend_name(&self) -> &'static str {
        "encrypting"
    }

    fn execution_model(&self) -> ExecutionModel {
        self.inner.execution_model()
    }

    fn flush_behavior(&self) -> FlushBehavior {
        self.inner.flush_behavior()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::crypto::provider::{LocalCryptoProvider, NoOpCryptoProvider};
    use crate::storage::inmemory_backend::InMemoryNamespacedStore;
    use crate::storage::traits::NamespacedStore;

    /// Helper to get a fresh in-memory KvStore
    async fn memory_store() -> Arc<dyn KvStore> {
        let ns = InMemoryNamespacedStore::new();
        ns.open_namespace("test").await.unwrap()
    }

    #[tokio::test]
    async fn test_noop_passthrough() {
        let inner = memory_store().await;
        let provider = Arc::new(NoOpCryptoProvider);
        let store = EncryptingKvStore::new(inner, provider, false);

        let key = b"atom:123";
        let value = b"{\"name\": \"test\"}".to_vec();

        store.put(key, value.clone()).await.unwrap();
        let retrieved = store.get(key).await.unwrap().unwrap();
        assert_eq!(retrieved, value);
    }

    #[tokio::test]
    async fn test_encrypted_roundtrip() {
        let inner = memory_store().await;
        let provider = Arc::new(LocalCryptoProvider::from_key([0x42u8; 32]));
        let store = EncryptingKvStore::new(inner.clone(), provider, false);

        let key = b"atom:456";
        let value = b"{\"content\": \"sensitive\"}".to_vec();

        store.put(key, value.clone()).await.unwrap();

        // Verify the raw data in the inner store is encrypted (not plaintext)
        let raw = inner.get(key).await.unwrap().unwrap();
        assert_ne!(raw, value, "Raw data should be encrypted");

        // But reading through the encrypting store gives us plaintext
        let retrieved = store.get(key).await.unwrap().unwrap();
        assert_eq!(retrieved, value);
    }

    #[tokio::test]
    async fn test_batch_roundtrip() {
        let inner = memory_store().await;
        let provider = Arc::new(LocalCryptoProvider::from_key([0x42u8; 32]));
        let store = EncryptingKvStore::new(inner, provider, false);

        let items: Vec<(Vec<u8>, Vec<u8>)> = vec![
            (b"k1".to_vec(), b"value1".to_vec()),
            (b"k2".to_vec(), b"value2".to_vec()),
            (b"k3".to_vec(), b"value3".to_vec()),
        ];

        store.batch_put(items.clone()).await.unwrap();

        for (key, value) in &items {
            let retrieved = store.get(key).await.unwrap().unwrap();
            assert_eq!(&retrieved, value);
        }
    }

    #[tokio::test]
    async fn test_scan_prefix_decrypts() {
        let inner = memory_store().await;
        let provider = Arc::new(LocalCryptoProvider::from_key([0x42u8; 32]));
        let store = EncryptingKvStore::new(inner, provider, false);

        store.put(b"atom:a", b"val_a".to_vec()).await.unwrap();
        store.put(b"atom:b", b"val_b".to_vec()).await.unwrap();
        store.put(b"other:c", b"val_c".to_vec()).await.unwrap();

        let results = store.scan_prefix(b"atom:").await.unwrap();
        assert_eq!(results.len(), 2);

        for (_, value) in &results {
            assert!(value == b"val_a" || value == b"val_b");
        }
    }

    #[tokio::test]
    async fn test_migration_mode_plaintext_fallback() {
        let inner = memory_store().await;
        let provider = Arc::new(LocalCryptoProvider::from_key([0x42u8; 32]));

        // Write plaintext data directly to inner store (simulating pre-migration)
        let key = b"legacy:atom";
        let plaintext = b"{\"old\": \"data\"}".to_vec();
        inner.put(key, plaintext.clone()).await.unwrap();

        // Read with migration_mode=true — should succeed with plaintext fallback
        let store = EncryptingKvStore::new(inner.clone(), provider, true);
        let retrieved = store.get(key).await.unwrap().unwrap();
        assert_eq!(retrieved, plaintext);
    }

    #[tokio::test]
    async fn test_strict_mode_rejects_plaintext() {
        let inner = memory_store().await;
        let provider = Arc::new(LocalCryptoProvider::from_key([0x42u8; 32]));

        // Write plaintext data directly to inner store
        let key = b"legacy:atom";
        let plaintext = b"{\"old\": \"data\"}".to_vec();
        inner.put(key, plaintext).await.unwrap();

        // Read with migration_mode=false — should fail
        let store = EncryptingKvStore::new(inner, provider, false);
        let result = store.get(key).await;
        assert!(result.is_err());
    }

    #[tokio::test]
    async fn test_delete_passthrough() {
        let inner = memory_store().await;
        let provider = Arc::new(NoOpCryptoProvider);
        let store = EncryptingKvStore::new(inner, provider, false);

        store.put(b"k", b"v".to_vec()).await.unwrap();
        assert!(store.exists(b"k").await.unwrap());

        store.delete(b"k").await.unwrap();
        assert!(!store.exists(b"k").await.unwrap());
    }
}
