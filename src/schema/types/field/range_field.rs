use serde::{Deserialize, Serialize};
use serde_json::Value as JsonValue;
use std::collections::HashMap;
use std::sync::Arc;

use crate::atom::MoleculeRange;
use crate::impl_field;
use crate::schema::types::field::common::FieldCommon;
use crate::schema::types::field::FieldValue;
use crate::schema::types::field::{HashRangeFilter, HashRangeFilterResult, FilterApplicator, RangeOperations, apply_range_filter};
use crate::schema::types::SchemaError;
use crate::db_operations::DbOperations;
use crate::schema::types::key_value::KeyValue;
use log::{info, error};

// RangeFilter has been unified into HashRangeFilter
/// Field storing a range of values.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RangeField {
    pub inner: FieldCommon,
    pub molecule: Option<MoleculeRange>,
}

impl RangeField {
    #[must_use]
    pub fn new(
        field_mappers: HashMap<String, String>,
    ) -> Self {
        Self {
            inner: FieldCommon::new(field_mappers),
            molecule: None,
        }
    }

    /// Creates a new RangeField with a MoleculeRange
    #[must_use]
    pub fn new_with_range(
        field_mappers: HashMap<String, String>,
        source_pub_key: String,
    ) -> Self {
        Self {
            inner: FieldCommon::new(field_mappers),
            molecule: Some(MoleculeRange::new(source_pub_key)),
        }
    }

    /// Returns a reference to the MoleculeRange if it exists
    pub fn molecule(&self) -> Option<&MoleculeRange> {
        self.molecule.as_ref()
    }

    /// Returns a mutable reference to the MoleculeRange if it exists
    pub fn molecule_mut(&mut self) -> Option<&mut MoleculeRange> {
        self.molecule.as_mut()
    }

    /// Sets the MoleculeRange for this field
    pub fn set_molecule(&mut self, molecule: MoleculeRange) {
        self.molecule = Some(molecule);
    }

    /// Initializes the MoleculeRange if it doesn't exist
    pub fn ensure_molecule(&mut self, source_pub_key: String) -> &mut MoleculeRange {
        if self.molecule.is_none() {
            self.molecule = Some(MoleculeRange::new(source_pub_key));
        }
        self.molecule.as_mut().unwrap()
    }

    /// Applies a filter from a JSON Value (for use with Operation::Query filter)
    pub fn apply_json_filter(&self, filter_value: &JsonValue) -> Result<HashRangeFilterResult, String> {
        let filter: HashRangeFilter = serde_json::from_value(filter_value.clone())
            .map_err(|e| format!("Invalid range filter format: {}", e))?;
        Ok(self.apply_filter(&filter))
    }


    /// Gets all keys in the range (useful for pagination or listing)
    pub fn get_all_keys(&self) -> Vec<String> {
        self.molecule
            .as_ref()
            .map(|range| range.atom_uuids.keys().cloned().collect())
            .unwrap_or_default()
    }

    /// Gets a subset of keys within a range (useful for pagination)
    pub fn get_keys_in_range(&self, start: &str, end: &str) -> Vec<String> {
        self.molecule
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
        self.molecule
            .as_ref()
            .map(|range| range.atom_uuids.len())
            .unwrap_or(0)
    }
}

impl FilterApplicator for RangeField {
    fn apply_filter(&self, filter: &HashRangeFilter) -> HashRangeFilterResult {
        let Some(molecule) = &self.molecule else {
            return HashRangeFilterResult::empty();
        };

        apply_range_filter(molecule, filter)
    }
}

impl crate::schema::types::field::Field for RangeField {
    fn common(&self) -> &crate::schema::types::field::FieldCommon {
        &self.inner
    }
    
    fn common_mut(&mut self) -> &mut crate::schema::types::field::FieldCommon {
        &mut self.inner
    }

    fn refresh_from_db(&mut self, db_ops: &crate::db_operations::DbOperations) {
        // If we have a molecule_uuid, look up the corresponding MoleculeRange
        if let Some(molecule_uuid) = self.inner.molecule_uuid() {
            let ref_key = format!("ref:{}", molecule_uuid);
            if let Ok(Some(molecule)) = db_ops.get_item::<MoleculeRange>(&ref_key) {
                self.molecule = Some(molecule);
            }
        }
    }

    fn write_mutation(&mut self, key_value: &crate::schema::types::key_value::KeyValue, atom: crate::atom::Atom, pub_key: String) {
        // Initialize molecule if needed
        if self.molecule.is_none() {
            self.ensure_molecule(pub_key.clone());
        }
        
        // For RangeField, we use the range key to store the atom
        if let Some(range_key) = &key_value.range {
            if let Some(molecule) = &mut self.molecule {
                molecule.set_atom_uuid(range_key.clone(), atom.uuid().to_string());
                log::debug!("Writing atom to RangeField with pub_key '{}' and range key '{}': {:?}", pub_key, range_key, atom);
            }
        }
    }

    fn resolve_value(
        &mut self,
        db_ops: &Arc<DbOperations>,
        filter: Option<HashRangeFilter>,
    ) -> Result<HashMap<KeyValue, FieldValue>, SchemaError> { 
        info!("🔍 RangeField: Resolving range values with filter: {:?}", filter);

        self.refresh_from_db(db_ops);

        // Apply filters to get matching atom UUIDs
        let filter_result = if let Some(ref filter) = filter {
            self.apply_filter(filter)
        } else {
            // No filter - return all range keys
            let mut matches = HashMap::new();
            if let Some(molecule) = &self.molecule {
                for (key, atom_uuid) in &molecule.atom_uuids {
                    matches.insert(KeyValue::new(None, Some(key.clone())), atom_uuid.clone());
                }
            }
            HashRangeFilterResult::new(matches)
        };

        info!("🔍 RangeField: Filter applied, found {} matches", filter_result.matches.len());

        // Fetch actual atom content from database
        let mut resolved_values = HashMap::new();

        for (key, atom_uuid) in filter_result.matches {
            match db_ops.get_item::<crate::atom::Atom>(&format!("atom:{}", atom_uuid)) {
                Ok(Some(atom)) => {
                    resolved_values.insert(key, FieldValue { value: atom.content().clone(), atom_uuid });
                }
                Ok(None) => {
                    return Err(SchemaError::InvalidField(format!(
                        "Atom '{}' not found for key '{}'",
                        atom_uuid, key
                    )))
                }
                Err(_e) => {
                    return Err(SchemaError::InvalidField(format!(
                        "Failed to fetch atom '{}' for key '{}'",
                        atom_uuid, key
                    )));
                }
            }
        }

        info!("✅ RangeField: Value resolution completed successfully");
        Ok(resolved_values)
    }
}

