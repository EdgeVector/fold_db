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
                log::warn!("Failed to deserialize node_id (possibly old format): {}, creating new", e);
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
        
        self.metadata_store().put_item("node_id", &node_id.to_string()).await
            .map_err(|e| SchemaError::InvalidData(format!("Failed to set node_id: {}", e)))?;
        
        // Flush to ensure persistence
        self.metadata_store().inner().flush().await
            .map_err(|e| SchemaError::InvalidData(format!("Failed to flush metadata: {}", e)))?;
        
        Ok(())
    }

    /// Retrieves the list of permitted schemas for the given node
    pub async fn get_schema_permissions(&self, node_id: &str) -> Result<Vec<String>, SchemaError> {
        use crate::storage::traits::TypedStore;
        
        self.permissions_store().get_item::<Vec<String>>(node_id).await
            .map_err(|e| SchemaError::InvalidData(format!("Failed to get permissions: {}", e)))
            .map(|opt| opt.unwrap_or_default())
    }

    /// Sets the permitted schemas for the given node
    pub async fn set_schema_permissions(
        &self,
        node_id: &str,
        schemas: &[String],
    ) -> Result<(), SchemaError> {
        use crate::storage::traits::TypedStore;
        
        let schemas_vec: Vec<String> = schemas.to_vec();
        self.permissions_store().put_item(node_id, &schemas_vec).await
            .map_err(|e| SchemaError::InvalidData(format!("Failed to set permissions: {}", e)))
    }

    /// Lists all nodes with permissions
    pub async fn list_nodes_with_permissions(&self) -> Result<Vec<String>, SchemaError> {
        use crate::storage::traits::TypedStore;
        
        self.permissions_store().list_keys_with_prefix("").await
            .map_err(|e| SchemaError::InvalidData(format!("Failed to list nodes: {}", e)))
    }

    /// Deletes permissions for a node
    pub async fn delete_schema_permissions(&self, node_id: &str) -> Result<bool, SchemaError> {
        use crate::storage::traits::TypedStore;
        
        self.permissions_store().delete_item(node_id).await
            .map_err(|e| SchemaError::InvalidData(format!("Failed to delete permissions: {}", e)))
    }

    /// Checks if a node has permissions set
    pub async fn node_has_permissions(&self, node_id: &str) -> Result<bool, SchemaError> {
        use crate::storage::traits::TypedStore;
        
        self.permissions_store().exists_item(node_id).await
            .map_err(|e| SchemaError::InvalidData(format!("Failed to check permissions: {}", e)))
    }
}

