//! MoleculeHashRange type for HashRange field semantics
//!
//! Provides a molecule type that combines hash and range functionality
//! for efficient indexing with complex fan-out operations.

use crate::schema::types::key_config::KeyConfig;
use crate::schema::types::key_value::KeyValue;
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::collections::{BTreeMap, HashMap};
use uuid::Uuid;

/// A hash-range-based collection of atom references stored in a nested HashMap<BTreeMap> structure.
///
/// This molecule type supports complex indexing where atoms are organized by:
/// - Hash field: Groups related atoms together
/// - Range field: Provides ordered access within each hash group
///
/// Structure: HashMap<hash_value, BTreeMap<range_value, atom_uuid>>
#[derive(Debug, Clone, Serialize, Deserialize, utoipa::ToSchema)]
pub struct MoleculeHashRange {
    /// Unique identifier for this molecule
    uuid: String,
    /// Atom UUIDs organized by hash and range values
    /// Structure: HashMap<hash_value, BTreeMap<range_value, atom_uuid>>
    atom_uuids: HashMap<String, BTreeMap<String, String>>,
    /// Timestamp when this molecule was last updated
    #[schema(value_type = String, format = "date-time")]
    updated_at: DateTime<Utc>,
    /// Order in which atoms were added (for deterministic sampling)
    #[serde(default)]
    update_order: Vec<KeyValue>,
    /// Monotonic version counter, bumped on each actual change
    #[serde(default)]
    version: u64,
    /// Per-key metadata organized by hash and range values
    /// Structure: HashMap<hash_value, BTreeMap<range_value, KeyMetadata>>
    #[serde(default)]
    key_metadata: HashMap<String, BTreeMap<String, crate::atom::KeyMetadata>>,
}

impl MoleculeHashRange {
    /// Creates a new empty MoleculeHashRange.
    #[must_use]
    pub fn new(_source_pub_key: String) -> Self {
        Self {
            uuid: Uuid::new_v4().to_string(),
            atom_uuids: HashMap::new(),
            updated_at: Utc::now(),
            update_order: vec![],
            version: 0,
            key_metadata: HashMap::new(),
        }
    }

    /// Creates a new MoleculeHashRange with existing atom UUIDs.
    #[must_use]
    pub fn with_atoms(
        _source_pub_key: String,
        atom_uuids: HashMap<String, BTreeMap<String, String>>,
    ) -> Self {
        let update_order = atom_uuids
            .iter()
            .flat_map(|(hash_value, range_map)| {
                range_map.keys().map(move |range_value| {
                    KeyValue::new(Some(hash_value.clone()), Some(range_value.clone()))
                })
            })
            .collect();

        Self {
            uuid: Uuid::new_v4().to_string(),
            atom_uuids,
            updated_at: Utc::now(),
            update_order,
            version: 0,
            key_metadata: HashMap::new(),
        }
    }

    /// Returns the unique identifier of this molecule.
    #[must_use]
    pub fn uuid(&self) -> &str {
        &self.uuid
    }

    /// Returns the timestamp of the last update.
    #[must_use]
    pub fn updated_at(&self) -> DateTime<Utc> {
        self.updated_at
    }

    /// Adds an atom UUID using a KeyConfig for field mapping.
    ///
    /// # Arguments
    /// * `atom_uuid` - The UUID of the atom to store
    /// * `key_config` - Configuration specifying which fields to use as hash and range
    pub fn set_atom_uuid(&mut self, key_config: &KeyConfig, atom_uuid: String) {
        let hash = key_config.hash_field.clone().unwrap();
        let range = key_config.range_field.clone().unwrap();
        let changed = self.get_atom_uuid(&hash, &range) != Some(&atom_uuid);
        if changed {
            self.version += 1;
            let key_value = KeyValue::new(
                key_config.hash_field.clone(),
                key_config.range_field.clone(),
            );
            self.update_order.push(key_value);
        }
        self.atom_uuids
            .entry(hash)
            .or_default()
            .insert(range, atom_uuid);
        self.updated_at = Utc::now();
    }

    /// Adds an atom UUID using explicit hash and range values.
    /// Bumps the version counter and update_order only when the atom actually changes.
    pub fn set_atom_uuid_from_values(
        &mut self,
        hash_value: String,
        range_value: String,
        atom_uuid: String,
    ) {
        let changed = self.get_atom_uuid(&hash_value, &range_value) != Some(&atom_uuid);
        if changed {
            self.version += 1;
            let key_value = KeyValue::new(Some(hash_value.clone()), Some(range_value.clone()));
            self.update_order.push(key_value);
        }
        self.atom_uuids
            .entry(hash_value)
            .or_default()
            .insert(range_value, atom_uuid);
        self.updated_at = Utc::now();
    }

    /// Returns the UUID of the Atom referenced by the specified hash and range values.
    #[must_use]
    pub fn get_atom_uuid(&self, hash_value: &str, range_value: &str) -> Option<&String> {
        self.atom_uuids
            .get(hash_value)
            .and_then(|range_map| range_map.get(range_value))
    }

