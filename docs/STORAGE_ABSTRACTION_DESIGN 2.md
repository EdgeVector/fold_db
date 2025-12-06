# Storage Abstraction Layer Design for fold_db

## Goal

Make fold_db **storage-backend agnostic** by introducing a trait-based abstraction layer that allows pluggable storage implementations (Sled, DynamoDB, PostgreSQL, etc.).

## Current Architecture (Sled-Coupled)

```rust
// fold_db/src/db_operations/core.rs (current)
pub struct DbOperations {
    db: sled::Db,                      // ❌ Tightly coupled to Sled
    metadata_tree: sled::Tree,          // ❌ Sled-specific
    permissions_tree: sled::Tree,
    transforms_tree: sled::Tree,
    // ...
}

impl DbOperations {
    pub fn new(db: sled::Db) -> Result<Self, sled::Error> {
        // ❌ Only works with Sled
    }
    
    pub fn store_item<T: Serialize>(&self, key: &str, item: &T) -> Result<(), SchemaError> {
        self.db.insert(key.as_bytes(), bytes)?;  // ❌ Direct Sled API calls
        self.db.flush()?;
        Ok(())
    }
}
```

**Problems**:
1. All code directly uses `sled::Db` and `sled::Tree`
2. Can't swap storage backends without major refactoring
3. Storage logic mixed with business logic

---

## Proposed Architecture (Storage-Agnostic)

### Design Principles

1. **Trait-Based Abstraction**: Define traits for storage operations
2. **Multiple Implementations**: Sled, DynamoDB, PostgreSQL, in-memory, etc.
3. **Backward Compatible**: Existing code should work with minimal changes
4. **Performance**: Zero-cost abstractions where possible
5. **Async-First**: Support both sync (Sled) and async (DynamoDB) backends

---

## Core Storage Traits

```rust
// fold_db/src/storage/traits.rs

use async_trait::async_trait;
use serde::{Serialize, de::DeserializeOwned};
use std::sync::Arc;

/// Result type for storage operations
pub type StorageResult<T> = Result<T, StorageError>;

/// Error type for storage operations
#[derive(Debug, thiserror::Error)]
pub enum StorageError {
    #[error("Item not found: {0}")]
    NotFound(String),
    
    #[error("Serialization error: {0}")]
    SerializationError(String),
    
    #[error("Storage backend error: {0}")]
    BackendError(String),
    
    #[error("Key already exists: {0}")]
    AlreadyExists(String),
    
    #[error("Invalid operation: {0}")]
    InvalidOperation(String),
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
    async fn put_item<T: Serialize + Send>(&self, key: &str, item: &T) -> StorageResult<()>;
    
    /// Get a typed item
    async fn get_item<T: DeserializeOwned + Send>(&self, key: &str) -> StorageResult<Option<T>>;
    
    /// Delete an item
    async fn delete_item(&self, key: &str) -> StorageResult<bool>;
    
    /// List all keys with a given prefix
    async fn list_keys_with_prefix(&self, prefix: &str) -> StorageResult<Vec<String>>;
    
    /// Get all items with a given prefix
    async fn scan_items_with_prefix<T: DeserializeOwned + Send>(&self, prefix: &str) -> StorageResult<Vec<(String, T)>>;
    
    /// Batch store items
    async fn batch_put_items<T: Serialize + Send>(&self, items: Vec<(String, T)>) -> StorageResult<()>;
}

/// Adapter that wraps a KvStore and provides TypedStore functionality
pub struct TypedKvStore<S: KvStore> {
    inner: Arc<S>,
}

impl<S: KvStore> TypedKvStore<S> {
    pub fn new(store: Arc<S>) -> Self {
        Self { inner: store }
    }
}

#[async_trait]
impl<S: KvStore + 'static> TypedStore for TypedKvStore<S> {
    async fn put_item<T: Serialize + Send>(&self, key: &str, item: &T) -> StorageResult<()> {
        let bytes = serde_json::to_vec(item)
            .map_err(|e| StorageError::SerializationError(e.to_string()))?;
        self.inner.put(key.as_bytes(), bytes).await
    }
    
    async fn get_item<T: DeserializeOwned + Send>(&self, key: &str) -> StorageResult<Option<T>> {
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
    
    async fn scan_items_with_prefix<T: DeserializeOwned + Send>(&self, prefix: &str) -> StorageResult<Vec<(String, T)>> {
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
    
    async fn batch_put_items<T: Serialize + Send>(&self, items: Vec<(String, T)>) -> StorageResult<()> {
        let serialized: Result<Vec<_>, _> = items.into_iter()
            .map(|(k, v)| {
                let bytes = serde_json::to_vec(&v)
                    .map_err(|e| StorageError::SerializationError(e.to_string()))?;
                Ok((k.into_bytes(), bytes))
            })
            .collect();
        
        self.inner.batch_put(serialized?).await
    }
}
```

