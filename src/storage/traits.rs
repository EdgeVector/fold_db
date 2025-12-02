use async_trait::async_trait;
use serde::{de::DeserializeOwned, Serialize};
use std::sync::Arc;

use super::error::{StorageError, StorageResult};

/// Describes how the backend executes operations
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ExecutionModel {
    /// Backend is truly async (network I/O, e.g., DynamoDB)
    Async,
    /// Backend is sync but wrapped in async (local I/O, e.g., Sled)
    SyncWrapped,
}

/// Describes flush behavior for the backend
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FlushBehavior {
    /// Flush is a no-op (eventually consistent backend, e.g., DynamoDB)
    NoOp,
    /// Flush performs actual persistence (strongly consistent, e.g., Sled)
    Persists,
}

/// Core key-value storage trait
/// 
/// This is the fundamental storage interface that all backends must implement.
/// Operations are async to support both local (Sled) and remote (DynamoDB) backends.
#[async_trait]
pub trait KvStore: Send + Sync {
    /// Get a value by key
    async fn get(&self, key: &[u8]) -> StorageResult<Option<Vec<u8>>>;
    
    /// Put a key-value pair
    async fn put(&self, key: &[u8], value: Vec<u8>) -> StorageResult<()>;
    
    /// Delete a key
    async fn delete(&self, key: &[u8]) -> StorageResult<bool>;
    
    /// Check if a key exists
    async fn exists(&self, key: &[u8]) -> StorageResult<bool>;
    
    /// Scan keys with a given prefix
    async fn scan_prefix(&self, prefix: &[u8]) -> StorageResult<Vec<(Vec<u8>, Vec<u8>)>>;
    
    /// Batch put operations (atomic if backend supports it)
    async fn batch_put(&self, items: Vec<(Vec<u8>, Vec<u8>)>) -> StorageResult<()>;
    
    /// Batch delete operations
    async fn batch_delete(&self, keys: Vec<Vec<u8>>) -> StorageResult<()>;
    
    /// Flush pending operations to storage (no-op for backends that auto-flush)
    async fn flush(&self) -> StorageResult<()>;
    
    /// Get storage backend name (for debugging/metrics)
    fn backend_name(&self) -> &'static str;
    
    /// Get the execution model of this backend
    /// 
    /// This describes whether the backend is truly async (network I/O)
    /// or sync wrapped in async (local I/O).
    fn execution_model(&self) -> ExecutionModel;
    
    /// Get the flush behavior of this backend
    /// 
    /// This describes whether flush is a no-op (eventually consistent)
    /// or performs actual persistence (strongly consistent).
    fn flush_behavior(&self) -> FlushBehavior;
}

/// Namespace storage trait
///
/// Provides logical separation of data into "namespaces" or "trees"
/// (like Sled's trees, DynamoDB tables, or Postgres schemas)
#[async_trait]
pub trait NamespacedStore: Send + Sync {
    /// Open or create a namespace (like sled::Tree)
    async fn open_namespace(&self, name: &str) -> StorageResult<Arc<dyn KvStore>>;
    
    /// List all namespaces
    async fn list_namespaces(&self) -> StorageResult<Vec<String>>;
    
    /// Delete a namespace and all its data
    async fn delete_namespace(&self, name: &str) -> StorageResult<bool>;
}

/// High-level typed storage operations
///
/// This provides a convenient API on top of KvStore for storing/retrieving
/// typed Rust structs (serialized as JSON or bincode)
#[async_trait]
pub trait TypedStore: Send + Sync {
    /// Store a typed item
    async fn put_item<T: Serialize + Send + Sync>(&self, key: &str, item: &T) -> StorageResult<()>;
    
    /// Get a typed item
    async fn get_item<T: DeserializeOwned + Send + Sync>(&self, key: &str) -> StorageResult<Option<T>>;
    
    /// Delete an item
    async fn delete_item(&self, key: &str) -> StorageResult<bool>;
    
    /// List all keys with a given prefix
    async fn list_keys_with_prefix(&self, prefix: &str) -> StorageResult<Vec<String>>;
    
    /// Get all items with a given prefix
    async fn scan_items_with_prefix<T: DeserializeOwned + Send + Sync>(&self, prefix: &str) -> StorageResult<Vec<(String, T)>>;
    
    /// Batch store items
    async fn batch_put_items<T: Serialize + Send + Sync>(&self, items: Vec<(String, T)>) -> StorageResult<()>;
    
    /// Check if key exists
    async fn exists_item(&self, key: &str) -> StorageResult<bool>;
}

/// Adapter that wraps a KvStore and provides TypedStore functionality
pub struct TypedKvStore<S: KvStore + ?Sized> {
    inner: Arc<S>,
}

impl<S: KvStore + ?Sized> TypedKvStore<S> {
    pub fn new(store: Arc<S>) -> Self {
        Self { inner: store }
    }
    
    /// Get a reference to the underlying KvStore
    pub fn inner(&self) -> &Arc<S> {
        &self.inner
    }
}

#[async_trait]
impl<S: KvStore + ?Sized + 'static> TypedStore for TypedKvStore<S> {
    async fn put_item<T: Serialize + Send + Sync>(&self, key: &str, item: &T) -> StorageResult<()> {
        let bytes = serde_json::to_vec(item)
            .map_err(|e| StorageError::SerializationError(e.to_string()))?;
        self.inner.put(key.as_bytes(), bytes).await
    }
    
    async fn get_item<T: DeserializeOwned + Send + Sync>(&self, key: &str) -> StorageResult<Option<T>> {
        match self.inner.get(key.as_bytes()).await? {
            Some(bytes) => {
                let item = serde_json::from_slice(&bytes)
                    .map_err(|e| StorageError::SerializationError(e.to_string()))?;
                Ok(Some(item))
            }
            None => Ok(None),
        }
    }
    
    async fn delete_item(&self, key: &str) -> StorageResult<bool> {
        self.inner.delete(key.as_bytes()).await
    }
    
    async fn list_keys_with_prefix(&self, prefix: &str) -> StorageResult<Vec<String>> {
        let results = self.inner.scan_prefix(prefix.as_bytes()).await?;
        Ok(results.into_iter()
            .map(|(k, _)| String::from_utf8_lossy(&k).to_string())
            .collect())
    }
    
    async fn scan_items_with_prefix<T: DeserializeOwned + Send + Sync>(&self, prefix: &str) -> StorageResult<Vec<(String, T)>> {
        let results = self.inner.scan_prefix(prefix.as_bytes()).await?;
        let mut items = Vec::new();
        
        for (key_bytes, value_bytes) in results {
            let key = String::from_utf8_lossy(&key_bytes).to_string();
            let value = serde_json::from_slice(&value_bytes)
                .map_err(|e| StorageError::SerializationError(format!("Failed to deserialize {}: {}", key, e)))?;
            items.push((key, value));
        }
        
        Ok(items)
    }
    
    async fn batch_put_items<T: Serialize + Send + Sync>(&self, items: Vec<(String, T)>) -> StorageResult<()> {
        let serialized: Result<Vec<_>, StorageError> = items.into_iter()
            .map(|(k, v)| {
                let bytes = serde_json::to_vec(&v)
                    .map_err(|e| StorageError::SerializationError(e.to_string()))?;
                Ok::<(Vec<u8>, Vec<u8>), StorageError>((k.into_bytes(), bytes))
            })
            .collect();
        
        self.inner.batch_put(serialized?).await
    }
    
    async fn exists_item(&self, key: &str) -> StorageResult<bool> {
        self.inner.exists(key.as_bytes()).await
    }
}
