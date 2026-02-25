use super::NativeIndexManager;
use crate::schema::SchemaError;
use crate::storage::traits::*;
#[cfg(feature = "aws-backend")]
use crate::storage::DynamoDbNamespacedStore;
use crate::storage::{SledNamespacedStore, TypedKvStore};
use serde::{de::DeserializeOwned, Serialize};
use std::collections::HashMap;
use std::sync::Arc;

/// Enhanced database operations with pluggable storage backend
///
/// This version uses the storage abstraction layer, allowing the same
/// DbOperations API to work with different backends (Sled, DynamoDB, etc.)
#[derive(Clone)]
pub struct DbOperations {
    /// Main storage namespace - using concrete type instead of trait object
    main_store: Arc<TypedKvStore<dyn KvStore>>,

    /// Named namespaces (like sled trees)
    metadata_store: Arc<TypedKvStore<dyn KvStore>>,
    permissions_store: Arc<TypedKvStore<dyn KvStore>>,
    transforms_store: Arc<TypedKvStore<dyn KvStore>>,
    orchestrator_store: Arc<TypedKvStore<dyn KvStore>>,
    schema_states_store: Arc<TypedKvStore<dyn KvStore>>,
    schemas_store: Arc<TypedKvStore<dyn KvStore>>,
    public_keys_store: Arc<TypedKvStore<dyn KvStore>>,
    transform_queue_store: Arc<TypedKvStore<dyn KvStore>>,
    idempotency_store: Arc<TypedKvStore<dyn KvStore>>,
    process_results_store: Arc<TypedKvStore<dyn KvStore>>,

    native_index_manager: Option<NativeIndexManager>,

    /// Optional reference to underlying orchestrator tree for TransformOrchestrator
    /// This is a temporary bridge until TransformOrchestrator is abstracted
    pub orchestrator_tree: Option<sled::Tree>,
}

impl DbOperations {
    /// Create from a NamespacedStore (works with any backend)
    pub async fn from_namespaced_store(
        store: Arc<dyn NamespacedStore>,
        e2e_index_key: Option<[u8; 32]>,
    ) -> Result<Self, crate::storage::StorageError> {
        // Open all required namespaces
        let main_kv = store.open_namespace("main").await?;
        let metadata_kv = store.open_namespace("metadata").await?;
        let permissions_kv = store.open_namespace("node_id_schema_permissions").await?;
        let transforms_kv = store.open_namespace("transforms").await?;
        let orchestrator_kv = store.open_namespace("orchestrator_state").await?;
        let schema_states_kv = store.open_namespace("schema_states").await?;
        let schemas_kv = store.open_namespace("schemas").await?;
        let public_keys_kv = store.open_namespace("public_keys").await?;
        let transform_queue_kv = store.open_namespace("transform_queue_tree").await?;
        let idempotency_kv = store.open_namespace("idempotency").await?;
        let process_results_kv = store.open_namespace("process_results").await?;
        let native_index_kv = store.open_namespace("native_index").await?;

        // Wrap KvStores in TypedKvStore adapters
        let main_store = Arc::new(TypedKvStore::new(main_kv));
        let metadata_store = Arc::new(TypedKvStore::new(metadata_kv));
        let permissions_store = Arc::new(TypedKvStore::new(permissions_kv));
        let transforms_store = Arc::new(TypedKvStore::new(transforms_kv));
        let orchestrator_store = Arc::new(TypedKvStore::new(orchestrator_kv));
        let schema_states_store = Arc::new(TypedKvStore::new(schema_states_kv));
        let schemas_store = Arc::new(TypedKvStore::new(schemas_kv));
        let public_keys_store = Arc::new(TypedKvStore::new(public_keys_kv));
        let transform_queue_store = Arc::new(TypedKvStore::new(transform_queue_kv));
        let idempotency_store = Arc::new(TypedKvStore::new(idempotency_kv));
        let process_results_store = Arc::new(TypedKvStore::new(process_results_kv));

        // Create native index manager with the store and optional E2E index key
        let native_index_manager = NativeIndexManager::new(native_index_kv, e2e_index_key);

        Ok(Self {
            main_store,
            metadata_store,
            permissions_store,
            transforms_store,
            orchestrator_store,
            schema_states_store,
            schemas_store,
            public_keys_store,
            transform_queue_store,
            idempotency_store,
            process_results_store,
            native_index_manager: Some(native_index_manager),
            orchestrator_tree: None,
        })
    }

