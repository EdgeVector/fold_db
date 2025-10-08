use crate::fold_db_core::infrastructure::message_bus::MessageBus;
use crate::schema::types::{Schema, SchemaError, Field};
use crate::schema::{
    SchemaState,
    SchemaWithState,
};
use serde_json;
use std::collections::HashMap;
use std::fs;
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
        Ok(self
            .schema_states
            .lock()
            .map_err(|_| SchemaError::InvalidData("Failed to acquire schema_states lock".to_string()))?
            .clone())
    }

    pub fn get_schemas_with_states(&self) -> Result<Vec<SchemaWithState>, SchemaError> {
        let schemas = self.get_schemas()?;
        let schema_states = self.get_schema_states()?;

        let mut with_states = Vec::with_capacity(schemas.len());
        for (name, schema) in schemas {
            let state = schema_states.get(&name).copied().unwrap_or_default();
            with_states.push(SchemaWithState::new(schema, state));
        }

        Ok(with_states)
    }

    pub fn set_schema_state(&self, schema_name: &str, schema_state: SchemaState) -> Result<(), SchemaError> {
        self.set_schema_state_with_backfill(schema_name, schema_state, None)
    }

    pub fn set_schema_state_with_backfill(&self, schema_name: &str, schema_state: SchemaState, backfill_hash: Option<String>) -> Result<(), SchemaError> {
        // Persist to database first - this is the source of truth
        self.db_ops.store_schema_state(schema_name, schema_state)?;
        
        // Update in-memory cache only after successful persistence
        self.schema_states.lock().map_err(|_| SchemaError::InvalidData("Failed to acquire schema_states lock".to_string()))?.insert(schema_name.to_string(), schema_state);
        
        // If schema is being approved, publish SchemaApproved event to trigger backfill
        if schema_state == SchemaState::Approved {
            use crate::fold_db_core::infrastructure::message_bus::events::schema_events::SchemaApproved;
            let event = SchemaApproved {
                schema_name: schema_name.to_string(),
                backfill_hash,
            };
            self.message_bus.publish(event)
                .map_err(|e| SchemaError::InvalidData(format!("Failed to publish SchemaApproved event: {}", e)))?;
        }
        
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
        
        // Always update the schema in-memory to preserve molecule_uuids and other runtime state
        // This is needed after mutations to ensure the updated schema state is loaded
        
        // Check if schema exists in database
        let existing_schema = self.db_ops.get_schema(&name)?;
        
        if let Some(_existing) = existing_schema {
            // Schema exists in DB - preserve molecule_uuids from existing schema
            let mut schemas = self.schemas.lock().map_err(|_| SchemaError::InvalidData("Failed to acquire schemas lock".to_string()))?;
            
            // Get the existing schema from memory (which has molecule_uuids)
            let existing_in_memory = schemas.get(&name).cloned();
            
            // Use the new schema structure but preserve molecule_uuids from existing schema
            let mut updated_schema = schema;
            if let Some(existing) = existing_in_memory {
                // Preserve molecule_uuids from existing schema
                for (field_name, existing_field) in existing.fields {
                    if let Some(new_field) = updated_schema.fields.get_mut(&field_name) {
                        if let Some(molecule_uuid) = existing_field.common().molecule_uuid() {
                            new_field.common_mut().set_molecule_uuid(molecule_uuid.clone());
                        }
                    }
                }
            }
            
            schemas.insert(name.clone(), updated_schema);
            
            // Preserve existing state
            let existing_state = self.db_ops.get_schema_state(&name)?;
            let mut schema_states = self.schema_states.lock().map_err(|_| SchemaError::InvalidData("Failed to acquire schema_states lock".to_string()))?;
            let state = existing_state.unwrap_or(SchemaState::Available);
            schema_states.insert(name.clone(), state);
        } else {
            // New schema - persist to database
            self.db_ops.store_schema(&name, &schema)?;
            self.db_ops.store_schema_state(&name, SchemaState::Available)?;
            
            // Update in-memory caches
            let mut schemas = self.schemas.lock().map_err(|_| SchemaError::InvalidData("Failed to acquire schemas lock".to_string()))?;
            schemas.insert(name.clone(), schema);
            
            let mut schema_states = self.schema_states.lock().map_err(|_| SchemaError::InvalidData("Failed to acquire schema_states lock".to_string()))?;
            schema_states.insert(name.clone(), SchemaState::Available);
        }
        
        Ok(())
    }

    /// Load schema from JSON string (creates Available schema)
    /// Only supports declarative schema format
    pub fn load_schema_from_json(&self, json_str: &str) -> Result<(), SchemaError> {
        use crate::schema::types::DeclarativeSchemaDefinition;
        
        // Parse JSON string to DeclarativeSchemaDefinition
        let declarative_schema: DeclarativeSchemaDefinition = serde_json::from_str(json_str)
            .map_err(|e| SchemaError::InvalidData(format!("Failed to parse declarative schema: {}", e)))?;
        
        // Convert declarative schema to Schema
        let schema = self.interpret_declarative_schema(declarative_schema)?;
        
        // Load the schema using the existing method
        self.load_schema_internal(schema)
    }

    /// Load schema from file (creates Available schema)
    /// Only supports declarative schema format
    pub fn load_schema_from_file<P: AsRef<std::path::Path>>(&self, path: P) -> Result<(), SchemaError> {
        
        // Use the existing parse_schema_file method which handles declarative schemas
        if let Some(schema) = self.parse_schema_file(path.as_ref())? {
            self.load_schema_internal(schema)
        } else {
            Err(SchemaError::InvalidData("No schema found in file".to_string()))
        }
    }

    /// Load all schema files from a directory (creates Available schemas)
    /// Only processes .json files; ignores non-existent directories
    pub fn load_schemas_from_directory<P: AsRef<std::path::Path>>(
        &self,
        directory: P,
    ) -> Result<usize, SchemaError> {
        let dir_path = directory.as_ref();
        if !dir_path.exists() {
            return Ok(0);
        }

        let mut loaded_count: usize = 0;
        let entries = fs::read_dir(dir_path).map_err(|e| {
            SchemaError::InvalidData(format!("Failed to read directory {}: {}", dir_path.display(), e))
        })?;

        for entry in entries {
            let entry = entry.map_err(|e| {
                SchemaError::InvalidData(format!("Failed to read entry in {}: {}", dir_path.display(), e))
            })?;
            let path = entry.path();
            if path.extension().map(|ext| ext == "json").unwrap_or(false) {
                match self.load_schema_from_file(&path) {
                    Ok(()) => {
                        loaded_count += 1;
                    }
                    Err(e) => {
                        log::warn!("Failed to load schema from file {}: {}", path.display(), e);
                    }
                }
            }
        }

        Ok(loaded_count)
    }

    /// Creates a new SchemaCore for testing purposes with a temporary database
    pub fn new_for_testing() -> Result<Self, SchemaError> {
        let db = sled::Config::new().temporary(true).open()?;
        let db_ops = std::sync::Arc::new(crate::db_operations::DbOperations::new(db)?);
        let message_bus = Arc::new(MessageBus::new());
        Self::new(db_ops, message_bus)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn blogpost_schema_json() -> String {
        // Declarative schema format used in available_schemas
        // Minimal fields map is acceptable per current parser
        r#"{
            "name": "BlogPost",
            "key": { "range_field": "publish_date" },
            "fields": {
                "title": {},
                "content": {},
                "author": {},
                "publish_date": {}
            }
        }"#.to_string()
    }

    fn wordindex_schema_json() -> String {
        r#"{
            "name": "BlogPostWordIndex",
            "key": { "hash_field": "word", "range_field": "publish_date" },
            "fields": {
                "word": {},
                "publish_date": {}
            }
        }"#.to_string()
    }

    #[test]
    fn new_for_testing_starts_with_empty_schemas() {
        let core = SchemaCore::new_for_testing().expect("init core");
        let schemas = core.get_schemas().expect("get_schemas");
        assert!(schemas.is_empty(), "expected no schemas at start");
    }

    #[test]
    fn load_schema_from_json_adds_available_schema() {
        let core = SchemaCore::new_for_testing().expect("init core");
        core.load_schema_from_json(&blogpost_schema_json()).expect("load blogpost");

        let schemas = core.get_schemas().expect("get_schemas");
        assert!(schemas.contains_key("BlogPost"), "BlogPost should be loaded");

        let states = core.get_schema_states().expect("get states");
        assert_eq!(states.get("BlogPost"), Some(&SchemaState::Available));
    }

    #[test]
    fn get_schemas_with_states_returns_default_available() {
        let core = SchemaCore::new_for_testing().expect("init core");
        core.load_schema_from_json(&blogpost_schema_json()).expect("load blogpost");

        let schemas_with_states = core.get_schemas_with_states().expect("get with states");
        assert_eq!(schemas_with_states.len(), 1);
        let schema_entry = schemas_with_states
            .iter()
            .find(|entry| entry.name() == "BlogPost")
            .expect("BlogPost entry");
        assert_eq!(schema_entry.state, SchemaState::Available);
    }

    #[test]
    fn load_multiple_schemas_from_json() {
        let core = SchemaCore::new_for_testing().expect("init core");
        core.load_schema_from_json(&blogpost_schema_json()).expect("load blogpost");
        core.load_schema_from_json(&wordindex_schema_json()).expect("load wordindex");

        let schemas = core.get_schemas().expect("get_schemas");
        assert!(schemas.contains_key("BlogPost"));
        assert!(schemas.contains_key("BlogPostWordIndex"));

        let states = core.get_schema_states().expect("get states");
        assert_eq!(states.get("BlogPost"), Some(&SchemaState::Available));
        assert_eq!(states.get("BlogPostWordIndex"), Some(&SchemaState::Available));
    }

    #[test]
    fn load_schema_from_file_works_with_declarative_format() {
        let core = SchemaCore::new_for_testing().expect("init core");
        let dir = tempfile::tempdir().expect("tempdir");
        let path = dir.path().join("BlogPost.json");
        std::fs::write(&path, blogpost_schema_json()).expect("write schema json");

        core.load_schema_from_file(&path).expect("load from file");
        let schemas = core.get_schemas().expect("get_schemas");
        assert!(schemas.contains_key("BlogPost"));
    }

    #[test]
    fn blogpost_wordindex_sets_hashrange_keyconfig() {
        use crate::schema::types::schema::SchemaType;

        let core = SchemaCore::new_for_testing().expect("init core");
        core.load_schema_from_json(&wordindex_schema_json()).expect("load wordindex");

        let schemas = core.get_schemas().expect("get_schemas");
        let s = schemas.get("BlogPostWordIndex").expect("schema exists");
        match &s.schema_type {
            SchemaType::HashRange { keyconfig } => {
                assert_eq!(keyconfig.hash_field.as_deref(), Some("word"));
                assert_eq!(keyconfig.range_field.as_deref(), Some("publish_date"));
            }
            other => panic!("expected HashRange schema_type, got {:?}", other),
        }
    }

    #[test]
    fn load_wordindex_schema_from_file() {
        let core = SchemaCore::new_for_testing().expect("init core");
        let dir = tempfile::tempdir().expect("tempdir");
        let path = dir.path().join("BlogPostWordIndex.json");
        std::fs::write(&path, wordindex_schema_json()).expect("write schema json");

        core.load_schema_from_file(&path).expect("load from file");
        let schemas = core.get_schemas().expect("get_schemas");
        assert!(schemas.contains_key("BlogPostWordIndex"));
    }
}
