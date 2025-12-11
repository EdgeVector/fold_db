use super::core::DbOperations;
use crate::schema::SchemaError;
use crate::schema::types::Transform;

impl DbOperations {
    /// Synchronize in-memory transform state with persistent storage
    pub async fn sync_transform_state(
        &self,
        registered_transforms: &std::collections::HashMap<String, Transform>,
        schema_field_to_transforms: &std::collections::BTreeMap<String, std::collections::HashSet<String>>,
    ) -> Result<(), SchemaError> {
        use crate::storage::traits::TypedStore;
        
        // Store all transforms
        for (id, transform) in registered_transforms {
            self.transforms_store().put_item(id, transform).await
                .map_err(|e| SchemaError::InvalidData(format!("Failed to store transform {}: {}", id, e)))?;
        }
        
        // Store schema-field mappings
        self.transforms_store().put_item("__schema_field_mappings__", schema_field_to_transforms).await
            .map_err(|e| SchemaError::InvalidData(format!("Failed to store transform mappings: {}", e)))?;
        
        // Flush to ensure persistence
        self.transforms_store().inner().flush().await
            .map_err(|e| SchemaError::InvalidData(format!("Failed to flush transforms: {}", e)))?;
        
        Ok(())
    }
    
    /// Load persisted transform state from storage
    pub async fn load_transform_state(
        &self,
    ) -> Result<(std::collections::HashMap<String, Transform>, std::collections::BTreeMap<String, std::collections::HashSet<String>>), SchemaError> {
        use crate::storage::traits::TypedStore;
        
        // Load all transforms
        let keys = match self.transforms_store().list_keys_with_prefix("").await {
            Ok(k) => k,
            Err(e) => {
                // If listing keys fails, return empty state (fresh start)
                log::warn!("Failed to list transform keys (possibly fresh DB): {}", e);
                return Ok((std::collections::HashMap::new(), std::collections::BTreeMap::new()));
            }
        };
        
        let mut transforms = std::collections::HashMap::new();
        for key in keys {
            if key == "__schema_field_mappings__" {
                continue; // Skip the mappings key
            }
            // Gracefully handle deserialization errors (e.g., format changes)
            match self.transforms_store().get_item::<Transform>(&key).await {
                Ok(Some(transform)) => {
                    transforms.insert(key, transform);
                }
                Ok(None) => {
                    // Transform not found - skip
                }
                Err(e) => {
                    log::warn!("Failed to deserialize transform {} (possibly old format): {}", key, e);
                    // Continue loading other transforms
                }
            }
        }
        
        // Load schema-field mappings (gracefully handle missing/invalid data)
        let mappings = match self.transforms_store().get_item("__schema_field_mappings__").await {
            Ok(Some(m)) => m,
            Ok(None) => {
                log::info!("No transform mappings found - starting with empty state");
                std::collections::BTreeMap::new()
            }
            Err(e) => {
                log::warn!("Failed to load transform mappings (possibly old format): {}", e);
                std::collections::BTreeMap::new()
            }
        };
        
        Ok((transforms, mappings))
    }
}
