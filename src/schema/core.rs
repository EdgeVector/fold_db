use super::{schema_lock_error};
use crate::fold_db_core::infrastructure::message_bus::MessageBus;
use crate::logging::features::{log_feature, LogFeature};
use crate::schema::types::{Schema, SchemaError, FieldVariant, Field};
use crate::schema::{
    SchemaState,
};
use log::{info};
use serde::{Serialize};
use serde_json;
use std::collections::HashMap;
use std::path::PathBuf;
use std::sync::{Arc, Mutex};

/// Core schema management system that combines schema interpretation, validation, and management.
///
/// SchemaCore is responsible for:
/// - Loading and validating schemas from JSON
/// - Managing schema storage and persistence
/// - Handling schema field mappings
/// - Providing schema access and validation services
///
/// This unified component simplifies the schema system by combining the functionality
/// previously split across SchemaManager and SchemaInterpreter.
pub struct SchemaCore {
    /// Storage for loaded schemas
    schemas: Arc<Mutex<HashMap<String, Schema>>>,
    /// Storage for all schemas known to the system and their load state
    schema_states: Arc<Mutex<HashMap<String, SchemaState>>>,
    /// Unified database operations (required)
    db_ops: std::sync::Arc<crate::db_operations::DbOperations>,
    /// Message bus for event-driven communication
    message_bus: Arc<MessageBus>,
}

impl SchemaCore {
    /// Creates a new SchemaCore with DbOperations (unified approach)
    pub fn new(
        db_ops: std::sync::Arc<crate::db_operations::DbOperations>,
        message_bus: Arc<MessageBus>,
    ) -> Result<Self, SchemaError> {
        // load schemas from db
        // TODO: implement get_all_schemas and get_all_schema_states
        let schemas = db_ops.get_all_schemas()?;
        let schema_states = db_ops.get_all_schema_states()?;

        Ok(Self {
            schemas: Arc::new(Mutex::new(schemas)),
            schema_states: Arc::new(Mutex::new(schema_states)),
            db_ops,
            message_bus,
        })
    }

    pub fn get_schemas(&self) -> Result<HashMap<String, Schema>, SchemaError> {
        Ok(self.schemas.lock().map_err(|_| SchemaError::InvalidData("Failed to acquire schemas lock".to_string()))?.clone())
    }

    pub fn get_schema_states(&self) -> Result<HashMap<String, SchemaState>, SchemaError> {
        Ok(self.schema_states.lock().map_err(|_| SchemaError::InvalidData("Failed to acquire schema_states lock".to_string()))?.clone())
    }

    pub fn set_schema_state(&self, schema_name: &str, schema_state: SchemaState) -> Result<(), SchemaError> {
        self.schema_states.lock().map_err(|_| SchemaError::InvalidData("Failed to acquire schema_states lock".to_string()))?.insert(schema_name.to_string(), schema_state);
        self.db_ops.store_schema_state(schema_name, schema_state);
        Ok(())
    }

    pub fn block_schema(&self, schema_name: &str) -> Result<(), SchemaError> {
        self.set_schema_state(schema_name, SchemaState::Blocked)
    }

    pub fn get_message_bus(&self) -> Arc<MessageBus> {
        Arc::clone(&self.message_bus)
    }

    pub fn get_schema(&self, schema_name: &str) -> Result<Option<Schema>, SchemaError> {
        Ok(self.schemas.lock().map_err(|_| SchemaError::InvalidData("Failed to acquire schemas lock".to_string()))?.get(schema_name).cloned())
    }

    pub fn add_schema_available(&self, schema: Schema) -> Result<(), SchemaError> {
        let mut schemas = self.schemas.lock().map_err(|_| SchemaError::InvalidData("Failed to acquire schemas lock".to_string()))?;
        let mut schema_states = self.schema_states.lock().map_err(|_| SchemaError::InvalidData("Failed to acquire schema_states lock".to_string()))?;
        schemas.insert(schema.name.clone(), schema.clone());
        schema_states.insert(schema.name.clone(), SchemaState::Available);
        Ok(())
    }

    pub fn load_schema_internal(&self, schema: Schema) -> Result<(), SchemaError> {
        let name = schema.name.clone();

        if self.schemas.lock().map_err(|_| SchemaError::InvalidData("Failed to acquire schemas lock".to_string()))?.contains_key(&name) {
            return Ok(());
        } else {
            self.add_schema_available(schema)?;
        };
        Ok(())
    }

    /// Creates a new SchemaCore for testing purposes with a temporary database
    pub fn new_for_testing() -> Result<Self, SchemaError> {
        let db = sled::Config::new().temporary(true).open()?;
        let db_ops = std::sync::Arc::new(crate::db_operations::DbOperations::new(db)?);
        let message_bus = Arc::new(MessageBus::new());
        Self::new(db_ops, message_bus)
    }
}
