use super::encrypting_store::EncryptingKvStore;
use super::error::StorageResult;
use super::traits::{KvStore, NamespacedStore};
use crate::crypto::CryptoProvider;
use async_trait::async_trait;
use std::collections::{HashMap, HashSet};
use std::sync::Arc;
use tokio::sync::RwLock;

/// Namespaces that contain personal data and are encrypted via E2E encryption.
///
/// E2E encryption is the primary mechanism — content is encrypted with
/// AES-256-GCM before reaching storage. Only `main` (atom content) and
/// `metadata` (derived personal data) are encrypted. Index terms use
/// HMAC-SHA256 blind tokens instead of encryption (see `E2eKeys::blind_token`).
pub const ENCRYPTED_NAMESPACES: &[&str] = &["main", "metadata"];

/// A decorator over any `NamespacedStore` that conditionally wraps
/// returned `KvStore` instances in `EncryptingKvStore` for sensitive namespaces.
///
/// Namespaces not in the encrypted set are returned unwrapped (plaintext).
pub struct EncryptingNamespacedStore {
    inner: Arc<dyn NamespacedStore>,
    crypto: Arc<dyn CryptoProvider>,
    encrypted_namespaces: HashSet<String>,
    migration_mode: bool,
    /// Per-org crypto providers keyed by org_hash. When a storage key starts
    /// with `{org_hash}:`, the corresponding provider is used instead of the
    /// default `crypto`. Shared with all child `EncryptingKvStore` instances.
    org_crypto: Arc<RwLock<HashMap<String, Arc<dyn CryptoProvider>>>>,
}

impl EncryptingNamespacedStore {
    /// Create a new encrypting namespaced store.
    ///
    /// - `inner`: The underlying namespaced store (Sled, DynamoDB, InMemory).
    /// - `crypto`: The crypto provider to use for encryption/decryption.
    /// - `migration_mode`: When `true`, plaintext reads are permitted (dual-read).
    pub fn new(
        inner: Arc<dyn NamespacedStore>,
        crypto: Arc<dyn CryptoProvider>,
        migration_mode: bool,
    ) -> Self {
        let encrypted_namespaces = ENCRYPTED_NAMESPACES.iter().map(|s| s.to_string()).collect();
        Self {
            inner,
            crypto,
            encrypted_namespaces,
            migration_mode,
            org_crypto: Arc::new(RwLock::new(HashMap::new())),
        }
    }

    /// Create with a custom set of encrypted namespaces (for testing).
    pub fn with_namespaces(
        inner: Arc<dyn NamespacedStore>,
        crypto: Arc<dyn CryptoProvider>,
        namespaces: Vec<String>,
        migration_mode: bool,
    ) -> Self {
        Self {
            inner,
            crypto,
            encrypted_namespaces: namespaces.into_iter().collect(),
            migration_mode,
            org_crypto: Arc::new(RwLock::new(HashMap::new())),
        }
    }

    /// Register a crypto provider for an organization.
    ///
    /// After this call, any storage key starting with `{org_hash}:` in an
    /// encrypted namespace will use this provider instead of the default.
    /// This allows org members to share data encrypted with the org's E2E key.
    pub async fn register_org_crypto(&self, org_hash: String, crypto: Arc<dyn CryptoProvider>) {
        self.org_crypto.write().await.insert(org_hash, crypto);
    }

    /// Remove a crypto provider for an organization (e.g. after leaving).
    pub async fn remove_org_crypto(&self, org_hash: &str) {
        self.org_crypto.write().await.remove(org_hash);
    }

    /// Check if a namespace should be encrypted.
    fn should_encrypt(&self, namespace: &str) -> bool {
        self.encrypted_namespaces.contains(namespace)
    }
}

#[async_trait]
impl NamespacedStore for EncryptingNamespacedStore {
    async fn open_namespace(&self, name: &str) -> StorageResult<Arc<dyn KvStore>> {
        let inner_store = self.inner.open_namespace(name).await?;

        if self.should_encrypt(name) {
            Ok(Arc::new(EncryptingKvStore::with_org_crypto(
                inner_store,
                self.crypto.clone(),
                self.migration_mode,
                self.org_crypto.clone(),
            )))
        } else {
            // Non-sensitive namespaces: pass through without encryption
            Ok(inner_store)
        }
    }

    async fn list_namespaces(&self) -> StorageResult<Vec<String>> {
        self.inner.list_namespaces().await
    }