---

## Implementation 1: Sled Backend

```rust
// fold_db/src/storage/sled_backend.rs

use super::traits::*;
use async_trait::async_trait;
use sled::{Db, Tree};
use std::sync::Arc;

/// Sled-backed KvStore implementation
pub struct SledKvStore {
    tree: Tree,
}

impl SledKvStore {
    pub fn new(tree: Tree) -> Self {
        Self { tree }
    }
    
    /// Create from main database (uses default tree)
    pub fn from_db(db: &Db) -> Self {
        Self { tree: db.clone() }
    }
}

#[async_trait]
impl KvStore for SledKvStore {
    async fn get(&self, key: &[u8]) -> StorageResult<Option<Vec<u8>>> {
        self.tree
            .get(key)
            .map_err(|e| StorageError::BackendError(e.to_string()))?
            .map(|ivec| Ok(ivec.to_vec()))
            .transpose()
    }
    
    async fn put(&self, key: &[u8], value: Vec<u8>) -> StorageResult<()> {
        self.tree
            .insert(key, value)
            .map_err(|e| StorageError::BackendError(e.to_string()))?;
        
        // Sled doesn't auto-flush, so we flush immediately
        self.flush().await?;
        Ok(())
    }
    
    async fn delete(&self, key: &[u8]) -> StorageResult<bool> {
        let existed = self.tree
            .remove(key)
            .map_err(|e| StorageError::BackendError(e.to_string()))?
            .is_some();
        Ok(existed)
    }
    
    async fn exists(&self, key: &[u8]) -> StorageResult<bool> {
        self.tree
            .contains_key(key)
            .map_err(|e| StorageError::BackendError(e.to_string()))
    }
    
    async fn scan_prefix(&self, prefix: &[u8]) -> StorageResult<Vec<(Vec<u8>, Vec<u8>)>> {
        self.tree
            .scan_prefix(prefix)
            .map(|result| {
                result.map(|(k, v)| (k.to_vec(), v.to_vec()))
                    .map_err(|e| StorageError::BackendError(e.to_string()))
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
            .map_err(|e| StorageError::BackendError(e.to_string()))?;
        
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
            .map_err(|e| StorageError::BackendError(e.to_string()))?;
        
        Ok(())
    }
    
    async fn flush(&self) -> StorageResult<()> {
        self.tree
            .flush()
            .map_err(|e| StorageError::BackendError(e.to_string()))?;
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
            .map_err(|e| StorageError::BackendError(e.to_string()))?;
        Ok(Self { db })
    }
}

#[async_trait]
impl NamespacedStore for SledNamespacedStore {
    async fn open_namespace(&self, name: &str) -> StorageResult<Arc<dyn KvStore>> {
        let tree = self.db
            .open_tree(name)
            .map_err(|e| StorageError::BackendError(e.to_string()))?;
        
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
            .map_err(|e| StorageError::BackendError(e.to_string()))
    }
}
```

---

## Implementation 2: DynamoDB Backend

