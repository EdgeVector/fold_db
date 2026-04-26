use super::core::DbOperations;
use crate::schema::types::field::build_storage_key;
use crate::schema::SchemaError;
use crate::storage::traits::TypedStore;
use crate::sync::SyncConflict;

impl DbOperations {
    /// List all unresolved sync conflicts, optionally filtered by molecule UUID.
    /// When `org_hash` is `Some`, scans org-prefixed keys.
    pub async fn get_unresolved_conflicts(
        &self,
        molecule_uuid: Option<&str>,
        org_hash: Option<&str>,
    ) -> Result<Vec<SyncConflict>, SchemaError> {
        let base_prefix = match molecule_uuid {
            Some(mol) => format!("conflict:{}:", mol),
            None => "conflict:".to_string(),
        };
        let prefix = build_storage_key(org_hash, &base_prefix);

        let items: Vec<(String, SyncConflict)> = self
            .atoms()
            .raw()
            .scan_items_with_prefix(&prefix)
            .await
            .map_err(|e| SchemaError::InvalidData(format!("Failed to scan conflicts: {}", e)))?;

        let conflicts: Vec<SyncConflict> = items
            .into_iter()
            .map(|(_, c)| c)
            .filter(|c| !c.resolved)
            .collect();

        Ok(conflicts)
    }

    /// Mark a conflict as resolved by its ID.
    /// When `org_hash` is `Some`, keys are org-prefixed.
    pub async fn resolve_conflict(
        &self,
        conflict_id: &str,
        org_hash: Option<&str>,
    ) -> Result<(), SchemaError> {
        // The conflict_id is "{mol_uuid}:{ts}", stored at key "conflict:{id}"
        let base_key = format!("conflict:{}", conflict_id);
        let key = build_storage_key(org_hash, &base_key);

        let mut conflict: SyncConflict = self
            .atoms()
            .raw()
            .get_item(&key)
            .await
            .map_err(|e| SchemaError::InvalidData(format!("Failed to read conflict: {}", e)))?
            .ok_or_else(|| SchemaError::NotFound(format!("Conflict not found: {}", conflict_id)))?;

        conflict.resolved = true;
        self.atoms()
            .raw()
            .put_item(&key, &conflict)
            .await
            .map_err(|e| SchemaError::InvalidData(format!("Failed to update conflict: {}", e)))?;

        Ok(())
    }
}
