use super::error::{StorageError, StorageResult};
use super::traits::{KvStore, NamespacedStore};
use async_trait::async_trait;
use sled::{Db, Tree};
use std::sync::Arc;

/// Sled-backed KvStore implementation
/// 
/// Note: In Sled, both `Db` and `Tree` implement the same interface.
/// `Db` is actually a type alias for `Tree` in Sled's API.
pub struct SledKvStore {
    tree: Tree,
}

impl SledKvStore {
    pub fn new(tree: Tree) -> Self {
        Self { tree }
    }
}

#[async_trait]
impl KvStore for SledKvStore {
    async fn get(&self, key: &[u8]) -> StorageResult<Option<Vec<u8>>> {
        let tree = self.tree.clone();
        let key = key.to_vec();
        
        tokio::task::spawn_blocking(move || {
            tree.get(&key)
                .map_err(|e| StorageError::SledError(e.to_string()))?
                .map(|ivec| Ok(ivec.to_vec()))
                .transpose()
        })
        .await
        .map_err(|e| StorageError::BackendError(e.to_string()))?
    }
    
    async fn put(&self, key: &[u8], value: Vec<u8>) -> StorageResult<()> {
        let tree = self.tree.clone();
        let key = key.to_vec();
        
        tokio::task::spawn_blocking({
            let tree_clone = tree.clone();
            move || -> Result<(), StorageError> {
                tree_clone
                    .insert(&key, value)
                    .map_err(|e| StorageError::SledError(e.to_string()))?;
                Ok(())
            }
        })
        .await
        .map_err(|e| StorageError::BackendError(e.to_string()))??;
        
        // Flush after put (wrapped in async)
        self.flush().await?;
        Ok(())
    }
    
    async fn delete(&self, key: &[u8]) -> StorageResult<bool> {
        let tree = self.tree.clone();
        let key = key.to_vec();
        
        tokio::task::spawn_blocking(move || {
            let existed = tree
                .remove(&key)
                .map_err(|e| StorageError::SledError(e.to_string()))?
                .is_some();
            Ok(existed)
        })
        .await
        .map_err(|e| StorageError::BackendError(e.to_string()))?
    }
    
    async fn exists(&self, key: &[u8]) -> StorageResult<bool> {
        let tree = self.tree.clone();
        let key = key.to_vec();
        
        tokio::task::spawn_blocking(move || {
            tree.contains_key(&key)
                .map_err(|e| StorageError::SledError(e.to_string()))
        })
        .await
        .map_err(|e| StorageError::BackendError(e.to_string()))?
    }
    
    async fn scan_prefix(&self, prefix: &[u8]) -> StorageResult<Vec<(Vec<u8>, Vec<u8>)>> {
        let tree = self.tree.clone();
        let prefix = prefix.to_vec();
        
        tokio::task::spawn_blocking(move || {
            tree.scan_prefix(&prefix)
                .map(|result| {
                    result.map(|(k, v)| (k.to_vec(), v.to_vec()))
                        .map_err(|e| StorageError::SledError(e.to_string()))
                })
                .collect()
        })
        .await
        .map_err(|e| StorageError::BackendError(e.to_string()))?
    }
    
    async fn batch_put(&self, items: Vec<(Vec<u8>, Vec<u8>)>) -> StorageResult<()> {
        let tree = self.tree.clone();
        
        tokio::task::spawn_blocking({
            let tree_clone = tree.clone();
            move || -> Result<(), StorageError> {
                let mut batch = sled::Batch::default();
                for (key, value) in items {
                    batch.insert(key, value);
                }
                
                tree_clone
                    .apply_batch(batch)
                    .map_err(|e| StorageError::SledError(e.to_string()))?;
                Ok(())
            }
        })
        .await
        .map_err(|e| StorageError::BackendError(e.to_string()))??;
        
        // Flush after batch put
        self.flush().await?;
        Ok(())
    }
    
    async fn batch_delete(&self, keys: Vec<Vec<u8>>) -> StorageResult<()> {
        let tree = self.tree.clone();
        
        tokio::task::spawn_blocking(move || {
            let mut batch = sled::Batch::default();
            for key in keys {
                batch.remove(key);
            }
            
            tree.apply_batch(batch)
                .map_err(|e| StorageError::SledError(e.to_string()))?;
            Ok(())
        })
        .await
        .map_err(|e| StorageError::BackendError(e.to_string()))?
    }
    
    async fn flush(&self) -> StorageResult<()> {
        let tree = self.tree.clone();
        
        tokio::task::spawn_blocking(move || -> Result<(), StorageError> {
            tree.flush()
                .map_err(|e| StorageError::SledError(e.to_string()))?;
            Ok(())
        })
        .await
        .map_err(|e| StorageError::BackendError(e.to_string()))?
    }
    
    fn backend_name(&self) -> &'static str {
        "sled"
    }
    
    fn execution_model(&self) -> super::traits::ExecutionModel {
        // Sled is sync but will be wrapped in async via spawn_blocking
        super::traits::ExecutionModel::SyncWrapped
    }
    
    fn flush_behavior(&self) -> super::traits::FlushBehavior {
        // Sled flush performs actual disk write
        super::traits::FlushBehavior::Persists
    }
}

/// Sled-backed NamespacedStore implementation
pub struct SledNamespacedStore {
    db: Db,
}

impl SledNamespacedStore {
    pub fn new(db: Db) -> Self {
        Self { db }
    }
    
    pub fn open(path: impl AsRef<std::path::Path>) -> StorageResult<Self> {
        let db = sled::open(path)
            .map_err(|e| StorageError::SledError(e.to_string()))?;
        Ok(Self { db })
    }
    
    /// Get access to the underlying sled database
    pub fn db(&self) -> &Db {
        &self.db
    }
}

#[async_trait]
impl NamespacedStore for SledNamespacedStore {
    async fn open_namespace(&self, name: &str) -> StorageResult<Arc<dyn KvStore>> {
        let db = self.db.clone();
        let name = name.to_string();
        
        tokio::task::spawn_blocking(move || {
            let tree = db
                .open_tree(&name)
                .map_err(|e| StorageError::SledError(e.to_string()))?;
            Ok(Arc::new(SledKvStore::new(tree)) as Arc<dyn KvStore>)
        })
        .await
        .map_err(|e| StorageError::BackendError(e.to_string()))?
    }
    
    async fn list_namespaces(&self) -> StorageResult<Vec<String>> {
        let db = self.db.clone();
        
        tokio::task::spawn_blocking(move || {
            let tree_names = db
                .tree_names()
                .into_iter()
                .map(|name| String::from_utf8_lossy(&name).to_string())
                .collect();
            Ok(tree_names)
        })
        .await
        .map_err(|e| StorageError::BackendError(e.to_string()))?
    }
    
    async fn delete_namespace(&self, name: &str) -> StorageResult<bool> {
        let db = self.db.clone();
        let name = name.to_string();
        
        tokio::task::spawn_blocking(move || {
            db.drop_tree(&name)
                .map_err(|e| StorageError::SledError(e.to_string()))
        })
        .await
        .map_err(|e| StorageError::BackendError(e.to_string()))?
    }
}