```rust
// fold_db/src/storage/dynamodb_backend.rs

use super::traits::*;
use async_trait::async_trait;
use aws_sdk_dynamodb::{Client, types::AttributeValue};
use std::sync::Arc;
use std::collections::HashMap;

/// DynamoDB-backed KvStore implementation
/// 
/// Uses a single DynamoDB table with:
/// - PK: namespace#key
/// - Value: binary data
pub struct DynamoDbKvStore {
    client: Arc<Client>,
    table_name: String,
    namespace: String,
    /// Optional user_id for multi-tenant isolation
    user_id: Option<String>,
}

impl DynamoDbKvStore {
    pub fn new(client: Arc<Client>, table_name: String, namespace: String) -> Self {
        Self {
            client,
            table_name,
            namespace,
            user_id: None,
        }
    }
    
    pub fn with_user_id(mut self, user_id: String) -> Self {
        self.user_id = Some(user_id);
        self
    }
    
    /// Generate the full partition key including user_id if present
    fn make_pk(&self, key: &[u8]) -> String {
        let key_str = String::from_utf8_lossy(key);
        
        if let Some(user_id) = &self.user_id {
            // Multi-tenant: user_id#namespace#key
            format!("{}#{}#{}", user_id, self.namespace, key_str)
        } else {
            // Single-tenant: namespace#key
            format!("{}#{}", self.namespace, key_str)
        }
    }
}

#[async_trait]
impl KvStore for DynamoDbKvStore {
    async fn get(&self, key: &[u8]) -> StorageResult<Option<Vec<u8>>> {
        let pk = self.make_pk(key);
        
        let result = self.client
            .get_item()
            .table_name(&self.table_name)
            .key("PK", AttributeValue::S(pk))
            .send()
            .await
            .map_err(|e| StorageError::BackendError(e.to_string()))?;
        
        if let Some(item) = result.item {
            if let Some(AttributeValue::B(data)) = item.get("Value") {
                return Ok(Some(data.as_ref().to_vec()));
            }
        }
        
        Ok(None)
    }
    
    async fn put(&self, key: &[u8], value: Vec<u8>) -> StorageResult<()> {
        let pk = self.make_pk(key);
        
        self.client
            .put_item()
            .table_name(&self.table_name)
            .item("PK", AttributeValue::S(pk))
            .item("Value", AttributeValue::B(value.into()))
            .send()
            .await
            .map_err(|e| StorageError::BackendError(e.to_string()))?;
        
        Ok(())
    }
    
    async fn delete(&self, key: &[u8]) -> StorageResult<bool> {
        let pk = self.make_pk(key);
        
        let result = self.client
            .delete_item()
            .table_name(&self.table_name)
            .key("PK", AttributeValue::S(pk))
            .return_values(aws_sdk_dynamodb::types::ReturnValue::AllOld)
            .send()
            .await
            .map_err(|e| StorageError::BackendError(e.to_string()))?;
        
        Ok(result.attributes.is_some())
    }
    
    async fn exists(&self, key: &[u8]) -> StorageResult<bool> {
        let pk = self.make_pk(key);
        
        let result = self.client
            .get_item()
            .table_name(&self.table_name)
            .key("PK", AttributeValue::S(pk))
            .projection_expression("PK") // Only fetch key, not value
            .send()
            .await
            .map_err(|e| StorageError::BackendError(e.to_string()))?;
        
        Ok(result.item.is_some())
    }
    
    async fn scan_prefix(&self, prefix: &[u8]) -> StorageResult<Vec<(Vec<u8>, Vec<u8>)>> {
        let prefix_str = String::from_utf8_lossy(prefix);
        let pk_prefix = if let Some(user_id) = &self.user_id {
            format!("{}#{}#{}", user_id, self.namespace, prefix_str)
        } else {
            format!("{}#{}", self.namespace, prefix_str)
        };
        
        let mut results = Vec::new();
        let mut last_evaluated_key: Option<HashMap<String, AttributeValue>> = None;
        
        loop {
            let mut query = self.client
                .scan()
                .table_name(&self.table_name)
                .filter_expression("begins_with(PK, :prefix)")
                .expression_attribute_values(":prefix", AttributeValue::S(pk_prefix.clone()));
            
            if let Some(key) = last_evaluated_key {
                query = query.set_exclusive_start_key(Some(key));
            }
            
            let response = query.send().await
                .map_err(|e| StorageError::BackendError(e.to_string()))?;
            
            if let Some(items) = response.items {
                for item in items {
                    if let (Some(AttributeValue::S(pk)), Some(AttributeValue::B(value))) = 
                        (item.get("PK"), item.get("Value")) {
                        // Extract the key part from PK (strip user_id and namespace)
                        let key_part = if let Some(user_id) = &self.user_id {
                            pk.strip_prefix(&format!("{}#{}#", user_id, self.namespace))
                        } else {
                            pk.strip_prefix(&format!("{}#", self.namespace))
                        };
                        
                        if let Some(key_str) = key_part {
                            results.push((key_str.as_bytes().to_vec(), value.as_ref().to_vec()));
                        }
                    }
                }
            }
            
            last_evaluated_key = response.last_evaluated_key;
            if last_evaluated_key.is_none() {
                break;
            }
        }
        
        Ok(results)
    }
    
    async fn batch_put(&self, items: Vec<(Vec<u8>, Vec<u8>)>) -> StorageResult<()> {
        // DynamoDB batch write supports up to 25 items per request
        for chunk in items.chunks(25) {
            let mut write_requests = Vec::new();
            
            for (key, value) in chunk {
                let pk = self.make_pk(key);
                let mut item = HashMap::new();
                item.insert("PK".to_string(), AttributeValue::S(pk));
                item.insert("Value".to_string(), AttributeValue::B(value.clone().into()));
                
                write_requests.push(
                    aws_sdk_dynamodb::types::WriteRequest::builder()
                        .put_request(
                            aws_sdk_dynamodb::types::PutRequest::builder()
                                .set_item(Some(item))
                                .build()
                        )
                        .build()
                );
            }
            
            let mut requests = HashMap::new();
            requests.insert(self.table_name.clone(), write_requests);
            
            self.client
                .batch_write_item()
                .set_request_items(Some(requests))
                .send()
                .await
                .map_err(|e| StorageError::BackendError(e.to_string()))?;
        }
        
        Ok(())
    }
    
    async fn batch_delete(&self, keys: Vec<Vec<u8>>) -> StorageResult<()> {
        for chunk in keys.chunks(25) {
            let mut write_requests = Vec::new();
            
            for key in chunk {
                let pk = self.make_pk(key);
                let mut key_map = HashMap::new();
                key_map.insert("PK".to_string(), AttributeValue::S(pk));
                
                write_requests.push(
                    aws_sdk_dynamodb::types::WriteRequest::builder()
                        .delete_request(
                            aws_sdk_dynamodb::types::DeleteRequest::builder()
                                .set_key(Some(key_map))
                                .build()
                        )
                        .build()
                );
            }
            
            let mut requests = HashMap::new();
            requests.insert(self.table_name.clone(), write_requests);
            
            self.client
                .batch_write_item()
                .set_request_items(Some(requests))
                .send()
                .await
                .map_err(|e| StorageError::BackendError(e.to_string()))?;
        }
        
        Ok(())
    }
    
    async fn flush(&self) -> StorageResult<()> {
        // DynamoDB is always consistent, no flush needed
        Ok(())
    }
    
    fn backend_name(&self) -> &'static str {
        "dynamodb"
    }
}

/// DynamoDB-backed NamespacedStore
pub struct DynamoDbNamespacedStore {
    client: Arc<Client>,
    table_name: String,
    user_id: Option<String>,
}

impl DynamoDbNamespacedStore {
    pub fn new(client: Client, table_name: String) -> Self {
        Self {
            client: Arc::new(client),
            table_name,
            user_id: None,
        }
    }
    
    pub fn with_user_id(mut self, user_id: String) -> Self {
        self.user_id = Some(user_id);
        self
    }
}

#[async_trait]
impl NamespacedStore for DynamoDbNamespacedStore {
    async fn open_namespace(&self, name: &str) -> StorageResult<Arc<dyn KvStore>> {
        let mut store = DynamoDbKvStore::new(
            self.client.clone(),
            self.table_name.clone(),
            name.to_string()
        );
        
        if let Some(user_id) = &self.user_id {
            store = store.with_user_id(user_id.clone());
        }
        
        Ok(Arc::new(store))
    }
    
    async fn list_namespaces(&self) -> StorageResult<Vec<String>> {
        // This would require scanning all keys and extracting unique namespaces
        // Implementation depends on your specific needs
        unimplemented!("list_namespaces for DynamoDB requires custom implementation")
    }
    
    async fn delete_namespace(&self, _name: &str) -> StorageResult<bool> {
        // Would need to scan and delete all items with the namespace prefix
        unimplemented!("delete_namespace for DynamoDB requires custom implementation")
    }
}
```

