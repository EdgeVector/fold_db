use super::common::FieldCommon;
use crate::db_operations::DbOperations;
use crate::schema::types::declarative_schemas::FieldMapper;
use serde::{de::DeserializeOwned, Deserialize, Serialize};
use std::collections::HashMap;

/// Base generic implementation for schema fields
///
/// Encapsulates common state and logic:
/// - `inner`: FieldCommon metadata
/// - `molecule`: Optional type-specific molecule (state)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FieldBase<M> {
    pub inner: FieldCommon,
    pub molecule: Option<M>,
}

impl<M> FieldBase<M> {
    /// Create a new FieldBase
    pub fn new(field_mappers: HashMap<String, FieldMapper>, molecule: Option<M>) -> Self {
        Self {
            inner: FieldCommon::new(field_mappers),
            molecule,
        }
    }

    /// Access inner common configuration
    pub fn common(&self) -> &FieldCommon {
        &self.inner
    }

    /// Mutable access to inner common configuration
    pub fn common_mut(&mut self) -> &mut FieldCommon {
        &mut self.inner
    }
}

impl<M> FieldBase<M>
where
    M: DeserializeOwned + Send + Sync + Clone,
{
    /// Refresh molecule state from database.
    ///
    /// When the field has an `org_hash`, the ref key is org-prefixed. If
    /// nothing is present at the prefixed key, falls back to the unprefixed
    /// (personal) key so molecules that existed before a schema was tagged
    /// with an `org_hash` remain resolvable. See
    /// `docs/designs/org_shared_sync.md` — `set-org-hash` does not rewrite
    /// pre-existing keys.
    pub async fn refresh_from_db(&mut self, db_ops: &DbOperations) {
        if let Some(molecule_uuid) = self.inner.molecule_uuid() {
            let base_key = format!("ref:{}", molecule_uuid);
            let ref_key = self.inner.storage_key(&base_key);
            use crate::storage::traits::TypedStore;
            let store = db_ops.atoms().raw();
            match store.get_item::<M>(&ref_key).await {
                Ok(Some(molecule)) => {
                    self.molecule = Some(molecule);
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

            if self.inner.org_hash().is_some() {
                match store.get_item::<M>(&base_key).await {
                    Ok(Some(molecule)) => {
                        tracing::debug!(
                            "FieldBase: resolved molecule via pre-tag (unprefixed) key"
                        );
                        self.molecule = Some(molecule);
                    }
                    Ok(None) => {}
                    Err(e) => {
                        tracing::warn!(
                            "FieldBase: pre-tag fallback for molecule ref failed: {}",
                            e
                        );
                    }
                }
            }
        }
    }
}
