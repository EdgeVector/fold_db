use crate::atom::molecule_behavior::MoleculeBehavior;
use crate::atom::molecule_types::{apply_status_update, MoleculeStatus, MoleculeUpdate};
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;
use uuid::Uuid;

/// A range-based collection of atom references stored in a BTreeMap.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MoleculeRange {
    uuid: String,
    pub(crate) atom_uuids: BTreeMap<String, String>,
    updated_at: DateTime<Utc>,
    status: MoleculeStatus,
    update_history: Vec<MoleculeUpdate>,
}

impl MoleculeRange {
    /// Creates a new empty MoleculeRange.
    #[must_use]
    pub fn new(source_pub_key: String) -> Self {
        Self {
            uuid: Uuid::new_v4().to_string(),
            atom_uuids: BTreeMap::new(),
            updated_at: Utc::now(),
            status: MoleculeStatus::Active,
            update_history: vec![MoleculeUpdate {
                timestamp: Utc::now(),
                status: MoleculeStatus::Active,
                source_pub_key,
            }],
        }
    }

    /// Updates or adds a reference at the specified key.
    /// If the key already exists, the atom_uuid replaces the existing value.
    pub fn set_atom_uuid(&mut self, key: String, atom_uuid: String) {
        log::debug!("Setting atom_uuid for molecule_uuid: {} -> key: {} -> atom: {}", self.uuid, key, atom_uuid);
        self.atom_uuids.insert(key, atom_uuid);
        self.updated_at = Utc::now();
    }

    /// Returns the UUID of the Atom referenced by the specified key.
    #[must_use]
    pub fn get_atom_uuid(&self, key: &str) -> Option<&String> {
        self.atom_uuids.get(key)
    }


    /// Removes the reference at the specified key.
    #[allow(clippy::manual_inspect)]
    pub fn remove_atom_uuid(&mut self, key: &str) -> Option<String> {
        self.atom_uuids.remove(key).map(|uuid| {
            self.updated_at = Utc::now();
            uuid
        })
    }

}

impl MoleculeBehavior for MoleculeRange {
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
