use super::core::DbOperations;
use crate::schema::SchemaError;
use crate::storage::traits::{TypedStore, KvStore};
use crate::storage::TypedKvStore;
use std::sync::Arc;

impl DbOperations {
    /// Securely purges all organization-prefixed data from the local database.
    /// 
    /// This method iterates through every namespace store and performs a prefix scan
    /// for `{org_hash}:`. It then batch deletes those keys, effectively removing all
    /// schemas, atoms, molecules, and history events that were synced under this org's
    /// encrypted shared prefix.
    /// 
    /// Personal data (which lacks the prefix) will remain completely untouched.
    /// 
    /// Returns the total number of keys purged.
    pub async fn purge_org_data(&self, org_hash: &str) -> Result<usize, SchemaError> {
        let prefix = format!("{}:", org_hash);
        let mut total_deleted = 0;

        // Collect all namespace stores
        let stores: Vec<&Arc<TypedKvStore<dyn KvStore>>> = vec![
            self.metadata_store(),
            self.permissions_store(),
            self.schema_states_store(),
            self.schemas_store(),
            self.public_keys_store(),
            self.idempotency_store(),
            self.process_results_store(),
            self.superseded_by_store(),
            self.views_store(),
            self.view_states_store(),
            self.transform_field_states_store(),
            self.atoms_store(),
        ];

        for store in stores {
            // Find all keys in this store that start with the org prefix
            let keys_to_delete = store
                .list_keys_with_prefix(&prefix)
                .await
                .map_err(|e| SchemaError::InvalidData(format!("Failed to scan prefix: {}", e)))?;

            if !keys_to_delete.is_empty() {
                let count = keys_to_delete.len();
                // Batch delete them
                store
                    .batch_delete_keys(keys_to_delete)
                    .await
                    .map_err(|e| SchemaError::InvalidData(format!("Failed to batch delete keys: {}", e)))?;
                
                total_deleted += count;
            }
        }

        log::info!("successfully purged {} keys for org {}", total_deleted, org_hash);

        Ok(total_deleted)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::storage::{NamespacedStore, SledNamespacedStore};
    use crate::storage::traits::TypedStore;
    use std::sync::Arc;

    #[tokio::test]
    async fn test_purge_org_data() {
        // Setup in-memory sled db
        let sled_db = sled::Config::new().temporary(true).open().unwrap();
        let store = Arc::new(SledNamespacedStore::new(sled_db.clone())) as Arc<dyn NamespacedStore>;
        let ops = DbOperations::from_namespaced_store(store).await.unwrap();

        let org_hash = "abc123def456";
        let prefix = format!("{}:", org_hash);

        // Put a mix of personal and org data
        let atoms_store = ops.atoms_store();
        
        let personal_key = "atom:uuid-personal";
        let org_key = format!("{}atom:uuid-org", prefix);

        // Store some dummy data (String instead of Atom to simply test dbops)
        atoms_store.put_item(personal_key, &"personal_data".to_string()).await.unwrap();
        atoms_store.put_item(&org_key, &"org_data".to_string()).await.unwrap();

        // Verify both exist
        let val_p: Option<String> = atoms_store.get_item(personal_key).await.unwrap();
        assert!(val_p.is_some());
        let val_o: Option<String> = atoms_store.get_item(&org_key).await.unwrap();
        assert!(val_o.is_some());

        // Purge org data
        let deleted_count = ops.purge_org_data(org_hash).await.unwrap();
        assert_eq!(deleted_count, 1);

        // Verify personal remains, org is gone
        let val_p_after: Option<String> = atoms_store.get_item(personal_key).await.unwrap();
        assert!(val_p_after.is_some());
        
        let val_o_after: Option<String> = atoms_store.get_item(&org_key).await.unwrap();
        assert!(val_o_after.is_none());
    }
}
