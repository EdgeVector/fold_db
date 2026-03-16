//! Hash field type for schema indexing
//!
//! Provides a field type keyed by a single hash key (unordered collection).

use crate::db_operations::DbOperations;
use crate::schema::types::declarative_schemas::FieldMapper;
use crate::schema::types::field::base::FieldBase;
use crate::schema::types::field::hash_range_filter::{HashRangeFilter, HashRangeFilterResult};
use crate::schema::types::field::FieldValue;
use crate::schema::types::field::{apply_hash_filter, FilterApplicator};
use crate::schema::types::key_value::KeyValue;
use crate::schema::types::SchemaError;
use serde::{Deserialize, Serialize};

use crate::atom::MoleculeHash;
use std::collections::HashMap;
use std::sync::Arc;

/// Field keyed by a single hash key (unordered collection).
#[derive(Debug, Clone, Serialize, Deserialize, utoipa::ToSchema)]
pub struct HashField {
    #[serde(flatten)]
    pub base: FieldBase<MoleculeHash>,
}

impl HashField {
    /// Creates a new Hash field
    #[must_use]
    pub fn new(
        field_mappers: HashMap<String, FieldMapper>,
        molecule: Option<MoleculeHash>,
    ) -> Self {
        Self {
            base: FieldBase::new(field_mappers, molecule),
        }
    }
}

#[async_trait::async_trait]
impl crate::schema::types::field::Field for HashField {
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
            let new_molecule = crate::atom::MoleculeHash::new(pub_key.clone());
            self.base
                .inner
                .set_molecule_uuid(new_molecule.uuid().to_string());
            self.base.molecule = Some(new_molecule);
        }

        // For HashField, we use the hash key to store the atom
        if let Some(hash_key) = &key_value.hash {
            if let Some(molecule) = &mut self.base.molecule {
                molecule.set_atom_uuid(hash_key.clone(), atom.uuid().to_string());
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
        super::fetch_atoms_for_matches_async(db_ops, result.matches).await
    }
}

impl HashField {
    /// Gets all keys in the hash (useful for pagination or listing)
    pub fn get_all_keys(&self) -> Vec<String> {
        self.base
            .molecule
            .as_ref()
            .map(|molecule| molecule.keys().cloned().collect())
            .unwrap_or_default()
    }
}

impl FilterApplicator for HashField {
    fn apply_filter(&self, filter: Option<HashRangeFilter>) -> HashRangeFilterResult {
        let Some(molecule) = &self.base.molecule else {
            return HashRangeFilterResult::empty();
        };

        apply_hash_filter(molecule, filter)
    }
}
