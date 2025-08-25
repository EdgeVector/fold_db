//! Field value processing logic for AtomManager

use super::AtomManager;
use crate::atom::{Atom, AtomStatus};
use crate::fold_db_core::infrastructure::message_bus::{
    atom_events::FieldValueSet,
    request_events::{FieldValueSetRequest, FieldValueSetResponse},
};
use log::{info, warn, error};
use std::time::Instant;

/// Handle FieldValueSetRequest by creating atom and appropriate Molecule - CRITICAL MUTATION BUG FIX
pub(super) fn handle_fieldvalueset_request(manager: &AtomManager, request: FieldValueSetRequest) -> Result<(), Box<dyn std::error::Error>> {
    info!("📝 Processing FieldValueSetRequest for field: {}.{}", request.schema_name, request.field_name);
    info!("🔍 DIAGNOSTIC: FieldValueSetRequest details - correlation_id: {}, value: {}", request.correlation_id, request.value);
    
    update_processing_stats(manager);

    // Step 1: Create atom with the field value
    let atom_result = create_atom_for_field_value(manager, &request);

    let response = match atom_result {
        Ok(atom) => {
            let atom_uuid = atom.uuid().to_string();
            store_atom_in_cache(manager, atom.clone());
            
            // Step 2: Create appropriate Molecule based on field type
            let molecule_result = create_molecule_for_field(manager, &request, &atom_uuid);
            
            match molecule_result {
                Ok(molecule_uuid) => {
                    handle_successful_field_value_processing(manager, &request, &atom_uuid, &molecule_uuid)
                }
                Err(e) => {
                    update_failure_stats(manager);
                    create_molecule_error_response(&request.correlation_id, e)
                }
            }
        }
        Err(e) => {
            update_failure_stats(manager);
            create_atom_error_response(&request.correlation_id, e)
        }
    };

    // Publish response - Don't fail the operation if response publishing fails
    if let Err(e) = manager.message_bus.publish(response) {
        warn!("⚠️ Failed to publish FieldValueSetResponse: {}. Operation completed successfully.", e);
    }
    Ok(())
}

/// Update processing stats for a new request
fn update_processing_stats(manager: &AtomManager) {
    let mut stats = manager.stats.lock().unwrap();
    stats.requests_processed += 1;
    stats.last_activity = Some(Instant::now());
}

/// Update failure stats
fn update_failure_stats(manager: &AtomManager) {
    let mut stats = manager.stats.lock().unwrap();
    stats.requests_failed += 1;
}

/// Create atom for field value
fn create_atom_for_field_value(manager: &AtomManager, request: &FieldValueSetRequest) -> Result<Atom, Box<dyn std::error::Error>> {
    info!("🔍 DIAGNOSTIC: Step 1 - Creating atom for schema: {}", request.schema_name);
    
    let atom_result = manager.db_ops.create_atom(
        &request.schema_name,
        request.source_pub_key.clone(),
        None, // No previous atom for field value sets
        request.value.clone(),
        Some(AtomStatus::Active),
    );
    
    match atom_result {
        Ok(atom) => {
            info!("🔍 DIAGNOSTIC: Step 1 SUCCESS - Created atom with UUID: {}", atom.uuid());
            Ok(atom)
        }
        Err(e) => Err(Box::new(e))
    }
}

/// Store atom in memory cache
fn store_atom_in_cache(manager: &AtomManager, atom: Atom) {
    let atom_uuid = atom.uuid().to_string();
    manager.atoms.lock().unwrap().insert(atom_uuid, atom);
    info!("🔍 DIAGNOSTIC: Stored atom in memory cache");
}

/// Create appropriate Molecule for the field based on its type
fn create_molecule_for_field(manager: &AtomManager, request: &FieldValueSetRequest, atom_uuid: &str) -> Result<String, Box<dyn std::error::Error>> {
    let field_type = determine_field_type(manager, &request.schema_name, &request.field_name);
    info!("🔍 DIAGNOSTIC: Step 2 - Determined field type: {}", field_type);
    
    match field_type.as_str() {
        "Range" => create_range_molecule(manager, request, atom_uuid),
        _ => create_single_molecule(manager, request, atom_uuid),
    }
}

