use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

use super::{deterministic_molecule_uuid, now_nanos, AtomEntry, MergeConflict};
use crate::security::Ed25519KeyPair;

/// A hash-based collection of atom references stored in a HashMap.
/// Used for collections keyed by a single hash key (no ordering needed).
#[derive(Debug, Clone, Serialize, Deserialize, utoipa::ToSchema)]
pub struct MoleculeHash {
    uuid: String,
    pub(crate) atom_uuids: HashMap<String, AtomEntry>,
    #[schema(value_type = String, format = "date-time")]
    updated_at: DateTime<Utc>,
    #[serde(default)]
    version: u64,
    #[serde(default)]
    key_metadata: HashMap<String, super::KeyMetadata>,
}

impl MoleculeHash {
    /// Creates a new empty MoleculeHash with a deterministic UUID.
    #[must_use]
    pub fn new(schema_name: &str, field_name: &str) -> Self {
        Self {
            uuid: deterministic_molecule_uuid(schema_name, field_name),
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

    /// Updates or adds a reference at the specified key and signs the entry.
    /// Bumps the version counter only when the atom actually changes.
    pub fn set_atom_uuid(&mut self, key: String, atom_uuid: String, keypair: &Ed25519KeyPair) {
        let changed = self
            .atom_uuids
            .get(&key)
            .is_none_or(|e| e.atom_uuid != atom_uuid);
        if changed {
            self.version += 1;
        }
        let written_at = now_nanos();
        let canonical = Self::build_canonical_bytes(&self.uuid, &key, &atom_uuid, written_at);
        let (sig, pubkey) = crate::security::sign_molecule_update(&canonical, keypair);
        self.atom_uuids.insert(
            key,
            AtomEntry {
                atom_uuid,
                written_at,
                writer_pubkey: pubkey,
                signature: sig,
                signature_version: 1,
            },
        );
        self.updated_at = Utc::now();
    }

    /// Returns the UUID of the Atom referenced by the specified key.
    #[must_use]
    pub fn get_atom_uuid(&self, key: &str) -> Option<&String> {
        self.atom_uuids.get(key).map(|e| &e.atom_uuid)
    }

    /// Returns the full AtomEntry for a given key, if present.
    #[must_use]
    pub fn get_atom_entry(&self, key: &str) -> Option<&AtomEntry> {
        self.atom_uuids.get(key)
    }

    /// Removes the reference at the specified key.
    /// Bumps the version counter if an entry was actually removed.
    #[allow(clippy::manual_inspect)]
    pub fn remove_atom_uuid(&mut self, key: &str) -> Option<String> {
        self.atom_uuids.remove(key).map(|entry| {
            self.version += 1;
            self.updated_at = Utc::now();
            entry.atom_uuid
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

    /// Updates a key WITHOUT signing. Only for ephemeral in-memory operations (rewind).
    pub(crate) fn set_atom_uuid_unsigned(&mut self, key: String, atom_uuid: String) {
        let changed = self
            .atom_uuids
            .get(&key)
            .is_none_or(|e| e.atom_uuid != atom_uuid);
        if changed {
            self.version += 1;
        }
        self.atom_uuids.insert(
            key,
            AtomEntry {
                atom_uuid,
                written_at: now_nanos(),
                writer_pubkey: String::new(),
                signature: String::new(),
                signature_version: 0,
            },
        );
        self.updated_at = Utc::now();
    }

    /// Builds canonical bytes for per-key signing/verification.
    /// Layout: molecule_uuid | 0x00 | key | 0x00 | atom_uuid | 0x00 | written_at(u64 BE)
    fn build_canonical_bytes(
        molecule_uuid: &str,
        key: &str,
        atom_uuid: &str,
        written_at: u64,
    ) -> Vec<u8> {
        let mut buf = Vec::new();
        buf.extend_from_slice(molecule_uuid.as_bytes());
        buf.push(0x00);
        buf.extend_from_slice(key.as_bytes());
        buf.push(0x00);
        buf.extend_from_slice(atom_uuid.as_bytes());
        buf.push(0x00);
        buf.extend_from_slice(&written_at.to_be_bytes());
        buf
    }

    /// Verifies the signature for a specific key entry.
    /// Returns false if the key doesn't exist or the entry is unsigned.
    #[must_use]
    pub fn verify_key(&self, key: &str) -> bool {
        let entry = match self.atom_uuids.get(key) {
            Some(e) => e,
            None => return false,
        };
        if entry.signature_version == 0 {
            return false;
        }
        let canonical =
            Self::build_canonical_bytes(&self.uuid, key, &entry.atom_uuid, entry.written_at);
        crate::security::verify_molecule_signature(
            &canonical,
            &entry.signature,
            &entry.writer_pubkey,
        )
    }

    /// Merges another MoleculeHash into this one using last-writer-wins per key.
    /// Returns a list of conflicts where both sides had different atoms for the same key.
    pub fn merge(&mut self, other: &MoleculeHash, _keypair: &Ed25519KeyPair) -> Vec<MergeConflict> {
        let mut conflicts = Vec::new();
        for (key, other_entry) in &other.atom_uuids {
            match self.atom_uuids.get(key) {
                None => {
                    self.atom_uuids.insert(key.clone(), other_entry.clone());
                    self.version += 1;
                }
                Some(self_entry) => {
                    if self_entry.atom_uuid == other_entry.atom_uuid {
                        continue;
                    }
                    let (winner, loser) = if other_entry.written_at >= self_entry.written_at {
                        (other_entry, self_entry)
                    } else {
                        (self_entry, other_entry)
                    };
                    conflicts.push(MergeConflict {
                        key: key.clone(),
                        winner_atom: winner.atom_uuid.clone(),
                        loser_atom: loser.atom_uuid.clone(),
                        winner_written_at: winner.written_at,
                        loser_written_at: loser.written_at,
                    });
                    if other_entry.written_at >= self_entry.written_at {
                        self.atom_uuids.insert(key.clone(), other_entry.clone());
                        self.version += 1;
                    }
                }
            }
        }
        if !conflicts.is_empty() || !self.atom_uuids.is_empty() {
            self.updated_at = Utc::now();
        }
        conflicts
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::security::Ed25519KeyPair;

    fn test_keypair() -> Ed25519KeyPair {
        Ed25519KeyPair::generate().unwrap()
    }

    #[test]
    fn test_version_starts_at_zero() {
        let mol = MoleculeHash::new("schema", "field");
        assert_eq!(mol.version(), 0);
    }

    #[test]
    fn test_version_bumps_on_insert() {
        let kp = test_keypair();
        let mut mol = MoleculeHash::new("schema", "field");
        mol.set_atom_uuid("k1".to_string(), "atom-1".to_string(), &kp);
        assert_eq!(mol.version(), 1);
    }

    #[test]
    fn test_version_no_bump_on_same_value() {
        let kp = test_keypair();
        let mut mol = MoleculeHash::new("schema", "field");
        mol.set_atom_uuid("k1".to_string(), "atom-1".to_string(), &kp);
        mol.set_atom_uuid("k1".to_string(), "atom-1".to_string(), &kp);
        assert_eq!(mol.version(), 1);
    }

    #[test]
    fn test_version_bumps_on_remove() {
        let kp = test_keypair();
        let mut mol = MoleculeHash::new("schema", "field");
        mol.set_atom_uuid("k1".to_string(), "atom-1".to_string(), &kp);
        assert_eq!(mol.version(), 1);
        mol.remove_atom_uuid("k1");
        assert_eq!(mol.version(), 2);
    }

    #[test]
    fn test_version_no_bump_on_remove_missing() {
        let mut mol = MoleculeHash::new("schema", "field");
        mol.remove_atom_uuid("nonexistent");
        assert_eq!(mol.version(), 0);
    }

    #[test]
    fn test_key_metadata_per_key_isolation() {
        let kp = test_keypair();
        let mut mol = MoleculeHash::new("schema", "field");
        mol.set_atom_uuid("photo_a".to_string(), "atom-shared".to_string(), &kp);
        mol.set_atom_uuid("photo_b".to_string(), "atom-shared".to_string(), &kp);
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
            mol.get_key_metadata("photo_a")
                .unwrap()
                .source_file_name
                .as_deref(),
            Some("beach.jpg")
        );
        assert_eq!(
            mol.get_key_metadata("photo_b")
                .unwrap()
                .source_file_name
                .as_deref(),
            Some("sunset.jpg")
        );
        assert_eq!(mol.get_atom_uuid("photo_a"), mol.get_atom_uuid("photo_b"));
    }

    #[test]
    fn test_key_metadata_serde_roundtrip() {
        let kp = test_keypair();
        let mut mol = MoleculeHash::new("schema", "field");
        mol.set_atom_uuid("k1".to_string(), "atom-1".to_string(), &kp);
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
            meta.metadata
                .as_ref()
                .unwrap()
                .get("file_hash")
                .map(|s| s.as_str()),
            Some("abc123")
        );
    }

    #[test]
    fn test_key_metadata_backward_compat() {
        let json = r#"{
            "uuid": "test-uuid",
            "atom_uuids": {"k1": {"atom_uuid": "atom-1", "written_at": 0}},
            "updated_at": "2024-01-01T00:00:00Z",
            "version": 1
        }"#;
        let mol: MoleculeHash = serde_json::from_str(json).unwrap();
        assert_eq!(mol.get_atom_uuid("k1"), Some(&"atom-1".to_string()));
        assert!(mol.get_key_metadata("k1").is_none());
        assert!(mol.key_metadata.is_empty());
    }

    #[test]
    fn test_deterministic_uuid() {
        let mol1 = MoleculeHash::new("my_schema", "my_field");
        let mol2 = MoleculeHash::new("my_schema", "my_field");
        assert_eq!(mol1.uuid(), mol2.uuid());
    }

    #[test]
    fn test_merge_new_key() {
        let kp = test_keypair();
        let mut mol1 = MoleculeHash::new("s", "f");
        mol1.set_atom_uuid("k1".to_string(), "atom-1".to_string(), &kp);

        let mut mol2 = MoleculeHash::new("s", "f");
        mol2.set_atom_uuid("k2".to_string(), "atom-2".to_string(), &kp);

        let conflicts = mol1.merge(&mol2, &kp);
        assert!(conflicts.is_empty());
        assert_eq!(mol1.get_atom_uuid("k1"), Some(&"atom-1".to_string()));
        assert_eq!(mol1.get_atom_uuid("k2"), Some(&"atom-2".to_string()));
    }

    #[test]
    fn test_merge_conflict_later_wins() {
        let kp = test_keypair();
        let mut mol1 = MoleculeHash::new("s", "f");
        mol1.set_atom_uuid("k1".to_string(), "atom-old".to_string(), &kp);

        std::thread::sleep(std::time::Duration::from_millis(1));

        let mut mol2 = MoleculeHash::new("s", "f");
        mol2.set_atom_uuid("k1".to_string(), "atom-new".to_string(), &kp);

        let conflicts = mol1.merge(&mol2, &kp);
        assert_eq!(conflicts.len(), 1);
        assert_eq!(conflicts[0].winner_atom, "atom-new");
        assert_eq!(mol1.get_atom_uuid("k1"), Some(&"atom-new".to_string()));
    }

    #[test]
    fn test_per_key_signing() {
        let kp = test_keypair();
        let mut mol = MoleculeHash::new("s", "f");
        mol.set_atom_uuid("k1".to_string(), "atom-1".to_string(), &kp);
        mol.set_atom_uuid("k2".to_string(), "atom-2".to_string(), &kp);
        assert!(mol.verify_key("k1"), "k1 should verify");
        assert!(mol.verify_key("k2"), "k2 should verify");
        assert!(!mol.verify_key("k3"), "nonexistent key should not verify");
    }
}
