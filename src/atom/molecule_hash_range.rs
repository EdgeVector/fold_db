//! MoleculeHashRange type for HashRange field semantics
//!
//! Provides a molecule type that combines hash and range functionality
//! for efficient indexing with complex fan-out operations.

use crate::atom::molecule_behavior::MoleculeBehavior;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

/// Molecule type for HashRange fields that combines hash and range functionality
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MoleculeHashRange {
    /// Unique identifier for this molecule
    pub id: String,
    /// Hash field value used for indexing
    pub hash_value: String,
    /// Range field value used for sorting/filtering
    pub range_value: String,
    /// Atom UUIDs associated with this hash-range combination
    pub atom_uuids: Vec<String>,
    /// Metadata for the hash-range relationship
    pub metadata: HashMap<String, String>,
    /// Timestamp when this molecule was created
    pub created_at: chrono::DateTime<chrono::Utc>,
    /// Timestamp when this molecule was last updated
    pub updated_at: chrono::DateTime<chrono::Utc>,
}

impl MoleculeHashRange {
    /// Creates a new MoleculeHashRange with the given ID
    pub fn new(id: String) -> Self {
        let now = chrono::Utc::now();
        Self {
            id,
            hash_value: String::new(),
            range_value: String::new(),
            atom_uuids: Vec::new(),
            metadata: HashMap::new(),
            created_at: now,
            updated_at: now,
        }
    }

    /// Creates a new MoleculeHashRange with hash and range values
    pub fn with_values(id: String, hash_value: String, range_value: String) -> Self {
        let now = chrono::Utc::now();
        Self {
            id,
            hash_value,
            range_value,
            atom_uuids: Vec::new(),
            metadata: HashMap::new(),
            created_at: now,
            updated_at: now,
        }
    }

    /// Sets the hash value
    pub fn set_hash_value(&mut self, hash_value: String) {
        self.hash_value = hash_value;
        self.updated_at = chrono::Utc::now();
    }

    /// Sets the range value
    pub fn set_range_value(&mut self, range_value: String) {
        self.range_value = range_value;
        self.updated_at = chrono::Utc::now();
    }

    /// Adds an atom UUID to this molecule
    pub fn add_atom_uuid(&mut self, atom_uuid: String) {
        if !self.atom_uuids.contains(&atom_uuid) {
            self.atom_uuids.push(atom_uuid);
            self.updated_at = chrono::Utc::now();
        }
    }

    /// Removes an atom UUID from this molecule
    pub fn remove_atom_uuid(&mut self, atom_uuid: &str) -> bool {
        if let Some(pos) = self.atom_uuids.iter().position(|x| x == atom_uuid) {
            self.atom_uuids.remove(pos);
            self.updated_at = chrono::Utc::now();
            true
        } else {
            false
        }
    }

    /// Sets metadata for this molecule
    pub fn set_metadata(&mut self, key: String, value: String) {
        self.metadata.insert(key, value);
        self.updated_at = chrono::Utc::now();
    }

    /// Gets metadata value
    pub fn get_metadata(&self, key: &str) -> Option<&String> {
        self.metadata.get(key)
    }

    /// Returns the number of atoms in this molecule
    pub fn atom_count(&self) -> usize {
        self.atom_uuids.len()
    }

    /// Checks if this molecule is empty (no atoms)
    pub fn is_empty(&self) -> bool {
        self.atom_uuids.is_empty()
    }

    /// Clears all atom UUIDs from this molecule
    pub fn clear_atoms(&mut self) {
        self.atom_uuids.clear();
        self.updated_at = chrono::Utc::now();
    }
}

impl MoleculeBehavior for MoleculeHashRange {
    fn uuid(&self) -> &str {
        &self.id
    }

    fn updated_at(&self) -> chrono::DateTime<chrono::Utc> {
        self.updated_at
    }

    fn status(&self) -> &crate::atom::molecule_types::MoleculeStatus {
        &crate::atom::molecule_types::MoleculeStatus::Active
    }

    fn set_status(
        &mut self,
        _status: &crate::atom::molecule_types::MoleculeStatus,
        _source_pub_key: String,
    ) {
        // HashRange molecules don't support status changes, but we need to implement the trait
        self.updated_at = chrono::Utc::now();
    }

    fn update_history(&self) -> &Vec<crate::atom::molecule_types::MoleculeUpdate> {
        // HashRange molecules don't have update history, return empty vector
        static EMPTY_HISTORY: Vec<crate::atom::molecule_types::MoleculeUpdate> = Vec::new();
        &EMPTY_HISTORY
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_molecule_hash_range_creation() {
        let molecule = MoleculeHashRange::new("test_id".to_string());
        assert_eq!(molecule.id, "test_id");
        assert!(molecule.hash_value.is_empty());
        assert!(molecule.range_value.is_empty());
        assert!(molecule.atom_uuids.is_empty());
        assert!(molecule.metadata.is_empty());
    }

    #[test]
    fn test_molecule_hash_range_with_values() {
        let molecule = MoleculeHashRange::with_values(
            "test_id".to_string(),
            "hash123".to_string(),
            "range456".to_string(),
        );
        assert_eq!(molecule.id, "test_id");
        assert_eq!(molecule.hash_value, "hash123");
        assert_eq!(molecule.range_value, "range456");
    }

    #[test]
    fn test_add_and_remove_atom_uuid() {
        let mut molecule = MoleculeHashRange::new("test_id".to_string());

        molecule.add_atom_uuid("atom1".to_string());
        assert_eq!(molecule.atom_count(), 1);
        assert!(molecule.atom_uuids.contains(&"atom1".to_string()));

        molecule.add_atom_uuid("atom2".to_string());
        assert_eq!(molecule.atom_count(), 2);

        // Adding duplicate should not increase count
        molecule.add_atom_uuid("atom1".to_string());
        assert_eq!(molecule.atom_count(), 2);

        // Remove atom
        assert!(molecule.remove_atom_uuid("atom1"));
        assert_eq!(molecule.atom_count(), 1);
        assert!(!molecule.atom_uuids.contains(&"atom1".to_string()));

        // Remove non-existent atom
        assert!(!molecule.remove_atom_uuid("nonexistent"));
        assert_eq!(molecule.atom_count(), 1);
    }

    #[test]
    fn test_metadata_operations() {
        let mut molecule = MoleculeHashRange::new("test_id".to_string());

        molecule.set_metadata("key1".to_string(), "value1".to_string());
        assert_eq!(molecule.get_metadata("key1"), Some(&"value1".to_string()));
        assert_eq!(molecule.get_metadata("nonexistent"), None);
    }

    #[test]
    fn test_clear_atoms() {
        let mut molecule = MoleculeHashRange::new("test_id".to_string());

        molecule.add_atom_uuid("atom1".to_string());
        molecule.add_atom_uuid("atom2".to_string());
        assert_eq!(molecule.atom_count(), 2);

        molecule.clear_atoms();
        assert_eq!(molecule.atom_count(), 0);
        assert!(molecule.is_empty());
    }
}
