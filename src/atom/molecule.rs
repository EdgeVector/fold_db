use crate::atom::molecule_behavior::MoleculeBehavior;
use crate::atom::molecule_types::{apply_status_update, MoleculeStatus, MoleculeUpdate};
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

/// A reference to a single atom version.
#[derive(Debug, Clone, Serialize, Deserialize, utoipa::ToSchema)]
pub struct Molecule {
    molecule_uuid: String,
    atom_uuid: String,
    #[schema(value_type = String, format = "date-time")]
    updated_at: DateTime<Utc>,
    status: MoleculeStatus,
    update_history: Vec<MoleculeUpdate>,
    #[serde(default)]
    version: u64,
}

impl Molecule {
    /// Creates a new Molecule pointing to the specified Atom.
    #[must_use]
    pub fn new(atom_uuid: String, source_pub_key: String) -> Self {
        Self {
            molecule_uuid: Uuid::new_v4().to_string(),
            atom_uuid,
            updated_at: Utc::now(),
            status: MoleculeStatus::Active,
            update_history: vec![MoleculeUpdate {
                timestamp: Utc::now(),
                status: MoleculeStatus::Active,
                source_pub_key,
            }],
            version: 0,
        }
    }

    /// Updates the reference to point to a new Atom version.
    /// Bumps the version counter only when the atom actually changes.
    pub fn set_atom_uuid(&mut self, atom_uuid: String) {
        if self.atom_uuid != atom_uuid {
            self.version += 1;
        }
        self.atom_uuid = atom_uuid;
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
}

impl MoleculeBehavior for Molecule {
    fn uuid(&self) -> &str {
        &self.molecule_uuid
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

    fn version(&self) -> u64 {
        self.version
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_version_starts_at_zero() {
        let mol = Molecule::new("atom-1".to_string(), "key".to_string());
        assert_eq!(mol.version(), 0);
    }

    #[test]
    fn test_version_bumps_on_change() {
        let mut mol = Molecule::new("atom-1".to_string(), "key".to_string());
        mol.set_atom_uuid("atom-2".to_string());
        assert_eq!(mol.version(), 1);
        mol.set_atom_uuid("atom-3".to_string());
        assert_eq!(mol.version(), 2);
    }

    #[test]
    fn test_version_no_bump_on_same_value() {
        let mut mol = Molecule::new("atom-1".to_string(), "key".to_string());
        mol.set_atom_uuid("atom-1".to_string());
        assert_eq!(mol.version(), 0);
    }
}
