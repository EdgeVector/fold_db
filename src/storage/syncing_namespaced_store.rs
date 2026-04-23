use super::error::StorageResult;
use super::syncing_store::SyncingKvStore;
use super::traits::{KvStore, NamespacedStore};
use crate::sync::SyncEngine;
use async_trait::async_trait;
use std::sync::Arc;

/// Namespaces that must never append to the sync log.
///
/// These hold derived / per-device state that any node can recompute or
/// regenerate locally. Routing them through `SyncingKvStore` would leak
/// non-canonical state into the unified sync log and reintroduce the
/// cache-coherence problem the multi-device transform design avoids
/// (see `docs/design/multi_device_transforms.md`, "What Syncs vs. What Doesn't").
const LOCAL_ONLY_NAMESPACES: &[&str] = &["transform_cache"];

/// A NamespacedStore decorator that wraps each opened namespace with a SyncingKvStore.
///
/// Every namespace opened through this store will automatically record
/// write operations to the SyncEngine for S3 sync, **except** for names
/// in [`LOCAL_ONLY_NAMESPACES`], which are returned unwrapped so their
/// writes stay on-device.
///
/// ```text
/// SyncingNamespacedStore
///   ├── open_namespace("main")
///   │     └── SyncingKvStore("main")  ← records ops
///   │           └── inner KvStore     ← actual backend
///   └── open_namespace("transform_cache")
///         └── inner KvStore           ← no wrapper, never syncs
/// ```
pub struct SyncingNamespacedStore {
    inner: Arc<dyn NamespacedStore>,
    sync_engine: Arc<SyncEngine>,
}

impl SyncingNamespacedStore {
    pub fn new(inner: Arc<dyn NamespacedStore>, sync_engine: Arc<SyncEngine>) -> Self {
        Self { inner, sync_engine }
    }

    /// Returns true if writes to `name` must stay local and never append to the sync log.
    pub fn is_local_only(name: &str) -> bool {
        LOCAL_ONLY_NAMESPACES.contains(&name)
    }
}

#[async_trait]
impl NamespacedStore for SyncingNamespacedStore {
    async fn open_namespace(&self, name: &str) -> StorageResult<Arc<dyn KvStore>> {
        let inner_kv = self.inner.open_namespace(name).await?;
        if Self::is_local_only(name) {
            return Ok(inner_kv);
        }
        let syncing = SyncingKvStore::new(inner_kv, self.sync_engine.clone(), name.to_string());
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

    #[tokio::test]
    async fn transform_cache_writes_are_not_recorded() {
        let (store, engine) = test_setup().await;

        let cache = store.open_namespace("transform_cache").await.unwrap();
        assert_eq!(engine.pending_count().await, 0);

        cache
            .put(b"view:analytics", b"cached-output".to_vec())
            .await
            .unwrap();
        cache
            .put(b"view:other", b"another-output".to_vec())
            .await
            .unwrap();
        cache.delete(b"view:other").await.unwrap();
        cache
            .batch_put(vec![
                (b"view:a".to_vec(), b"va".to_vec()),
                (b"view:b".to_vec(), b"vb".to_vec()),
            ])
            .await
            .unwrap();

        assert_eq!(
            engine.pending_count().await,
            0,
            "transform_cache namespace must never append to the sync log"
        );
        assert_eq!(
            engine.state().await,
            crate::sync::SyncState::Idle,
            "sync engine state must stay Idle after transform_cache writes"
        );
    }

    /// Positive control paired with `transform_cache_writes_are_not_recorded`:
    /// writes to a synced namespace must record, writes to the local-only
    /// namespace must not, under the same engine instance.
    #[tokio::test]
    async fn cache_writes_never_mix_with_synced_writes() {
        let (store, engine) = test_setup().await;

        let main = store.open_namespace("main").await.unwrap();
        let cache = store.open_namespace("transform_cache").await.unwrap();

        // Full cycle: compute → cache → read → recompute → cache again.
        // None of this should enter the sync log.
        for i in 0..5 {
            let key = format!("view:v{}", i);
            cache
                .put(key.as_bytes(), format!("output-{}", i).into_bytes())
                .await
                .unwrap();
            let got = cache.get(key.as_bytes()).await.unwrap();
            assert!(got.is_some());
            cache
                .put(key.as_bytes(), format!("recomputed-{}", i).into_bytes())
                .await
                .unwrap();
        }
        assert_eq!(engine.pending_count().await, 0);

        // A source-data write on the same engine must still record.
        main.put(b"atom:1", b"source-data".to_vec()).await.unwrap();
        assert_eq!(engine.pending_count().await, 1);
    }

    #[test]
    fn is_local_only_classification() {
        assert!(SyncingNamespacedStore::is_local_only("transform_cache"));
        assert!(!SyncingNamespacedStore::is_local_only("main"));
        assert!(!SyncingNamespacedStore::is_local_only("metadata"));
        assert!(!SyncingNamespacedStore::is_local_only("schemas"));
    }
}
