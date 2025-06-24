//! Unified transform manager utilities eliminating ALL duplication
//!
//! AGGRESSIVE CLEANUP: This module consolidates:
//! - conversion_helper.rs: JsonValue conversion utilities
//! - serialization_helper.rs: Mapping serialization utilities  
//! - event_publisher.rs: Event publishing utilities
//! - field_resolver.rs: Field value resolution utilities
//! - default_value_helper.rs: Default value generation utilities
//! - lock_helper.rs: Lock acquisition utilities
//! - logging_helper.rs: Debug logging utilities
#![allow(unused_imports)]
//! - validation_helper.rs: Validation utilities
//! - Plus multiple duplicate logging/helper patterns found throughout

use crate::fold_db_core::infrastructure::message_bus::{
    MessageBus,
    schema_events::TransformExecuted,
};
use crate::schema::types::{SchemaError, Schema};
use crate::schema::types::field::variant::FieldVariant;
use crate::schema::types::field::common::Field;
use serde_json::Value as JsonValue;
use log::{info, error, warn};
use std::sync::Arc;
use std::collections::{HashMap, HashSet};

// Re-export commonly used types to avoid import duplication
pub use serde_json::Value;

/// Single unified utility combining ALL transform manager utilities
pub struct TransformUtils;

pub mod conversion;
pub mod default_values;
pub mod locking;
pub mod serialization;
pub mod validation;

pub use conversion::*;
pub use default_values::*;
pub use locking::*;
pub use serialization::*;
pub use validation::*;

impl TransformUtils {
    // ========== EVENT PUBLISHING UTILITIES ==========
    
    /// Publish a TransformExecuted event with consistent error handling
    pub fn publish_transform_executed(
        message_bus: &Arc<MessageBus>,
        transform_id: &str,
        status: &str,
    ) -> Result<(), SchemaError> {
        info!("📢 Publishing TransformExecuted {} event for: {}", status, transform_id);
        
        let event = TransformExecuted::new(transform_id, status);
        
        match message_bus.publish(event) {
            Ok(_) => {
                info!("✅ Published TransformExecuted {} event for transform: {}", status, transform_id);
                Ok(())
            }
            Err(e) => {
                let error_msg = format!("Failed to publish TransformExecuted {} event for {}: {}", status, transform_id, e);
                error!("❌ {}", error_msg);
                Err(SchemaError::InvalidData(error_msg))
            }
        }
    }

    /// Handle execution result and publish event
    pub fn handle_execution_result_and_publish(
        message_bus: &Arc<MessageBus>,
        transform_id: &str,
        execution_result: &Result<serde_json::Value, crate::schema::types::SchemaError>,
    ) {
        match execution_result {
            Ok(value) => {
                info!("✅ Transform {} execution completed successfully", transform_id);
                info!("📊 Execution result details: {:?}", value);
                
                if let Err(e) = Self::publish_transform_executed(message_bus, transform_id, "success") {
                    error!("❌ Event publishing failed after successful execution: {}", e);
                }
            }
            Err(e) => {
                error!("❌ Transform {} execution failed", transform_id);
                error!("❌ Failure details: {:?}", e);
                
                if let Err(publish_err) = Self::publish_transform_executed(message_bus, transform_id, "failed") {
                    error!("❌ Event publishing failed after execution failure: {}", publish_err);
                }
            }
        }
    }

    // ========== FIELD RESOLUTION UTILITIES ==========
    
    /// Extract simplified value from range field atom content
    /// Converts {"range_key":"2","value":"2"} to "2"
    /// Converts {"range_key":"2","value":{"value":"b"}} to "b"
    fn extract_simplified_value(content: &JsonValue) -> Result<JsonValue, SchemaError> {
        info!("🎯 Extracting simplified value from: {}", content);
        
        // Try to extract the "value" field
        if let Some(value_field) = content.get("value") {
            // If the value field is itself an object with a nested "value", extract that
            if let Some(nested_value) = value_field.get("value") {
                info!("✅ Extracted nested value: {}", nested_value);
                return Ok(nested_value.clone());
            } else {
                info!("✅ Extracted direct value: {}", value_field);
                return Ok(value_field.clone());
            }
        }
        
        // If no "value" field found, return the content as-is
        warn!("⚠️ No 'value' field found, returning content as-is");
        Ok(content.clone())
    }
    
