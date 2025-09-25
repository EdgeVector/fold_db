//! HashRange field type for schema indexing iterator stack model
//!
//! Provides a field type that combines hash and range functionality for
//! efficient indexing with complex fan-out operations.

use crate::fees::types::config::FieldPaymentConfig;
use crate::impl_field;
use crate::permissions::types::policy::PermissionsPolicy;
use crate::schema::types::field::common::FieldCommon;
use crate::schema::types::field::hash_range_filter::{HashRangeFilter, HashRangeFilterResult, create_composite_key, parse_composite_key};
use crate::schema::types::field::range_filter::matches_pattern;
use serde::{Deserialize, Serialize};
use serde_json::Value;
use crate::atom::MoleculeHashRange;
use std::collections::{BTreeMap, HashMap};

/// Field that combines hash and range functionality for indexing
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HashRangeField {
    pub inner: FieldCommon,
    pub molecule: Option<MoleculeHashRange>,
}

/// Configuration for HashRange field indexing
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HashRangeConfig {
    /// Maximum iterator depth allowed
    pub max_depth: usize,
    /// Whether to enable caching of parsed chains
    pub enable_caching: bool,
}

impl Default for HashRangeConfig {
    fn default() -> Self {
        Self {
            max_depth: 10,
            enable_caching: true,
        }
    }
}

impl HashRangeField {
    /// Creates a new HashRange field
    #[must_use]
    pub fn new(
        field_mappers: HashMap<String, String>,
        molecule: Option<MoleculeHashRange>,
    ) -> Self {
        Self {
            inner: FieldCommon::new(field_mappers),
            molecule,
        }
    }
}

impl_field!(HashRangeField);

impl HashRangeField {
    /// Refreshes the field's data from the database using the provided key configuration.
    /// For HashRangeField, this looks up the MoleculeHashRange data from sled.
    pub fn refresh_from_db(&mut self, db_ops: &crate::db_operations::DbOperations) {
        // If we have a molecule_uuid, look up the corresponding MoleculeHashRange
        if let Some(molecule_uuid) = self.inner.molecule_uuid() {
            let ref_key = format!("ref:{}", molecule_uuid);
            if let Ok(Some(molecule_hash_range)) = db_ops.get_item::<MoleculeHashRange>(&ref_key) {
                self.molecule = Some(molecule_hash_range);
            }
        }
    }

    /// Writes a mutation to the HashRangeField
    pub fn write_mutation(&mut self, key_config: &crate::schema::types::key_config::KeyConfig, atom: crate::atom::Atom, pub_key: String) {
        // Initialize molecule if needed
        if self.molecule.is_none() {
            self.molecule = Some(crate::atom::MoleculeHashRange::new(pub_key.clone()));
        }
        
        // For HashRangeField, we use both hash and range keys to store the atom
        if let (Some(hash_key), Some(range_key)) = (&key_config.hash_field, &key_config.range_field) {
            if let Some(molecule) = &mut self.molecule {
                molecule.set_atom_uuid(key_config, atom.uuid().to_string());
                log::debug!("Writing atom to HashRangeField with pub_key '{}', hash '{}' and range '{}': {:?}", pub_key, hash_key, range_key, atom);
            }
        }
    }

