//! Metadata domain store.
//!
//! Owns the `metadata`, `idempotency`, and `process_results` namespaces.
//! External callers reach these via `DbOperations::metadata()`.
//!
//! Responsibilities:
//! - Node-level metadata (e.g. `node_id`)
//! - Idempotency cache for mutations
//! - Process-result bookkeeping

use std::sync::Arc;

use serde::de::DeserializeOwned;
use serde::Serialize;
use uuid::Uuid;

use crate::schema::SchemaError;
use crate::storage::traits::{KvStore, TypedStore};
use crate::storage::TypedKvStore;

/// Domain store for node metadata / idempotency / process-results persistence.
#[derive(Clone)]
pub struct MetadataStore {
    metadata_store: Arc<TypedKvStore<dyn KvStore>>,
    idempotency_store: Arc<TypedKvStore<dyn KvStore>>,
    process_results_store: Arc<TypedKvStore<dyn KvStore>>,
}

impl MetadataStore {
    pub(crate) fn new(
        metadata_store: Arc<TypedKvStore<dyn KvStore>>,
        idempotency_store: Arc<TypedKvStore<dyn KvStore>>,
        process_results_store: Arc<TypedKvStore<dyn KvStore>>,
    ) -> Self {
        Self {
            metadata_store,
            idempotency_store,
            process_results_store,
        }
    }

    /// Crate-internal access to the raw metadata namespace (used by org purge).
    pub(crate) fn raw_metadata(&self) -> &Arc<TypedKvStore<dyn KvStore>> {
        &self.metadata_store
    }

    /// Crate-internal access to the raw idempotency namespace (used by org purge).
    pub(crate) fn raw_idempotency(&self) -> &Arc<TypedKvStore<dyn KvStore>> {
        &self.idempotency_store
    }

    /// Crate-internal access to the raw process-results namespace (used by org purge).
    pub(crate) fn raw_process_results(&self) -> &Arc<TypedKvStore<dyn KvStore>> {
        &self.process_results_store
    }

    /// Escape hatch for external modules that need generic typed access to the
    /// metadata namespace (e.g. discovery configs, async queries).
    pub fn raw_metadata_kv(&self) -> Arc<dyn KvStore> {
        self.metadata_store.inner().clone()
    }

    // ===== Node-id =====

    /// Retrieves or generates and persists the node identifier.
    pub async fn get_node_id(&self) -> Result<String, SchemaError> {
        match self.metadata_store.get_item::<String>("node_id").await {
            Ok(Some(id)) if !id.is_empty() => {
                return Ok(id);
            }
            Ok(Some(_)) | Ok(None) => {}
            Err(e) => {
                log::warn!(
                    "Failed to deserialize node_id (possibly old format): {}, creating new",
                    e
                );
            }
        }

        let new_id = Uuid::new_v4().to_string();
        self.set_node_id(&new_id).await?;
        Ok(new_id)
    }

    /// Sets the node identifier
    pub async fn set_node_id(&self, node_id: &str) -> Result<(), SchemaError> {
        self.metadata_store
            .put_item("node_id", &node_id.to_string())
            .await?;
        self.metadata_store.inner().flush().await?;
        Ok(())
    }

    // ===== Idempotency store =====

    /// Retrieve an item from the idempotency store by key.
    pub async fn get_idempotency_item<T: DeserializeOwned + Send + Sync>(
        &self,
        key: &str,
    ) -> Result<Option<T>, SchemaError> {
        self.idempotency_store
            .get_item::<T>(key)
            .await
            .map_err(|e| SchemaError::InvalidData(format!("Failed to get idempotency item: {}", e)))
    }

    /// Store an item in the idempotency store.
    pub async fn put_idempotency_item<T: Serialize + Send + Sync>(
        &self,
        key: &str,
        item: &T,
    ) -> Result<(), SchemaError> {
        self.idempotency_store.put_item(key, item).await?;
        Ok(())
    }

    /// Batch store idempotency entries (`(key, uuid)` pairs).
    pub async fn batch_put_idempotency(
        &self,
        entries: Vec<(String, String)>,
    ) -> Result<(), SchemaError> {
        self.idempotency_store
            .batch_put_items(entries)
            .await
            .map_err(|e| SchemaError::InvalidData(format!("Idempotency store failed: {}", e)))
    }

    // ===== Process results store =====

    /// Scan process results by key prefix.
    pub async fn scan_process_results<T: DeserializeOwned + Send + Sync>(
        &self,
        prefix: &str,
    ) -> Result<Vec<(String, T)>, SchemaError> {
        self.process_results_store
            .scan_items_with_prefix(prefix)
            .await
            .map_err(|e| SchemaError::InvalidData(format!("Failed to scan process results: {}", e)))
    }

    /// Store a process result.
    pub async fn put_process_result<T: Serialize + Send + Sync>(
        &self,
        key: &str,
        value: &T,
    ) -> Result<(), SchemaError> {
        self.process_results_store
            .put_item(key, value)
            .await
            .map_err(|e| SchemaError::InvalidData(format!("Failed to store process result: {}", e)))
    }
}
