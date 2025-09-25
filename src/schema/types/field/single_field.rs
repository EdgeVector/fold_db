use serde::{Deserialize, Serialize};
use std::collections::HashMap;

use crate::fees::types::config::FieldPaymentConfig;
use crate::impl_field;
use crate::permissions::types::policy::PermissionsPolicy;
use crate::schema::types::field::common::FieldCommon;
use crate::atom::Molecule;
/// Field storing a single value.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SingleField {
    pub inner: FieldCommon,
    pub molecule: Option<Molecule>,
}

impl SingleField {
    #[must_use]
    pub fn new(
        field_mappers: HashMap<String, String>,
        molecule: Option<Molecule>,
    ) -> Self {
        Self {
            inner: FieldCommon::new(field_mappers),
            molecule,
        }
    }
}

impl_field!(SingleField);

impl SingleField {
    /// Refreshes the field's data from the database using the provided key configuration.
    /// For SingleField, this looks up the Molecule data from sled.
    pub fn refresh_from_db(&mut self, db_ops: &crate::db_operations::DbOperations) {
        if let Some(molecule_uuid) = self.inner.molecule_uuid() {
            let ref_key = format!("ref:{}", molecule_uuid);
            if let Ok(Some(molecule)) = db_ops.get_item::<crate::atom::Molecule>(&ref_key) {
                self.molecule = Some(molecule.clone());
            }
        }
    }

    /// Writes a mutation to the SingleField
    pub fn write_mutation(&mut self, _key_config: &crate::schema::types::key_config::KeyConfig, atom: crate::atom::Atom, pub_key: String) {
        // Initialize molecule if needed
        if self.molecule.is_none() {
            self.molecule = Some(crate::atom::Molecule::new(atom.uuid().to_string(), pub_key.clone()));
        }
        
        // For SingleField, we store the atom using the pub_key
        if let Some(molecule) = &mut self.molecule {
            molecule.set_atom_uuid(atom.uuid().to_string());
            log::debug!("Writing atom to SingleField with pub_key '{}': {:?}", pub_key, atom);
        }
    }
}
