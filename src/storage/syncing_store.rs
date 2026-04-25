use super::error::StorageResult;
use super::traits::{ExecutionModel, FlushBehavior, KvStore};
use crate::sync::SyncEngine;
use async_trait::async_trait;
use std::sync::Arc;

/// A KvStore decorator that records all write operations to the SyncEngine.
///
/// Sits between EncryptingKvStore and the storage backend:
///
/// ```text
/// TypedKvStore (JSON serialization)
///       ↓
/// EncryptingKvStore (AES-256-GCM)
///       ↓
/// SyncingKvStore (records ops for S3 sync)  ← THIS
///       ↓
/// SledKvStore (actual persistence)
/// ```
///
/// Write operations are recorded non-blocking — the sync engine queues them
/// and uploads on its timer cycle. Reads pass through directly.
pub struct SyncingKvStore {
    inner: Arc<dyn KvStore>,
    sync_engine: Arc<SyncEngine>,
    namespace: String,
}

impl SyncingKvStore {
    pub fn new(inner: Arc<dyn KvStore>, sync_engine: Arc<SyncEngine>, namespace: String) -> Self {
        Self {
            inner,
            sync_engine,
            namespace,
        }
    }
}

#[async_trait]
impl KvStore for SyncingKvStore {
    async fn get(&self, key: &[u8]) -> StorageResult<Option<Vec<u8>>> {
        self.inner.get(key).await
    }

    async fn put(&self, key: &[u8], value: Vec<u8>) -> StorageResult<()> {
        // Write to backend first
        self.inner.put(key, value.clone()).await?;
        // Record for sync (non-blocking, just queues)
        self.sync_engine
            .record_put(&self.namespace, key, &value)
            .await;
        Ok(())
    }

    async fn delete(&self, key: &[u8]) -> StorageResult<bool> {
        let existed = self.inner.delete(key).await?;
        if existed {
            self.sync_engine.record_delete(&self.namespace, key).await;
        }
        Ok(existed)
    }

    async fn exists(&self, key: &[u8]) -> StorageResult<bool> {
        self.inner.exists(key).await
    }

    async fn scan_prefix(&self, prefix: &[u8]) -> StorageResult<Vec<(Vec<u8>, Vec<u8>)>> {
        self.inner.scan_prefix(prefix).await
    }

    async fn batch_put(&self, items: Vec<(Vec<u8>, Vec<u8>)>) -> StorageResult<()> {
        // Write to backend first
        self.inner.batch_put(items.clone()).await?;
        // Record for sync
        self.sync_engine
            .record_batch_put(&self.namespace, &items)
            .await;
        Ok(())
    }

    async fn batch_delete(&self, keys: Vec<Vec<u8>>) -> StorageResult<()> {
        self.inner.batch_delete(keys.clone()).await?;
        self.sync_engine
            .record_batch_delete(&self.namespace, &keys)
            .await;
        Ok(())
    }

    async fn flush(&self) -> StorageResult<()> {
        self.inner.flush().await
    }

    fn backend_name(&self) -> &'static str {
        "syncing"
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
    use crate::crypto::provider::LocalCryptoProvider;
    use crate::storage::inmemory_backend::InMemoryNamespacedStore;
    use crate::storage::traits::NamespacedStore;
    use crate::sync::auth::{AuthClient, SyncAuth};
    use crate::sync::s3::S3Client;
    use crate::sync::SyncConfig;

    async fn test_store_with_sync() -> (Arc<dyn KvStore>, Arc<SyncEngine>) {
        let ns_store = InMemoryNamespacedStore::new();
        let inner = ns_store.open_namespace("main").await.unwrap();

        let crypto: Arc<dyn crate::crypto::CryptoProvider> =
            Arc::new(LocalCryptoProvider::from_key([0x42u8; 32]));
        let http = Arc::new(reqwest::Client::new());
        let s3 = S3Client::new(http.clone());
        let auth = AuthClient::new(
            http,
            "http://localhost:0".to_string(),
            SyncAuth::ApiKey("test".to_string()),
        );
        let ns_store_arc: Arc<dyn NamespacedStore> = Arc::new(ns_store);

        let signer = Arc::new(crate::security::Ed25519KeyPair::generate().unwrap());
        let engine = Arc::new(SyncEngine::new(
            "test-device".to_string(),
            crypto,
            s3,
            auth,
            ns_store_arc,
            SyncConfig::default(),
            signer,
        ));

        let syncing = Arc::new(SyncingKvStore::new(
            inner,
            engine.clone(),
            "main".to_string(),
        ));

        (syncing, engine)
    }

    #[tokio::test]
    async fn put_records_to_sync_engine() {
        let (store, engine) = test_store_with_sync().await;

        assert_eq!(engine.pending_count().await, 0);

        store.put(b"key1", b"val1".to_vec()).await.unwrap();
        assert_eq!(engine.pending_count().await, 1);

        store.put(b"key2", b"val2".to_vec()).await.unwrap();
        assert_eq!(engine.pending_count().await, 2);

        // Verify data is still readable
        let val = store.get(b"key1").await.unwrap();
        assert_eq!(val, Some(b"val1".to_vec()));
    }

    #[tokio::test]
    async fn delete_records_to_sync_engine() {
        let (store, engine) = test_store_with_sync().await;

        store.put(b"key1", b"val1".to_vec()).await.unwrap();
        assert_eq!(engine.pending_count().await, 1);

        store.delete(b"key1").await.unwrap();
        assert_eq!(engine.pending_count().await, 2); // put + delete

        let val = store.get(b"key1").await.unwrap();
        assert_eq!(val, None);
    }

    #[tokio::test]
    async fn batch_put_records_to_sync_engine() {
        let (store, engine) = test_store_with_sync().await;

        let items = vec![
            (b"k1".to_vec(), b"v1".to_vec()),
            (b"k2".to_vec(), b"v2".to_vec()),
        ];

        store.batch_put(items).await.unwrap();
        assert_eq!(engine.pending_count().await, 1); // one batch op

        let val = store.get(b"k1").await.unwrap();
        assert_eq!(val, Some(b"v1".to_vec()));
    }

    #[tokio::test]
    async fn reads_dont_record() {
        let (store, engine) = test_store_with_sync().await;

        store.put(b"key1", b"val1".to_vec()).await.unwrap();
        assert_eq!(engine.pending_count().await, 1);

        // These should not add to pending
        store.get(b"key1").await.unwrap();
        store.exists(b"key1").await.unwrap();
        store.scan_prefix(b"key").await.unwrap();

        assert_eq!(engine.pending_count().await, 1);
    }

    #[tokio::test]
    async fn state_transitions_to_dirty() {
        let (store, engine) = test_store_with_sync().await;

        assert_eq!(engine.state().await, crate::sync::SyncState::Idle);

        store.put(b"key1", b"val1".to_vec()).await.unwrap();

        assert_eq!(engine.state().await, crate::sync::SyncState::Dirty);
    }
}
