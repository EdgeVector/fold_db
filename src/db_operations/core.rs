use super::native_index::Embedder;
use super::NativeIndexManager;
use crate::schema::SchemaError;
use crate::storage::traits::*;
#[cfg(feature = "aws-backend")]
use crate::storage::DynamoDbNamespacedStore;
use crate::storage::{SledNamespacedStore, TypedKvStore};
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
    schema_states_store: Arc<TypedKvStore<dyn KvStore>>,
    schemas_store: Arc<TypedKvStore<dyn KvStore>>,
    public_keys_store: Arc<TypedKvStore<dyn KvStore>>,
    idempotency_store: Arc<TypedKvStore<dyn KvStore>>,
    process_results_store: Arc<TypedKvStore<dyn KvStore>>,
    superseded_by_store: Arc<TypedKvStore<dyn KvStore>>,

    /// Transform view storage namespaces
    views_store: Arc<TypedKvStore<dyn KvStore>>,
    view_states_store: Arc<TypedKvStore<dyn KvStore>>,
    transform_field_states_store: Arc<TypedKvStore<dyn KvStore>>,

    native_index_manager: Option<NativeIndexManager>,
}

impl DbOperations {
    /// Create from a NamespacedStore (works with any backend)
    pub async fn from_namespaced_store(
        store: Arc<dyn NamespacedStore>,
        embedder: Arc<dyn Embedder>,
    ) -> Result<Self, crate::storage::StorageError> {
        // Open all required namespaces
        let main_kv = store.open_namespace("main").await?;
        let metadata_kv = store.open_namespace("metadata").await?;
        let permissions_kv = store.open_namespace("node_id_schema_permissions").await?;
        let schema_states_kv = store.open_namespace("schema_states").await?;
        let schemas_kv = store.open_namespace("schemas").await?;
        let public_keys_kv = store.open_namespace("public_keys").await?;
        let idempotency_kv = store.open_namespace("idempotency").await?;
        let process_results_kv = store.open_namespace("process_results").await?;
        let superseded_by_kv = store.open_namespace("schema_superseded_by").await?;
        let views_kv = store.open_namespace("views").await?;
        let view_states_kv = store.open_namespace("view_states").await?;
        let transform_field_states_kv = store.open_namespace("transform_field_states").await?;
        let native_index_kv = store.open_namespace("native_index").await?;

        // Wrap KvStores in TypedKvStore adapters
        let main_store = Arc::new(TypedKvStore::new(main_kv));
        let metadata_store = Arc::new(TypedKvStore::new(metadata_kv));
        let permissions_store = Arc::new(TypedKvStore::new(permissions_kv));
        let schema_states_store = Arc::new(TypedKvStore::new(schema_states_kv));
        let schemas_store = Arc::new(TypedKvStore::new(schemas_kv));
        let public_keys_store = Arc::new(TypedKvStore::new(public_keys_kv));
        let idempotency_store = Arc::new(TypedKvStore::new(idempotency_kv));
        let process_results_store = Arc::new(TypedKvStore::new(process_results_kv));
        let superseded_by_store = Arc::new(TypedKvStore::new(superseded_by_kv));
        let views_store = Arc::new(TypedKvStore::new(views_kv));
        let view_states_store = Arc::new(TypedKvStore::new(view_states_kv));
        let transform_field_states_store = Arc::new(TypedKvStore::new(transform_field_states_kv));

        // Create native index manager and load any previously stored embeddings
        let native_index_manager = NativeIndexManager::new(native_index_kv, embedder);
        native_index_manager.restore_from_store().await;

        Ok(Self {
            main_store,
            metadata_store,
            permissions_store,
            schema_states_store,
            schemas_store,
            public_keys_store,
            idempotency_store,
            process_results_store,
            superseded_by_store,
            views_store,
            view_states_store,
            transform_field_states_store,
            native_index_manager: Some(native_index_manager),
        })
    }

    /// Convenience constructor for Sled backend
    pub async fn from_sled(db: sled::Db, embedder: Arc<dyn Embedder>) -> Result<Self, crate::storage::StorageError> {
        let store = Arc::new(SledNamespacedStore::new(db)) as Arc<dyn NamespacedStore>;
        Self::from_namespaced_store(store, embedder).await
    }

    /// Convenience constructor for Cloud backend with simplified config
    #[cfg(feature = "aws-backend")]
    pub async fn from_cloud(
        client: aws_sdk_dynamodb::Client,
        table_name: String,
        user_id: String,
        embedder: Arc<dyn Embedder>,
    ) -> Result<Self, crate::storage::StorageError> {
        let _ = user_id; // Suppress unused warning - user_id will be obtained from request context
        let store = DynamoDbNamespacedStore::new_with_prefix(client, table_name);
        Self::from_namespaced_store(Arc::new(store), embedder).await
    }

    /// Constructor for Cloud backend with detailed configuration
    #[cfg(feature = "aws-backend")]
    pub async fn from_cloud_flexible(
        client: aws_sdk_dynamodb::Client,
        resolver: crate::storage::TableNameResolver,
        auto_create: bool,
        user_id: String,
        embedder: Arc<dyn Embedder>,
    ) -> Result<Self, crate::storage::StorageError> {
        let _ = user_id; // Suppress unused warning - user_id will be obtained from request context
        let store = DynamoDbNamespacedStore::new(client, resolver, auto_create);
        Self::from_namespaced_store(Arc::new(store), embedder).await
    }

    // ===== Namespace-specific store getters =====

    pub fn metadata_store(&self) -> &Arc<TypedKvStore<dyn KvStore>> {
        &self.metadata_store
    }

    pub fn permissions_store(&self) -> &Arc<TypedKvStore<dyn KvStore>> {
        &self.permissions_store
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

    pub fn idempotency_store(&self) -> &Arc<TypedKvStore<dyn KvStore>> {
        &self.idempotency_store
    }

    pub fn process_results_store(&self) -> &Arc<TypedKvStore<dyn KvStore>> {
        &self.process_results_store
    }

    pub fn superseded_by_store(&self) -> &Arc<TypedKvStore<dyn KvStore>> {
        &self.superseded_by_store
    }

    pub fn native_index_manager(&self) -> Option<&NativeIndexManager> {
        self.native_index_manager.as_ref()
    }

    pub fn views_store(&self) -> &Arc<TypedKvStore<dyn KvStore>> {
        &self.views_store
    }

    pub fn view_states_store(&self) -> &Arc<TypedKvStore<dyn KvStore>> {
        &self.view_states_store
    }

    pub fn transform_field_states_store(&self) -> &Arc<TypedKvStore<dyn KvStore>> {
        &self.transform_field_states_store
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
    pub async fn flush(&self) -> Result<(), SchemaError> {
        Ok(self.main_store.inner().flush().await?)
    }
}
