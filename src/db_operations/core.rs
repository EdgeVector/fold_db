use super::atom_store::AtomStore;
use super::schema_store::SchemaStore;
use super::view_store::ViewStore;
use super::NativeIndexManager;
use crate::schema::SchemaError;
use crate::storage::traits::*;
use crate::storage::{SledNamespacedStore, TypedKvStore};
use std::sync::Arc;

/// Database operations with pluggable storage backend
///
/// Uses the storage abstraction layer (Sled locally, with optional cloud sync).
///
/// The three core domains (schemas, atoms, views) are each encapsulated in
/// a dedicated store struct whose fields are private. External callers
/// reach them through `schemas()`, `atoms()`, and `views()`.
#[derive(Clone)]
pub struct DbOperations {
    /// Schema / schema-state / superseded-by namespaces
    schema_store: SchemaStore,
    /// Main namespace — atoms, molecules, mutation events, sync conflicts
    atom_store: AtomStore,
    /// Transform view definitions, view states, field cache state
    view_store: ViewStore,

    // ----- Remaining raw namespaces not yet wrapped in a domain store -----
    metadata_store: Arc<TypedKvStore<dyn KvStore>>,
    permissions_store: Arc<TypedKvStore<dyn KvStore>>,
    public_keys_store: Arc<TypedKvStore<dyn KvStore>>,
    idempotency_store: Arc<TypedKvStore<dyn KvStore>>,
    process_results_store: Arc<TypedKvStore<dyn KvStore>>,

    native_index_manager: Option<NativeIndexManager>,
}

impl DbOperations {
    /// Create from a NamespacedStore (works with any backend)
    pub async fn from_namespaced_store(
        store: Arc<dyn NamespacedStore>,
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

        // Domain stores
        let schema_store =
            SchemaStore::new(schemas_store, schema_states_store, superseded_by_store);
        let atom_store = AtomStore::new(main_store);
        let view_store =
            ViewStore::new(views_store, view_states_store, transform_field_states_store);

        // Create native index manager and load any previously stored embeddings
        let native_index_manager = NativeIndexManager::new(native_index_kv);
        native_index_manager.restore_from_store().await;

        Ok(Self {
            schema_store,
            atom_store,
            view_store,
            metadata_store,
            permissions_store,
            public_keys_store,
            idempotency_store,
            process_results_store,
            native_index_manager: Some(native_index_manager),
        })
    }

    /// Convenience constructor for Sled backend (backward compatible, no E2E)
    pub async fn from_sled(
        pool: Arc<crate::storage::SledPool>,
    ) -> Result<Self, crate::storage::StorageError> {
        let store = Arc::new(SledNamespacedStore::new(pool)) as Arc<dyn NamespacedStore>;
        Self::from_namespaced_store(store).await
    }

    // ===== Domain store accessors (public) =====

    /// Access the schema domain store.
    pub fn schemas(&self) -> &SchemaStore {
        &self.schema_store
    }

    /// Access the atom domain store.
    pub fn atoms(&self) -> &AtomStore {
        &self.atom_store
    }

    /// Access the view domain store.
    pub fn views(&self) -> &ViewStore {
        &self.view_store
    }

    // ===== Non-domain store getters (crate-only) =====
    //
    // These namespaces are used by smaller sibling modules
    // (metadata_operations, trust_operations, public_key_operations,
    // conflict_operations, org_operations). Wrapping them in their
    // own domain structs is left for a follow-up refactor.

    pub(crate) fn metadata_store(&self) -> &Arc<TypedKvStore<dyn KvStore>> {
        &self.metadata_store
    }

    pub(crate) fn permissions_store(&self) -> &Arc<TypedKvStore<dyn KvStore>> {
        &self.permissions_store
    }

    pub(crate) fn public_keys_store(&self) -> &Arc<TypedKvStore<dyn KvStore>> {
        &self.public_keys_store
    }

    pub(crate) fn idempotency_store(&self) -> &Arc<TypedKvStore<dyn KvStore>> {
        &self.idempotency_store
    }

    pub(crate) fn process_results_store(&self) -> &Arc<TypedKvStore<dyn KvStore>> {
        &self.process_results_store
    }

    /// Access the native index manager for embedding and search operations.
    pub fn native_index_manager(&self) -> Option<&NativeIndexManager> {
        self.native_index_manager.as_ref()
    }

    // ===== Public accessors for external callers =====

    /// Get the raw metadata KvStore for external modules that need generic key-value
    /// access (e.g., discovery configs, async queries).
    pub fn raw_metadata_store(&self) -> Arc<dyn KvStore> {
        self.metadata_store.inner().clone()
    }

    /// Flush all pending writes to durable storage
    pub async fn flush(&self) -> Result<(), SchemaError> {
        Ok(self.atom_store.flush().await?)
    }
}
