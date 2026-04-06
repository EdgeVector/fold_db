use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::sync::Arc;

use crate::atom::MoleculeRange;
// Removed unused impl_field import
use crate::db_operations::DbOperations;
use crate::schema::types::declarative_schemas::FieldMapper;
use crate::schema::types::field::base::FieldBase;
use crate::schema::types::field::FieldValue;
use crate::schema::types::field::WriteContext;
use crate::schema::types::field::{
    apply_range_filter, FilterApplicator, HashRangeFilter, HashRangeFilterResult,
};
use crate::schema::types::key_value::KeyValue;
use crate::schema::types::SchemaError;

// RangeFilter has been unified into HashRangeFilter
/// Field storing a range of values.
#[derive(Debug, Clone, Serialize, Deserialize, utoipa::ToSchema)]
pub struct RangeField {
    #[serde(flatten)]
    pub base: FieldBase<MoleculeRange>,
}

impl RangeField {
    #[must_use]
    pub fn new(
        field_mappers: HashMap<String, FieldMapper>,
        molecule: Option<MoleculeRange>,
    ) -> Self {
        Self {
            base: FieldBase::new(field_mappers, molecule),
        }
    }

    /// Initializes the MoleculeRange if it doesn't exist
    pub fn ensure_molecule(&mut self, schema_name: &str, field_name: &str) -> &mut MoleculeRange {
        if self.base.molecule.is_none() {
            self.base.molecule = Some(MoleculeRange::new(schema_name, field_name));
        }
        self.base.molecule.as_mut().unwrap()
    }

    /// Gets all keys in the range (useful for pagination or listing)
    pub fn get_all_keys(&self) -> Vec<String> {
        self.base
            .molecule
            .as_ref()
            .map(|range| range.atom_uuids.keys().cloned().collect())
            .unwrap_or_default()
    }
}

impl FilterApplicator for RangeField {
    fn apply_filter(&self, filter: Option<HashRangeFilter>) -> HashRangeFilterResult {
        let Some(molecule) = &self.base.molecule else {
            return HashRangeFilterResult::empty();
        };

        apply_range_filter(molecule, filter)
    }
}

#[async_trait::async_trait]
impl crate::schema::types::field::Field for RangeField {
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
            self.ensure_molecule(&ctx.schema_name, &ctx.field_name);
            // After creating the molecule, get its UUID and set it in FieldCommon
            if let Some(mol) = &self.base.molecule {
                self.base.inner.set_molecule_uuid(mol.uuid().to_string());
            }
        }

        // For RangeField, we use the range key to store the atom
        if let Some(range_key) = &key_value.range {
            if let Some(molecule) = &mut self.base.molecule {
                molecule.set_atom_uuid(range_key.clone(), ctx.atom.uuid().to_string());
                // Store per-key metadata on the molecule
                molecule.set_key_metadata(
                    range_key.clone(),
                    crate::atom::KeyMetadata {
                        source_file_name: ctx.source_file_name,
                        metadata: ctx.metadata,
                    },
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

        // Fetch actual atom content from database using shared helper
        let result = self.apply_filter(filter);
        // Attach per-key metadata from molecule to each match
        let matches_with_meta: Vec<(KeyValue, String, Option<crate::atom::KeyMetadata>)> = result
            .matches
            .into_iter()
            .map(|(kv, atom_uuid)| {
                let key_meta = kv.range.as_ref().and_then(|r| {
                    self.base
                        .molecule
                        .as_ref()
                        .and_then(|m| m.get_key_metadata(r).cloned())
                });
                (kv, atom_uuid, key_meta)
            })
            .collect();
        super::fetch_atoms_with_key_metadata_async_with_org(
            db_ops,
            matches_with_meta.into_iter(),
            self.base.inner.org_hash(),
        )
        .await
    }
}