---

## Refactored DbOperations

```rust
// fold_db/src/db_operations/core_refactored.rs

use crate::storage::traits::*;
use crate::schema::SchemaError;
use super::NativeIndexManager;
use serde::{Serialize, de::DeserializeOwned};
use std::sync::Arc;

/// Enhanced database operations with pluggable storage backend
#[derive(Clone)]
pub struct DbOperations {
    /// Main storage namespace
    main_store: Arc<dyn TypedStore>,
    
    /// Named namespaces (like sled trees)
    metadata_store: Arc<dyn TypedStore>,
    permissions_store: Arc<dyn TypedStore>,
    transforms_store: Arc<dyn TypedStore>,
    orchestrator_store: Arc<dyn TypedStore>,
    schema_states_store: Arc<dyn TypedStore>,
    schemas_store: Arc<dyn TypedStore>,
    public_keys_store: Arc<dyn TypedStore>,
    transform_queue_store: Arc<dyn TypedStore>,
    native_index_store: Arc<dyn KvStore>,
    
    native_index_manager: NativeIndexManager,
}

impl DbOperations {
    /// Create from a NamespacedStore (works with any backend)
    pub async fn from_namespaced_store(
        store: Arc<dyn NamespacedStore>
    ) -> Result<Self, StorageError> {
        // Open all required namespaces
        let main_store = TypedKvStore::new(store.open_namespace("main").await?);
        let metadata_store = TypedKvStore::new(store.open_namespace("metadata").await?);
        let permissions_store = TypedKvStore::new(store.open_namespace("node_id_schema_permissions").await?);
        let transforms_store = TypedKvStore::new(store.open_namespace("transforms").await?);
        let orchestrator_store = TypedKvStore::new(store.open_namespace("orchestrator_state").await?);
        let schema_states_store = TypedKvStore::new(store.open_namespace("schema_states").await?);
        let schemas_store = TypedKvStore::new(store.open_namespace("schemas").await?);
        let public_keys_store = TypedKvStore::new(store.open_namespace("public_keys").await?);
        let transform_queue_store = TypedKvStore::new(store.open_namespace("transform_queue_tree").await?);
        let native_index_store = store.open_namespace("native_index").await?;
        
        let native_index_manager = NativeIndexManager::new_with_store(native_index_store.clone());
        
        Ok(Self {
            main_store: Arc::new(main_store),
            metadata_store: Arc::new(metadata_store),
            permissions_store: Arc::new(permissions_store),
            transforms_store: Arc::new(transforms_store),
            orchestrator_store: Arc::new(orchestrator_store),
            schema_states_store: Arc::new(schema_states_store),
            schemas_store: Arc::new(schemas_store),
            public_keys_store: Arc::new(public_keys_store),
            transform_queue_store: Arc::new(transform_queue_store),
            native_index_store,
            native_index_manager,
        })
    }
    
    /// Convenience constructor for Sled backend (backward compatible)
    pub fn from_sled(db: sled::Db) -> Result<Self, StorageError> {
        let store = Arc::new(SledNamespacedStore::new(db));
        // Use blocking runtime for compatibility
        tokio::runtime::Runtime::new()
            .unwrap()
            .block_on(Self::from_namespaced_store(store))
    }
    
    /// Convenience constructor for DynamoDB backend
    pub async fn from_dynamodb(
        client: aws_sdk_dynamodb::Client,
        table_name: String,
        user_id: Option<String>
    ) -> Result<Self, StorageError> {
        let mut store = DynamoDbNamespacedStore::new(client, table_name);
        if let Some(uid) = user_id {
            store = store.with_user_id(uid);
        }
        Self::from_namespaced_store(Arc::new(store)).await
    }
    
    // ===== Generic storage operations (same API as before) =====
    
    /// Store an item in the main namespace
    pub async fn store_item<T: Serialize + Send>(&self, key: &str, item: &T) -> Result<(), SchemaError> {
        self.main_store.put_item(key, item).await
            .map_err(|e| SchemaError::InvalidData(e.to_string()))
    }
    
    /// Get an item from the main namespace
    pub async fn get_item<T: DeserializeOwned + Send>(&self, key: &str) -> Result<Option<T>, SchemaError> {
        self.main_store.get_item(key).await
            .map_err(|e| SchemaError::InvalidData(e.to_string()))
    }
    
    /// Delete an item from the main namespace
    pub async fn delete_item(&self, key: &str) -> Result<bool, SchemaError> {
        self.main_store.delete_item(key).await
            .map_err(|e| SchemaError::InvalidData(e.to_string()))
    }
    
    /// List keys with prefix
    pub async fn list_items_with_prefix(&self, prefix: &str) -> Result<Vec<String>, SchemaError> {
        self.main_store.list_keys_with_prefix(prefix).await
            .map_err(|e| SchemaError::InvalidData(e.to_string()))
    }
    
    // ===== Namespace-specific operations =====
    
    pub fn metadata_store(&self) -> &Arc<dyn TypedStore> {
        &self.metadata_store
    }
    
    pub fn permissions_store(&self) -> &Arc<dyn TypedStore> {
        &self.permissions_store
    }
    
    pub fn transforms_store(&self) -> &Arc<dyn TypedStore> {
        &self.transforms_store
    }
    
    // ... other store getters
    
    pub fn native_index_manager(&self) -> &NativeIndexManager {
        &self.native_index_manager
    }
}
```

