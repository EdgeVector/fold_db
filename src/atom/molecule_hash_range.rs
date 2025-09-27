//! MoleculeHashRange type for HashRange field semantics
//!
//! Provides a molecule type that combines hash and range functionality
//! for efficient indexing with complex fan-out operations.

use crate::atom::molecule_behavior::MoleculeBehavior;
use crate::atom::molecule_types::{apply_status_update, MoleculeStatus, MoleculeUpdate};
use crate::schema::types::key_config::KeyConfig;
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
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MoleculeHashRange {
    /// Unique identifier for this molecule
    uuid: String,
    /// Atom UUIDs organized by hash and range values
    /// Structure: HashMap<hash_value, BTreeMap<range_value, atom_uuid>>
    atom_uuids: HashMap<String, BTreeMap<String, String>>,
    /// Timestamp when this molecule was last updated
    updated_at: DateTime<Utc>,
    /// Current status of this molecule
    status: MoleculeStatus,
    /// History of status updates
    update_history: Vec<MoleculeUpdate>,
}

impl MoleculeHashRange {
    /// Creates a new empty MoleculeHashRange.
    #[must_use]
    pub fn new(source_pub_key: String) -> Self {
        Self {
            uuid: Uuid::new_v4().to_string(),
            atom_uuids: HashMap::new(),
            updated_at: Utc::now(),
            status: MoleculeStatus::Active,
            update_history: vec![MoleculeUpdate {
                timestamp: Utc::now(),
                status: MoleculeStatus::Active,
                source_pub_key,
            }],
        }
    }

    /// Creates a new MoleculeHashRange with existing atom UUIDs.
    #[must_use]
    pub fn with_atoms(
        source_pub_key: String,
        atom_uuids: HashMap<String, BTreeMap<String, String>>,
    ) -> Self {
        Self {
            uuid: Uuid::new_v4().to_string(),
            atom_uuids,
            updated_at: Utc::now(),
            status: MoleculeStatus::Active,
            update_history: vec![MoleculeUpdate {
                timestamp: Utc::now(),
                status: MoleculeStatus::Active,
                source_pub_key,
            }],
        }
    }

    /// Adds an atom UUID using a KeyConfig for field mapping.
    /// 
    /// # Arguments
    /// * `atom_uuid` - The UUID of the atom to store
    /// * `key_config` - Configuration specifying which fields to use as hash and range
    pub fn set_atom_uuid(&mut self, key_config: &KeyConfig, atom_uuid: String) {
        self.atom_uuids
            .entry(key_config.hash_field.clone().unwrap())
            .or_default()
            .insert(key_config.range_field.clone().unwrap(), atom_uuid);
        self.updated_at = Utc::now();
    }

    /// Adds an atom UUID using explicit hash and range values.
    pub fn set_atom_uuid_from_values(&mut self, hash_value: String, range_value: String, atom_uuid: String) {
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
    pub fn remove_atom_uuid(&mut self, hash_value: &str, range_value: &str) -> Option<String> {
        if let Some(range_map) = self.atom_uuids.get_mut(hash_value) {
            let result = range_map.remove(range_value);
            if result.is_some() {
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

    /// Returns the number of hash groups in this molecule.
    #[must_use]
    pub fn hash_count(&self) -> usize {
        self.atom_uuids.len()
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

    /// Returns an iterator over all hash groups and their range maps
    pub fn iter_hash_groups(&self) -> impl Iterator<Item = (&String, &BTreeMap<String, String>)> {
        self.atom_uuids.iter()
    }

    /// Returns an iterator over all atoms across all hash groups
    /// Each item is (hash_value, range_value, atom_uuid)
    pub fn iter_all_atoms(&self) -> impl Iterator<Item = (&String, &String, &String)> {
        self.atom_uuids
            .iter()
            .flat_map(|(hash_value, range_map)| {
                range_map.iter().map(move |(range_value, atom_uuid)| {
                    (hash_value, range_value, atom_uuid)
                })
            })
    }
}

impl MoleculeBehavior for MoleculeHashRange {
    fn uuid(&self) -> &str {
        &self.uuid
    }

    fn updated_at(&self) -> DateTime<Utc> {
        self.updated_at
    }

    fn status(&self) -> &MoleculeStatus {
        &self.status
    }

    fn set_status(&mut self, status: &MoleculeStatus, source_pub_key: String) {
        apply_status_update(
            &mut self.status,
            &mut self.updated_at,
            &mut self.update_history,
            status,
            source_pub_key,
        );
    }

    fn update_history(&self) -> &Vec<MoleculeUpdate> {
        &self.update_history
    }
}