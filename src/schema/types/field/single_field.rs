use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::sync::Arc;

use crate::impl_field;
use crate::schema::types::field::common::FieldCommon;
use crate::schema::types::field::{HashRangeFilter, HashRangeFilterResult};
use crate::schema::types::SchemaError;
use crate::atom::Molecule;
use crate::db_operations::DbOperations;
use serde_json::Value as JsonValue;
use log::{info, error};
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

    fn write_mutation(&mut self, _key_config: &crate::schema::types::key_config::KeyConfig, atom: crate::atom::Atom, pub_key: String) {
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

    fn resolve_value(
        &mut self,
        db_ops: &Arc<DbOperations>,
        _filter: Option<HashRangeFilter>,
    ) -> Result<JsonValue, SchemaError> {
        info!("🔍 SingleField: Resolving single value");

        // Refresh field data from database first
        self.refresh_from_db(db_ops);

        // For SingleField, get the single atom UUID if it exists
        if let Some(molecule) = &self.molecule {
            let atom_uuid = molecule.get_atom_uuid();
            info!("🔍 SingleField: Fetching atom content for UUID '{}'", atom_uuid);
            
            match db_ops.get_item::<crate::atom::Atom>(&format!("atom:{}", atom_uuid)) {
                Ok(Some(atom)) => {
                    info!("✅ SingleField: Successfully fetched atom content");
                    Ok(atom.content().clone())
                }
                Ok(None) => {
                    error!("❌ SingleField: Atom '{}' not found", atom_uuid);
                    Ok(JsonValue::Null)
                }
                Err(e) => {
                    error!("❌ SingleField: Failed to fetch atom '{}': {}", atom_uuid, e);
                    Err(SchemaError::InvalidField(format!(
                        "Failed to fetch atom '{}': {}",
                        atom_uuid, e
                    )))
                }
            }
        } else {
            info!("ℹ️ SingleField: No molecule found, returning null");
            Ok(JsonValue::Null)
        }
    }
}

