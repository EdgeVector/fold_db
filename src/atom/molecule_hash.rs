use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use uuid::Uuid;

/// A hash-based collection of atom references stored in a HashMap.
/// Used for collections keyed by a single hash key (no ordering needed).
#[derive(Debug, Clone, Serialize, Deserialize, utoipa::ToSchema)]
pub struct MoleculeHash {
    uuid: String,
    pub(crate) atom_uuids: HashMap<String, String>,
    #[schema(value_type = String, format = "date-time")]
    updated_at: DateTime<Utc>,
    #[serde(default)]
    version: u64,
}

impl MoleculeHash {
    /// Creates a new empty MoleculeHash.
    #[must_use]
    pub fn new(_source_pub_key: String) -> Self {
        Self {
            uuid: Uuid::new_v4().to_string(),
            atom_uuids: HashMap::new(),
            updated_at: Utc::now(),
            version: 0,
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

    /// Updates or adds a reference at the specified key.
    /// Bumps the version counter only when the atom actually changes.
    pub fn set_atom_uuid(&mut self, key: String, atom_uuid: String) {
        if self.atom_uuids.get(&key) != Some(&atom_uuid) {
            self.version += 1;
        }
        self.atom_uuids.insert(key, atom_uuid);
        self.updated_at = Utc::now();
    }

    /// Returns the UUID of the Atom referenced by the specified key.
    #[must_use]
    pub fn get_atom_uuid(&self, key: &str) -> Option<&String> {
        self.atom_uuids.get(key)
    }

    /// Removes the reference at the specified key.
    /// Bumps the version counter if an entry was actually removed.
    #[allow(clippy::manual_inspect)]
    pub fn remove_atom_uuid(&mut self, key: &str) -> Option<String> {
        self.atom_uuids.remove(key).map(|uuid| {
            self.version += 1;
            self.updated_at = Utc::now();
            uuid
        })
    }

    /// Returns the version counter for this molecule.
    #[must_use]
    pub fn version(&self) -> u64 {
        self.version
    }

    /// Returns all hash keys.
    pub fn keys(&self) -> impl Iterator<Item = &String> {
        self.atom_uuids.keys()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_version_starts_at_zero() {
        let mol = MoleculeHash::new("key".to_string());
        assert_eq!(mol.version(), 0);
    }

    #[test]
    fn test_version_bumps_on_insert() {
        let mut mol = MoleculeHash::new("key".to_string());
        mol.set_atom_uuid("k1".to_string(), "atom-1".to_string());
        assert_eq!(mol.version(), 1);
    }

    #[test]
    fn test_version_no_bump_on_same_value() {
        let mut mol = MoleculeHash::new("key".to_string());
        mol.set_atom_uuid("k1".to_string(), "atom-1".to_string());
        mol.set_atom_uuid("k1".to_string(), "atom-1".to_string());
        assert_eq!(mol.version(), 1);
    }

    #[test]
    fn test_version_bumps_on_remove() {
        let mut mol = MoleculeHash::new("key".to_string());
        mol.set_atom_uuid("k1".to_string(), "atom-1".to_string());
        assert_eq!(mol.version(), 1);
        mol.remove_atom_uuid("k1");
        assert_eq!(mol.version(), 2);
    }

    #[test]
    fn test_version_no_bump_on_remove_missing() {
        let mut mol = MoleculeHash::new("key".to_string());
        mol.remove_atom_uuid("nonexistent");
        assert_eq!(mol.version(), 0);
    }
}
