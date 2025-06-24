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
    pub molecule_range: Option<MoleculeRange>,
}

impl RangeField {
    #[must_use]
    pub fn new(
        permission_policy: PermissionsPolicy,
        payment_config: FieldPaymentConfig,
        field_mappers: HashMap<String, String>,
    ) -> Self {
        Self {
            inner: FieldCommon::new(permission_policy, payment_config, field_mappers),
            molecule_range: None,
        }
    }

    /// Creates a new RangeField with a MoleculeRange
    #[must_use]
    pub fn new_with_range(
        permission_policy: PermissionsPolicy,
        payment_config: FieldPaymentConfig,
        field_mappers: HashMap<String, String>,
        source_pub_key: String,
    ) -> Self {
        Self {
            inner: FieldCommon::new(permission_policy, payment_config, field_mappers),
            molecule_range: Some(MoleculeRange::new(source_pub_key)),
        }
    }

    /// Returns a reference to the MoleculeRange if it exists
    pub fn molecule_range(&self) -> Option<&MoleculeRange> {
        self.molecule_range.as_ref()
    }

    /// Returns a mutable reference to the MoleculeRange if it exists
    pub fn molecule_range_mut(&mut self) -> Option<&mut MoleculeRange> {
        self.molecule_range.as_mut()
    }

    /// Sets the MoleculeRange for this field
    pub fn set_molecule_range(&mut self, molecule_range: MoleculeRange) {
        self.molecule_range = Some(molecule_range);
    }

    /// Initializes the MoleculeRange if it doesn't exist
    pub fn ensure_molecule_range(&mut self, source_pub_key: String) -> &mut MoleculeRange {
        if self.molecule_range.is_none() {
            self.molecule_range = Some(MoleculeRange::new(source_pub_key));
        }
        self.molecule_range.as_mut().unwrap()
    }

    /// Applies a range filter to the field's data
    pub fn apply_filter(&self, filter: &RangeFilter) -> RangeFilterResult {
        let empty_result = RangeFilterResult {
            matches: HashMap::new(),
            total_count: 0,
        };

        let Some(molecule_range) = &self.molecule_range else {
            return empty_result;
        };

        let mut matches = HashMap::new();

        match filter {
            RangeFilter::Key(key) => {
                if let Some(atom_uuid) = molecule_range.get_atom_uuid(key) {
                    matches.insert(key.clone(), atom_uuid.clone());
                }
            }
            RangeFilter::KeyPrefix(prefix) => {
                for (key, atom_uuid) in &molecule_range.atom_uuids {
                    if key.starts_with(prefix) {
                        matches.insert(key.clone(), atom_uuid.clone());
                    }
                }
            }
            RangeFilter::KeyRange { start, end } => {
                for (key, atom_uuid) in &molecule_range.atom_uuids {
                    if key >= start && key < end {
                        matches.insert(key.clone(), atom_uuid.clone());
                    }
                }
            }
            RangeFilter::Value(target_value) => {
                for (key, atom_uuid) in &molecule_range.atom_uuids {
                    // Check if the value matches the target
                    if atom_uuid == target_value {
                        matches.insert(key.clone(), atom_uuid.clone());
                    }
                }
            }
            RangeFilter::Keys(keys) => {
                for key in keys {
                    if let Some(value) = molecule_range.get_atom_uuid(key) {
                        matches.insert(key.clone(), value.clone());
                    }
                }
            }
            RangeFilter::KeyPattern(pattern) => {
                for (key, atom_uuid) in &molecule_range.atom_uuids {
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
        self.molecule_range
            .as_ref()
            .map(|range| range.atom_uuids.keys().cloned().collect())
            .unwrap_or_default()
    }

    /// Gets a subset of keys within a range (useful for pagination)
    pub fn get_keys_in_range(&self, start: &str, end: &str) -> Vec<String> {
        self.molecule_range
            .as_ref()
            .map(|range| {
                let start_string = start.to_string();
                let end_string = end.to_string();
                range
                    .atom_uuids
                    .keys()
                    .filter(|key| **key >= start_string && **key < end_string)
                    .cloned()
                    .collect()
            })
            .unwrap_or_default()
    }

    /// Gets the total count of items in the range
    pub fn count(&self) -> usize {
        self.molecule_range
            .as_ref()
            .map(|range| range.atom_uuids.len())
            .unwrap_or(0)
    }
}

impl_field!(RangeField);
