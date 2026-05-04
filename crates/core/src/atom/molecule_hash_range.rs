//! MoleculeHashRange type for HashRange field semantics
//!
//! Provides a molecule type that combines hash and range functionality
//! for efficient indexing with complex fan-out operations.

use crate::schema::types::key_config::KeyConfig;
use crate::schema::types::key_value::KeyValue;
use crate::security::Ed25519KeyPair;
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::collections::{BTreeMap, HashMap};

use super::{deterministic_molecule_uuid, now_nanos, AtomEntry, KeyMetadata, MergeConflict};

/// A hash-range-based collection of atom references stored in a nested HashMap<BTreeMap> structure.
///
/// This molecule type supports complex indexing where atoms are organized by:
/// - Hash field: Groups related atoms together
/// - Range field: Provides ordered access within each hash group
///
/// Structure: HashMap<hash_value, BTreeMap<range_value, AtomEntry>>
#[derive(Debug, Clone, Serialize, Deserialize, utoipa::ToSchema)]
pub struct MoleculeHashRange {
    /// Unique identifier for this molecule
    uuid: String,
    /// Atom entries organized by hash and range values
    /// Structure: HashMap<hash_value, BTreeMap<range_value, AtomEntry>>
    atom_uuids: HashMap<String, BTreeMap<String, AtomEntry>>,
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
    key_metadata: HashMap<String, BTreeMap<String, KeyMetadata>>,
}

