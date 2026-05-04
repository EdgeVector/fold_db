use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;

use super::{deterministic_molecule_uuid, now_nanos, AtomEntry, KeyMetadata, MergeConflict};
use crate::security::Ed25519KeyPair;

/// A range-based collection of atom references stored in a BTreeMap.
#[derive(Debug, Clone, Serialize, Deserialize, utoipa::ToSchema)]
pub struct MoleculeRange {
    uuid: String,
    pub(crate) atom_uuids: BTreeMap<String, AtomEntry>,
    #[schema(value_type = String, format = "date-time")]
    updated_at: DateTime<Utc>,
    #[serde(default)]
    version: u64,
    #[serde(default)]
    key_metadata: BTreeMap<String, KeyMetadata>,
}

impl MoleculeRange {
    /// Creates a new empty MoleculeRange with a deterministic UUID.
    #[must_use]
    pub fn new(schema_name: &str, field_name: &str) -> Self {
        Self {
            uuid: deterministic_molecule_uuid(schema_name, field_name),
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

    /// Returns the UUID as a `&String` reference.
    #[must_use]
    pub fn uuid_string(&self) -> &String {
        &self.uuid
    }

    /// Returns the timestamp of the last update.
    #[must_use]
    pub fn updated_at(&self) -> DateTime<Utc> {
        self.updated_at
    }

    /// Updates or adds a reference at the specified key and signs the entry.
    /// If the key already exists, the atom_uuid replaces the existing value.
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
                writer_pubkey: pubkey.clone(),
                signature: sig.clone(),
                signature_version: 1,
                provenance: Some(super::Provenance::user(pubkey, sig)),
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

    /// Sets per-key metadata for a given range key.
    pub fn set_key_metadata(&mut self, key: String, meta: super::KeyMetadata) {
        self.key_metadata.insert(key, meta);
    }

    /// Returns the per-key metadata for a given range key, if any.
    #[must_use]
    pub fn get_key_metadata(&self, key: &str) -> Option<&super::KeyMetadata> {
        self.key_metadata.get(key)
    }

    /// Inserts an entry whose writer identity was supplied by the caller
    /// rather than produced by a local keypair. Used by the replay/import
    /// path (e.g. inbound `data_share` from another node). See
    /// `MoleculeHashRange::set_atom_uuid_from_values_imported` for full
    /// rationale on `signature_version` / `verify_key` semantics.
    pub fn set_atom_uuid_imported(
        &mut self,
        key: String,
        atom_uuid: String,
        writer_pubkey: String,
        signature: String,
        signature_version: u8,
    ) {
        let changed = self
            .atom_uuids
            .get(&key)
            .is_none_or(|e| e.atom_uuid != atom_uuid);
        if changed {
            self.version += 1;
        }
        let written_at = now_nanos();
        let provenance = if signature_version > 0 {
            Some(super::Provenance::User {
                pubkey: writer_pubkey.clone(),
                signature: signature.clone(),
                signature_version,
            })
        } else {
            None
        };
        self.atom_uuids.insert(
            key,
            AtomEntry {
                atom_uuid,
                written_at,
                writer_pubkey,
                signature,
                signature_version,
                provenance,
            },
        );
        self.updated_at = Utc::now();
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
                provenance: None,
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

    /// Merges another MoleculeRange into this one using last-writer-wins per key.
    /// Returns a list of conflicts where both sides had different atoms for the same key.
    pub fn merge(
        &mut self,
        other: &MoleculeRange,
        _keypair: &Ed25519KeyPair,
    ) -> Vec<MergeConflict> {
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
        if !conflicts.is_empty() {
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
        let mol = MoleculeRange::new("schema", "field");
        assert_eq!(mol.version(), 0);
    }

    #[test]
    fn test_version_bumps_on_insert() {
        let kp = test_keypair();
        let mut mol = MoleculeRange::new("schema", "field");
        mol.set_atom_uuid("k1".to_string(), "atom-1".to_string(), &kp);
        assert_eq!(mol.version(), 1);
    }

    #[test]
    fn test_version_no_bump_on_same_value() {
        let kp = test_keypair();
        let mut mol = MoleculeRange::new("schema", "field");
        mol.set_atom_uuid("k1".to_string(), "atom-1".to_string(), &kp);
        mol.set_atom_uuid("k1".to_string(), "atom-1".to_string(), &kp);
        assert_eq!(mol.version(), 1);
    }

    #[test]
    fn test_version_bumps_on_remove() {
        let kp = test_keypair();
        let mut mol = MoleculeRange::new("schema", "field");
        mol.set_atom_uuid("k1".to_string(), "atom-1".to_string(), &kp);
        assert_eq!(mol.version(), 1);
        mol.remove_atom_uuid("k1");
        assert_eq!(mol.version(), 2);
    }

    #[test]
    fn test_version_no_bump_on_remove_missing() {
        let mut mol = MoleculeRange::new("schema", "field");
        mol.remove_atom_uuid("nonexistent");
        assert_eq!(mol.version(), 0);
    }

    #[test]
    fn test_deterministic_uuid() {
        let mol1 = MoleculeRange::new("my_schema", "my_field");
        let mol2 = MoleculeRange::new("my_schema", "my_field");
        assert_eq!(mol1.uuid(), mol2.uuid());
    }

    #[test]
    fn test_merge_new_key() {
        let kp = test_keypair();
        let mut mol1 = MoleculeRange::new("s", "f");
        mol1.set_atom_uuid("k1".to_string(), "atom-1".to_string(), &kp);

        let mut mol2 = MoleculeRange::new("s", "f");
        mol2.set_atom_uuid("k2".to_string(), "atom-2".to_string(), &kp);

        let conflicts = mol1.merge(&mol2, &kp);
        assert!(conflicts.is_empty());
        assert_eq!(mol1.get_atom_uuid("k1"), Some(&"atom-1".to_string()));
        assert_eq!(mol1.get_atom_uuid("k2"), Some(&"atom-2".to_string()));
    }

    #[test]
    fn test_merge_conflict_later_wins() {
        let kp = test_keypair();
        let mut mol1 = MoleculeRange::new("s", "f");
        mol1.set_atom_uuid("k1".to_string(), "atom-old".to_string(), &kp);

        std::thread::sleep(std::time::Duration::from_millis(1));

        let mut mol2 = MoleculeRange::new("s", "f");
        mol2.set_atom_uuid("k1".to_string(), "atom-new".to_string(), &kp);

        let conflicts = mol1.merge(&mol2, &kp);
        assert_eq!(conflicts.len(), 1);
        assert_eq!(conflicts[0].winner_atom, "atom-new");
        assert_eq!(mol1.get_atom_uuid("k1"), Some(&"atom-new".to_string()));
    }
}
