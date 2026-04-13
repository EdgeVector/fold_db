//! Thin delegator methods on `DbOperations` for public-key persistence.
//! Real implementations live on
//! [`super::permissions_store::PermissionsStore`].

use super::core::DbOperations;
use crate::schema::SchemaError;
use crate::security::PublicKeyInfo;

impl DbOperations {
    /// Gets the system-wide public key
    pub async fn get_system_public_key(&self) -> Result<Option<PublicKeyInfo>, SchemaError> {
        self.permissions().get_system_public_key().await
    }

    /// Stores the system-wide public key
    pub async fn store_system_public_key(
        &self,
        key_info: &PublicKeyInfo,
    ) -> Result<(), SchemaError> {
        self.permissions().store_system_public_key(key_info).await
    }

    /// Deletes the system-wide public key
    pub async fn delete_system_public_key(&self) -> Result<bool, SchemaError> {
        self.permissions().delete_system_public_key().await
    }
}
