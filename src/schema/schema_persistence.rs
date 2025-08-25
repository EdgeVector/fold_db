use super::{schema_lock_error, validator::SchemaValidator};
use crate::fold_db_core::infrastructure::message_bus::MessageBus;
use crate::schema::types::{
    Field, FieldVariant, Schema, SchemaError,
};
use crate::schema::{MoleculeVariant, SchemaState, map_fields, interpret_schema, load_schema_from_json, load_schema_from_file};
use log::{info, error};
use crate::logging::features::{log_feature, LogFeature};
use serde::{Deserialize, Serialize};
use serde_json;
use std::collections::HashMap;
use std::path::PathBuf;
use std::sync::{Arc, Mutex};
use uuid::Uuid;

use super::core::SchemaCore;

impl SchemaCore {
    /// Gets the file path for a schema
    pub fn get_schema_path(&self, schema_name: &str) -> PathBuf {
        self.schema_path(schema_name)
    }

    /// Updates the molecule_uuid for a specific field in a schema and persists it to disk.
    ///
    /// **CRITICAL: This is the ONLY method that should set molecule_uuid on field definitions**
    ///
    /// This method is the central point for managing molecule_uuid values to prevent
    /// "ghost molecule_uuid" issues where UUIDs exist but don't point to actual Molecules.
    ///
    /// **Proper Usage Pattern:**
    /// 1. Field manager methods (set_field_value, update_field) create Molecule and return UUID
    /// 2. Mutation logic calls this method with the returned UUID
    /// 3. This method sets the UUID on the actual schema (not a clone)
    /// 4. This method persists the schema to disk immediately
    /// 5. This ensures molecule_uuid is only set when Molecule actually exists
    ///
    /// **Why this prevents "ghost molecule_uuid" issues:**
    /// - Centralizes all molecule_uuid setting in one place
    /// - Always persists changes immediately to disk
    /// - Only called after Molecule is confirmed to exist
    /// - Updates both in-memory and on-disk schema representations
    ///
    /// **DO NOT** set molecule_uuid directly on field definitions elsewhere in the code.
    pub fn update_field_molecule_uuid(
        &self,
        schema_name: &str,
        field_name: &str,
        molecule_uuid: String,
    ) -> Result<(), SchemaError> {
        info!(
            "🔧 UPDATE_FIELD_MOLECULE_UUID START - schema: {}, field: {}, uuid: {}",
            schema_name, field_name, molecule_uuid
        );

        let mut schemas = self
            .schemas
            .lock()
            .map_err(|_| schema_lock_error())?;

        if let Some(schema) = schemas.get_mut(schema_name) {
            if let Some(field) = schema.fields.get_mut(field_name) {
                field.set_molecule_uuid(molecule_uuid.clone());
                info!(
                    "Field {}.{} molecule_uuid updated in memory",
                    schema_name, field_name
                );

                // Persist the updated schema to disk
                info!("Persisting updated schema {} to disk", schema_name);
                self.persist_schema(schema)?;
                info!(
                    "Schema {} persisted successfully with updated molecule_uuid",
                    schema_name
                );

                // Also update the available schemas map to keep it in sync
                let mut available = self.available.lock().map_err(|_| {
                    SchemaError::InvalidData("Failed to acquire available schemas lock".to_string())
                })?;

                if let Some((available_schema, _state)) = available.get_mut(schema_name) {
                    if let Some(available_field) = available_schema.fields.get_mut(field_name) {
                        available_field.set_molecule_uuid(molecule_uuid);
                        info!(
                            "Available schema {}.{} molecule_uuid updated",
                            schema_name, field_name
                        );
                    }
                }

                Ok(())
            } else {
                Err(SchemaError::InvalidField(format!(
                    "Field {} not found in schema {}",
                    field_name, schema_name
                )))
            }
        } else {
            Err(SchemaError::NotFound(format!(
                "Schema {} not found",
                schema_name
            )))
        }
    }

    /// Retrieves a schema by name from any state (Available, Approved, Blocked).
    ///
    /// SCHEMA-004: This method returns schemas regardless of state to support UI field display.
    /// Available schemas show fields but with molecule_uuid: None as expected.
    pub fn get_schema(&self, schema_name: &str) -> Result<Option<Schema>, SchemaError> {
        // First check the available collection which contains all schemas regardless of state
        let available = self
            .available
            .lock()
            .map_err(|_| SchemaError::InvalidData("Failed to acquire available schema lock".to_string()))?;
        
        if let Some((schema, _state)) = available.get(schema_name) {
            return Ok(Some(schema.clone()));
        }
        
        // Fallback to approved schemas collection for backward compatibility
        let schemas = self
            .schemas
            .lock()
            .map_err(|_| schema_lock_error())?;
        Ok(schemas.get(schema_name).cloned())
    }
}
