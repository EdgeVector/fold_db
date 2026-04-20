use super::atom_store::AtomStore;
use super::metadata_store::MetadataStore;
use super::permissions_store::PermissionsStore;
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
/// All persistence is encapsulated in domain store structs whose
/// namespace fields are private. External callers reach them through
/// `schemas()`, `atoms()`, `views()`, `permissions()`, and `metadata()`.
#[derive(Clone)]
pub struct DbOperations {
    /// Schema / schema-state / superseded-by namespaces
    schema_store: SchemaStore,
    /// Main namespace — atoms, molecules, mutation events, sync conflicts
    atom_store: AtomStore,
    /// Transform view definitions, view states, field cache state
    view_store: ViewStore,
    /// Permissions + public keys
    permissions_store: PermissionsStore,
    /// Metadata + idempotency + process results
    metadata_store: MetadataStore,

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
        let metadata_typed = Arc::new(TypedKvStore::new(metadata_kv));
        let permissions_typed = Arc::new(TypedKvStore::new(permissions_kv));
        let schema_states_store = Arc::new(TypedKvStore::new(schema_states_kv));
        let schemas_store = Arc::new(TypedKvStore::new(schemas_kv));
        let public_keys_typed = Arc::new(TypedKvStore::new(public_keys_kv));
        let idempotency_typed = Arc::new(TypedKvStore::new(idempotency_kv));
        let process_results_typed = Arc::new(TypedKvStore::new(process_results_kv));
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
        let permissions_store = PermissionsStore::new(permissions_typed, public_keys_typed);
        let metadata_store =
            MetadataStore::new(metadata_typed, idempotency_typed, process_results_typed);

        // Create native index manager and load any previously stored embeddings.
        //
        // Setting `FOLD_DISABLE_NATIVE_INDEX=1` skips the manager entirely, which
        // causes `inline_index_mutations` in the mutation manager to become a no-op
        // (it guards on `native_index_manager().is_some()`). This is the headless /
        // agent-brain path: no fastembed init, no ONNX runtime dependency, no model
        // download. Search by embedding is unavailable — plain hash/range queries
        // still work unchanged.
        let native_index_disabled = std::env::var("FOLD_DISABLE_NATIVE_INDEX")
            .map(|v| matches!(v.trim(), "1" | "true" | "TRUE" | "yes" | "YES"))
            .unwrap_or(false);
        let native_index_manager = if native_index_disabled {
            log::info!(
                "Native index disabled via FOLD_DISABLE_NATIVE_INDEX — \
                 embedding-based search will not function; hash/range queries still work."
            );
            None
        } else {
            let mgr = NativeIndexManager::new(native_index_kv);
            mgr.restore_from_store().await;
            Some(mgr)
        };

        Ok(Self {
            schema_store,
            atom_store,
            view_store,
            permissions_store,
            metadata_store,
            native_index_manager,
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

    /// Access the permissions / public-keys / trust domain store.
    pub fn permissions(&self) -> &PermissionsStore {
        &self.permissions_store
    }

    /// Access the metadata / idempotency / process-results domain store.
    pub fn metadata(&self) -> &MetadataStore {
        &self.metadata_store
    }

    /// Access the native index manager for embedding and search operations.
    pub fn native_index_manager(&self) -> Option<&NativeIndexManager> {
        self.native_index_manager.as_ref()
    }

    // ===== Public escape hatches =====

    /// Get the raw metadata KvStore for external modules that need generic key-value
    /// access (e.g., discovery configs, async queries).
    pub fn raw_metadata_store(&self) -> Arc<dyn KvStore> {
        self.metadata_store.raw_metadata_kv()
    }

    /// Flush all pending writes to durable storage
    pub async fn flush(&self) -> Result<(), SchemaError> {
        Ok(self.atom_store.flush().await?)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::storage::SledPool;
    use tempfile::TempDir;

    async fn build_ops() -> (TempDir, DbOperations) {
        let tmp = TempDir::new().unwrap();
        let pool = Arc::new(SledPool::new(tmp.path().to_path_buf()));
        let ops = DbOperations::from_sled(pool).await.unwrap();
        (tmp, ops)
    }

    /// Guards against the tokio runtime being held across env-var mutations.
    /// `temp_env::with_var` is sync — calling `block_on` inside it keeps the
    /// env mutation atomic to this thread, which matters because `cargo test`
    /// runs tests in parallel on a shared process env.
    #[test]
    fn native_index_enabled_by_default() {
        temp_env::with_var_unset("FOLD_DISABLE_NATIVE_INDEX", || {
            let rt = tokio::runtime::Runtime::new().unwrap();
            rt.block_on(async {
                let (_tmp, ops) = build_ops().await;
                assert!(
                    ops.native_index_manager().is_some(),
                    "native_index_manager is present when env var unset"
                );
            });
        });
    }

    #[test]
    fn native_index_disabled_via_env() {
        temp_env::with_var("FOLD_DISABLE_NATIVE_INDEX", Some("1"), || {
            let rt = tokio::runtime::Runtime::new().unwrap();
            rt.block_on(async {
                let (_tmp, ops) = build_ops().await;
                assert!(
                    ops.native_index_manager().is_none(),
                    "native_index_manager is None when FOLD_DISABLE_NATIVE_INDEX=1"
                );
            });
        });
    }

    #[test]
    fn native_index_disabled_accepts_true_variants() {
        for val in ["1", "true", "TRUE", "yes", "YES"] {
            temp_env::with_var("FOLD_DISABLE_NATIVE_INDEX", Some(val), || {
                let rt = tokio::runtime::Runtime::new().unwrap();
                rt.block_on(async {
                    let (_tmp, ops) = build_ops().await;
                    assert!(
                        ops.native_index_manager().is_none(),
                        "FOLD_DISABLE_NATIVE_INDEX={} should disable",
                        val
                    );
                });
            });
        }
    }
}
