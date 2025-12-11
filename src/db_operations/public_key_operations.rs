use super::core::DbOperations;
use crate::schema::SchemaError;
use crate::security::PublicKeyInfo;
use crate::constants::SINGLE_PUBLIC_KEY_ID;

impl DbOperations {
    /// Gets the system-wide public key
    pub async fn get_system_public_key(&self) -> Result<Option<PublicKeyInfo>, SchemaError> {
        use crate::storage::traits::TypedStore;
        
        self.public_keys_store().get_item(SINGLE_PUBLIC_KEY_ID).await
            .map_err(|e| SchemaError::InvalidData(format!("Failed to get public key: {}", e)))
    }

    /// Stores the system-wide public key
    pub async fn store_system_public_key(&self, key_info: &PublicKeyInfo) -> Result<(), SchemaError> {
        use crate::storage::traits::TypedStore;
        
        self.public_keys_store().put_item(SINGLE_PUBLIC_KEY_ID, key_info).await
            .map_err(|e| SchemaError::InvalidData(format!("Failed to store public key: {}", e)))?;
        
        // Flush to ensure persistence
        self.public_keys_store().inner().flush().await
            .map_err(|e| SchemaError::InvalidData(format!("Failed to flush public keys: {}", e)))?;
        
        Ok(())
    }

    /// Deletes the system-wide public key
    pub async fn delete_system_public_key(&self) -> Result<bool, SchemaError> {
        use crate::storage::traits::TypedStore;
        
        self.public_keys_store().delete_item(SINGLE_PUBLIC_KEY_ID).await
            .map_err(|e| SchemaError::InvalidData(format!("Failed to delete public key: {}", e)))
    }
}
