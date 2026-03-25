use super::core::DbOperations;
use crate::schema::SchemaError;
use uuid::Uuid;

impl DbOperations {
    /// Retrieves or generates and persists the node identifier
    pub async fn get_node_id(&self) -> Result<String, SchemaError> {
        use crate::storage::traits::TypedStore;

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
        use crate::storage::traits::TypedStore;
        self.metadata_store()
            .put_item("node_id", &node_id.to_string())
            .await?;
        self.metadata_store().inner().flush().await?;
        Ok(())
    }
}
