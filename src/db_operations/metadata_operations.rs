use super::core::DbOperations;
use crate::schema::SchemaError;
use crate::storage::traits::TypedStore;
use serde::de::DeserializeOwned;
use serde::Serialize;
use uuid::Uuid;

impl DbOperations {
    /// Retrieves or generates and persists the node identifier
    pub async fn get_node_id(&self) -> Result<String, SchemaError> {
        // Try to get existing node_id (handle deserialization errors gracefully)
        match self.metadata_store().get_item::<String>("node_id").await {
            Ok(Some(id)) if !id.is_empty() => {
                return Ok(id);
            }
            Ok(Some(_)) | Ok(None) => {
                // Empty or missing - create new
            }
            Err(e) => {
                // Deserialization error (e.g., old format) - create new node_id
                log::warn!(
                    "Failed to deserialize node_id (possibly old format): {}, creating new",
                    e
                );
            }
        }

        // Generate new node_id
        let new_id = Uuid::new_v4().to_string();
        self.set_node_id(&new_id).await?;
        Ok(new_id)
    }

    /// Sets the node identifier
    pub async fn set_node_id(&self, node_id: &str) -> Result<(), SchemaError> {
        self.metadata_store()
            .put_item("node_id", &node_id.to_string())
            .await?;
        self.metadata_store().inner().flush().await?;
        Ok(())
    }

    // ===== Idempotency store operations =====

    /// Retrieve an item from the idempotency store by key.
    pub async fn get_idempotency_item<T: DeserializeOwned + Send + Sync>(
        &self,
        key: &str,
    ) -> Result<Option<T>, SchemaError> {
        self.idempotency_store()
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
        self.idempotency_store().put_item(key, item).await?;
        Ok(())
    }

    // ===== Process results store operations =====

    /// Scan process results by key prefix.
    pub async fn scan_process_results<T: DeserializeOwned + Send + Sync>(
        &self,
        prefix: &str,
    ) -> Result<Vec<(String, T)>, SchemaError> {
        self.process_results_store()
            .scan_items_with_prefix(prefix)
            .await
            .map_err(|e| {
                SchemaError::InvalidData(format!("Failed to scan process results: {}", e))
            })
    }
}
