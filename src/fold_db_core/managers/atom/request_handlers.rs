//! Request handlers for different types of AtomManager events

use super::AtomManager;
use crate::atom::{Atom, AtomStatus};
use crate::fold_db_core::infrastructure::message_bus::{
    atom_events::{AtomCreated, AtomUpdated, MoleculeCreated, MoleculeUpdated},
    request_events::{
        AtomCreateRequest, AtomCreateResponse, AtomUpdateRequest, AtomUpdateResponse,
        MoleculeCreateRequest, MoleculeCreateResponse, MoleculeUpdateRequest, MoleculeUpdateResponse,
        FieldValueSetRequest,
    },
};
use log::{info, warn};
use std::time::Instant;

impl AtomManager {
    /// Handle AtomCreateRequest by creating atom and publishing response
    pub(super) fn handle_atom_create_request(&self, request: AtomCreateRequest) -> Result<(), Box<dyn std::error::Error>> {
        info!("🔧 Processing AtomCreateRequest for schema: {}", request.schema_name);
        
        let mut stats = self.stats.lock().unwrap();
        stats.requests_processed += 1;
        stats.last_activity = Some(Instant::now());
        drop(stats);

        let result = self.db_ops.create_atom(
            &request.schema_name,
            request.source_pub_key.clone(),
            request.prev_atom_uuid.clone(),
            request.content.clone(),
            request.status.as_ref().and_then(|s| match s.as_str() {
                "Active" => Some(AtomStatus::Active),
                "Deleted" => Some(AtomStatus::Deleted),
                _ => None,
            }),
        );

        let response = match result {
            Ok(atom) => {
                // Store in memory cache
                self.atoms.lock().unwrap().insert(atom.uuid().to_string(), atom.clone());
                
                // Publish AtomCreated event
                let atom_created = AtomCreated::new(atom.uuid().to_string(), request.content.clone());
                if let Err(e) = self.message_bus.publish(atom_created) {
                    warn!("Failed to publish AtomCreated event: {}", e);
                }
                
                let mut stats = self.stats.lock().unwrap();
                stats.atoms_created += 1;
                drop(stats);
                
                AtomCreateResponse::new(
                    request.correlation_id,
                    true,
                    Some(atom.uuid().to_string()),
                    None,
                    Some(request.content),
                )
            }
            Err(e) => {
                let mut stats = self.stats.lock().unwrap();
                stats.requests_failed += 1;
                drop(stats);
                
                AtomCreateResponse::new(
                    request.correlation_id,
                    false,
                    None,
                    Some(e.to_string()),
                    None,
                )
            }
        };

        // Publish response - Don't fail the operation if response publishing fails
        if let Err(e) = self.message_bus.publish(response) {
            warn!("⚠️ Failed to publish AtomCreateResponse: {}. Operation completed successfully.", e);
        }
        Ok(())
    }

    /// Handle AtomUpdateRequest by updating atom and publishing response
    pub(super) fn handle_atom_update_request(&self, request: AtomUpdateRequest) -> Result<(), Box<dyn std::error::Error>> {
        info!("🔄 Processing AtomUpdateRequest for atom: {}", request.atom_uuid);
        
        let mut stats = self.stats.lock().unwrap();
        stats.requests_processed += 1;
        stats.last_activity = Some(Instant::now());
        drop(stats);

        // For simplicity, we'll create a new atom with the updated content
        // In a real implementation, you might want to update the existing atom
        let atom = Atom::new(
            "default_schema".to_string(),
            request.source_pub_key.clone(),
            request.content.clone(),
        );
        let atom_uuid = atom.uuid().to_string();

        let result = self.db_ops.db().insert(
            format!("atom:{}", atom_uuid),
            serde_json::to_vec(&atom)?,
        );

        let response = match result {
            Ok(_) => {
                // Store in memory cache
                self.atoms.lock().unwrap().insert(atom_uuid.clone(), atom);
                
                // Publish AtomUpdated event
                let atom_updated = AtomUpdated::new(atom_uuid, request.content);
                if let Err(e) = self.message_bus.publish(atom_updated) {
                    warn!("Failed to publish AtomUpdated event: {}", e);
                }
                
                let mut stats = self.stats.lock().unwrap();
                stats.atoms_updated += 1;
                drop(stats);
                
                AtomUpdateResponse::new(request.correlation_id, true, None)
            }
            Err(e) => {
                let mut stats = self.stats.lock().unwrap();
                stats.requests_failed += 1;
                drop(stats);
                
                AtomUpdateResponse::new(request.correlation_id, false, Some(e.to_string()))
            }
        };

        // Publish response - Don't fail the operation if response publishing fails
        if let Err(e) = self.message_bus.publish(response) {
            warn!("⚠️ Failed to publish AtomUpdateResponse: {}. Operation completed successfully.", e);
        }
        Ok(())
    }

