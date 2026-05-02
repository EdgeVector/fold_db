use super::common::FieldCommon;
use crate::db_operations::DbOperations;
use serde::de::DeserializeOwned;

/// Refresh a field's molecule state from the database.
///
/// When the field has an `org_hash`, the ref key is org-prefixed. If
/// nothing is present at the prefixed key, falls back to the unprefixed
/// (personal) key so molecules that existed before a schema was tagged
/// with an `org_hash` remain resolvable. See
/// `docs/designs/org_shared_sync.md` — `set-org-hash` does not rewrite
/// pre-existing keys.
pub async fn refresh_field_from_db<M>(
    inner: &mut FieldCommon,
    molecule_slot: &mut Option<M>,
    db_ops: &DbOperations,
) where
    M: DeserializeOwned + Send + Sync + Clone,
{
    if let Some(molecule_uuid) = inner.molecule_uuid() {
        let base_key = format!("ref:{}", molecule_uuid);
        let ref_key = inner.storage_key(&base_key);
        use crate::storage::traits::TypedStore;
        let store = db_ops.atoms().raw();
        match store.get_item::<M>(&ref_key).await {
            Ok(Some(molecule)) => {
                *molecule_slot = Some(molecule);
                return;
            }
            Ok(None) => {}
            Err(e) => {
                // Non-fatal: molecule ref may be in an old serialization format.
                // The field still works — data is read from atoms directly.
                tracing::warn!(
                    "FieldBase: skipping molecule ref for {}: {}",
                    molecule_uuid,
                    e
                );
                return;
            }
        }

        if inner.org_hash().is_some() {
            match store.get_item::<M>(&base_key).await {
                Ok(Some(molecule)) => {
                    tracing::debug!("FieldBase: resolved molecule via pre-tag (unprefixed) key");
                    *molecule_slot = Some(molecule);
                }
                Ok(None) => {}
                Err(e) => {
                    tracing::warn!("FieldBase: pre-tag fallback for molecule ref failed: {}", e);
                }
            }
        }
    }
}