    /// Convenience constructor for Sled backend (backward compatible, no E2E)
    pub async fn from_sled(db: sled::Db) -> Result<Self, crate::storage::StorageError> {
        let orchestrator_tree = db
            .open_tree("orchestrator_state")
            .map_err(|e| crate::storage::StorageError::SledError(e.to_string()))?;

        let store = Arc::new(SledNamespacedStore::new(db)) as Arc<dyn NamespacedStore>;
        let mut db_ops = Self::from_namespaced_store(store, None).await?;
        db_ops.orchestrator_tree = Some(orchestrator_tree);

        Ok(db_ops)
    }

    /// Convenience constructor for Cloud backend with simplified config
    #[cfg(feature = "aws-backend")]
    pub async fn from_cloud(
        client: aws_sdk_dynamodb::Client,
        table_name: String,
        user_id: String,
    ) -> Result<Self, crate::storage::StorageError> {
        // Note: user_id is no longer passed to the store - it's obtained from request context
        let _ = user_id; // Suppress unused warning - user_id will be obtained from request context
        let store = DynamoDbNamespacedStore::new_with_prefix(client, table_name);
        Self::from_namespaced_store(Arc::new(store), None).await
    }

    /// Constructor for Cloud backend with detailed configuration
    #[cfg(feature = "aws-backend")]
    pub async fn from_cloud_flexible(
        client: aws_sdk_dynamodb::Client,
        resolver: crate::storage::TableNameResolver,
        auto_create: bool,
        user_id: String,
    ) -> Result<Self, crate::storage::StorageError> {
        // Note: user_id is no longer passed to the store - it's obtained from request context
        let _ = user_id; // Suppress unused warning - user_id will be obtained from request context
        let store = DynamoDbNamespacedStore::new(client, resolver, auto_create);
        Self::from_namespaced_store(Arc::new(store), None).await
    }

    // ===== Generic storage operations (async API) =====

    /// Store an item in the main namespace
    pub async fn store_item<T: Serialize + Send + Sync>(
        &self,
        key: &str,
        item: &T,
    ) -> Result<(), SchemaError> {
        self.main_store
            .put_item(key, item)
            .await
            .map_err(|e| SchemaError::InvalidData(e.to_string()))
    }

    /// Get an item from the main namespace
    pub async fn get_item<T: DeserializeOwned + Send + Sync>(
        &self,
        key: &str,
    ) -> Result<Option<T>, SchemaError> {
        self.main_store
            .get_item(key)
            .await
            .map_err(|e| SchemaError::InvalidData(e.to_string()))
    }

    /// Delete an item from the main namespace
    pub async fn delete_item(&self, key: &str) -> Result<bool, SchemaError> {
        self.main_store
            .delete_item(key)
            .await
            .map_err(|e| SchemaError::InvalidData(e.to_string()))
    }

    /// List keys with prefix
    pub async fn list_items_with_prefix(&self, prefix: &str) -> Result<Vec<String>, SchemaError> {
        self.main_store
            .list_keys_with_prefix(prefix)
            .await
            .map_err(|e| SchemaError::InvalidData(e.to_string()))
    }