    /// Returns all atom UUIDs for a given hash value.
    #[must_use]
    pub fn get_atoms_for_hash(&self, hash_value: &str) -> Option<&BTreeMap<String, String>> {
        self.atom_uuids.get(hash_value)
    }

    /// Removes the reference at the specified hash and range values.
    /// Bumps the version counter if an entry was actually removed.
    pub fn remove_atom_uuid(&mut self, hash_value: &str, range_value: &str) -> Option<String> {
        if let Some(range_map) = self.atom_uuids.get_mut(hash_value) {
            let result = range_map.remove(range_value);
            if result.is_some() {
                self.version += 1;
                self.updated_at = Utc::now();
            }
            // Clean up empty hash entries
            if range_map.is_empty() {
                self.atom_uuids.remove(hash_value);
            }
            result
        } else {
            None
        }
    }

    /// Returns the total number of atoms in this molecule.
    #[must_use]
    pub fn atom_count(&self) -> usize {
        self.atom_uuids
            .values()
            .map(|range_map| range_map.len())
            .sum()
    }

    /// Checks if this molecule is empty (no atoms).
    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.atom_uuids.is_empty()
    }

    /// Returns an iterator over all hash values in this molecule.
    pub fn hash_values(&self) -> impl Iterator<Item = &String> {
        self.atom_uuids.keys()
    }

    /// Returns an iterator over all range values for a given hash.
    pub fn range_values_for_hash(&self, hash_value: &str) -> Option<impl Iterator<Item = &String>> {
        self.atom_uuids
            .get(hash_value)
            .map(|range_map| range_map.keys())
    }

    /// Returns an iterator over all atoms across all hash groups
    /// Each item is (hash_value, range_value, atom_uuid)
    pub fn iter_all_atoms(&self) -> impl Iterator<Item = (&String, &String, &String)> {
        self.atom_uuids.iter().flat_map(|(hash_value, range_map)| {
            range_map
                .iter()
                .map(move |(range_value, atom_uuid)| (hash_value, range_value, atom_uuid))
        })
    }

    /// Returns the version counter for this molecule.
    #[must_use]
    pub fn version(&self) -> u64 {
        self.version
    }

    /// Returns a deterministic sample of n KeyValues from the update order.
    /// If n is greater than the number of KeyValues, returns all KeyValues.
    #[must_use]
    pub fn sample(&self, n: usize) -> Vec<KeyValue> {
        self.update_order.iter().take(n).cloned().collect()
    }

    /// Sets per-key metadata for a given hash + range key combination.
    pub fn set_key_metadata(
        &mut self,
        hash: String,
        range: String,
        meta: crate::atom::KeyMetadata,
    ) {
        self.key_metadata
            .entry(hash)
            .or_default()
            .insert(range, meta);
    }

    /// Returns the per-key metadata for a given hash + range key, if any.
    #[must_use]
    pub fn get_key_metadata(&self, hash: &str, range: &str) -> Option<&crate::atom::KeyMetadata> {
        self.key_metadata
            .get(hash)
            .and_then(|range_map| range_map.get(range))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_version_starts_at_zero() {
        let mol = MoleculeHashRange::new("key".to_string());
        assert_eq!(mol.version(), 0);
    }

    #[test]
    fn test_version_bumps_on_insert() {
        let mut mol = MoleculeHashRange::new("key".to_string());
        mol.set_atom_uuid_from_values("h1".to_string(), "r1".to_string(), "atom-1".to_string());
        assert_eq!(mol.version(), 1);
    }

    #[test]
    fn test_version_no_bump_on_same_value() {
        let mut mol = MoleculeHashRange::new("key".to_string());
        mol.set_atom_uuid_from_values("h1".to_string(), "r1".to_string(), "atom-1".to_string());
        mol.set_atom_uuid_from_values("h1".to_string(), "r1".to_string(), "atom-1".to_string());
        assert_eq!(mol.version(), 1);
    }

    #[test]
    fn test_version_bumps_on_remove() {
        let mut mol = MoleculeHashRange::new("key".to_string());
        mol.set_atom_uuid_from_values("h1".to_string(), "r1".to_string(), "atom-1".to_string());
        assert_eq!(mol.version(), 1);
        mol.remove_atom_uuid("h1", "r1");
        assert_eq!(mol.version(), 2);
    }

    #[test]
    fn test_version_no_bump_on_remove_missing() {
        let mut mol = MoleculeHashRange::new("key".to_string());
        mol.remove_atom_uuid("h1", "r1");
        assert_eq!(mol.version(), 0);
    }

    #[test]
    fn test_with_atoms_starts_at_zero() {
        let mol = MoleculeHashRange::with_atoms(
            "key".to_string(),
            HashMap::from([(
                "h1".to_string(),
                std::collections::BTreeMap::from([("r1".to_string(), "a1".to_string())]),
            )]),
        );
        assert_eq!(mol.version(), 0);
    }
}