/// Create MoleculeRange for Range fields
fn create_range_molecule(manager: &AtomManager, request: &FieldValueSetRequest, atom_uuid: &str) -> Result<String, Box<dyn std::error::Error>> {
    let molecule_uuid = format!("{}_{}_range", request.schema_name, request.field_name);
    info!("🔍 DIAGNOSTIC: Creating MoleculeRange with UUID: {} -> atom: {}", molecule_uuid, atom_uuid);
    
    let range_key = extract_range_key_from_value(&request.value);
    info!("🔍 DIAGNOSTIC: Extracted range key: '{}' from value: {}", range_key, request.value);
    
    let range_result = manager.db_ops.update_molecule_range(
        &molecule_uuid,
        atom_uuid.to_string(),
        range_key,
        request.source_pub_key.clone(),
    );
    
    match range_result {
        Ok(range) => {
            manager.molecule_ranges.lock().unwrap().insert(molecule_uuid.clone(), range);
            info!("🔍 DIAGNOSTIC: Successfully created and stored MoleculeRange: {}", molecule_uuid);
            
            // Verify the MoleculeRange was properly stored in database
            match manager.db_ops.get_item::<crate::atom::MoleculeRange>(&format!("ref:{}", molecule_uuid)) {
                Ok(Some(_)) => {
                    info!("✅ VERIFICATION: MoleculeRange {} confirmed in database", molecule_uuid);
                }
                Ok(None) => {
                    error!("❌ VERIFICATION FAILED: MoleculeRange {} not found in database after storage", molecule_uuid);
                }
                Err(e) => {
                    error!("❌ VERIFICATION ERROR: Failed to verify MoleculeRange {}: {}", molecule_uuid, e);
                }
            }
            
            Ok(molecule_uuid)
        }
        Err(e) => {
            error!("❌ DIAGNOSTIC: Failed to create MoleculeRange: {}", e);
            Err(Box::new(e))
        }
    }
}

/// Create Molecule for Single fields (default)
fn create_single_molecule(manager: &AtomManager, request: &FieldValueSetRequest, atom_uuid: &str) -> Result<String, Box<dyn std::error::Error>> {
    let molecule_uuid = format!("{}_{}_single", request.schema_name, request.field_name);
    info!("🔍 DIAGNOSTIC: Creating Molecule (Single) with UUID: {} -> atom: {}", molecule_uuid, atom_uuid);
    
    let single_result = manager.db_ops.update_molecule(
        &molecule_uuid,
        atom_uuid.to_string(),
        request.source_pub_key.clone(),
    );
    
    match single_result {
        Ok(molecule) => {
            info!("🔍 DIAGNOSTIC: Molecule created successfully, final atom_uuid: {}", molecule.get_atom_uuid());
            manager.molecules.lock().unwrap().insert(molecule_uuid.clone(), molecule);
            info!("🔍 DIAGNOSTIC: Successfully created and stored Molecule: {}", molecule_uuid);

            // Verify the Molecule was properly stored in database
            match manager.db_ops.get_item::<crate::atom::Molecule>(&format!("ref:{}", molecule_uuid)) {
                Ok(Some(_)) => {
                    info!("✅ VERIFICATION: Molecule {} confirmed in database", molecule_uuid);
                }
                Ok(None) => {
                    error!("❌ VERIFICATION FAILED: Molecule {} not found in database after storage", molecule_uuid);
                }
                Err(e) => {
                    error!("❌ VERIFICATION ERROR: Failed to verify Molecule {}: {}", molecule_uuid, e);
                }
            }
            
            Ok(molecule_uuid)
        }
        Err(e) => {
            error!("❌ DIAGNOSTIC: Failed to create Molecule: {}", e);
            Err(Box::new(e))
        }
    }
}

/// Extract range key from request value for Range fields
fn extract_range_key_from_value(value: &serde_json::Value) -> String {
    if let Some(obj) = value.as_object() {
        // Extract the VALUE of the "range_key" field, not the field name itself
        if let Some(range_key_value) = obj.get("range_key") {
            if let Some(key_str) = range_key_value.as_str() {
                key_str.to_string()
            } else {
                // Handle non-string range keys by converting to string
                range_key_value.to_string().trim_matches('"').to_string()
            }
        } else {
            warn!("🔶 RANGE KEY WARNING: No 'range_key' field found in value, using 'default'");
            "default".to_string()
        }
    } else {
        warn!("🔶 RANGE KEY WARNING: Value is not an object, using 'default'");
        "default".to_string()
    }
}

