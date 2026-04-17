//! HashRange field type for schema indexing iterator stack model
//!
//! Provides a field type that combines hash and range functionality for
//! efficient indexing with complex fan-out operations.

// Removed unused impl_field import
use crate::db_operations::DbOperations;
use crate::schema::types::declarative_schemas::FieldMapper;
use crate::schema::types::field::hash_range_filter::{HashRangeFilter, HashRangeFilterResult};
use crate::schema::types::field::FieldValue;
use crate::schema::types::field::WriteContext;
use crate::schema::types::field::{apply_hash_range_filter, FilterApplicator};
use crate::schema::types::key_value::KeyValue;
use crate::schema::types::SchemaError;
use serde::{Deserialize, Serialize};
// Removed unused JsonValue import
use crate::atom::MoleculeHashRange;
use crate::schema::types::field::base::FieldBase;
use std::collections::HashMap;
use std::sync::Arc;
// Removed unused log imports

/// Field that combines hash and range functionality for indexing
#[derive(Debug, Clone, Serialize, Deserialize, utoipa::ToSchema)]
pub struct HashRangeField {
    #[serde(flatten)]
    pub base: FieldBase<MoleculeHashRange>,
}

impl HashRangeField {
    /// Creates a new HashRange field
    #[must_use]
    pub fn new(
        field_mappers: HashMap<String, FieldMapper>,
        molecule: Option<MoleculeHashRange>,
    ) -> Self {
        Self {
            base: FieldBase::new(field_mappers, molecule),
        }
    }
}

#[async_trait::async_trait]
impl crate::schema::types::field::Field for HashRangeField {
    fn common(&self) -> &crate::schema::types::field::FieldCommon {
        &self.base.inner
    }

    fn common_mut(&mut self) -> &mut crate::schema::types::field::FieldCommon {
        &mut self.base.inner
    }

    async fn refresh_from_db(&mut self, db_ops: &crate::db_operations::DbOperations) {
        self.base.refresh_from_db(db_ops).await;
    }

    fn write_mutation(
        &mut self,
        key_value: &crate::schema::types::key_value::KeyValue,
        ctx: WriteContext,
    ) {
        // Initialize molecule if needed and set molecule_uuid in FieldCommon
        if self.base.molecule.is_none() {
            let new_molecule =
                crate::atom::MoleculeHashRange::new(&ctx.schema_name, &ctx.field_name);
            // Get the molecule's UUID and set it in FieldCommon for persistence lookup
            self.base
                .inner
                .set_molecule_uuid(new_molecule.uuid().to_string());
            self.base.molecule = Some(new_molecule);
        }

        // For HashRangeField, we use both hash and range keys to store the atom
        match (&key_value.hash, &key_value.range) {
            (Some(hash_key), Some(range_key)) => {
                if let Some(molecule) = &mut self.base.molecule {
                    molecule.set_atom_uuid_from_values(
                        hash_key.clone(),
                        range_key.clone(),
                        ctx.atom.uuid().to_string(),
                        &ctx.signer,
                    );
                    // Store per-key metadata on the molecule
                    molecule.set_key_metadata(
                        hash_key.clone(),
                        range_key.clone(),
                        crate::atom::KeyMetadata {
                            source_file_name: ctx.source_file_name,
                            metadata: ctx.metadata,
                        },
                    );
                }
            }
            _ => {
                log::warn!(
                    "HashRangeField::write_mutation: atom {} not indexed — hash={:?}, range={:?}. \
                     Both hash and range keys are required for HashRange fields.",
                    ctx.atom.uuid(),
                    key_value.hash,
                    key_value.range
                );
            }
        }
    }

    async fn resolve_value(
        &mut self,
        db_ops: &Arc<DbOperations>,
        filter: Option<HashRangeFilter>,
        _as_of: Option<chrono::DateTime<chrono::Utc>>,
    ) -> Result<HashMap<KeyValue, FieldValue>, SchemaError> {
        self.refresh_from_db(db_ops).await;
        let result = self.apply_filter(filter);
        if result.matches.is_empty() {
            // No matches found
        }
        // Attach per-key metadata from molecule to each match
        let matches_with_meta: Vec<(KeyValue, String, Option<crate::atom::KeyMetadata>)> = result
            .matches
            .into_iter()
            .map(|(kv, atom_uuid)| {
                let key_meta = match (&kv.hash, &kv.range) {
                    (Some(h), Some(r)) => self
                        .base
                        .molecule
                        .as_ref()
                        .and_then(|m| m.get_key_metadata(h, r).cloned()),
                    _ => None,
                };
                (kv, atom_uuid, key_meta)
            })
            .collect();
        super::fetch_atoms_with_key_metadata_async_with_org(
            db_ops,
            matches_with_meta,
            self.base.inner.org_hash(),
        )
        .await
    }
}

impl HashRangeField {
    /// Gets all keys in the hash range (useful for pagination or listing)
    /// Returns composite keys in the format "hash_value:range_value"
    pub fn get_all_keys(&self) -> Vec<KeyValue> {
        self.base
            .molecule
            .as_ref()
            .map(|molecule| {
                molecule
                    .iter_all_atoms()
                    .map(|(hash_value, range_key, _)| {
                        KeyValue::new(Some(hash_value.clone()), Some(range_key.clone()))
                    })
                    .collect()
            })
            .unwrap_or_default()
    }

    /// Gets all hash values in the molecule
    pub fn get_hash_values(&self) -> Vec<String> {
        self.base
            .molecule
            .as_ref()
            .map(|molecule| molecule.hash_values().cloned().collect())
            .unwrap_or_default()
    }
}

impl FilterApplicator for HashRangeField {
    fn apply_filter(&self, filter: Option<HashRangeFilter>) -> HashRangeFilterResult {
        let Some(molecule) = &self.base.molecule else {
            return HashRangeFilterResult::empty();
        };

        apply_hash_range_filter(molecule, filter)
    }
}
