use super::error::StorageResult;
use super::syncing_store::SyncingKvStore;
use super::traits::{KvStore, NamespacedStore};
use crate::sync::SyncEngine;
use async_trait::async_trait;
use std::sync::Arc;

/// A NamespacedStore decorator that wraps each opened namespace with a SyncingKvStore.
///
/// Every namespace opened through this store will automatically record
/// write operations to the SyncEngine for S3 sync.
///
/// ```text
/// SyncingNamespacedStore
///   └── open_namespace("main")
///         └── SyncingKvStore("main")  ← records ops
///               └── inner KvStore     ← actual backend
/// ```
pub struct SyncingNamespacedStore {
    inner: Arc<dyn NamespacedStore>,
    sync_engine: Arc<SyncEngine>,
}

impl SyncingNamespacedStore {
    pub fn new(
        inner: Arc<dyn NamespacedStore>,
        sync_engine: Arc<SyncEngine>,
    ) -> Self {
        Self {
            inner,
            sync_engine,
        }
    }
}

#[async_trait]
impl NamespacedStore for SyncingNamespacedStore {
    async fn open_namespace(&self, name: &str) -> StorageResult<Arc<dyn KvStore>> {
        let inner_kv = self.inner.open_namespace(name).await?;
        let syncing = SyncingKvStore::new(
            inner_kv,
            self.sync_engine.clone(),
            name.to_string(),
        );
        Ok(Arc::new(syncing))
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
    use crate::crypto::provider::LocalCryptoProvider;
    use crate::storage::inmemory_backend::InMemoryNamespacedStore;
    use crate::sync::auth::{AuthClient, SyncAuth};
    use crate::sync::s3::S3Client;
    use crate::sync::SyncConfig;

    async fn test_setup() -> (SyncingNamespacedStore, Arc<SyncEngine>) {
        let inner = Arc::new(InMemoryNamespacedStore::new());
        let crypto: Arc<dyn crate::crypto::CryptoProvider> =
            Arc::new(LocalCryptoProvider::from_key([0x42u8; 32]));
        let http = Arc::new(reqwest::Client::new());
        let s3 = S3Client::new(http.clone());
        let auth = AuthClient::new(
            http,
            "http://localhost:0".to_string(),
            SyncAuth::ApiKey("test".to_string()),
        );

        let engine = Arc::new(SyncEngine::new(
            "test-device".to_string(),
            crypto,
            s3,
            auth,
            inner.clone(),
            SyncConfig::default(),
        ));

        let syncing = SyncingNamespacedStore::new(inner, engine.clone());
        (syncing, engine)
    }

    #[tokio::test]
    async fn writes_through_namespace_are_recorded() {
        let (store, engine) = test_setup().await;

        let main = store.open_namespace("main").await.unwrap();
        assert_eq!(engine.pending_count().await, 0);

        main.put(b"atom:1", b"data1".to_vec()).await.unwrap();
        assert_eq!(engine.pending_count().await, 1);

        let meta = store.open_namespace("metadata").await.unwrap();
        meta.put(b"schema:foo", b"schema".to_vec()).await.unwrap();
        assert_eq!(engine.pending_count().await, 2);
    }

    #[tokio::test]
    async fn list_namespaces_passthrough() {
        let (store, _engine) = test_setup().await;

        // Open some namespaces
        store.open_namespace("main").await.unwrap();
        store.open_namespace("metadata").await.unwrap();

        let names = store.list_namespaces().await.unwrap();
        assert!(names.contains(&"main".to_string()));
        assert!(names.contains(&"metadata".to_string()));
    }
}