    /// Unified field value resolution from schema using database operations
    pub fn resolve_field_value(
        db_ops: &Arc<crate::db_operations::DbOperations>,
        schema: &Schema,
        field_name: &str,
        range_key: Option<String>,
    ) -> Result<JsonValue, SchemaError> {
        info!("🔍 FieldValueResolver: Looking up field '{}' in schema '{}'", field_name, schema.name);
        
        let field = schema.fields.get(field_name)
            .ok_or_else(|| {
                error!("❌ Field '{}' not found in schema '{}'", field_name, schema.name);
                SchemaError::InvalidField(format!("Field '{}' not found in schema '{}'", field_name, schema.name))
            })?;
        
        info!("✅ Field '{}' found in schema '{}'", field_name, schema.name);
        
        // Check if this is a range field first
        match field {
            FieldVariant::Range(_) => {
                info!("🔄 Detected range field, using MoleculeRange resolution");
                let range_molecule_uuid = format!("{}_{}_range", schema.name, field_name);
                info!("🔍 Looking for MoleculeRange: {}", range_molecule_uuid);
                
                match db_ops.get_item::<crate::atom::MoleculeRange>(&format!("ref:{}", range_molecule_uuid)) {
                    Ok(Some(range_molecule)) => {
                        info!("✅ Found MoleculeRange with {} entries", range_molecule.atom_uuids.len());
                        
                        // BUG FIX 1: Filter by specific range key if provided
                        let entries_to_process: Vec<_> = if let Some(ref target_key) = range_key {
                            info!("🎯 Filtering for specific range key: '{}'", target_key);
                            range_molecule.atom_uuids.iter()
                                .filter(|(key, _)| *key == target_key)
                                .collect()
                        } else {
                            info!("📋 Processing all range keys");
                            range_molecule.atom_uuids.iter().collect()
                        };
                        
                        let mut combined_data = serde_json::Map::new();
                        
                        for (key, atom_uuid) in entries_to_process {
                            info!("🔗 Processing range key '{}' -> atom: {}", key, atom_uuid);
                            
                            match Self::load_atom(db_ops, atom_uuid) {
                                Ok(atom) => {
                                    let content = atom.content();
                                    info!("📦 Range entry '{}' content: {}", key, content);
                                    
                                    // BUG FIX 2: Extract simplified value instead of full structure
                                    let simplified_value = Self::extract_simplified_value(content)?;
                                    info!("🎯 Simplified value for key '{}': {}", key, simplified_value);
                                    
                                    combined_data.insert(key.clone(), simplified_value);
                                }
                                Err(e) => {
                                    error!("❌ Failed to load atom {} for range key '{}': {}", atom_uuid, key, e);
                                }
                            }
                        }
                        
                        let result = JsonValue::Object(combined_data);
                        info!("✅ Range field resolution complete - combined result: {}", result);
                        return Ok(result);
                    }
                    Ok(None) => {
                        error!("❌ MoleculeRange '{}' not found", range_molecule_uuid);
                        return Err(SchemaError::InvalidField(format!("MoleculeRange '{}' not found", range_molecule_uuid)));
                    }
                    Err(e) => {
                        error!("❌ Error loading MoleculeRange '{}': {}", range_molecule_uuid, e);
                        return Err(SchemaError::InvalidField(format!("Error loading MoleculeRange '{}': {}", range_molecule_uuid, e)));
                    }
                }
            }
            FieldVariant::Single(_) => {
                info!("🔄 Detected single field, using Molecule resolution");
            }
        }
        
        // CRITICAL BUG DIAGNOSIS: This reads STATIC schema reference, not dynamic Molecule!
        let molecule_uuid = Self::extract_molecule_uuid(field, field_name)?;
        error!("🚨 CRITICAL BUG: Reading STATIC schema molecule_uuid: {}", molecule_uuid);
        error!("🚨 This should be reading from DYNAMIC Molecule system instead!");

        // DIAGNOSTIC: Check what the dynamic Molecule system has
        let dynamic_molecule_uuid = format!("{}_{}_single", schema.name, field_name);
        error!("🔍 DIAGNOSTIC: Checking dynamic Molecule UUID: {}", dynamic_molecule_uuid);
        
        match db_ops.get_item::<crate::atom::Molecule>(&format!("ref:{}", dynamic_molecule_uuid)) {
            Ok(Some(dynamic_molecule)) => {
                let dynamic_atom_uuid = dynamic_molecule.get_atom_uuid();
                error!("🔍 DIAGNOSTIC: Dynamic Molecule points to atom: {}", dynamic_atom_uuid);
                error!("🚨 MISMATCH DETECTED: Static schema: {} vs Dynamic Molecule: {}", molecule_uuid, dynamic_atom_uuid);

                // Use the CORRECT dynamic Molecule instead of broken static schema reference
                info!("🔧 APPLYING FIX: Using dynamic Molecule instead of static schema reference");
                let atom = Self::load_atom(db_ops, dynamic_atom_uuid)?;
                let content = atom.content().clone();
                info!("✅ Fixed query using dynamic Molecule - content: {}", content);
                return Ok(content);
            }
            Ok(None) => {
                error!("🔍 DIAGNOSTIC: Dynamic Molecule not found, falling back to static schema reference");
            }
            Err(e) => {
                error!("🔍 DIAGNOSTIC: Error checking dynamic Molecule: {}", e);
            }
        }
        
        info!("� Field molecule_uuid: {}", molecule_uuid);
        
        let molecule = Self::load_molecule(db_ops, &molecule_uuid)?;
        let atom_uuid = molecule.get_atom_uuid();
        info!("🔗 Molecule points to atom: {}", atom_uuid);
        
        let atom = Self::load_atom(db_ops, atom_uuid)?;
        
        info!("✅ Atom loaded successfully");
        let content = atom.content().clone();
        info!("📦 Atom content: {}", content);
        
        Ok(content)
    }
    
