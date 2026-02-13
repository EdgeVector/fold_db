use serde::{Deserialize, Serialize};
use serde_json::Value as JsonValue;
use std::collections::HashMap;
use std::sync::Arc;

use crate::atom::MoleculeRange;
// Removed unused impl_field import
use crate::db_operations::DbOperations;
use crate::schema::types::declarative_schemas::FieldMapper;
use crate::schema::types::field::base::FieldBase;
use crate::schema::types::field::FieldValue;
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

    /// Creates a new RangeField with a MoleculeRange
    #[must_use]
    pub fn new_with_range(
        field_mappers: HashMap<String, FieldMapper>,
        source_pub_key: String,
    ) -> Self {
        Self {
            base: FieldBase::new(field_mappers, Some(MoleculeRange::new(source_pub_key))),
        }
    }

    /// Returns a reference to the MoleculeRange if it exists
    pub fn molecule(&self) -> Option<&MoleculeRange> {
        self.base.molecule.as_ref()
    }

    /// Returns a mutable reference to the MoleculeRange if it exists
    pub fn molecule_mut(&mut self) -> Option<&mut MoleculeRange> {
        self.base.molecule.as_mut()
    }

    /// Sets the MoleculeRange for this field
    pub fn set_molecule(&mut self, molecule: MoleculeRange) {
        self.base.molecule = Some(molecule);
    }

    /// Initializes the MoleculeRange if it doesn't exist
    pub fn ensure_molecule(&mut self, source_pub_key: String) -> &mut MoleculeRange {
        if self.base.molecule.is_none() {
            self.base.molecule = Some(MoleculeRange::new(source_pub_key));
        }
        self.base.molecule.as_mut().unwrap()
    }

    /// Applies a filter from a JSON Value (delegates to trait default)
    pub fn apply_json_filter(
        &self,
        filter_value: &JsonValue,
    ) -> Result<HashRangeFilterResult, String> {
        serde_json::from_value::<HashRangeFilter>(filter_value.clone())
            .map(|f| self.apply_filter(Some(f)))
            .or_else(|_| Ok(self.apply_filter(None)))
    }

    /// Gets all keys in the range (useful for pagination or listing)
    pub fn get_all_keys(&self) -> Vec<String> {
        self.base
            .molecule
            .as_ref()
            .map(|range| range.atom_uuids.keys().cloned().collect())
            .unwrap_or_default()
    }

    /// Gets a subset of keys within a range (useful for pagination)
    pub fn get_keys_in_range(&self, start: &str, end: &str) -> Vec<String> {
        self.base
            .molecule
            .as_ref()
            .map(|range| {
                // Leverage BTree's efficient range operations
                range
                    .atom_uuids
                    .range(start.to_string()..end.to_string())
                    .map(|(key, _)| key.clone())
                    .collect()
            })
            .unwrap_or_default()
    }

    /// Gets the total count of items in the range
    pub fn count(&self) -> usize {
        self.base
            .molecule
            .as_ref()
            .map(|range| range.atom_uuids.len())
            .unwrap_or(0)
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
        atom: crate::atom::Atom,
        pub_key: String,
    ) {
        // Initialize molecule if needed and set molecule_uuid in FieldCommon
        if self.base.molecule.is_none() {
            self.ensure_molecule(pub_key.clone());
            // After creating the molecule, get its UUID and set it in FieldCommon
            if let Some(mol) = &self.base.molecule {
                self.base.inner.set_molecule_uuid(mol.uuid().to_string());
            }
        }

        // For RangeField, we use the range key to store the atom
        if let Some(range_key) = &key_value.range {
            if let Some(molecule) = &mut self.base.molecule {
                molecule.set_atom_uuid(range_key.clone(), atom.uuid().to_string());
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
        super::fetch_atoms_for_matches_async(db_ops, result.matches).await
    }
}