    /// Store an item in a specific namespace
    pub async fn store_in_namespace<T: Serialize + Send + Sync>(
        &self,
        namespace: &str,
        key: &str,
        item: &T,
    ) -> Result<(), SchemaError> {
        let store = self.get_namespace_store(namespace)?;
        store
            .put_item(key, item)
            .await
            .map_err(|e| SchemaError::InvalidData(e.to_string()))
    }

    /// Get an item from a specific namespace
    pub async fn get_from_namespace<T: DeserializeOwned + Send + Sync>(
        &self,
        namespace: &str,
        key: &str,
    ) -> Result<Option<T>, SchemaError> {
        let store = self.get_namespace_store(namespace)?;
        store
            .get_item(key)
            .await
            .map_err(|e| SchemaError::InvalidData(e.to_string()))
    }

    /// List all keys in a namespace
    pub async fn list_keys_in_namespace(
        &self,
        namespace: &str,
    ) -> Result<Vec<String>, SchemaError> {
        let store = self.get_namespace_store(namespace)?;
        store
            .list_keys_with_prefix("")
            .await
            .map_err(|e| SchemaError::InvalidData(e.to_string()))
    }

    /// Delete an item from a specific namespace
    pub async fn delete_from_namespace(
        &self,
        namespace: &str,
        key: &str,
    ) -> Result<bool, SchemaError> {
        let store = self.get_namespace_store(namespace)?;
        store
            .delete_item(key)
            .await
            .map_err(|e| SchemaError::InvalidData(e.to_string()))
    }

    /// Check if a key exists in a specific namespace
    pub async fn exists_in_namespace(
        &self,
        namespace: &str,
        key: &str,
    ) -> Result<bool, SchemaError> {
        let store = self.get_namespace_store(namespace)?;
        store
            .exists_item(key)
            .await
            .map_err(|e| SchemaError::InvalidData(e.to_string()))
    }

    // ===== Namespace-specific store getters =====

    pub fn metadata_store(&self) -> &Arc<TypedKvStore<dyn KvStore>> {
        &self.metadata_store
    }

    pub fn permissions_store(&self) -> &Arc<TypedKvStore<dyn KvStore>> {
        &self.permissions_store
    }

    pub fn transforms_store(&self) -> &Arc<TypedKvStore<dyn KvStore>> {
        &self.transforms_store
    }

    pub fn orchestrator_store(&self) -> &Arc<TypedKvStore<dyn KvStore>> {
        &self.orchestrator_store
    }

    pub fn schema_states_store(&self) -> &Arc<TypedKvStore<dyn KvStore>> {
        &self.schema_states_store
    }

    pub fn schemas_store(&self) -> &Arc<TypedKvStore<dyn KvStore>> {
        &self.schemas_store
    }

    pub fn public_keys_store(&self) -> &Arc<TypedKvStore<dyn KvStore>> {
        &self.public_keys_store
    }

    pub fn transform_queue_store(&self) -> &Arc<TypedKvStore<dyn KvStore>> {
        &self.transform_queue_store
    }

    pub fn idempotency_store(&self) -> &Arc<TypedKvStore<dyn KvStore>> {
        &self.idempotency_store
    }

    pub fn process_results_store(&self) -> &Arc<TypedKvStore<dyn KvStore>> {
        &self.process_results_store
    }

    pub fn native_index_manager(&self) -> Option<&NativeIndexManager> {
        self.native_index_manager.as_ref()
    }

    /// Get atoms/molecules store (same as main_store for backward compatibility)
    pub fn atoms_store(&self) -> &Arc<TypedKvStore<dyn KvStore>> {
        &self.main_store
    }

    /// Get molecules store (same as main_store for backward compatibility)
    pub fn molecules_store(&self) -> &Arc<TypedKvStore<dyn KvStore>> {
        &self.main_store
    }

