use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;
use uuid::Uuid;

/// A range-based collection of atom references stored in a BTreeMap.
#[derive(Debug, Clone, Serialize, Deserialize, utoipa::ToSchema)]
pub struct MoleculeRange {
    uuid: String,
    pub(crate) atom_uuids: BTreeMap<String, String>,
    #[schema(value_type = String, format = "date-time")]
    updated_at: DateTime<Utc>,
    #[serde(default)]
    version: u64,
    #[serde(default)]
    key_metadata: BTreeMap<String, super::KeyMetadata>,
}

impl MoleculeRange {
    /// Creates a new empty MoleculeRange.
    #[must_use]
    pub fn new(_source_pub_key: String) -> Self {
        Self {
            uuid: Uuid::new_v4().to_string(),
            atom_uuids: BTreeMap::new(),
            updated_at: Utc::now(),
            version: 0,
            key_metadata: BTreeMap::new(),
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
    /// If the key already exists, the atom_uuid replaces the existing value.
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

    /// Sets per-key metadata for a given range key.
    pub fn set_key_metadata(&mut self, key: String, meta: super::KeyMetadata) {
        self.key_metadata.insert(key, meta);
    }

    /// Returns the per-key metadata for a given range key, if any.
    #[must_use]
    pub fn get_key_metadata(&self, key: &str) -> Option<&super::KeyMetadata> {
        self.key_metadata.get(key)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_version_starts_at_zero() {
        let mol = MoleculeRange::new("key".to_string());
        assert_eq!(mol.version(), 0);
    }

    #[test]
    fn test_version_bumps_on_insert() {
        let mut mol = MoleculeRange::new("key".to_string());
        mol.set_atom_uuid("k1".to_string(), "atom-1".to_string());
        assert_eq!(mol.version(), 1);
    }

    #[test]
    fn test_version_no_bump_on_same_value() {
        let mut mol = MoleculeRange::new("key".to_string());
        mol.set_atom_uuid("k1".to_string(), "atom-1".to_string());
        mol.set_atom_uuid("k1".to_string(), "atom-1".to_string());
        assert_eq!(mol.version(), 1);
    }

    #[test]
    fn test_version_bumps_on_remove() {
        let mut mol = MoleculeRange::new("key".to_string());
        mol.set_atom_uuid("k1".to_string(), "atom-1".to_string());
        assert_eq!(mol.version(), 1);
        mol.remove_atom_uuid("k1");
        assert_eq!(mol.version(), 2);
    }

    #[test]
    fn test_version_no_bump_on_remove_missing() {
        let mut mol = MoleculeRange::new("key".to_string());
        mol.remove_atom_uuid("nonexistent");
        assert_eq!(mol.version(), 0);
    }
}
