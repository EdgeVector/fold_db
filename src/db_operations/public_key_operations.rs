use super::core::DbOperations;
use crate::constants::SINGLE_PUBLIC_KEY_ID;
use crate::schema::SchemaError;
use crate::security::PublicKeyInfo;
use crate::storage::traits::TypedStore;

impl DbOperations {
    /// Gets the system-wide public key
    pub async fn get_system_public_key(&self) -> Result<Option<PublicKeyInfo>, SchemaError> {
        Ok(self
            .public_keys_store()
            .get_item(SINGLE_PUBLIC_KEY_ID)
            .await?)
    }

    /// Stores the system-wide public key
    pub async fn store_system_public_key(
        &self,
        key_info: &PublicKeyInfo,
    ) -> Result<(), SchemaError> {
        self.public_keys_store()
            .put_item(SINGLE_PUBLIC_KEY_ID, key_info)
            .await?;
        self.public_keys_store().inner().flush().await?;
        Ok(())
    }

    /// Deletes the system-wide public key
    pub async fn delete_system_public_key(&self) -> Result<bool, SchemaError> {
        Ok(self
            .public_keys_store()
            .delete_item(SINGLE_PUBLIC_KEY_ID)
            .await?)
    }
}