    /// Applies a hash-range filter to the field's data
    /// For HashRangeField, this searches across hash groups and their range values
    pub fn apply_filter(&self, filter: &HashRangeFilter) -> HashRangeFilterResult {
        let empty_result = HashRangeFilterResult::empty();

        let Some(molecule) = &self.molecule else {
            return empty_result;
        };

        let mut matches = HashMap::new();

        match filter {
            HashRangeFilter::HashRangeKey { hash, range } => {
                if let Some(atom_uuid) = molecule.get_atom_uuid(hash, range) {
                    let composite_key = create_composite_key(hash, range);
                    matches.insert(composite_key, atom_uuid.clone());
                }
            }
            HashRangeFilter::HashKey(hash) => {
                if let Some(range_map) = molecule.get_atoms_for_hash(hash) {
                    for (range_key, atom_uuid) in range_map {
                        let composite_key = create_composite_key(hash, range_key);
                        matches.insert(composite_key, atom_uuid.clone());
                    }
                }
            }
            HashRangeFilter::HashRangePrefix { hash, prefix } => {
                if let Some(range_map) = molecule.get_atoms_for_hash(hash) {
                    // Leverage BTree's efficient range operations for prefix search
                    let mut prefix_end = prefix.to_string();
                    if let Some(last_char) = prefix_end.chars().last() {
                        if let Some(next_char) = char::from_u32(last_char as u32 + 1) {
                            prefix_end.pop();
                            prefix_end.push(next_char);
                        } else {
                            prefix_end.push('\0');
                        }
                    } else {
                        prefix_end = "\0".to_string();
                    }
                    
                    let range = range_map.range(prefix.to_string()..prefix_end);
                    for (range_key, atom_uuid) in range {
                        let composite_key = create_composite_key(hash, range_key);
                        matches.insert(composite_key, atom_uuid.clone());
                    }
                }
            }
            HashRangeFilter::RangePrefix(prefix) => {
                // Search across all hash groups for range keys with the prefix
                let mut prefix_end = prefix.to_string();
                if let Some(last_char) = prefix_end.chars().last() {
                    if let Some(next_char) = char::from_u32(last_char as u32 + 1) {
                        prefix_end.pop();
                        prefix_end.push(next_char);
                    } else {
                        prefix_end.push('\0');
                    }
                } else {
                    prefix_end = "\0".to_string();
                }
                
                for (hash_value, range_map) in molecule.iter_hash_groups() {
                    let range = range_map.range(prefix.to_string()..prefix_end.clone());
                    for (range_key, atom_uuid) in range {
                        let composite_key = create_composite_key(hash_value, range_key);
                        matches.insert(composite_key, atom_uuid.clone());
                    }
                }
            }
            HashRangeFilter::HashRangeRange { hash, start, end } => {
                if let Some(range_map) = molecule.get_atoms_for_hash(hash) {
                    // Leverage BTree's efficient range operations
                    let range = range_map.range(start.clone()..end.clone());
                    for (range_key, atom_uuid) in range {
                        let composite_key = create_composite_key(hash, range_key);
                        matches.insert(composite_key, atom_uuid.clone());
                    }
                }
            }
            HashRangeFilter::RangeRange { start, end } => {
                // Search across all hash groups for range keys in the specified range
                for (hash_value, range_map) in molecule.iter_hash_groups() {
                    let range = range_map.range(start.clone()..end.clone());
                    for (range_key, atom_uuid) in range {
                        let composite_key = create_composite_key(hash_value, range_key);
                        matches.insert(composite_key, atom_uuid.clone());
                    }
                }
            }
            HashRangeFilter::Value(target_value) => {
                // Search across all hash groups for matching values
                for (hash_value, range_key, atom_uuid) in molecule.iter_all_atoms() {
                    if atom_uuid == target_value {
                        let composite_key = create_composite_key(hash_value, range_key);
                        matches.insert(composite_key, atom_uuid.clone());
                    }
                }
            }
            HashRangeFilter::HashRangeKeys(keys) => {
                // Search for specific hash-range key pairs
                for (hash, range) in keys {
                    if let Some(atom_uuid) = molecule.get_atom_uuid(hash, range) {
                        let composite_key = create_composite_key(hash, range);
                        matches.insert(composite_key, atom_uuid.clone());
                    }
                }
            }
            HashRangeFilter::HashRangePattern { hash, pattern } => {
                if let Some(range_map) = molecule.get_atoms_for_hash(hash) {
                    for (range_key, atom_uuid) in range_map {
                        if matches_pattern(range_key, pattern) {
                            let composite_key = create_composite_key(hash, range_key);
                            matches.insert(composite_key, atom_uuid.clone());
                        }
                    }
                }
            }
            HashRangeFilter::RangePattern(pattern) => {
                // Search across all hash groups using pattern matching
                for (hash_value, range_key, atom_uuid) in molecule.iter_all_atoms() {
                    if matches_pattern(range_key, pattern) {
                        let composite_key = create_composite_key(hash_value, range_key);
                        matches.insert(composite_key, atom_uuid.clone());
                    }
                }
            }
            HashRangeFilter::HashPattern(pattern) => {
                // Search for hash values matching the pattern
                for hash_value in molecule.hash_values() {
                    if matches_pattern(hash_value, pattern) {
                        if let Some(range_map) = molecule.get_atoms_for_hash(hash_value) {
                            for (range_key, atom_uuid) in range_map {
                                let composite_key = create_composite_key(hash_value, range_key);
                                matches.insert(composite_key, atom_uuid.clone());
                            }
                        }
                    }
                }
            }
            HashRangeFilter::HashRange { start, end } => {
                // Filter by hash value range (inclusive start, exclusive end)
                for hash_value in molecule.hash_values() {
                    if hash_value >= start && hash_value < end {
                        if let Some(range_map) = molecule.get_atoms_for_hash(hash_value) {
                            for (range_key, atom_uuid) in range_map {
                                let composite_key = create_composite_key(hash_value, range_key);
                                matches.insert(composite_key, atom_uuid.clone());
                            }
                        }
                    }
                }
            }
        }

        HashRangeFilterResult::new(matches)
    }

