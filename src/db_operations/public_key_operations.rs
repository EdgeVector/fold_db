use crate::constants::SINGLE_PUBLIC_KEY_ID;
use crate::db_operations::error_utils::ErrorUtils;
use crate::db_operations::DbOperations;
use crate::logging::features::{log_feature, LogFeature};
use crate::schema::types::SchemaError;
use crate::security::types::PublicKeyInfo;

impl DbOperations {
    /// Store the system-wide public key. This will overwrite any existing key.
    pub fn set_system_public_key(&self, key_info: &PublicKeyInfo) -> Result<(), SchemaError> {
        // Clear all existing keys to ensure only one remains.
        self.public_keys_tree
            .clear()
            .map_err(|e| ErrorUtils::database_error("clear public keys tree", e))?;

        // Clone and overwrite the ID to enforce a single key.
        let mut key_to_store = key_info.clone();
        key_to_store.id = SINGLE_PUBLIC_KEY_ID.to_string();

        self.store_in_tree(&self.public_keys_tree, SINGLE_PUBLIC_KEY_ID, &key_to_store)
    }

    /// Retrieve the system-wide public key.
    pub fn get_system_public_key(&self) -> Result<Option<PublicKeyInfo>, SchemaError> {
        self.get_from_tree(&self.public_keys_tree, SINGLE_PUBLIC_KEY_ID)
    }

    /// Delete the system-wide public key from the database.
    pub fn delete_system_public_key(&self) -> Result<bool, SchemaError> {
        match self
            .public_keys_tree
            .remove(SINGLE_PUBLIC_KEY_ID.as_bytes())
        {
            Ok(old_value) => Ok(old_value.is_some()),
            Err(e) => Err(ErrorUtils::database_error("delete system public key", e)),
        }
    }

    pub fn close(&self) {
        if let Err(e) = self.db().flush() {
            log_feature!(
                LogFeature::Database,
                error,
                "Failed to flush database: {}",
                e
            );
        }
    }
}

#[cfg(test)]
mod tests {
    use crate::constants::SINGLE_PUBLIC_KEY_ID;
    use crate::security::types::PublicKeyInfo;
    use crate::testing_utils::TestDatabaseFactory;

    #[test]
    fn test_set_and_get_system_public_key() {
        let (db_ops, _) = TestDatabaseFactory::create_test_environment().unwrap();

        let key_info = PublicKeyInfo::new(
            "some_id_that_will_be_ignored".to_string(),
            "test_public_key_base64".to_string(),
            "test_owner".to_string(),
            vec!["read".to_string()],
        );

        // Store the key
        db_ops.set_system_public_key(&key_info).unwrap();

        // Retrieve the key
        let retrieved = db_ops.get_system_public_key().unwrap();
        assert!(retrieved.is_some());
        let retrieved_key = retrieved.unwrap();
        assert_eq!(retrieved_key.id, SINGLE_PUBLIC_KEY_ID);
        assert_eq!(retrieved_key.public_key, "test_public_key_base64");
        assert_eq!(retrieved_key.owner_id, "test_owner");
        assert_eq!(retrieved_key.permissions, vec!["read".to_string()]);
    }

    #[test]
    fn test_get_nonexistent_system_public_key() {
        let (db_ops, _) = TestDatabaseFactory::create_test_environment().unwrap();

        // Try to retrieve a key that doesn't exist
        let result = db_ops.get_system_public_key().unwrap();
        assert!(result.is_none());
    }

    #[test]
    fn test_delete_system_public_key() {
        let (db_ops, _) = TestDatabaseFactory::create_test_environment().unwrap();

        let key_info = PublicKeyInfo::new(
            "key_to_delete".to_string(),
            "test_key".to_string(),
            "test_owner".to_string(),
            vec!["read".to_string()],
        );
        db_ops.set_system_public_key(&key_info).unwrap();

        // Make sure it's there
        assert!(db_ops.get_system_public_key().unwrap().is_some());

        // Delete the key
        let deleted = db_ops.delete_system_public_key().unwrap();
        assert!(deleted);

        // Verify deletion
        assert!(db_ops.get_system_public_key().unwrap().is_none());

        // Try to delete it again
        let deleted_again = db_ops.delete_system_public_key().unwrap();
        assert!(!deleted_again);
    }

    #[test]
    fn test_set_system_public_key_overwrites() {
        let (db_ops, _) = TestDatabaseFactory::create_test_environment().unwrap();

        // Store initial key
        let key1 = PublicKeyInfo::new(
            "key_1".to_string(),
            "public_key_1".to_string(),
            "owner_1".to_string(),
            vec!["read".to_string()],
        );
        db_ops.set_system_public_key(&key1).unwrap();

        let retrieved1 = db_ops.get_system_public_key().unwrap().unwrap();
        assert_eq!(retrieved1.public_key, "public_key_1");
        assert_eq!(retrieved1.id, SINGLE_PUBLIC_KEY_ID);

        // Store another key, which should overwrite the first one.
        let key2 = PublicKeyInfo::new(
            "key_2".to_string(),
            "public_key_2".to_string(),
            "owner_2".to_string(),
            vec!["write".to_string()],
        );

        db_ops.set_system_public_key(&key2).unwrap();

        // Get the key and check it's the second one.
        let retrieved2 = db_ops.get_system_public_key().unwrap().unwrap();
        assert_eq!(retrieved2.public_key, "public_key_2");
        assert_eq!(retrieved2.owner_id, "owner_2");
        assert_eq!(retrieved2.id, SINGLE_PUBLIC_KEY_ID);
    }
}
