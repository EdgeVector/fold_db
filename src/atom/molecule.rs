use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

use super::{deterministic_molecule_uuid, now_nanos, MergeConflict};

/// A reference to a single atom version.
#[derive(Debug, Clone, Serialize, Deserialize, utoipa::ToSchema)]
pub struct Molecule {
    molecule_uuid: String,
    /// The current atom entry with write timestamp.
    /// Kept as a flattened pair for backward-compat: old data without
    /// `written_at` will deserialize with `written_at: 0` via serde default.
    atom_uuid: String,
    #[serde(default)]
    written_at: u64,
    #[schema(value_type = String, format = "date-time")]
    updated_at: DateTime<Utc>,
    #[serde(default)]
    version: u64,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    key_metadata: Option<super::KeyMetadata>,
}

impl Molecule {
    /// Creates a new Molecule with a deterministic UUID derived from schema + field name.
    #[must_use]
    pub fn new(atom_uuid: String, schema_name: &str, field_name: &str) -> Self {
        Self {
            molecule_uuid: deterministic_molecule_uuid(schema_name, field_name),
            atom_uuid,
            written_at: now_nanos(),
            updated_at: Utc::now(),
            version: 0,
            key_metadata: None,
        }
    }

    /// Returns the unique identifier of this molecule.
    #[must_use]
    pub fn uuid(&self) -> &str {
        &self.molecule_uuid
    }

    /// Returns the timestamp of the last update.
    #[must_use]
    pub fn updated_at(&self) -> DateTime<Utc> {
        self.updated_at
    }

    /// Updates the reference to point to a new Atom version.
    /// Bumps the version counter only when the atom actually changes.
    pub fn set_atom_uuid(&mut self, atom_uuid: String) {
        if self.atom_uuid != atom_uuid {
            self.version += 1;
        }
        self.atom_uuid = atom_uuid;
        self.written_at = now_nanos();
        self.updated_at = Utc::now();
    }

    /// Returns the version counter for this molecule.
    #[must_use]
    pub fn version(&self) -> u64 {
        self.version
    }

    /// Returns the UUID of the referenced Atom.
    #[must_use]
    pub fn get_atom_uuid(&self) -> &String {
        &self.atom_uuid
    }

    /// Returns the write timestamp (nanos since epoch) for the current atom.
    #[must_use]
    pub fn written_at(&self) -> u64 {
        self.written_at
    }

    /// Sets per-key metadata on the molecule.
    pub fn set_key_metadata(&mut self, meta: super::KeyMetadata) {
        self.key_metadata = Some(meta);
    }

    /// Returns the per-key metadata, if any.
    #[must_use]
    pub fn get_key_metadata(&self) -> Option<&super::KeyMetadata> {
        self.key_metadata.as_ref()
    }

    /// Merges another Molecule into this one using last-writer-wins.
    /// If both have different atom_uuids, the one with a later `written_at` wins.
    /// Returns a `MergeConflict` if there was a genuine conflict (different atoms).
    pub fn merge(&mut self, other: &Molecule) -> Option<MergeConflict> {
        if self.atom_uuid == other.atom_uuid {
            return None;
        }
        let (winner_atom, loser_atom, winner_ts, loser_ts) = if other.written_at >= self.written_at
        {
            (
                other.atom_uuid.clone(),
                self.atom_uuid.clone(),
                other.written_at,
                self.written_at,
            )
        } else {
            (
                self.atom_uuid.clone(),
                other.atom_uuid.clone(),
                self.written_at,
                other.written_at,
            )
        };
        self.atom_uuid = winner_atom.clone();
        self.written_at = winner_ts;
        self.version += 1;
        self.updated_at = Utc::now();
        Some(MergeConflict {
            key: "single".to_string(),
            winner_atom,
            loser_atom,
            winner_written_at: winner_ts,
            loser_written_at: loser_ts,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_version_starts_at_zero() {
        let mol = Molecule::new("atom-1".to_string(), "schema", "field");
        assert_eq!(mol.version(), 0);
    }

    #[test]
    fn test_version_bumps_on_change() {
        let mut mol = Molecule::new("atom-1".to_string(), "schema", "field");
        mol.set_atom_uuid("atom-2".to_string());
        assert_eq!(mol.version(), 1);
        mol.set_atom_uuid("atom-3".to_string());
        assert_eq!(mol.version(), 2);
    }

    #[test]
    fn test_version_no_bump_on_same_value() {
        let mut mol = Molecule::new("atom-1".to_string(), "schema", "field");
        mol.set_atom_uuid("atom-1".to_string());
        assert_eq!(mol.version(), 0);
    }

    #[test]
    fn test_deterministic_uuid() {
        let mol1 = Molecule::new("atom-1".to_string(), "my_schema", "my_field");
        let mol2 = Molecule::new("atom-2".to_string(), "my_schema", "my_field");
        assert_eq!(
            mol1.uuid(),
            mol2.uuid(),
            "same schema+field => same molecule UUID"
        );
    }

    #[test]
    fn test_merge_no_conflict_same_atom() {
        let mut mol1 = Molecule::new("atom-1".to_string(), "s", "f");
        let mol2 = Molecule::new("atom-1".to_string(), "s", "f");
        assert!(mol1.merge(&mol2).is_none());
    }

    #[test]
    fn test_merge_conflict_later_wins() {
        let mut mol1 = Molecule::new("atom-1".to_string(), "s", "f");
        std::thread::sleep(std::time::Duration::from_millis(1));
        let mol2 = Molecule::new("atom-2".to_string(), "s", "f");
        let conflict = mol1.merge(&mol2).expect("should conflict");
        assert_eq!(conflict.winner_atom, "atom-2");
        assert_eq!(conflict.loser_atom, "atom-1");
        assert_eq!(mol1.get_atom_uuid(), "atom-2");
    }

    #[test]
    fn test_written_at_updates_on_set() {
        let mut mol = Molecule::new("atom-1".to_string(), "s", "f");
        let ts1 = mol.written_at();
        std::thread::sleep(std::time::Duration::from_millis(1));
        mol.set_atom_uuid("atom-2".to_string());
        assert!(mol.written_at() >= ts1);
    }
}
