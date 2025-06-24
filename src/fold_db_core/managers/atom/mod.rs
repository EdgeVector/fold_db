//! Pure Event-Driven AtomManager Module
//!
//! This module contains the AtomManager implementation broken down into logical components:
//! - Main AtomManager struct and interface
//! - Event processing threads
//! - Request handlers
//! - Field processing utilities
//! - Helper methods

mod event_processing;
mod request_handlers;
mod field_processing;
mod helpers;

use crate::atom::{Atom, Molecule, MoleculeRange};
use crate::db_operations::DbOperations;
use crate::fold_db_core::infrastructure::message_bus::MessageBus;
use std::collections::HashMap;
use std::sync::{Arc, Mutex};
use std::thread::JoinHandle;

/// Re-export unified statistics from shared stats module
pub use crate::fold_db_core::shared::EventDrivenAtomStats;

/// Pure event-driven AtomManager that only communicates via events
pub struct AtomManager {
    pub(crate) db_ops: Arc<DbOperations>,
    pub(crate) atoms: Arc<Mutex<HashMap<String, Atom>>>,
    pub(crate) molecules: Arc<Mutex<HashMap<String, Molecule>>>,
    pub(crate) molecule_ranges: Arc<Mutex<HashMap<String, MoleculeRange>>>,
    pub(crate) message_bus: Arc<MessageBus>,
    pub(crate) stats: Arc<Mutex<EventDrivenAtomStats>>,
    pub(crate) event_threads: Arc<Mutex<Vec<JoinHandle<()>>>>,
}

impl AtomManager {
    pub fn new(db_ops: DbOperations, message_bus: Arc<MessageBus>) -> Self {
        let mut atoms = HashMap::new();
        let mut molecules = HashMap::new();
        let mut molecule_ranges = HashMap::new();

        // Load existing data from database
        for result in db_ops.db().iter().flatten() {
            let key_str = String::from_utf8_lossy(result.0.as_ref());
            let bytes = result.1.as_ref();

            if let Some(stripped) = key_str.strip_prefix("atom:") {
                if let Ok(atom) = serde_json::from_slice(bytes) {
                    atoms.insert(stripped.to_string(), atom);
                }
            } else if let Some(stripped) = key_str.strip_prefix("ref:") {
                if let Ok(molecule) = serde_json::from_slice::<Molecule>(bytes) {
                    molecules.insert(stripped.to_string(), molecule);
                } else if let Ok(mol_range) = serde_json::from_slice::<MoleculeRange>(bytes) {
                    molecule_ranges.insert(stripped.to_string(), mol_range);
                }
            }
        }

        let manager = Self {
            db_ops: Arc::new(db_ops),
            atoms: Arc::new(Mutex::new(atoms)),
            molecules: Arc::new(Mutex::new(molecules)),
            molecule_ranges: Arc::new(Mutex::new(molecule_ranges)),
            message_bus: Arc::clone(&message_bus),
            stats: Arc::new(Mutex::new(EventDrivenAtomStats::new())),
            event_threads: Arc::new(Mutex::new(Vec::new())),
        };

        // Start pure event-driven processing
        manager.start_event_processing();
        manager
    }

    /// Public API methods for direct access (for backward compatibility)
    pub fn create_atom(
        &self,
        schema_name: &str,
        source_pub_key: String,
        content: serde_json::Value,
    ) -> Result<Atom, Box<dyn std::error::Error>> {
        helpers::create_atom(&self.db_ops, schema_name, source_pub_key, content)
    }

    pub fn update_molecule(
        &self,
        molecule_uuid: &str,
        atom_uuid: String,
        source_pub_key: String,
    ) -> Result<Molecule, Box<dyn std::error::Error>> {
        helpers::update_molecule(&self.db_ops, molecule_uuid, atom_uuid, source_pub_key)
    }

    pub fn update_molecule_range(
        &self,
        molecule_uuid: &str,
        atom_uuid: String,
        key: String,
        source_pub_key: String,
    ) -> Result<MoleculeRange, Box<dyn std::error::Error>> {
        helpers::update_molecule_range(&self.db_ops, molecule_uuid, atom_uuid, key, source_pub_key)
    }

    pub fn get_atom_history(
        &self,
        molecule_uuid: &str,
    ) -> Result<Vec<crate::atom::Atom>, Box<dyn std::error::Error>> {
        helpers::get_atom_history(&self.db_ops, molecule_uuid)
    }

    /// Get current statistics for testing
    pub fn get_stats(&self) -> EventDrivenAtomStats {
        self.stats.lock().unwrap().clone()
    }
}

impl Clone for AtomManager {
    fn clone(&self) -> Self {
        Self {
            db_ops: Arc::clone(&self.db_ops),
            atoms: Arc::clone(&self.atoms),
            molecules: Arc::clone(&self.molecules),
            molecule_ranges: Arc::clone(&self.molecule_ranges),
            message_bus: Arc::clone(&self.message_bus),
            stats: Arc::clone(&self.stats),
            event_threads: Arc::clone(&self.event_threads),
        }
    }
}