/// Handle successful field value processing
fn handle_successful_field_value_processing(manager: &AtomManager, request: &FieldValueSetRequest, atom_uuid: &str, molecule_uuid: &str) -> FieldValueSetResponse {
    let mut stats = manager.stats.lock().unwrap();
    stats.atoms_created += 1;
    stats.molecules_created += 1;
    drop(stats);
    
    info!("✅ Successfully processed FieldValueSetRequest - atom: {}, molecule: {}", atom_uuid, molecule_uuid);
    info!("🔍 DIAGNOSTIC: Final mapping - Molecule {} -> Atom {}", molecule_uuid, atom_uuid);
    
    // Publish FieldValueSet event to trigger transform chain
    publish_field_value_set_event(manager, request);
    
    FieldValueSetResponse::new(
        request.correlation_id.clone(),
        true,
        Some(molecule_uuid.to_string()),
        None,
    )
}

/// Publish FieldValueSet event to trigger transform chain
fn publish_field_value_set_event(manager: &AtomManager, request: &FieldValueSetRequest) {
    let field_key = format!("{}.{}", request.schema_name, request.field_name);
    let field_value_event = FieldValueSet {
        field: field_key.clone(),
        value: request.value.clone(),
        source: "AtomManager".to_string(),
    };
    
    info!("🔔 DIAGNOSTIC FIX: Publishing FieldValueSet event - field: {}, source: AtomManager", field_key);
    match manager.message_bus.publish(field_value_event) {
        Ok(_) => {
            info!("✅ DIAGNOSTIC FIX: Successfully published FieldValueSet event for: {}", field_key);
        }
        Err(e) => {
            error!("❌ DIAGNOSTIC FIX: Failed to publish FieldValueSet event for {}: {}", field_key, e);
            // Continue processing even if event publication fails
        }
    }
}

/// Create error response for Molecule creation failure
fn create_molecule_error_response(correlation_id: &str, error: Box<dyn std::error::Error>) -> FieldValueSetResponse {
    error!("❌ Failed to create Molecule for FieldValueSetRequest: {}", error);
    FieldValueSetResponse::new(
        correlation_id.to_string(),
        false,
        None,
        Some(format!("Failed to create Molecule: {}", error)),
    )
}

/// Create error response for Atom creation failure
fn create_atom_error_response(correlation_id: &str, error: Box<dyn std::error::Error>) -> FieldValueSetResponse {
    error!("❌ Failed to create Atom for FieldValueSetRequest: {}", error);
    FieldValueSetResponse::new(
        correlation_id.to_string(),
        false,
        None,
        Some(format!("Failed to create Atom: {}", error)),
    )
}

/// Determine field type based on schema and field name
fn determine_field_type(manager: &AtomManager, schema_name: &str, field_name: &str) -> String {
    // Look up the actual schema to determine field type
    match manager.db_ops.get_schema(schema_name) {
        Ok(Some(schema)) => {
            match schema.fields.get(field_name) {
                Some(crate::schema::types::field::FieldVariant::Range(_)) => {
                    info!("🔍 FIELD TYPE: {} in schema {} is Range", field_name, schema_name);
                    "Range".to_string()
                }
                Some(crate::schema::types::field::FieldVariant::Single(_)) => {
                    info!("🔍 FIELD TYPE: {} in schema {} is Single", field_name, schema_name);
                    "Single".to_string()
                }
                Some(crate::schema::types::field::FieldVariant::HashRange(_)) => {
                    info!("🔍 FIELD TYPE: {} in schema {} is HashRange", field_name, schema_name);
                    "HashRange".to_string()
                }
                None => {
                    warn!("⚠️ FIELD TYPE: Field {} not found in schema {}, defaulting to Single", field_name, schema_name);
                    "Single".to_string()
                }
            }
        }
        Ok(None) => {
            warn!("⚠️ FIELD TYPE: Schema {} not found, defaulting to Single", schema_name);
            "Single".to_string()
        }
        Err(e) => {
            error!("❌ FIELD TYPE: Error loading schema {}: {}, defaulting to Single", schema_name, e);
            "Single".to_string()
        }
    }
}