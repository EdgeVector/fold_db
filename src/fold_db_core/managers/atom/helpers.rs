//! Helper functions and utilities for AtomManager

use crate::atom::{Atom, AtomStatus, Molecule, MoleculeRange};
use crate::db_operations::DbOperations;
use serde_json::Value;
use std::sync::Arc;

/// Create a new atom in the database
pub fn create_atom(
    db_ops: &Arc<DbOperations>,
    schema_name: &str,
    source_pub_key: String,
    content: Value,
) -> Result<Atom, Box<dyn std::error::Error>> {
    db_ops.create_atom(
        schema_name,
        source_pub_key,
        None,
        content,
        Some(AtomStatus::Active),
    ).map_err(|e| Box::new(e) as Box<dyn std::error::Error>)
}

/// Update a molecule reference
pub fn update_molecule(
    db_ops: &Arc<DbOperations>,
    molecule_uuid: &str,
    atom_uuid: String,
    source_pub_key: String,
) -> Result<Molecule, Box<dyn std::error::Error>> {
    db_ops.update_molecule(molecule_uuid, atom_uuid, source_pub_key)
        .map_err(|e| Box::new(e) as Box<dyn std::error::Error>)
}

/// Update a molecule range
pub fn update_molecule_range(
    db_ops: &Arc<DbOperations>,
    molecule_uuid: &str,
    atom_uuid: String,
    key: String,
    source_pub_key: String,
) -> Result<MoleculeRange, Box<dyn std::error::Error>> {
    db_ops.update_molecule_range(molecule_uuid, atom_uuid, key, source_pub_key)
        .map_err(|e| Box::new(e) as Box<dyn std::error::Error>)
}

/// Get atom history for a given atom reference
pub fn get_atom_history(
    db_ops: &Arc<DbOperations>,
    aref_uuid: &str,
) -> Result<Vec<Atom>, Box<dyn std::error::Error>> {
    // Load the atom ref from database
    let key = format!("ref:{}", aref_uuid);
    
    match db_ops.db().get(&key)? {
        Some(bytes) => {
            // Try to deserialize as Molecule first
            if let Ok(molecule) = serde_json::from_slice::<Molecule>(&bytes) {
                let atom_uuid = molecule.get_atom_uuid();
                
                // Get the current atom
                let atom_key = format!("atom:{}", atom_uuid);
                match db_ops.db().get(&atom_key)? {
                    Some(atom_bytes) => {
                        let atom: Atom = serde_json::from_slice(&atom_bytes)?;
                        Ok(vec![atom])
                    }
                    None => Ok(vec![])
                }
            }
            // Try as MoleculeRange
            else if let Ok(_range) = serde_json::from_slice::<MoleculeRange>(&bytes) {
                // For ranges, we would need to iterate through all atoms in the range
                // For now, return empty vector
                Ok(vec![])
            } else {
                Err("Failed to deserialize atom reference or molecule".into())
            }
        }
        None => Ok(vec![])
    }
}