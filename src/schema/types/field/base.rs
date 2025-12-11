use super::common::FieldCommon;
use crate::db_operations::DbOperationsV2;
use serde::{de::DeserializeOwned, Deserialize, Serialize};
use std::collections::HashMap;
use crate::schema::types::declarative_schemas::FieldMapper;

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
    M: DeserializeOwned + Send + Sync + Clone
{
    /// Refresh molecule state from database
    pub async fn refresh_from_db(&mut self, db_ops: &DbOperationsV2) {
        // If we have a molecule_uuid, look up the corresponding Molecule
        if let Some(molecule_uuid) = self.inner.molecule_uuid() {
            let ref_key = format!("ref:{}", molecule_uuid);
            match db_ops.get_item::<M>(&ref_key).await {
                Ok(Some(molecule)) => {
                    self.molecule = Some(molecule);
                }
                Ok(None) => {
                    // Normal behavior for new fields or if cleaned up
                }
                Err(e) => {
                    log::error!("❌ FieldBase error loading molecule from DB: {}", e);
                }
            }
        }
    }
}
