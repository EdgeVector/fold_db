use super::molecule_variants::MoleculeVariant;
use crate::schema::types::{FieldVariant, Schema, SchemaError, field::common::Field};
use crate::atom::{Molecule, MoleculeRange, MoleculeHashRange};
use log::info;
use uuid::Uuid;

/// Maps fields between schemas based on their defined relationships.
/// 
/// This function handles field mapping and creates new molecule references for unmapped fields.
/// Range and HashRange fields are persisted directly as MoleculeRange/MoleculeHashRange records.
/// Only Single fields result in generic Molecule records being returned.
/// 
/// Note: The returned vector contains Molecule records for Single fields only and is not intended
/// for persistence - Range and HashRange fields are already persisted by this function.
pub fn map_fields(
    db_ops: &crate::db_operations::DbOperations,
    schema: &mut Schema,
) -> Result<Vec<MoleculeVariant>, SchemaError> {
    let mut molecules = Vec::new();

    // For unmapped fields, create a new molecule_uuid and Molecule
    // Only create new Molecules for fields that truly don't have them (None or empty)
    for field in schema.fields.values_mut() {
        let needs_new_molecule = match field.molecule_uuid() {
            None => true,
            Some(uuid) => uuid.is_empty(),
        };

        if needs_new_molecule {
            let molecule_uuid = Uuid::new_v4().to_string();

            // Create and store the appropriate atom reference type based on field type
            let key = format!("ref:{}", molecule_uuid);
            
            match field {
                FieldVariant::Range(_) => {
                    // For range fields, create MoleculeRange
                    let molecule_range = MoleculeRange::new(molecule_uuid.clone());
                    if let Err(e) = db_ops.store_item(&key, &molecule_range) {
                        info!("Failed to persist MoleculeRange '{}': {}", molecule_uuid, e);
                    } else {
                        info!("✅ Persisted MoleculeRange: {}", key);
                    }
                    molecules.push(MoleculeVariant::Range(molecule_range));
                }
                FieldVariant::Single(_) => {
                    // For single fields, create Molecule with matching ID
                    let molecule = Molecule::new(molecule_uuid.clone(), "system".to_string());
                    if let Err(e) = db_ops.store_item(&key, &molecule) {
                        info!("Failed to persist Molecule '{}': {}", molecule_uuid, e);
                    } else {
                        info!("✅ Persisted Molecule: {}", key);
                    }
                    molecules.push(MoleculeVariant::Single(molecule));
                }
                FieldVariant::HashRange(_) => {
                    // For HashRange fields, create MoleculeHashRange with matching ID
                    let molecule_hash_range = MoleculeHashRange::new(molecule_uuid.clone());
                    if let Err(e) = db_ops.store_item(&key, &molecule_hash_range) {
                        info!("Failed to persist MoleculeHashRange '{}': {}", molecule_uuid, e);
                    } else {
                        info!("✅ Persisted MoleculeHashRange: {}", key);
                    }
                    molecules.push(MoleculeVariant::HashRange(molecule_hash_range));
                }
            };

            // Set the molecule_uuid in the field - this will be used as the key to find the Molecule
            field.set_molecule_uuid(molecule_uuid);
        }
    }

    Ok(molecules)
}
