//! Thin delegator methods on `DbOperations` for metadata / idempotency /
//! process-results. The real implementations live on
//! [`super::metadata_store::MetadataStore`]; these wrappers exist purely
//! for backward compatibility with older call sites.

use super::core::DbOperations;
use crate::schema::SchemaError;
use serde::de::DeserializeOwned;
use serde::Serialize;

impl DbOperations {
    /// Retrieves or generates and persists the node identifier
    pub async fn get_node_id(&self) -> Result<String, SchemaError> {
        self.metadata().get_node_id().await
    }

    /// Sets the node identifier
    pub async fn set_node_id(&self, node_id: &str) -> Result<(), SchemaError> {
        self.metadata().set_node_id(node_id).await
    }

    /// Retrieve an item from the idempotency store by key.
    pub async fn get_idempotency_item<T: DeserializeOwned + Send + Sync>(
        &self,
        key: &str,
    ) -> Result<Option<T>, SchemaError> {
        self.metadata().get_idempotency_item::<T>(key).await
    }

    /// Store an item in the idempotency store.
    pub async fn put_idempotency_item<T: Serialize + Send + Sync>(
        &self,
        key: &str,
        item: &T,
    ) -> Result<(), SchemaError> {
        self.metadata().put_idempotency_item(key, item).await
    }

    /// Scan process results by key prefix.
    pub async fn scan_process_results<T: DeserializeOwned + Send + Sync>(
        &self,
        prefix: &str,
    ) -> Result<Vec<(String, T)>, SchemaError> {
        self.metadata().scan_process_results::<T>(prefix).await
    }
}