---

## Usage Examples

### Example 1: Local Development with Sled

```rust
// main.rs - local development
use fold_db::db_operations::DbOperations;
use fold_db::storage::SledNamespacedStore;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Option A: Direct Sled (backward compatible)
    let db = sled::open("data/local")?;
    let db_ops = DbOperations::from_sled(db)?;
    
    // Option B: Explicit storage abstraction
    let store = SledNamespacedStore::open("data/local")?;
    let db_ops = DbOperations::from_namespaced_store(Arc::new(store)).await?;
    
    // Use DbOperations as before
    db_ops.store_item("key", &my_data).await?;
    
    Ok(())
}
```

### Example 2: Production with DynamoDB

```rust
// lambda handler - production
use fold_db::db_operations::DbOperations;
use fold_db::storage::DynamoDbNamespacedStore;
use aws_config::BehaviorVersion;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let config = aws_config::load_defaults(BehaviorVersion::latest()).await;
    let client = aws_sdk_dynamodb::Client::new(&config);
    
    // Create DbOperations with DynamoDB backend
    let db_ops = DbOperations::from_dynamodb(
        client,
        "FoldDbStorage".to_string(),
        Some("user_123".to_string()) // Multi-tenant!
    ).await?;
    
    // Same API works with DynamoDB!
    db_ops.store_item("key", &my_data).await?;
    let data = db_ops.get_item::<MyStruct>("key").await?;
    
    Ok(())
}
```

