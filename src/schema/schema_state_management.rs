use super::{schema_lock_error, validator::SchemaValidator};
use crate::fold_db_core::infrastructure::message_bus::MessageBus;
use crate::logging::features::{log_feature, LogFeature};
use crate::schema::types::{Field, FieldVariant, Schema, SchemaError};
use crate::schema::{
    interpret_schema, load_schema_from_file, load_schema_from_json, map_fields, MoleculeVariant,
    SchemaState,
};
use log::{error, info};
use serde::{Deserialize, Serialize};
use serde_json;
use std::collections::HashMap;
use std::path::PathBuf;
use std::sync::{Arc, Mutex};
use uuid::Uuid;

use super::core::SchemaCore;

impl SchemaCore {
    /// Get schemas by state
    pub fn list_schemas_by_state(&self, state: SchemaState) -> Result<Vec<String>, SchemaError> {
        let available = self.available.lock().map_err(|_| schema_lock_error())?;

        let schemas: Vec<String> = available
            .iter()
            .filter(|(_, (_, s))| *s == state)
            .map(|(name, _)| name.clone())
            .collect();

        Ok(schemas)
    }

    /// Check if a schema can be queried (must be Approved)
    pub fn can_query_schema(&self, schema_name: &str) -> bool {
        matches!(
            self.get_schema_state(schema_name),
            Some(SchemaState::Approved)
        )
    }

    /// Check if a schema can be mutated (must be Approved)
    pub fn can_mutate_schema(&self, schema_name: &str) -> bool {
        matches!(
            self.get_schema_state(schema_name),
            Some(SchemaState::Approved)
        )
    }

    /// Get all available schemas (any state)
    pub fn list_all_schemas(&self) -> Result<Vec<String>, SchemaError> {
        self.list_available_schemas()
    }

    /// Persist a schema to disk in Available state.
    pub fn add_schema_available(&self, mut schema: Schema) -> Result<(), SchemaError> {
        info!(
            "Adding schema '{}' as Available with {} fields: {:?}",
            schema.name,
            schema.fields.len(),
            schema.fields.keys().collect::<Vec<_>>()
        );
        
        // Persist the updated schema
        self.persist_schema(&schema)?;

        let name = schema.name.clone();
        let state_to_use = {
            let mut available = self.available.lock().map_err(|_| schema_lock_error())?;

            // Check if schema already exists and preserve its state
            let existing_state = available.get(&name).map(|(_, state)| *state);
            let state_to_use = existing_state.unwrap_or(SchemaState::Available);

            available.insert(name.clone(), (schema, state_to_use));

            // If the existing state was Approved, also add to the active schemas
            if state_to_use == SchemaState::Approved {
                let mut schemas = self.schemas.lock().map_err(|_| schema_lock_error())?;
                schemas.insert(name.clone(), available.get(&name).unwrap().0.clone());
            }

            state_to_use
        };

        // Persist state changes
        self.persist_states()?;
        info!(
            "Schema '{}' added with preserved state: {:?}",
            name, state_to_use
        );

        Ok(())
    }

    /// Lists all schema names currently loaded.
    pub fn list_loaded_schemas(&self) -> Result<Vec<String>, SchemaError> {
        let schemas = self.schemas.lock().map_err(|_| schema_lock_error())?;
        Ok(schemas.keys().cloned().collect())
    }

    /// Lists all schemas available on disk and their state.
    pub fn list_available_schemas(&self) -> Result<Vec<String>, SchemaError> {
        let available = self.available.lock().map_err(|_| schema_lock_error())?;
        Ok(available.keys().cloned().collect())
    }

    /// Retrieve the persisted state for a schema if known.
    pub fn get_schema_state(&self, schema_name: &str) -> Option<SchemaState> {
        let available = self.available.lock().ok()?;
        available.get(schema_name).map(|(_, s)| *s)
    }

    /// Sets the state for a schema and persists all schema states.
    pub fn set_schema_state(
        &self,
        schema_name: &str,
        state: SchemaState,
    ) -> Result<(), SchemaError> {
        let mut available = self.available.lock().map_err(|_| schema_lock_error())?;
        if let Some((_, st)) = available.get_mut(schema_name) {
            *st = state;
        } else {
            return Err(SchemaError::NotFound(format!(
                "Schema {} not found",
                schema_name
            )));
        }
        drop(available);
        self.persist_states()
    }

    /// Backwards compatible method for listing loaded schemas.
    pub fn list_schemas(&self) -> Result<Vec<String>, SchemaError> {
        self.list_loaded_schemas()
    }

    /// Checks if a schema exists in the manager.
    pub fn schema_exists(&self, schema_name: &str) -> Result<bool, SchemaError> {
        let schemas = self.schemas.lock().map_err(|_| schema_lock_error())?;
        Ok(schemas.contains_key(schema_name))
    }

    /// Mark a schema as Available (remove from active schemas but keep discoverable)
    pub fn set_available(&self, schema_name: &str) -> Result<(), SchemaError> {
        info!("Setting schema '{}' to Available", schema_name);
        let mut schemas = self.schemas.lock().map_err(|_| schema_lock_error())?;
        schemas.remove(schema_name);
        drop(schemas);
        self.set_schema_state(schema_name, SchemaState::Available)?;
        info!("Schema '{}' marked as Available", schema_name);
        Ok(())
    }

    /// Unload schema from active memory and set to Available state (preserving field assignments)
    pub fn unload_schema(&self, schema_name: &str) -> Result<(), SchemaError> {
        info!(
            "Unloading schema '{}' from active memory and setting to Available",
            schema_name
        );
        let mut schemas = self.schemas.lock().map_err(|_| schema_lock_error())?;
        schemas.remove(schema_name);
        drop(schemas);
        self.set_schema_state(schema_name, SchemaState::Available)?;
        info!("Schema '{}' unloaded and marked as Available", schema_name);
        Ok(())
    }
}