impl MoleculeHashRange {
    /// Creates a new empty MoleculeHashRange with a deterministic UUID.
    #[must_use]
    pub fn new(schema_name: &str, field_name: &str) -> Self {
        Self {
            uuid: deterministic_molecule_uuid(schema_name, field_name),
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
        schema_name: &str,
        field_name: &str,
        atom_uuids: HashMap<String, BTreeMap<String, String>>,
    ) -> Self {
        let ts = now_nanos();
        let update_order = atom_uuids
            .iter()
            .flat_map(|(hash_value, range_map)| {
                range_map.keys().map(move |range_value| {
                    KeyValue::new(Some(hash_value.clone()), Some(range_value.clone()))
                })
            })
            .collect();

        let entries: HashMap<String, BTreeMap<String, AtomEntry>> = atom_uuids
            .into_iter()
            .map(|(hash, range_map)| {
                let entry_map: BTreeMap<String, AtomEntry> = range_map
                    .into_iter()
                    .map(|(range, atom_uuid)| {
                        (
                            range,
                            AtomEntry {
                                atom_uuid,
                                written_at: ts,
                                writer_pubkey: String::new(),
                                signature: String::new(),
                                signature_version: 0,
                                provenance: None,
                            },
                        )
                    })
                    .collect();
                (hash, entry_map)
            })
            .collect();

        Self {
            uuid: deterministic_molecule_uuid(schema_name, field_name),
            atom_uuids: entries,
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

    /// Adds an atom UUID using a KeyConfig for field mapping.
    pub fn set_atom_uuid(
        &mut self,
        key_config: &KeyConfig,
        atom_uuid: String,
        keypair: &Ed25519KeyPair,
    ) {
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
        let written_at = now_nanos();
        let canonical =
            Self::build_canonical_bytes(&self.uuid, &hash, &range, &atom_uuid, written_at);
        let (sig, pubkey) = crate::security::sign_molecule_update(&canonical, keypair);
        self.atom_uuids.entry(hash).or_default().insert(
            range,
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

    /// Adds an atom UUID using explicit hash and range values.
    /// Bumps the version counter and update_order only when the atom actually changes.
    pub fn set_atom_uuid_from_values(
        &mut self,
        hash_value: String,
        range_value: String,
        atom_uuid: String,
        keypair: &Ed25519KeyPair,
    ) {
        let changed = self.get_atom_uuid(&hash_value, &range_value) != Some(&atom_uuid);
        if changed {
            self.version += 1;
            let key_value = KeyValue::new(Some(hash_value.clone()), Some(range_value.clone()));
            self.update_order.push(key_value);
        }
        let written_at = now_nanos();
        let canonical = Self::build_canonical_bytes(
            &self.uuid,
            &hash_value,
            &range_value,
            &atom_uuid,
            written_at,
        );
        let (sig, pubkey) = crate::security::sign_molecule_update(&canonical, keypair);
        self.atom_uuids.entry(hash_value).or_default().insert(
            range_value,
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

    /// Returns the UUID of the Atom referenced by the specified hash and range values.
    #[must_use]
    pub fn get_atom_uuid(&self, hash_value: &str, range_value: &str) -> Option<&String> {
        self.atom_uuids
            .get(hash_value)
            .and_then(|range_map| range_map.get(range_value))
            .map(|e| &e.atom_uuid)
    }

    /// Returns the full AtomEntry at the specified hash and range values, if present.
    #[must_use]
    pub fn get_atom_entry(&self, hash_value: &str, range_value: &str) -> Option<&AtomEntry> {
        self.atom_uuids
            .get(hash_value)
            .and_then(|range_map| range_map.get(range_value))
    }

    /// Returns all atom UUIDs for a given hash value.
    #[must_use]
    pub fn get_atoms_for_hash(&self, hash_value: &str) -> Option<BTreeMap<String, String>> {
        self.atom_uuids.get(hash_value).map(|range_map| {
            range_map
                .iter()
                .map(|(k, e)| (k.clone(), e.atom_uuid.clone()))
                .collect()
        })
    }

    /// Removes the reference at the specified hash and range values.
    /// Bumps the version counter if an entry was actually removed.
    pub fn remove_atom_uuid(&mut self, hash_value: &str, range_value: &str) -> Option<String> {
        if let Some(range_map) = self.atom_uuids.get_mut(hash_value) {
            let result = range_map.remove(range_value);
            if let Some(entry) = result {
                self.version += 1;
                self.updated_at = Utc::now();
                // Clean up empty hash entries
                if range_map.is_empty() {
                    self.atom_uuids.remove(hash_value);
                }
                return Some(entry.atom_uuid);
            }
            None
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
                .map(move |(range_value, entry)| (hash_value, range_value, &entry.atom_uuid))
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

    /// Inserts an entry whose writer identity was supplied by the caller
    /// rather than produced by a local keypair. Used by the replay/import
    /// path (e.g. inbound `data_share` from another node): the original
    /// author's `writer_pubkey` is preserved on the AtomEntry so downstream
    /// queries can attribute the record to its sender.
    ///
    /// The caller is responsible for the meaning of `signature` /
    /// `signature_version`. Pass `signature_version = 0` and an empty
    /// `signature` when no verifiable signature is available — `verify_key`
    /// will then return false for this entry, which is the correct semantics
    /// for an imported record whose canonical bytes (built from the local
    /// `written_at`) differ from whatever the original author signed.
    pub fn set_atom_uuid_from_values_imported(
        &mut self,
        hash_value: String,
        range_value: String,
        atom_uuid: String,
        writer_pubkey: String,
        signature: String,
        signature_version: u8,
    ) {
        let changed = self.get_atom_uuid(&hash_value, &range_value) != Some(&atom_uuid);
        if changed {
            self.version += 1;
            let key_value = KeyValue::new(Some(hash_value.clone()), Some(range_value.clone()));
            self.update_order.push(key_value);
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
        self.atom_uuids.entry(hash_value).or_default().insert(
            range_value,
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
    pub(crate) fn set_atom_uuid_from_values_unsigned(
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
        self.atom_uuids.entry(hash_value).or_default().insert(
            range_value,
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
    /// Layout: molecule_uuid | 0x00 | hash_value | 0x00 | range_value | 0x00 | atom_uuid | 0x00 | written_at(u64 BE)
    fn build_canonical_bytes(
        molecule_uuid: &str,
        hash_value: &str,
        range_value: &str,
        atom_uuid: &str,
        written_at: u64,
    ) -> Vec<u8> {
        let mut buf = Vec::new();
        buf.extend_from_slice(molecule_uuid.as_bytes());
        buf.push(0x00);
        buf.extend_from_slice(hash_value.as_bytes());
        buf.push(0x00);
        buf.extend_from_slice(range_value.as_bytes());
        buf.push(0x00);
        buf.extend_from_slice(atom_uuid.as_bytes());
        buf.push(0x00);
        buf.extend_from_slice(&written_at.to_be_bytes());
        buf
    }

    /// Verifies the signature for a specific hash+range key entry.
    #[must_use]
    pub fn verify_key(&self, hash_value: &str, range_value: &str) -> bool {
        let entry = match self
            .atom_uuids
            .get(hash_value)
            .and_then(|rm| rm.get(range_value))
        {
            Some(e) => e,
            None => return false,
        };
        if entry.signature_version == 0 {
            return false;
        }
        let canonical = Self::build_canonical_bytes(
            &self.uuid,
            hash_value,
            range_value,
            &entry.atom_uuid,
            entry.written_at,
        );
        crate::security::verify_molecule_signature(
            &canonical,
            &entry.signature,
            &entry.writer_pubkey,
        )
    }

    /// Merges another MoleculeHashRange into this one using last-writer-wins per key.
    /// Returns a list of conflicts where both sides had different atoms for the same key.
    pub fn merge(
        &mut self,
        other: &MoleculeHashRange,
        _keypair: &Ed25519KeyPair,
    ) -> Vec<MergeConflict> {
        let mut conflicts = Vec::new();
        for (hash, other_range_map) in &other.atom_uuids {
            for (range, other_entry) in other_range_map {
                let self_entry = self.atom_uuids.get(hash).and_then(|rm| rm.get(range));

                match self_entry {
                    None => {
                        self.atom_uuids
                            .entry(hash.clone())
                            .or_default()
                            .insert(range.clone(), other_entry.clone());
                        self.version += 1;
                    }
                    Some(se) => {
                        if se.atom_uuid == other_entry.atom_uuid {
                            continue;
                        }
                        let (winner, loser) = if other_entry.written_at >= se.written_at {
                            (other_entry, se)
                        } else {
                            (se, other_entry)
                        };
                        conflicts.push(MergeConflict {
                            key: format!("{}:{}", hash, range),
                            winner_atom: winner.atom_uuid.clone(),
                            loser_atom: loser.atom_uuid.clone(),
                            winner_written_at: winner.written_at,
                            loser_written_at: loser.written_at,
                        });
                        if other_entry.written_at >= se.written_at {
                            self.atom_uuids
                                .entry(hash.clone())
                                .or_default()
                                .insert(range.clone(), other_entry.clone());
                            self.version += 1;
                        }
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
        let mol = MoleculeHashRange::new("schema", "field");
        assert_eq!(mol.version(), 0);
    }

    #[test]
    fn test_version_bumps_on_insert() {
        let kp = test_keypair();
        let mut mol = MoleculeHashRange::new("schema", "field");
        mol.set_atom_uuid_from_values(
            "h1".to_string(),
            "r1".to_string(),
            "atom-1".to_string(),
            &kp,
        );
        assert_eq!(mol.version(), 1);
    }

    #[test]
    fn test_version_no_bump_on_same_value() {
        let kp = test_keypair();
        let mut mol = MoleculeHashRange::new("schema", "field");
        mol.set_atom_uuid_from_values(
            "h1".to_string(),
            "r1".to_string(),
            "atom-1".to_string(),
            &kp,
        );
        mol.set_atom_uuid_from_values(
            "h1".to_string(),
            "r1".to_string(),
            "atom-1".to_string(),
            &kp,
        );
        assert_eq!(mol.version(), 1);
    }

    #[test]
    fn test_version_bumps_on_remove() {
        let kp = test_keypair();
        let mut mol = MoleculeHashRange::new("schema", "field");
        mol.set_atom_uuid_from_values(
            "h1".to_string(),
            "r1".to_string(),
            "atom-1".to_string(),
            &kp,
        );
        assert_eq!(mol.version(), 1);
        mol.remove_atom_uuid("h1", "r1");
        assert_eq!(mol.version(), 2);
    }

    #[test]
    fn test_version_no_bump_on_remove_missing() {
        let mut mol = MoleculeHashRange::new("schema", "field");
        mol.remove_atom_uuid("h1", "r1");
        assert_eq!(mol.version(), 0);
    }

    #[test]
    fn test_with_atoms_starts_at_zero() {
        let mol = MoleculeHashRange::with_atoms(
            "schema",
            "field",
            HashMap::from([(
                "h1".to_string(),
                std::collections::BTreeMap::from([("r1".to_string(), "a1".to_string())]),
            )]),
        );
        assert_eq!(mol.version(), 0);
    }

    #[test]
    fn test_deterministic_uuid() {
        let mol1 = MoleculeHashRange::new("my_schema", "my_field");
        let mol2 = MoleculeHashRange::new("my_schema", "my_field");
        assert_eq!(mol1.uuid(), mol2.uuid());
    }

    #[test]
    fn test_merge_new_keys() {
        let kp = test_keypair();
        let mut mol1 = MoleculeHashRange::new("s", "f");
        mol1.set_atom_uuid_from_values(
            "h1".to_string(),
            "r1".to_string(),
            "atom-1".to_string(),
            &kp,
        );

        let mut mol2 = MoleculeHashRange::new("s", "f");
        mol2.set_atom_uuid_from_values(
            "h2".to_string(),
            "r2".to_string(),
            "atom-2".to_string(),
            &kp,
        );

        let conflicts = mol1.merge(&mol2, &kp);
        assert!(conflicts.is_empty());
        assert_eq!(mol1.get_atom_uuid("h1", "r1"), Some(&"atom-1".to_string()));
        assert_eq!(mol1.get_atom_uuid("h2", "r2"), Some(&"atom-2".to_string()));
    }

    /// Imported entries preserve the supplied `writer_pubkey` verbatim
    /// rather than overwriting it with a locally-derived signer pubkey.
    /// This is the load-bearing property for cross-node share replay
    /// (face-discovery-3node `bob.shared_record_count[Photography]`).
    #[test]
    fn test_set_atom_uuid_from_values_imported_preserves_external_pubkey() {
        let mut mol = MoleculeHashRange::new("schema", "field");
        mol.set_atom_uuid_from_values_imported(
            "h1".to_string(),
            "r1".to_string(),
            "atom-1".to_string(),
            "alice-pubkey-base64".to_string(),
            String::new(),
            0,
        );
        let entry = mol.get_atom_entry("h1", "r1").expect("entry present");
        assert_eq!(entry.writer_pubkey, "alice-pubkey-base64");
        assert_eq!(entry.signature_version, 0);
        // signature_version=0 means "unverifiable replay"; verify_key
        // must therefore return false rather than claiming verification.
        assert!(!mol.verify_key("h1", "r1"));
        // No User provenance is attached when there's no verifiable
        // signature — anything else would lie about authenticity.
        assert!(entry.provenance.is_none());
    }

    /// Sibling test: when `signature_version > 0`, the imported entry
    /// stamps a `Provenance::User` whose pubkey/signature mirror the
    /// supplied values. This is the path a future "replay with original
    /// signature" feature would take.
    #[test]
    fn test_set_atom_uuid_from_values_imported_with_signature_records_provenance() {
        let mut mol = MoleculeHashRange::new("schema", "field");
        mol.set_atom_uuid_from_values_imported(
            "h1".to_string(),
            "r1".to_string(),
            "atom-1".to_string(),
            "pk".to_string(),
            "sig".to_string(),
            1,
        );
        let entry = mol.get_atom_entry("h1", "r1").expect("entry present");
        match entry.provenance.as_ref() {
            Some(crate::atom::Provenance::User {
                pubkey,
                signature,
                signature_version,
            }) => {
                assert_eq!(pubkey, "pk");
                assert_eq!(signature, "sig");
                assert_eq!(*signature_version, 1);
            }
            other => panic!("expected User variant, got {:?}", other),
        }
    }

    #[test]
    fn test_merge_conflict_later_wins() {
        let kp = test_keypair();
        let mut mol1 = MoleculeHashRange::new("s", "f");
        mol1.set_atom_uuid_from_values(
            "h1".to_string(),
            "r1".to_string(),
            "atom-old".to_string(),
            &kp,
        );

        std::thread::sleep(std::time::Duration::from_millis(1));

        let mut mol2 = MoleculeHashRange::new("s", "f");
        mol2.set_atom_uuid_from_values(
            "h1".to_string(),
            "r1".to_string(),
            "atom-new".to_string(),
            &kp,
        );

        let conflicts = mol1.merge(&mol2, &kp);
        assert_eq!(conflicts.len(), 1);
        assert_eq!(conflicts[0].winner_atom, "atom-new");
        assert_eq!(
            mol1.get_atom_uuid("h1", "r1"),
            Some(&"atom-new".to_string())
        );
    }
}