    /// Handle MoleculeCreateRequest by creating Molecule and publishing response
    pub(super) fn handle_molecule_create_request(&self, request: MoleculeCreateRequest) -> Result<(), Box<dyn std::error::Error>> {
        info!("🔗 Processing MoleculeCreateRequest for type: {}", request.molecule_type);
        
        let mut stats = self.stats.lock().unwrap();
        stats.requests_processed += 1;
        stats.last_activity = Some(Instant::now());
        drop(stats);

        let result: Result<(), Box<dyn std::error::Error>> = match request.molecule_type.as_str() {
            "Single" => {
                let molecule = self.db_ops.update_molecule(
                    &request.molecule_uuid,
                    request.atom_uuid.clone(),
                    request.source_pub_key.clone(),
                )?;
                self.molecules.lock().unwrap().insert(request.molecule_uuid.clone(), molecule);
                Ok(())
            }
            "Range" => {
                let range = self.db_ops.update_molecule_range(
                    &request.molecule_uuid,
                    request.atom_uuid.clone(),
                    "default".to_string(), // Default key
                    request.source_pub_key.clone(),
                )?;
                self.molecule_ranges.lock().unwrap().insert(request.molecule_uuid.clone(), range);
                Ok(())
            }
            _ => Err(format!("Unknown Molecule type: {}", request.molecule_type).into())
        };

        let response = match result {
            Ok(_) => {
                // Publish MoleculeCreated event
                let molecule_created = MoleculeCreated::new(
                    &request.molecule_uuid,
                    &request.molecule_type,
                    format!("{}:{}", request.molecule_type.to_lowercase(), request.molecule_uuid),
                );
                if let Err(e) = self.message_bus.publish(molecule_created) {
                    warn!("Failed to publish MoleculeCreated event: {}", e);
                }
                
                let mut stats = self.stats.lock().unwrap();
                stats.molecules_created += 1;
                drop(stats);
                
                MoleculeCreateResponse::new(request.correlation_id, true, None)
            }
            Err(e) => {
                let mut stats = self.stats.lock().unwrap();
                stats.requests_failed += 1;
                drop(stats);
                
                MoleculeCreateResponse::new(request.correlation_id, false, Some(e.to_string()))
            }
        };

        // Publish response - Don't fail the operation if response publishing fails
        if let Err(e) = self.message_bus.publish(response) {
            warn!("⚠️ Failed to publish MoleculeCreateResponse: {}. Operation completed successfully.", e);
        }
        Ok(())
    }

    /// Handle MoleculeUpdateRequest by updating Molecule and publishing response
    pub(super) fn handle_molecule_update_request(&self, request: MoleculeUpdateRequest) -> Result<(), Box<dyn std::error::Error>> {
        info!("🔄 Processing MoleculeUpdateRequest for: {}", request.molecule_uuid);
        
        let mut stats = self.stats.lock().unwrap();
        stats.requests_processed += 1;
        stats.last_activity = Some(Instant::now());
        drop(stats);

        let result: Result<(), Box<dyn std::error::Error>> = match request.molecule_type.as_str() {
            "Single" => {
                let molecule = self.db_ops.update_molecule(
                    &request.molecule_uuid,
                    request.atom_uuid.clone(),
                    request.source_pub_key.clone(),
                )?;
                self.molecules.lock().unwrap().insert(request.molecule_uuid.clone(), molecule);
                Ok(())
            }
            "Range" => {
                let key = request.additional_data
                    .as_ref()
                    .and_then(|d| d.get("key"))
                    .and_then(|v| v.as_str())
                    .unwrap_or("default");
                let range = self.db_ops.update_molecule_range(
                    &request.molecule_uuid,
                    request.atom_uuid.clone(),
                    key.to_string(),
                    request.source_pub_key.clone(),
                )?;
                self.molecule_ranges.lock().unwrap().insert(request.molecule_uuid.clone(), range);
                Ok(())
            }
            _ => Err(format!("Unknown Molecule type: {}", request.molecule_type).into())
        };

        let response = match result {
            Ok(_) => {
                // Publish MoleculeUpdated event
                let molecule_updated = MoleculeUpdated::new(
                    &request.molecule_uuid,
                    format!("{}:{}", request.molecule_type.to_lowercase(), request.molecule_uuid),
                    "update",
                );
                if let Err(e) = self.message_bus.publish(molecule_updated) {
                    warn!("Failed to publish MoleculeUpdated event: {}", e);
                }
                
                let mut stats = self.stats.lock().unwrap();
                stats.molecules_updated += 1;
                drop(stats);
                
                MoleculeUpdateResponse::new(request.correlation_id, true, None)
            }
            Err(e) => {
                let mut stats = self.stats.lock().unwrap();
                stats.requests_failed += 1;
                drop(stats);
                
                MoleculeUpdateResponse::new(request.correlation_id, false, Some(e.to_string()))
            }
        };

        // Publish response - Don't fail the operation if response publishing fails
        if let Err(e) = self.message_bus.publish(response) {
            warn!("⚠️ Failed to publish MoleculeUpdateResponse: {}. Operation completed successfully.", e);
        }
        Ok(())
    }

    /// Handle FieldValueSetRequest by creating atom and appropriate Molecule - CRITICAL MUTATION BUG FIX
    pub(super) fn handle_fieldvalueset_request(&self, request: FieldValueSetRequest) -> Result<(), Box<dyn std::error::Error>> {
        // Delegate to field processing module
        super::field_processing::handle_fieldvalueset_request(self, request)
    }
}