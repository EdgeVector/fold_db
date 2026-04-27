use super::core::DbOperations;
use crate::schema::SchemaError;
use crate::storage::traits::{KvStore, TypedStore};
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
            self.metadata().raw_metadata(),
            self.permissions().raw_permissions(),
            self.schemas().raw_schema_states(),
            self.schemas().raw_schemas(),
            self.permissions().raw_public_keys(),
            self.metadata().raw_idempotency(),
            self.metadata().raw_process_results(),
            self.schemas().raw_superseded_by(),
            self.views().raw_views(),
            self.views().raw_view_states(),
            self.views().raw_transform_field_overrides(),
            self.atoms().raw(),
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
                store.batch_delete_keys(keys_to_delete).await.map_err(|e| {
                    SchemaError::InvalidData(format!("Failed to batch delete keys: {}", e))
                })?;

                total_deleted += count;
            }
        }

        // Purge org embeddings from native_index (Sled + in-memory)
        if let Some(nim) = self.native_index_manager() {
            let emb_count = nim.purge_org_embeddings(org_hash).await?;
            total_deleted += emb_count;
        }

        tracing::info!(
            "successfully purged {} keys for org {}",
            total_deleted,
            org_hash
        );

        Ok(total_deleted)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::storage::traits::TypedStore;
    use crate::storage::{NamespacedStore, SledNamespacedStore, SledPool};
    use std::sync::Arc;

    #[tokio::test]
    async fn test_purge_org_data() {
        // Setup temporary sled pool
        let tmp = tempfile::TempDir::new().unwrap();
        let pool = Arc::new(SledPool::new(tmp.path().to_path_buf()));
        let store = Arc::new(SledNamespacedStore::new(pool)) as Arc<dyn NamespacedStore>;
        let ops = DbOperations::from_namespaced_store(store).await.unwrap();

        let org_hash = "abc123def456";
        let prefix = format!("{}:", org_hash);

        // Put a mix of personal and org data
        let atoms_store = ops.atoms().raw().clone();
        let atoms_store = &atoms_store;

        let personal_key = "atom:uuid-personal";
        let org_key = format!("{}atom:uuid-org", prefix);

        // Store some dummy data (String instead of Atom to simply test dbops)
        atoms_store
            .put_item(personal_key, &"personal_data".to_string())
            .await
            .unwrap();
        atoms_store
            .put_item(&org_key, &"org_data".to_string())
            .await
            .unwrap();

        // Also insert org and personal embedding entries into native_index
        let nim = ops
            .native_index_manager()
            .expect("native_index_manager should exist");
        let native_store = nim.store();

        // Org embedding: key starts with emb:{org_hash}:
        let org_emb_key = format!("emb:{}:test_schema:key1:field1:0", org_hash);
        let org_emb_data = serde_json::json!({
            "schema": format!("{}:test_schema", org_hash),
            "key": {"hash": "key1"},
            "field_name": "field1",
            "fragment_idx": 0,
            "embedding": [0.1, 0.2, 0.3]
        });
        native_store
            .put(
                org_emb_key.as_bytes(),
                serde_json::to_vec(&org_emb_data).unwrap(),
            )
            .await
            .unwrap();

        // Personal embedding: key does NOT have org prefix
        let personal_emb_key = "emb:personal_schema:key2:field1:0";
        let personal_emb_data = serde_json::json!({
            "schema": "personal_schema",
            "key": {"hash": "key2"},
            "field_name": "field1",
            "fragment_idx": 0,
            "embedding": [0.4, 0.5, 0.6]
        });
        native_store
            .put(
                personal_emb_key.as_bytes(),
                serde_json::to_vec(&personal_emb_data).unwrap(),
            )
            .await
            .unwrap();

        // Reload in-memory index so it picks up the entries we just wrote
        nim.reload_embeddings().await;

        // Verify both exist
        let val_p: Option<String> = atoms_store.get_item(personal_key).await.unwrap();
        assert!(val_p.is_some());
        let val_o: Option<String> = atoms_store.get_item(&org_key).await.unwrap();
        assert!(val_o.is_some());

        // Verify embeddings exist in Sled
        let org_emb_before = native_store.get(org_emb_key.as_bytes()).await.unwrap();
        assert!(
            org_emb_before.is_some(),
            "org embedding should exist before purge"
        );
        let personal_emb_before = native_store.get(personal_emb_key.as_bytes()).await.unwrap();
        assert!(
            personal_emb_before.is_some(),
            "personal embedding should exist before purge"
        );

        // Purge org data (now includes embedding purge)
        let deleted_count = ops.purge_org_data(org_hash).await.unwrap();
        // 1 atom key + 1 embedding = 2
        assert_eq!(deleted_count, 2);

        // Verify personal data remains, org data is gone
        let val_p_after: Option<String> = atoms_store.get_item(personal_key).await.unwrap();
        assert!(val_p_after.is_some());
        let val_o_after: Option<String> = atoms_store.get_item(&org_key).await.unwrap();
        assert!(val_o_after.is_none());

        // Verify org embedding is gone from Sled, personal embedding remains
        let org_emb_after = native_store.get(org_emb_key.as_bytes()).await.unwrap();
        assert!(org_emb_after.is_none(), "org embedding should be purged");
        let personal_emb_after = native_store.get(personal_emb_key.as_bytes()).await.unwrap();
        assert!(
            personal_emb_after.is_some(),
            "personal embedding should survive purge"
        );
    }
}
