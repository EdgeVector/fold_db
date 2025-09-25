use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::collections::HashMap;

use crate::atom::MoleculeRange;
use crate::fees::types::config::FieldPaymentConfig;
use crate::impl_field;
use crate::permissions::types::policy::PermissionsPolicy;
use crate::schema::types::field::common::FieldCommon;

use crate::schema::types::field::range_filter::{matches_pattern, RangeFilter, RangeFilterResult};
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

    /// Applies a range filter to the field's data
    pub fn apply_filter(&self, filter: &RangeFilter) -> RangeFilterResult {
        let empty_result = RangeFilterResult {
            matches: HashMap::new(),
            total_count: 0,
        };

        let Some(molecule) = &self.molecule else {
            return empty_result;
        };

        let mut matches = HashMap::new();

        match filter {
            RangeFilter::Key(key) => {
                if let Some(atom_uuid) = molecule.get_atom_uuid(key) {
                    matches.insert(key.clone(), atom_uuid.clone());
                }
            }
            RangeFilter::KeyPrefix(prefix) => {
                // Leverage BTree's efficient range operations for prefix search
                // Create a range from prefix to prefix + 1 (incrementing the last character)
                let mut prefix_end = prefix.to_string();
                if let Some(last_char) = prefix_end.chars().last() {
                    if let Some(next_char) = char::from_u32(last_char as u32 + 1) {
                        prefix_end.pop();
                        prefix_end.push(next_char);
                    } else {
                        // If we can't increment the last character, append a null character
                        prefix_end.push('\0');
                    }
                } else {
                    // Empty prefix case - search all keys starting with empty string
                    prefix_end = "\0".to_string();
                }
                
                let range = molecule.atom_uuids.range(prefix.to_string()..prefix_end);
                
                for (key, atom_uuid) in range {
                    matches.insert(key.clone(), atom_uuid.clone());
                }
            }
            RangeFilter::KeyRange { start, end } => {
                // Leverage BTree's efficient range operations
                let range = molecule.atom_uuids.range(start.clone()..end.clone());
                
                for (key, atom_uuid) in range {
                    matches.insert(key.clone(), atom_uuid.clone());
                }
            }
            RangeFilter::Value(target_value) => {
                for (key, atom_uuid) in &molecule.atom_uuids {
                    // Check if the value matches the target
                    if atom_uuid == target_value {
                        matches.insert(key.clone(), atom_uuid.clone());
                    }
                }
            }
            RangeFilter::Keys(keys) => {
                for key in keys {
                    if let Some(value) = molecule.get_atom_uuid(key) {
                        matches.insert(key.clone(), value.clone());
                    }
                }
            }
            RangeFilter::KeyPattern(pattern) => {
                for (key, atom_uuid) in &molecule.atom_uuids {
                    if matches_pattern(key, pattern) {
                        matches.insert(key.clone(), atom_uuid.clone());
                    }
                }
            }
        }

        RangeFilterResult {
            total_count: matches.len(),
            matches,
        }
    }

    /// Applies a filter from a JSON Value (for use with Operation::Query filter)
    pub fn apply_json_filter(&self, filter_value: &Value) -> Result<RangeFilterResult, String> {
        let filter: RangeFilter = serde_json::from_value(filter_value.clone())
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

impl_field!(RangeField);

impl RangeField {
    /// Refreshes the field's data from the database using the provided key configuration.
    /// For RangeField, this looks up the MoleculeRange data from sled.
    pub fn refresh_from_db(&mut self, db_ops: &crate::db_operations::DbOperations) {
        // If we have a molecule_uuid, look up the corresponding MoleculeRange
        if let Some(molecule_uuid) = self.inner.molecule_uuid() {
            let ref_key = format!("ref:{}", molecule_uuid);
            if let Ok(Some(molecule)) = db_ops.get_item::<MoleculeRange>(&ref_key) {
                self.molecule = Some(molecule);
            }
        }
    }

    /// Writes a mutation to the RangeField
    pub fn write_mutation(&mut self, key_config: &crate::schema::types::key_config::KeyConfig, atom: crate::atom::Atom, pub_key: String) {
        // Initialize molecule if needed
        if self.molecule.is_none() {
            self.ensure_molecule(pub_key.clone());
        }
        
        // For RangeField, we use the range key to store the atom
        if let Some(range_key) = &key_config.range_field {
            if let Some(molecule) = &mut self.molecule {
                molecule.set_atom_uuid(range_key.clone(), atom.uuid().to_string());
                log::debug!("Writing atom to RangeField with pub_key '{}' and range key '{}': {:?}", pub_key, range_key, atom);
            }
        }
    }
}
