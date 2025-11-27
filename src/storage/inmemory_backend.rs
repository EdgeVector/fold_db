use super::error::{StorageError, StorageResult};
use super::traits::{KvStore, NamespacedStore};
use async_trait::async_trait;
use std::collections::HashMap;
use std::sync::{Arc, RwLock};

/// In-memory KvStore implementation for testing
#[derive(Clone)]
pub struct InMemoryKvStore {
    data: Arc<RwLock<HashMap<Vec<u8>, Vec<u8>>>>,
}

impl InMemoryKvStore {
    pub fn new() -> Self {
        Self {
            data: Arc::new(RwLock::new(HashMap::new())),
        }
    }
}

impl Default for InMemoryKvStore {
    fn default() -> Self {
        Self::new()
    }
}

#[async_trait]
impl KvStore for InMemoryKvStore {
    async fn get(&self, key: &[u8]) -> StorageResult<Option<Vec<u8>>> {
        let data = self.data.read()
            .map_err(|e| StorageError::BackendError(format!("Lock poisoned: {}", e)))?;
        Ok(data.get(key).cloned())
    }
    
    async fn put(&self, key: &[u8], value: Vec<u8>) -> StorageResult<()> {
        let mut data = self.data.write()
            .map_err(|e| StorageError::BackendError(format!("Lock poisoned: {}", e)))?;
        data.insert(key.to_vec(), value);
        Ok(())
    }
    
    async fn delete(&self, key: &[u8]) -> StorageResult<bool> {
        let mut data = self.data.write()
            .map_err(|e| StorageError::BackendError(format!("Lock poisoned: {}", e)))?;
        Ok(data.remove(key).is_some())
    }
    
    async fn exists(&self, key: &[u8]) -> StorageResult<bool> {
        let data = self.data.read()
            .map_err(|e| StorageError::BackendError(format!("Lock poisoned: {}", e)))?;
        Ok(data.contains_key(key))
    }
    
    async fn scan_prefix(&self, prefix: &[u8]) -> StorageResult<Vec<(Vec<u8>, Vec<u8>)>> {
        let data = self.data.read()
            .map_err(|e| StorageError::BackendError(format!("Lock poisoned: {}", e)))?;
        
        let results: Vec<_> = data
            .iter()
            .filter(|(k, _)| k.starts_with(prefix))
            .map(|(k, v)| (k.clone(), v.clone()))
            .collect();
        
        Ok(results)
    }
    
    async fn batch_put(&self, items: Vec<(Vec<u8>, Vec<u8>)>) -> StorageResult<()> {
        let mut data = self.data.write()
            .map_err(|e| StorageError::BackendError(format!("Lock poisoned: {}", e)))?;
        
        for (key, value) in items {
            data.insert(key, value);
        }
        
        Ok(())
    }
    
    async fn batch_delete(&self, keys: Vec<Vec<u8>>) -> StorageResult<()> {
        let mut data = self.data.write()
            .map_err(|e| StorageError::BackendError(format!("Lock poisoned: {}", e)))?;
        
        for key in keys {
            data.remove(&key);
        }
        
        Ok(())
    }
    
    async fn flush(&self) -> StorageResult<()> {
        // In-memory storage doesn't need flushing
        Ok(())
    }
    
    fn backend_name(&self) -> &'static str {
        "in-memory"
    }
}

/// In-memory NamespacedStore for testing
#[derive(Clone)]
pub struct InMemoryNamespacedStore {
    namespaces: Arc<RwLock<HashMap<String, Arc<InMemoryKvStore>>>>,
}

impl InMemoryNamespacedStore {
    pub fn new() -> Self {
        Self {
            namespaces: Arc::new(RwLock::new(HashMap::new())),
        }
    }
}

impl Default for InMemoryNamespacedStore {
    fn default() -> Self {
        Self::new()
    }
}

#[async_trait]
impl NamespacedStore for InMemoryNamespacedStore {
    async fn open_namespace(&self, name: &str) -> StorageResult<Arc<dyn KvStore>> {
        let mut namespaces = self.namespaces.write()
            .map_err(|e| StorageError::BackendError(format!("Lock poisoned: {}", e)))?;
        
        let store = namespaces
            .entry(name.to_string())
            .or_insert_with(|| Arc::new(InMemoryKvStore::new()))
            .clone();
        
        Ok(store as Arc<dyn KvStore>)
    }
    
    async fn list_namespaces(&self) -> StorageResult<Vec<String>> {
        let namespaces = self.namespaces.read()
            .map_err(|e| StorageError::BackendError(format!("Lock poisoned: {}", e)))?;
        
        Ok(namespaces.keys().cloned().collect())
    }
    
    async fn delete_namespace(&self, name: &str) -> StorageResult<bool> {
        let mut namespaces = self.namespaces.write()
            .map_err(|e| StorageError::BackendError(format!("Lock poisoned: {}", e)))?;
        
        Ok(namespaces.remove(name).is_some())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_inmemory_basic_operations() {
        let store = InMemoryKvStore::new();
        
        // Test put and get
        store.put(b"key1", b"value1".to_vec()).await.unwrap();
        let value = store.get(b"key1").await.unwrap();
        assert_eq!(value, Some(b"value1".to_vec()));
        
        // Test exists
        assert!(store.exists(b"key1").await.unwrap());
        assert!(!store.exists(b"key2").await.unwrap());
        
        // Test delete
        assert!(store.delete(b"key1").await.unwrap());
        assert!(!store.exists(b"key1").await.unwrap());
    }
    
    #[tokio::test]
    async fn test_inmemory_scan_prefix() {
        let store = InMemoryKvStore::new();
        
        store.put(b"prefix:key1", b"value1".to_vec()).await.unwrap();
        store.put(b"prefix:key2", b"value2".to_vec()).await.unwrap();
        store.put(b"other:key3", b"value3".to_vec()).await.unwrap();
        
        let results = store.scan_prefix(b"prefix:").await.unwrap();
        assert_eq!(results.len(), 2);
    }
    
    #[tokio::test]
    async fn test_inmemory_batch_operations() {
        let store = InMemoryKvStore::new();
        
        let items = vec![
            (b"key1".to_vec(), b"value1".to_vec()),
            (b"key2".to_vec(), b"value2".to_vec()),
        ];
        
        store.batch_put(items).await.unwrap();
        
        assert!(store.exists(b"key1").await.unwrap());
        assert!(store.exists(b"key2").await.unwrap());
        
        store.batch_delete(vec![b"key1".to_vec(), b"key2".to_vec()]).await.unwrap();
        
        assert!(!store.exists(b"key1").await.unwrap());
        assert!(!store.exists(b"key2").await.unwrap());
    }
    
    #[tokio::test]
    async fn test_namespaced_store() {
        let store = InMemoryNamespacedStore::new();
        
        let ns1 = store.open_namespace("namespace1").await.unwrap();
        let ns2 = store.open_namespace("namespace2").await.unwrap();
        
        ns1.put(b"key", b"value1".to_vec()).await.unwrap();
        ns2.put(b"key", b"value2".to_vec()).await.unwrap();
        
        let val1 = ns1.get(b"key").await.unwrap();
        let val2 = ns2.get(b"key").await.unwrap();
        
        assert_eq!(val1, Some(b"value1".to_vec()));
        assert_eq!(val2, Some(b"value2".to_vec()));
        
        let namespaces = store.list_namespaces().await.unwrap();
        assert_eq!(namespaces.len(), 2);
        assert!(namespaces.contains(&"namespace1".to_string()));
        assert!(namespaces.contains(&"namespace2".to_string()));
    }
}
