use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::sync::Arc;

// Removed unused impl_field import
use crate::schema::types::declarative_schemas::FieldMapper;
use crate::schema::types::field::common::FieldCommon;
use crate::schema::types::field::{HashRangeFilter, HashRangeFilterResult, fetch_atoms_for_matches, FilterApplicator};
use crate::schema::types::SchemaError;
use crate::atom::{Molecule, MoleculeBehavior};
use crate::db_operations::DbOperations;
use crate::schema::types::key_value::KeyValue;
use crate::schema::types::field::FieldValue;
// Removed unused JsonValue import
// Removed unused log imports
/// Field storing a single value.
#[derive(Debug, Clone, Serialize, Deserialize, utoipa::ToSchema)]
pub struct SingleField {
    pub inner: FieldCommon,
    pub molecule: Option<Molecule>,
}

impl SingleField {
    #[must_use]
    pub fn new(
        field_mappers: HashMap<String, FieldMapper>,
        molecule: Option<Molecule>,
    ) -> Self {
        Self {
            inner: FieldCommon::new(field_mappers),
            molecule,
        }
    }
}

impl crate::schema::types::field::Field for SingleField {
    fn common(&self) -> &crate::schema::types::field::FieldCommon {
        &self.inner
    }
    
    fn common_mut(&mut self) -> &mut crate::schema::types::field::FieldCommon {
        &mut self.inner
    }

    fn refresh_from_db(&mut self, db_ops: &crate::db_operations::DbOperations) {
        if let Some(molecule_uuid) = self.inner.molecule_uuid() {
            let ref_key = format!("ref:{}", molecule_uuid);
            if let Ok(Some(molecule)) = db_ops.get_item::<crate::atom::Molecule>(&ref_key) {
                self.molecule = Some(molecule.clone());
            }
        }
    }

    fn write_mutation(&mut self, _key_value: &crate::schema::types::key_value::KeyValue, atom: crate::atom::Atom, pub_key: String) {
        // Initialize molecule if needed and set molecule_uuid in FieldCommon
        if self.molecule.is_none() {
            let new_molecule = crate::atom::Molecule::new(atom.uuid().to_string(), pub_key.clone());
            // Get the molecule's UUID and set it in FieldCommon for persistence lookup
            self.inner.set_molecule_uuid(new_molecule.uuid().to_string());
            self.molecule = Some(new_molecule);
        }
        
        // For SingleField, we store the atom using the pub_key
        if let Some(molecule) = &mut self.molecule {
            molecule.set_atom_uuid(atom.uuid().to_string());
        }
    }

    fn resolve_value(&mut self, db_ops: &Arc<DbOperations>, _filter: Option<HashRangeFilter>) -> Result<HashMap<KeyValue, FieldValue>, SchemaError> {
        self.refresh_from_db(db_ops);
        if let Some(molecule) = &self.molecule {
            let uuid = molecule.get_atom_uuid().clone();
            let result = fetch_atoms_for_matches(
                db_ops,
                vec![(KeyValue::new(None, None), uuid)].into_iter(),
            )?;
            Ok(result)
        } else {
            Ok(HashMap::new())
        }
    }
}

impl FilterApplicator for SingleField {
    fn apply_filter(&self, filter: Option<HashRangeFilter>) -> HashRangeFilterResult {
        let Some(molecule) = &self.molecule else {
            return HashRangeFilterResult::empty();
        };

        let uuid = molecule.get_atom_uuid().clone();
        let mut matches = std::collections::HashMap::new();
        if let HashRangeFilter::SampleN(n) = filter.unwrap_or(HashRangeFilter::SampleN(1)) {
            if n > 0 {
                matches.insert(crate::schema::types::key_value::KeyValue::new(None, None), uuid);
            }
        }

        HashRangeFilterResult::new(matches)
    }
}

impl SingleField {}