### Example 3: Testing with In-Memory Backend

```rust
// tests/integration_test.rs
use fold_db::storage::InMemoryKvStore;

#[tokio::test]
async fn test_mutations() {
    // In-memory backend for fast tests
    let store = Arc::new(InMemoryNamespacedStore::new());
    let db_ops = DbOperations::from_namespaced_store(store).await.unwrap();
    
    // Test your logic without touching disk or cloud
    db_ops.store_item("test", &test_data).await.unwrap();
    assert_eq!(db_ops.get_item::<TestData>("test").await.unwrap(), Some(test_data));
}
```

---

## Migration Path

### Phase 1: Add Trait Layer (No Breaking Changes)
1. Add `storage/traits.rs` with trait definitions
2. Add `storage/sled_backend.rs` implementing traits for Sled
3. Keep existing `DbOperations` unchanged (backward compatible)

### Phase 2: Add Alternative Backends
1. Add `storage/dynamodb_backend.rs`
2. Add `storage/in_memory_backend.rs` for testing
3. Add `DbOperations::from_namespaced_store()`

### Phase 3: Refactor Internals
1. Make `DbOperations` use trait objects internally
2. Add async versions of methods
3. Maintain sync wrapper for backward compatibility

### Phase 4: Full Migration
1. Update all callsites to use async methods
2. Remove direct Sled dependencies from DbOperations
3. Make storage backend configurable at runtime

---

## Benefits

1. **Storage Agnostic**: Swap backends without changing business logic
2. **Multi-Tenant Ready**: DynamoDB backend supports user isolation
3. **Testable**: In-memory backend for fast unit tests
4. **Cloud Native**: Easy integration with DynamoDB, S3, PostgreSQL, etc.
5. **Performance**: Zero-cost abstractions with trait objects
6. **Backward Compatible**: Existing Sled code keeps working
7. **Future Proof**: Easy to add new backends (Redis, Cassandra, etc.)

---

## Next Steps

1. Implement trait definitions in `storage/traits.rs`
2. Implement Sled backend in `storage/sled_backend.rs`
3. Implement DynamoDB backend in `storage/dynamodb_backend.rs`
4. Add tests for each backend
5. Refactor `DbOperations` to use traits
6. Update documentation

Would you like me to start implementing any of these components?