    /// Flush all pending writes to durable storage
    /// For Sled backends, this ensures data is written to disk
    /// For cloud backends like DynamoDB, this is typically a no-op (auto-flushed)
    pub async fn flush(&self) -> Result<(), SchemaError> {
        // Storage abstraction handles flushing internally
        // For Sled, this is done via the KvStore trait's flush method
        self.main_store
            .inner()
            .flush()
            .await
            .map_err(|e| SchemaError::InvalidData(format!("Flush failed: {}", e)))
    }

    // ===== Batch operations =====

    /// Batch store multiple items
    pub async fn batch_store_items<T: Serialize + Send + Sync + Clone>(
        &self,
        items: &[(String, T)],
    ) -> Result<(), SchemaError> {
        let items_vec: Vec<(String, T)> =
            items.iter().map(|(k, v)| (k.clone(), v.clone())).collect();

        self.main_store
            .batch_put_items(items_vec)
            .await
            .map_err(|e| SchemaError::InvalidData(e.to_string()))
    }

    /// Batch store items in a specific namespace
    pub async fn batch_store_in_namespace<T: Serialize + Send + Sync + Clone>(
        &self,
        namespace: &str,
        items: &[(String, T)],
    ) -> Result<(), SchemaError> {
        let store = self.get_namespace_store(namespace)?;
        let items_vec: Vec<(String, T)> =
            items.iter().map(|(k, v)| (k.clone(), v.clone())).collect();

        store
            .batch_put_items(items_vec)
            .await
            .map_err(|e| SchemaError::InvalidData(e.to_string()))
    }

    /// Get database statistics (approximate for non-Sled backends)
    pub async fn get_stats(&self) -> Result<HashMap<String, u64>, SchemaError> {
        let mut stats = HashMap::new();

        // Count items with prefixes in main store
        let atoms = self
            .main_store
            .list_keys_with_prefix("atom:")
            .await
            .map_err(|e| SchemaError::InvalidData(e.to_string()))?;
        stats.insert("atoms".to_string(), atoms.len() as u64);

        let refs = self
            .main_store
            .list_keys_with_prefix("ref:")
            .await
            .map_err(|e| SchemaError::InvalidData(e.to_string()))?;
        stats.insert("refs".to_string(), refs.len() as u64);

        // For other namespaces, count all keys
        let metadata_keys = self
            .metadata_store
            .list_keys_with_prefix("")
            .await
            .map_err(|e| SchemaError::InvalidData(e.to_string()))?;
        stats.insert("metadata".to_string(), metadata_keys.len() as u64);

        let permissions_keys = self
            .permissions_store
            .list_keys_with_prefix("")
            .await
            .map_err(|e| SchemaError::InvalidData(e.to_string()))?;
        stats.insert("permissions".to_string(), permissions_keys.len() as u64);

        let transforms_keys = self
            .transforms_store
            .list_keys_with_prefix("")
            .await
            .map_err(|e| SchemaError::InvalidData(e.to_string()))?;
        stats.insert("transforms".to_string(), transforms_keys.len() as u64);

        Ok(stats)
    }

    // ===== Helper methods =====

    fn get_namespace_store(
        &self,
        namespace: &str,
    ) -> Result<&Arc<TypedKvStore<dyn KvStore>>, SchemaError> {
        match namespace {
            "metadata" => Ok(&self.metadata_store),
            "permissions" | "node_id_schema_permissions" => Ok(&self.permissions_store),
            "transforms" => Ok(&self.transforms_store),
            "orchestrator" | "orchestrator_state" => Ok(&self.orchestrator_store),
            "schema_states" => Ok(&self.schema_states_store),
            "schemas" => Ok(&self.schemas_store),
            "public_keys" => Ok(&self.public_keys_store),
            "transform_queue" | "transform_queue_tree" => Ok(&self.transform_queue_store),
            "idempotency" => Ok(&self.idempotency_store),
            "process_results" => Ok(&self.process_results_store),
            "main" => Ok(&self.main_store),
            _ => Err(SchemaError::InvalidData(format!(
                "Unknown namespace: {}",
                namespace
            ))),
        }
    }
}