    async fn delete_namespace(&self, name: &str) -> StorageResult<bool> {
        self.inner.delete_namespace(name).await
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::crypto::{LocalCryptoProvider, NoOpCryptoProvider};
    use crate::storage::inmemory_backend::InMemoryNamespacedStore;

    #[tokio::test]
    async fn test_encrypted_namespace_encrypts_data() {
        let inner = Arc::new(InMemoryNamespacedStore::new());
        let crypto = Arc::new(LocalCryptoProvider::from_key([0x42u8; 32]));
        let enc_ns = EncryptingNamespacedStore::new(inner.clone(), crypto.clone(), false);

        // Write to encrypted namespace
        let main_store = enc_ns.open_namespace("main").await.unwrap();
        main_store
            .put(b"atom:1", b"secret content".to_vec())
            .await
            .unwrap();

        // Read through the encrypted namespace — should get plaintext back
        let retrieved = main_store.get(b"atom:1").await.unwrap().unwrap();
        assert_eq!(retrieved, b"secret content");

        // Verify raw storage is encrypted (read from inner directly)
        let raw_store = inner.open_namespace("main").await.unwrap();
        let raw_data = raw_store.get(b"atom:1").await.unwrap().unwrap();
        assert_ne!(raw_data, b"secret content", "Raw data should be encrypted");
    }

    #[tokio::test]
    async fn test_non_encrypted_namespace_is_plaintext() {
        let inner = Arc::new(InMemoryNamespacedStore::new());
        let crypto = Arc::new(LocalCryptoProvider::from_key([0x42u8; 32]));
        let enc_ns = EncryptingNamespacedStore::new(inner.clone(), crypto, false);

        // Write to non-encrypted namespace
        let schemas_store = enc_ns.open_namespace("schemas").await.unwrap();
        schemas_store
            .put(b"schema:1", b"public schema".to_vec())
            .await
            .unwrap();

        // Verify raw storage is NOT encrypted
        let raw_store = inner.open_namespace("schemas").await.unwrap();
        let raw_data = raw_store.get(b"schema:1").await.unwrap().unwrap();
        assert_eq!(
            raw_data, b"public schema",
            "Non-encrypted namespace should store plaintext"
        );
    }

    #[tokio::test]
    async fn test_default_encrypted_namespaces() {
        let inner = Arc::new(InMemoryNamespacedStore::new());
        let crypto = Arc::new(NoOpCryptoProvider);
        let enc_ns = EncryptingNamespacedStore::new(inner, crypto, false);

        assert!(enc_ns.should_encrypt("main"));
        assert!(enc_ns.should_encrypt("metadata"));
        assert!(!enc_ns.should_encrypt("schemas"));
        assert!(!enc_ns.should_encrypt("native_index"));
        assert!(!enc_ns.should_encrypt("transforms"));
        assert!(!enc_ns.should_encrypt("process"));
    }

    #[tokio::test]
    async fn test_custom_namespaces() {
        let inner = Arc::new(InMemoryNamespacedStore::new());
        let crypto = Arc::new(NoOpCryptoProvider);
        let enc_ns = EncryptingNamespacedStore::with_namespaces(
            inner,
            crypto,
            vec!["main".into(), "native_index".into()],
            false,
        );

        assert!(enc_ns.should_encrypt("main"));
        assert!(enc_ns.should_encrypt("native_index"));
        assert!(!enc_ns.should_encrypt("metadata"));
        assert!(!enc_ns.should_encrypt("schemas"));
    }

    #[tokio::test]
    async fn test_list_and_delete_passthrough() {
        let inner = Arc::new(InMemoryNamespacedStore::new());
        let crypto = Arc::new(NoOpCryptoProvider);
        let enc_ns = EncryptingNamespacedStore::new(inner, crypto, false);

        // Create a namespace through the encrypted store
        let _ = enc_ns.open_namespace("main").await.unwrap();
        let _ = enc_ns.open_namespace("schemas").await.unwrap();

        let namespaces = enc_ns.list_namespaces().await.unwrap();
        assert!(namespaces.contains(&"main".to_string()));
        assert!(namespaces.contains(&"schemas".to_string()));

        // Delete should pass through
        let deleted = enc_ns.delete_namespace("schemas").await.unwrap();
        assert!(deleted);
    }

    #[tokio::test]
    async fn test_migration_mode_reads_legacy_plaintext() {
        let inner = Arc::new(InMemoryNamespacedStore::new());
        let crypto = Arc::new(LocalCryptoProvider::from_key([0x42u8; 32]));

        // Write plaintext directly (simulating pre-migration data)
        let raw_store = inner.open_namespace("main").await.unwrap();
        raw_store
            .put(b"atom:old", b"legacy plaintext".to_vec())
            .await
            .unwrap();

        // Read through encrypted namespace with migration mode
        let enc_ns = EncryptingNamespacedStore::new(inner, crypto, true);
        let main_store = enc_ns.open_namespace("main").await.unwrap();
        let retrieved = main_store.get(b"atom:old").await.unwrap().unwrap();
        assert_eq!(retrieved, b"legacy plaintext");
    }
}