    /// Applies a filter from a JSON Value (for use with Operation::Query filter)
    pub fn apply_json_filter(&self, filter_value: &Value) -> Result<HashRangeFilterResult, String> {
        let filter: HashRangeFilter = serde_json::from_value(filter_value.clone())
            .map_err(|e| format!("Invalid hash-range filter format: {}", e))?;
        Ok(self.apply_filter(&filter))
    }

    /// Gets all keys in the hash range (useful for pagination or listing)
    /// Returns composite keys in the format "hash_value:range_value"
    pub fn get_all_keys(&self) -> Vec<String> {
        self.molecule
            .as_ref()
            .map(|molecule| {
                molecule
                    .iter_all_atoms()
                    .map(|(hash_value, range_key, _)| create_composite_key(hash_value, range_key))
                    .collect()
            })
            .unwrap_or_default()
    }

    /// Gets a subset of keys within a range for a specific hash group (useful for pagination)
    /// Returns composite keys in the format "hash_value:range_value"
    pub fn get_keys_in_range(&self, hash_value: &str, start: &str, end: &str) -> Vec<String> {
        self.molecule
            .as_ref()
            .and_then(|molecule| molecule.get_atoms_for_hash(hash_value))
            .map(|range_map| {
                // Leverage BTree's efficient range operations
                range_map
                    .range(start.to_string()..end.to_string())
                    .map(|(range_key, _)| create_composite_key(hash_value, range_key))
                    .collect()
            })
            .unwrap_or_default()
    }

    /// Gets all range keys for a specific hash group
    pub fn get_range_keys_for_hash(&self, hash_value: &str) -> Vec<String> {
        self.molecule
            .as_ref()
            .and_then(|molecule| molecule.range_values_for_hash(hash_value))
            .map(|iter| iter.cloned().collect())
            .unwrap_or_default()
    }

    /// Gets the total count of items in the hash range
    pub fn count(&self) -> usize {
        self.molecule
            .as_ref()
            .map(|molecule| molecule.atom_count())
            .unwrap_or(0)
    }

    /// Gets the count of items for a specific hash group
    pub fn count_for_hash(&self, hash_value: &str) -> usize {
        self.molecule
            .as_ref()
            .and_then(|molecule| molecule.get_atoms_for_hash(hash_value))
            .map(|range_map| range_map.len())
            .unwrap_or(0)
    }

    /// Gets all hash values in the molecule
    pub fn get_hash_values(&self) -> Vec<String> {
        self.molecule
            .as_ref()
            .map(|molecule| molecule.hash_values().cloned().collect())
            .unwrap_or_default()
    }
}