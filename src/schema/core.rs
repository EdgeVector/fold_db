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
    /// Thread-safe storage for loaded schemas
    pub(crate) schemas: Mutex<HashMap<String, Schema>>,
    /// All schemas known to the system and their load state
    pub(crate) available: Mutex<HashMap<String, (Schema, SchemaState)>>,
    /// Unified database operations (required)
    pub(crate) db_ops: std::sync::Arc<crate::db_operations::DbOperations>,
    /// Schema directory path
    pub(crate) schemas_dir: PathBuf,
    /// Message bus for event-driven communication
    pub(crate) message_bus: Arc<MessageBus>,
}

impl SchemaCore {
    /// Creates a new SchemaCore with DbOperations (unified approach)
    pub fn new(
        path: &str,
        db_ops: std::sync::Arc<crate::db_operations::DbOperations>,
        message_bus: Arc<MessageBus>,
    ) -> Result<Self, SchemaError> {
        let schemas_dir = PathBuf::from(path).join("schemas");

        // Create directory if it doesn't exist
        if let Err(e) = std::fs::create_dir_all(&schemas_dir) {
            if e.kind() != std::io::ErrorKind::AlreadyExists {
                return Err(SchemaError::InvalidData(format!(
                    "Failed to create schemas directory: {}",
                    e
                )));
            }
        }

        Ok(Self {
            schemas: Mutex::new(HashMap::new()),
            available: Mutex::new(HashMap::new()),
            db_ops,
            schemas_dir,
            message_bus,
        })
    }

    /// Gets the path for a schema file.
    pub fn schema_path(&self, schema_name: &str) -> PathBuf {
        self.schemas_dir.join(format!("{}.json", schema_name))
    }

    /// Creates a new SchemaCore for testing purposes with a temporary database
    pub fn new_for_testing(path: &str) -> Result<Self, SchemaError> {
        let db = sled::open(path)?;
        let db_ops = std::sync::Arc::new(crate::db_operations::DbOperations::new(db)?);
        let message_bus = Arc::new(MessageBus::new());
        Self::new(path, db_ops, message_bus)
    }

    /// Creates a default SchemaCore for testing purposes
    pub fn init_default() -> Result<Self, SchemaError> {
        Self::new_for_testing("data")
    }
}