    /// Extract molecule_uuid from field variant with consistent error handling
    fn extract_molecule_uuid(field: &FieldVariant, field_name: &str) -> Result<String, SchemaError> {
        let molecule_uuid = field.molecule_uuid()
            .ok_or_else(|| {
                error!("❌ Field '{}' has no molecule_uuid", field_name);
                SchemaError::InvalidField(format!("Field '{}' has no molecule_uuid", field_name))
            })?
            .clone();
        Ok(molecule_uuid)
    }
    
    /// Load Molecule from database with consistent error handling
    fn load_molecule(
        db_ops: &Arc<crate::db_operations::DbOperations>,
        molecule_uuid: &str,
    ) -> Result<crate::atom::Molecule, SchemaError> {
        info!("🔍 Loading Molecule from database...");
        
        match db_ops.get_item::<crate::atom::Molecule>(&format!("ref:{}", molecule_uuid)) {
            Ok(Some(molecule)) => Ok(molecule),
            Ok(None) => {
                error!("❌ Molecule '{}' not found", molecule_uuid);
                Err(SchemaError::InvalidField(format!("Molecule '{}' not found", molecule_uuid)))
            }
            Err(e) => {
                error!("❌ Failed to load Molecule {}: {}", molecule_uuid, e);
                Err(SchemaError::InvalidField(format!("Failed to load Molecule: {}", e)))
            }
        }
    }
    
    /// Load Atom from database with consistent error handling
    fn load_atom(
        db_ops: &Arc<crate::db_operations::DbOperations>,
        atom_uuid: &str,
    ) -> Result<crate::atom::Atom, SchemaError> {
        info!("🔍 Loading Atom from database...");
        db_ops.get_item(&format!("atom:{}", atom_uuid))?
            .ok_or_else(|| {
                error!("❌ Atom '{}' not found", atom_uuid);
                SchemaError::InvalidField(format!("Atom '{}' not found", atom_uuid))
            })
    }

    // ========== LOGGING UTILITIES ==========
    
    /// Standard logging for transform registration
    pub fn log_transform_registration(transform_id: &str, inputs: &[String], output: &str) {
        info!("🔧 Registering transform '{}' with inputs: {:?}, output: {}", transform_id, inputs, output);
    }

    /// Standard logging for field mapping creation
    pub fn log_field_mapping_creation(field_key: &str, transform_id: &str) {
        info!("🔗 Created field mapping: '{}' -> transform '{}'", field_key, transform_id);
    }

    /// Standard logging for verification results
    pub fn log_verification_result(item_type: &str, id: &str, details: Option<&str>) {
        match details {
            Some(detail) => info!("✅ Verified {}: {} - {}", item_type, id, detail),
            None => info!("✅ Verified {}: {}", item_type, id),
        }
    }

    /// Standard logging for atom ref operations
    pub fn log_molecule_operation(molecule_uuid: &str, atom_uuid: &str, operation: &str) {
        info!("🔗 Molecule {} - ref:{} -> atom:{}", operation, molecule_uuid, atom_uuid);
    }

    /// Standard logging for field mappings state
    pub fn log_field_mappings_state(mappings: &HashMap<String, HashSet<String>>, context: &str) {
        info!("🔍 DEBUG {}: Current field mappings ({} entries):", context, mappings.len());
        for (field_key, transforms) in mappings {
            info!("  📋 '{}' -> {:?}", field_key, transforms);
        }
        if mappings.is_empty() {
            info!("⚠️ No field mappings found in {}", context);
        }
    }

    /// Log collection state with consistent formatting
    pub fn log_collection_state<T: std::fmt::Debug>(
        collection_name: &str,
        collection: &T,
        operation: &str,
    ) {
        info!("🔍 DEBUG {}: {} collection state: {:?}", operation, collection_name, collection);
    }
}

// Type aliases for backward compatibility and reduced import burden
pub type LoggingHelper = TransformUtils;
pub type FieldValueResolver = TransformUtils;
pub type EventPublisher = TransformUtils;
pub type ConversionHelper = TransformUtils;
pub type SerializationHelper = TransformUtils;
pub type LockHelper = TransformUtils;
pub type DefaultValueHelper = TransformUtils;
pub type ValidationHelper = TransformUtils;
