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
    #[serde(default)]
    key_metadata: HashMap<String, super::KeyMetadata>,
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

    /// Sets per-key metadata for a given hash key.
    pub fn set_key_metadata(&mut self, key: String, meta: super::KeyMetadata) {
        self.key_metadata.insert(key, meta);
    }

    /// Returns the per-key metadata for a given hash key, if any.
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

    #[test]
    fn test_key_metadata_per_key_isolation() {
        let mut mol = MoleculeHash::new("pub".to_string());
        // Two keys sharing the same atom UUID (content-addressed dedup)
        mol.set_atom_uuid("photo_a".to_string(), "atom-shared".to_string());
        mol.set_atom_uuid("photo_b".to_string(), "atom-shared".to_string());
        // Different metadata per key
        mol.set_key_metadata(
            "photo_a".to_string(),
            super::super::KeyMetadata {
                source_file_name: Some("beach.jpg".to_string()),
                metadata: None,
            },
        );
        mol.set_key_metadata(
            "photo_b".to_string(),
            super::super::KeyMetadata {
                source_file_name: Some("sunset.jpg".to_string()),
                metadata: None,
            },
        );
        assert_eq!(
            mol.get_key_metadata("photo_a").unwrap().source_file_name.as_deref(),
            Some("beach.jpg")
        );
        assert_eq!(
            mol.get_key_metadata("photo_b").unwrap().source_file_name.as_deref(),
            Some("sunset.jpg")
        );
        // But both point to the same atom
        assert_eq!(mol.get_atom_uuid("photo_a"), mol.get_atom_uuid("photo_b"));
    }

    #[test]
    fn test_key_metadata_serde_roundtrip() {
        let mut mol = MoleculeHash::new("pub".to_string());
        mol.set_atom_uuid("k1".to_string(), "atom-1".to_string());
        let mut extra = std::collections::HashMap::new();
        extra.insert("file_hash".to_string(), "abc123".to_string());
        mol.set_key_metadata(
            "k1".to_string(),
            super::super::KeyMetadata {
                source_file_name: Some("test.txt".to_string()),
                metadata: Some(extra),
            },
        );
        let json = serde_json::to_string(&mol).unwrap();
        let deser: MoleculeHash = serde_json::from_str(&json).unwrap();
        let meta = deser.get_key_metadata("k1").unwrap();
        assert_eq!(meta.source_file_name.as_deref(), Some("test.txt"));
        assert_eq!(
            meta.metadata.as_ref().unwrap().get("file_hash").map(|s| s.as_str()),
            Some("abc123")
        );
    }

    #[test]
    fn test_key_metadata_backward_compat() {
        // Old serialized JSON without key_metadata should deserialize fine
        let json = r#"{
            "uuid": "test-uuid",
            "atom_uuids": {"k1": "atom-1"},
            "updated_at": "2024-01-01T00:00:00Z",
            "version": 1
        }"#;
        let mol: MoleculeHash = serde_json::from_str(json).unwrap();
        assert_eq!(mol.get_atom_uuid("k1"), Some(&"atom-1".to_string()));
        assert!(mol.get_key_metadata("k1").is_none());
        assert!(mol.key_metadata.is_empty());
    }
}
