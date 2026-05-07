use super::common::FieldCommon;
use crate::db_operations::DbOperations;
use crate::schema::types::SchemaError;
use serde::de::DeserializeOwned;

/// Refresh a field's molecule state from the database.
///
/// When the field has an `org_hash`, the ref key is org-prefixed. If
/// nothing is present at the prefixed key, falls back to the unprefixed
/// (personal) key so molecules that existed before a schema was tagged
/// with an `org_hash` remain resolvable. See
/// `docs/designs/org_shared_sync.md` — `set-org-hash` does not rewrite
/// pre-existing keys.
///
/// A successful read populates `molecule_slot`. A read miss leaves the
/// slot untouched and is not an error. A deserialize failure is fatal —
/// it means the bytes at the ref key don't match `M`'s shape, which
/// historically meant a cross-`schema_type` field_mapper carry-over had
/// corrupted molecule storage (see fold_db_node PR #923). Returning
/// `Err` makes the corruption surface immediately at the read/write
/// boundary instead of silently returning empty results from
/// `resolve_value`.
pub async fn refresh_field_from_db<M>(
    inner: &mut FieldCommon,
    molecule_slot: &mut Option<M>,
    db_ops: &DbOperations,
) -> Result<(), SchemaError>
where
    M: DeserializeOwned + Send + Sync + Clone,
{
    let Some(molecule_uuid) = inner.molecule_uuid().cloned() else {
        return Ok(());
    };
    let base_key = format!("ref:{}", molecule_uuid);
    let ref_key = inner.storage_key(&base_key);
    use crate::storage::traits::TypedStore;
    let store = db_ops.atoms().raw();
    match store.get_item::<M>(&ref_key).await {
        Ok(Some(molecule)) => {
            *molecule_slot = Some(molecule);
            return Ok(());
        }
        Ok(None) => {}
        Err(e) => {
            return Err(SchemaError::InvalidData(format!(
                "refresh_field_from_db: failed to deserialize molecule ref \
                 key={} target_type={} error={}",
                ref_key,
                std::any::type_name::<M>(),
                e
            )));
        }
    }

    if inner.org_hash().is_some() {
        match store.get_item::<M>(&base_key).await {
            Ok(Some(molecule)) => {
                tracing::debug!(
                    "refresh_field_from_db: resolved molecule via pre-tag (unprefixed) key"
                );
                *molecule_slot = Some(molecule);
            }
            Ok(None) => {}
            Err(e) => {
                return Err(SchemaError::InvalidData(format!(
                    "refresh_field_from_db: failed to deserialize pre-tag molecule ref \
                     key={} target_type={} error={}",
                    base_key,
                    std::any::type_name::<M>(),
                    e
                )));
            }
        }
    }
    Ok(())
}
