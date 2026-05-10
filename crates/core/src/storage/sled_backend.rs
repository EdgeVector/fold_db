use super::error::{StorageError, StorageResult};
use super::sled_pool::SledPool;
use super::traits::{KvStore, NamespacedStore};
use async_trait::async_trait;
use std::sync::Arc;

/// Sled-backed KvStore implementation using SledPool for on-demand locking.
///
/// Each operation acquires the Sled handle from the pool, opens the tree,
/// performs the operation, and drops the guard. The file lock is only held
/// during active operations.
pub struct SledKvStore {
    pool: Arc<SledPool>,
    tree_name: String,
}

impl SledKvStore {
    pub fn new(pool: Arc<SledPool>, tree_name: String) -> Self {
        Self { pool, tree_name }
    }
}

#[async_trait]
impl KvStore for SledKvStore {
    async fn get(&self, key: &[u8]) -> StorageResult<Option<Vec<u8>>> {
        let pool = Arc::clone(&self.pool);
        let tree_name = self.tree_name.clone();
        let key = key.to_vec();

        tokio::task::spawn_blocking(move || {
            let guard = pool.acquire_arc()?;
            let tree = guard.db().open_tree(&tree_name)?;
            tree.get(&key)?.map(|ivec| Ok(ivec.to_vec())).transpose()
        })
        .await?
    }

    async fn put(&self, key: &[u8], value: Vec<u8>) -> StorageResult<()> {
        let pool = Arc::clone(&self.pool);
        let tree_name = self.tree_name.clone();
        let key = key.to_vec();

        tokio::task::spawn_blocking(move || -> Result<(), StorageError> {
            let guard = pool.acquire_arc()?;
            let tree = guard.db().open_tree(&tree_name)?;
            tree.insert(&key, value)?;
            tree.flush()?;
            Ok(())
        })
        .await?
    }

    async fn delete(&self, key: &[u8]) -> StorageResult<bool> {
        let pool = Arc::clone(&self.pool);
        let tree_name = self.tree_name.clone();
        let key = key.to_vec();

        tokio::task::spawn_blocking(move || {
            let guard = pool.acquire_arc()?;
            let tree = guard.db().open_tree(&tree_name)?;
            let existed = tree.remove(&key)?.is_some();
            Ok(existed)
        })
        .await?
    }

    async fn exists(&self, key: &[u8]) -> StorageResult<bool> {
        let pool = Arc::clone(&self.pool);
        let tree_name = self.tree_name.clone();
        let key = key.to_vec();

        tokio::task::spawn_blocking(move || -> StorageResult<bool> {
            let guard = pool.acquire_arc()?;
            let tree = guard.db().open_tree(&tree_name)?;
            Ok(tree.contains_key(&key)?)
        })
        .await?
    }

    async fn scan_prefix(&self, prefix: &[u8]) -> StorageResult<Vec<(Vec<u8>, Vec<u8>)>> {
        let pool = Arc::clone(&self.pool);
        let tree_name = self.tree_name.clone();
        let prefix = prefix.to_vec();

        tokio::task::spawn_blocking(move || {
            let guard = pool.acquire_arc()?;
            let tree = guard.db().open_tree(&tree_name)?;
            tree.scan_prefix(&prefix)
                .map(|result| -> StorageResult<(Vec<u8>, Vec<u8>)> {
                    let (k, v) = result?;
                    Ok((k.to_vec(), v.to_vec()))
                })
                .collect()
        })
        .await?
    }

    async fn batch_put(&self, items: Vec<(Vec<u8>, Vec<u8>)>) -> StorageResult<()> {
        let pool = Arc::clone(&self.pool);
        let tree_name = self.tree_name.clone();

        tokio::task::spawn_blocking(move || -> Result<(), StorageError> {
            let guard = pool.acquire_arc()?;
            let tree = guard.db().open_tree(&tree_name)?;
            let mut batch = sled::Batch::default();
            for (key, value) in items {
                batch.insert(key, value);
            }
            tree.apply_batch(batch)?;
            tree.flush()?;
            Ok(())
        })
        .await?
    }

    async fn batch_delete(&self, keys: Vec<Vec<u8>>) -> StorageResult<()> {
        let pool = Arc::clone(&self.pool);
        let tree_name = self.tree_name.clone();

        tokio::task::spawn_blocking(move || {
            let guard = pool.acquire_arc()?;
            let tree = guard.db().open_tree(&tree_name)?;
            let mut batch = sled::Batch::default();
            for key in keys {
                batch.remove(key);
            }
            tree.apply_batch(batch)?;
            Ok(())
        })
        .await?
    }

    async fn flush(&self) -> StorageResult<()> {
        let pool = Arc::clone(&self.pool);
        let tree_name = self.tree_name.clone();

        tokio::task::spawn_blocking(move || -> Result<(), StorageError> {
            let guard = pool.acquire_arc()?;
            let tree = guard.db().open_tree(&tree_name)?;
            tree.flush()?;
            Ok(())
        })
        .await?
    }

    fn backend_name(&self) -> &'static str {
        "sled"
    }

    fn execution_model(&self) -> super::traits::ExecutionModel {
        super::traits::ExecutionModel::SyncWrapped
    }

    fn flush_behavior(&self) -> super::traits::FlushBehavior {
        super::traits::FlushBehavior::Persists
    }
}

/// Sled-backed NamespacedStore implementation using SledPool.
pub struct SledNamespacedStore {
    pool: Arc<SledPool>,
}

impl SledNamespacedStore {
    pub fn new(pool: Arc<SledPool>) -> Self {
        Self { pool }
    }

    /// Convenience constructor that creates a SledPool from a path.
    pub fn open(path: &std::path::Path) -> StorageResult<Self> {
        let pool = Arc::new(SledPool::new(path.to_path_buf()));
        // Validate we can actually open the database
        let _guard = pool.acquire_arc()?;
        Ok(Self { pool })
    }

    /// Get access to the underlying pool.
    pub fn pool(&self) -> &Arc<SledPool> {
        &self.pool
    }
}

#[async_trait]
impl NamespacedStore for SledNamespacedStore {
    async fn open_namespace(&self, name: &str) -> StorageResult<Arc<dyn KvStore>> {
        Ok(Arc::new(SledKvStore::new(
            Arc::clone(&self.pool),
            name.to_string(),
        )))
    }

    async fn list_namespaces(&self) -> StorageResult<Vec<String>> {
        let pool = Arc::clone(&self.pool);

        tokio::task::spawn_blocking(move || {
            let guard = pool.acquire_arc()?;
            let tree_names = guard
                .db()
                .tree_names()
                .into_iter()
                .map(|name| String::from_utf8_lossy(&name).to_string())
                .collect();
            Ok(tree_names)
        })
        .await?
    }

    async fn delete_namespace(&self, name: &str) -> StorageResult<bool> {
        let pool = Arc::clone(&self.pool);
        let name = name.to_string();

        tokio::task::spawn_blocking(move || -> StorageResult<bool> {
            let guard = pool.acquire_arc()?;
            Ok(guard.db().drop_tree(&name)?)
        })
        .await?
    }
}
