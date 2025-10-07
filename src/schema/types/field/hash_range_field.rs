//! HashRange field type for schema indexing iterator stack model
//!
//! Provides a field type that combines hash and range functionality for
//! efficient indexing with complex fan-out operations.

// Removed unused impl_field import
use crate::schema::types::field::common::FieldCommon;
use crate::schema::types::field::hash_range_filter::{HashRangeFilter, HashRangeFilterResult};
use crate::schema::types::key_value::KeyValue;
use crate::schema::types::field::FieldValue;
use crate::schema::types::field::{FilterApplicator, apply_hash_range_filter, fetch_atoms_for_matches};
use crate::schema::types::SchemaError;
use crate::db_operations::DbOperations;
use serde::{Deserialize, Serialize};
// Removed unused JsonValue import
use crate::atom::{MoleculeHashRange, MoleculeBehavior};
use std::collections::HashMap;
use std::sync::Arc;
// Removed unused log imports

/// Field that combines hash and range functionality for indexing
#[derive(Debug, Clone, Serialize, Deserialize, utoipa::ToSchema)]
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

impl crate::schema::types::field::Field for HashRangeField {
    fn common(&self) -> &crate::schema::types::field::FieldCommon {
        &self.inner
    }
    
    fn common_mut(&mut self) -> &mut crate::schema::types::field::FieldCommon {
        &mut self.inner
    }

    fn refresh_from_db(&mut self, db_ops: &crate::db_operations::DbOperations) {
        // If we have a molecule_uuid, look up the corresponding MoleculeHashRange
        if let Some(molecule_uuid) = self.inner.molecule_uuid() {
            let ref_key = format!("ref:{}", molecule_uuid);
            if let Ok(Some(molecule_hash_range)) = db_ops.get_item::<MoleculeHashRange>(&ref_key) {
                self.molecule = Some(molecule_hash_range);
            }
        }
    }

    fn write_mutation(&mut self, key_value: &crate::schema::types::key_value::KeyValue, atom: crate::atom::Atom, pub_key: String) {
        // Initialize molecule if needed and set molecule_uuid in FieldCommon
        if self.molecule.is_none() {
            let new_molecule = crate::atom::MoleculeHashRange::new(pub_key.clone());
            // Get the molecule's UUID and set it in FieldCommon for persistence lookup
            self.inner.set_molecule_uuid(new_molecule.uuid().to_string());
            self.molecule = Some(new_molecule);
        }
        
        // For HashRangeField, we use both hash and range keys to store the atom
        if let (Some(hash_key), Some(range_key)) = (&key_value.hash, &key_value.range) {
            if let Some(molecule) = &mut self.molecule {
                molecule.set_atom_uuid_from_values(hash_key.clone(), range_key.clone(), atom.uuid().to_string());
            }
        }
    }

    fn resolve_value(&mut self, db_ops: &Arc<DbOperations>, filter: Option<HashRangeFilter>) -> Result<HashMap<KeyValue, FieldValue>, SchemaError> {
        self.refresh_from_db(db_ops);
        let result = self.apply_filter(filter);
        fetch_atoms_for_matches(db_ops, result.matches)
    }
}

impl HashRangeField {

    /// Gets all keys in the hash range (useful for pagination or listing)
    /// Returns composite keys in the format "hash_value:range_value"
    pub fn get_all_keys(&self) -> Vec<KeyValue> {
        self.molecule
            .as_ref()
            .map(|molecule| {
                molecule
                    .iter_all_atoms()
                    .map(|(hash_value, range_key, _)| KeyValue::new(Some(hash_value.clone()), Some(range_key.clone())))
                    .collect()
            })
            .unwrap_or_default()
    }

    /// Gets a subset of keys within a range for a specific hash group (useful for pagination)
    /// Returns composite keys in the format "hash_value:range_value"
    pub fn get_keys_in_range(&self, hash_value: &str, start: &str, end: &str) -> Vec<KeyValue> {
        self.molecule
            .as_ref()
            .and_then(|molecule| molecule.get_atoms_for_hash(hash_value))
            .map(|range_map| {
                // Leverage BTree's efficient range operations
                range_map
                    .range(start.to_string()..end.to_string())
                    .map(|(range_key, _)| KeyValue::new(Some(hash_value.to_string()), Some(range_key.clone())))
                    .collect()
            })
            .unwrap_or_default()
    }

    /// Gets all range keys for a specific hash group
    pub fn get_range_keys_for_hash(&self, hash_value: &str) -> Vec<KeyValue> {
        self.molecule
            .as_ref()
            .and_then(|molecule| molecule.range_values_for_hash(hash_value))
            .map(|iter| iter.map(|range_key| KeyValue::new(Some(hash_value.to_string()), Some(range_key.clone()))).collect())
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

impl FilterApplicator for HashRangeField {
    fn apply_filter(&self, filter: Option<HashRangeFilter>) -> HashRangeFilterResult {
        let Some(molecule) = &self.molecule else {
            return HashRangeFilterResult::empty();
        };

        apply_hash_range_filter(molecule, filter)
    }
}