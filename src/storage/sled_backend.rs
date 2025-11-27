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
        self.tree
            .get(key)
            .map_err(|e| StorageError::SledError(e.to_string()))?
            .map(|ivec| Ok(ivec.to_vec()))
            .transpose()
    }
    
    async fn put(&self, key: &[u8], value: Vec<u8>) -> StorageResult<()> {
        self.tree
            .insert(key, value)
            .map_err(|e| StorageError::SledError(e.to_string()))?;
        
        // Sled doesn't auto-flush, so we flush immediately
        self.flush().await?;
        Ok(())
    }
    
    async fn delete(&self, key: &[u8]) -> StorageResult<bool> {
        let existed = self.tree
            .remove(key)
            .map_err(|e| StorageError::SledError(e.to_string()))?
            .is_some();
        Ok(existed)
    }
    
    async fn exists(&self, key: &[u8]) -> StorageResult<bool> {
        self.tree
            .contains_key(key)
            .map_err(|e| StorageError::SledError(e.to_string()))
    }
    
    async fn scan_prefix(&self, prefix: &[u8]) -> StorageResult<Vec<(Vec<u8>, Vec<u8>)>> {
        self.tree
            .scan_prefix(prefix)
            .map(|result| {
                result.map(|(k, v)| (k.to_vec(), v.to_vec()))
                    .map_err(|e| StorageError::SledError(e.to_string()))
            })
            .collect()
    }
    
    async fn batch_put(&self, items: Vec<(Vec<u8>, Vec<u8>)>) -> StorageResult<()> {
        let mut batch = sled::Batch::default();
        for (key, value) in items {
            batch.insert(key, value);
        }
        
        self.tree
            .apply_batch(batch)
            .map_err(|e| StorageError::SledError(e.to_string()))?;
        
        self.flush().await?;
        Ok(())
    }
    
    async fn batch_delete(&self, keys: Vec<Vec<u8>>) -> StorageResult<()> {
        let mut batch = sled::Batch::default();
        for key in keys {
            batch.remove(key);
        }
        
        self.tree
            .apply_batch(batch)
            .map_err(|e| StorageError::SledError(e.to_string()))?;
        
        Ok(())
    }
    
    async fn flush(&self) -> StorageResult<()> {
        self.tree
            .flush()
            .map_err(|e| StorageError::SledError(e.to_string()))?;
        Ok(())
    }
    
    fn backend_name(&self) -> &'static str {
        "sled"
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
        let tree = self.db
            .open_tree(name)
            .map_err(|e| StorageError::SledError(e.to_string()))?;
        
        Ok(Arc::new(SledKvStore::new(tree)))
    }
    
    async fn list_namespaces(&self) -> StorageResult<Vec<String>> {
        let tree_names = self.db
            .tree_names()
            .into_iter()
            .map(|name| String::from_utf8_lossy(&name).to_string())
            .collect();
        Ok(tree_names)
    }
    
    async fn delete_namespace(&self, name: &str) -> StorageResult<bool> {
        self.db
            .drop_tree(name)
            .map_err(|e| StorageError::SledError(e.to_string()))
    }
}
