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
    /// Load a schema into memory and persist it to disk.
    /// This preserves existing schema state if it exists, otherwise defaults to Available.
    pub fn load_schema_internal(&self, mut schema: Schema) -> Result<(), SchemaError> {
        info!(
            "🔄 DEBUG: LOAD_SCHEMA_INTERNAL START - schema: '{}' with {} fields: {:?}",
            schema.name,
            schema.fields.len(),
            schema.fields.keys().collect::<Vec<_>>()
        );

        schema = self.resolve_persisted_schema(schema)?;

        self.log_field_refs(&schema);

        self.fix_transform_outputs(&mut schema);
        self.register_schema_transforms(&schema)?;

        self.persist_if_needed(&schema)?;

        self.update_state_and_memory(schema)?;

        Ok(())
    }

    fn resolve_persisted_schema(&self, schema: Schema) -> Result<Schema, SchemaError> {
        if let Ok(Some(persisted_schema)) = self.db_ops.get_schema(&schema.name) {
            info!(
                "📂 Found persisted schema for '{}' in database, using persisted version with field assignments",
                schema.name
            );
            
            // DIAGNOSTIC: Check if persisted schema actually has field assignments
            let assigned_fields = persisted_schema.fields.values()
                .filter(|f| f.molecule_uuid().is_some())
                .count();
            let total_fields = persisted_schema.fields.len();
            
            info!("🔍 DIAGNOSTIC: Persisted schema '{}' has {}/{} fields with molecule_uuid assignments",
                  schema.name, assigned_fields, total_fields);
                  
            if assigned_fields == 0 && total_fields > 0 {
                log_feature!(LogFeature::Schema, error, "🚨 CRITICAL: Persisted schema '{}' has NO field assignments! This will break transforms!", schema.name);
            }
            
            Ok(persisted_schema)
        } else {
            info!("📋 No persisted schema found for '{}', using JSON version", schema.name);
            Ok(schema)
        }
    }

    fn log_field_refs(&self, schema: &Schema) {
        for (field_name, field) in &schema.fields {
            let molecule_uuid = field
                .molecule_uuid()
                .map(|s| s.to_string())
                .unwrap_or_else(|| "None".to_string());
            info!("📋 Field {}.{} has molecule_uuid: {}", schema.name, field_name, molecule_uuid);
        }
    }

    fn persist_if_needed(&self, schema: &Schema) -> Result<(), SchemaError> {
        // CRITICAL FIX: Logic was backwards! We should persist when fields DON'T have molecule_uuid
        // to create them, OR when they DO have molecule_uuid to save the assignments
        let has_empty_fields = schema.fields.values().any(|f| f.molecule_uuid().is_none());
        let has_assigned_fields = schema.fields.values().any(|f| f.molecule_uuid().is_some());
        
        if has_empty_fields {
            info!("🔧 DIAGNOSTIC: Schema '{}' has empty fields, persisting to create molecule_uuid assignments", schema.name);
            self.persist_schema(schema)?;
            info!(
                "After persist_schema, schema '{}' has {} fields: {:?}",
                schema.name,
                schema.fields.len(),
                schema.fields.keys().collect::<Vec<_>>()
            );
        } else if has_assigned_fields {
            info!("🔧 DIAGNOSTIC: Schema '{}' has assigned fields, persisting to save molecule_uuid assignments", schema.name);
            self.persist_schema(schema)?;
        } else {
            info!(
                "🔧 DIAGNOSTIC: Schema '{}' has no fields, skipping persistence",
                schema.name
            );
        }
        Ok(())
    }

    fn update_state_and_memory(&self, schema: Schema) -> Result<(), SchemaError> {
        let name = schema.name.clone();
        let existing_state = self.db_ops.get_schema_state(&name).unwrap_or(None);
        let schema_state = existing_state.unwrap_or(SchemaState::Available);

        info!(
            "Schema '{}' existing state: {:?}, using state: {:?}",
            name, existing_state, schema_state
        );

        {
            let mut all = self.available.lock().map_err(|_| {
                schema_lock_error()
            })?;
            all.insert(name.clone(), (schema, schema_state));
        }

        if existing_state.is_none() {
            self.set_schema_state(&name, SchemaState::Available)?;
            info!("Schema '{}' loaded and marked as Available (new schema)", name);
        } else {
            info!("Schema '{}' loaded with preserved state: {:?}", name, schema_state);
        }

        self.publish_schema_loaded(&name);
        Ok(())
    }

    fn publish_schema_loaded(&self, name: &str) {
        use crate::fold_db_core::infrastructure::message_bus::schema_events::SchemaLoaded;
        let schema_loaded_event = SchemaLoaded::new(name.to_string(), "loaded");
        if let Err(e) = self.message_bus.publish(schema_loaded_event) {
            log_feature!(LogFeature::Schema, warn, "Failed to publish SchemaLoaded event: {}", e);
        }
    }

    /// Approve a schema for queries and mutations
    pub fn approve_schema(&self, schema_name: &str) -> Result<(), SchemaError> {
        info!("Approving schema '{}'", schema_name);

        // Check if schema exists and validate current state
        let (schema, current_state) = {
            let available = self.available.lock().map_err(|_| {
                schema_lock_error()
            })?;
            available.get(schema_name).cloned()
                .ok_or_else(|| SchemaError::NotFound(format!("Schema '{}' not found", schema_name)))?
        };

        // Validate state transition: Available and Blocked schemas can be approved
        match current_state {
            SchemaState::Available => {
                info!("✅ Schema '{}' is in Available state, proceeding with approval", schema_name);
            }
            SchemaState::Approved => {
                return Err(SchemaError::InvalidData(format!(
                    "Schema '{}' is already approved. Use block operation to change to blocked state.",
                    schema_name
                )));
            }
            SchemaState::Blocked => {
                info!("✅ Schema '{}' is in Blocked state, re-approving", schema_name);
            }
        }

        info!(
            "Schema '{}' to approve has {} fields: {:?}",
            schema_name,
            schema.fields.len(),
            schema.fields.keys().collect::<Vec<_>>()
        );

        // Update both in-memory stores and persist immediately
        {
            let mut schemas = self.schemas.lock().map_err(|_| {
                schema_lock_error()
            })?;
            let mut available = self.available.lock().map_err(|_| {
                schema_lock_error()
            })?;

            // Add to active schemas
            schemas.insert(schema_name.to_string(), schema.clone());
            // Update state in available
            available.insert(schema_name.to_string(), (schema, SchemaState::Approved));
        }

        // Persist the state change immediately
        self.persist_states()?;

        // Ensure fields have proper ARefs assigned (persistence happens in map_fields)
        match self.map_fields(schema_name) {
            Ok(molecules) => {
                info!(
                    "Schema '{}' field mapping successful: created {} atom references with proper types",
                    schema_name, molecules.len()
                );
                
                // CRITICAL: Persist the schema with field assignments to sled
                match self.get_schema(schema_name) {
                    Ok(Some(updated_schema)) => {
                        if let Err(e) = self.persist_schema(&updated_schema) {
                            log_feature!(LogFeature::Schema, warn, "Failed to persist schema '{}' with field assignments: {}", schema_name, e);
                        } else {
                            info!("✅ Schema '{}' with field assignments persisted to sled database", schema_name);
                        }
                    }
                    Ok(None) => {
                        log_feature!(LogFeature::Schema, warn, "Schema '{}' not found after field mapping", schema_name);
                    }
                    Err(e) => {
                        log_feature!(LogFeature::Schema, warn, "Failed to retrieve schema '{}' for persistence: {}", schema_name, e);
                    }
                }
            }
            Err(e) => {
                info!(
                    "Schema '{}' field mapping failed: {}. Schema approved but fields may not work correctly.",
                    schema_name, e
                );
            }
        }

        // CRITICAL: Re-register transforms that target this newly approved schema
        // When a schema is approved, transforms in OTHER schemas that were previously
        // skipped due to target schema state validation should now be registered
        info!("🔄 Re-registering transforms that target newly approved schema '{}'", schema_name);
        if let Err(e) = self.reregister_transforms_for_approved_schema(schema_name) {
            log_feature!(LogFeature::Schema, warn, "Failed to re-register transforms for approved schema '{}': {}", schema_name, e);
        }

        // Publish SchemaLoaded event for approval
        use crate::fold_db_core::infrastructure::message_bus::schema_events::SchemaLoaded;
        let schema_loaded_event = SchemaLoaded::new(schema_name, "approved");
        if let Err(e) = self.message_bus.publish(schema_loaded_event) {
            log_feature!(LogFeature::Schema, warn, "Failed to publish SchemaLoaded event for approval: {}", e);
        }

        // Publish SchemaChanged event for approval
        use crate::fold_db_core::infrastructure::message_bus::schema_events::SchemaChanged;
        let schema_changed_event = SchemaChanged::new(schema_name);
        if let Err(e) = self.message_bus.publish(schema_changed_event) {
            log_feature!(LogFeature::Schema, warn, "Failed to publish SchemaChanged event for approval: {}", e);
        }

        info!("Schema '{}' approved successfully", schema_name);
        Ok(())
    }

    /// Re-register transforms that target a newly approved schema
    /// This method is called when a schema is approved to ensure that transforms
    /// in OTHER schemas that were previously skipped due to target schema state
    /// validation are now registered
    fn reregister_transforms_for_approved_schema(&self, target_schema_name: &str) -> Result<(), SchemaError> {
        info!("🔍 Checking all schemas for transforms targeting newly approved schema '{}'", target_schema_name);
        
        let available_schemas = {
            let available = self.available.lock().map_err(|_| {
                schema_lock_error()
            })?;
            available.clone()
        };

        let mut transforms_registered = 0;
        
        for (schema_name, (schema, _)) in available_schemas {
            info!("🔍 Checking schema '{}' for transforms targeting '{}'", schema_name, target_schema_name);
            
            for (field_name, field) in &schema.fields {
                if let Some(transform) = field.transform() {
                    // Parse the transform output to get the target schema
                    let output_parts: Vec<&str> = transform.get_output().split('.').collect();
                    if output_parts.len() == 2 && output_parts[0] == target_schema_name {
                        let transform_id = format!("{}.{}", schema_name, field_name);
                        
                        info!("🎯 Found transform '{}' targeting newly approved schema '{}'", transform_id, target_schema_name);
                        
                        // Check if this transform is already registered
                        match self.db_ops.get_transform(&transform_id) {
                            Ok(Some(_)) => {
                                info!("✅ Transform '{}' already registered, skipping", transform_id);
                                continue;
                            }
                            Ok(None) => {
                                info!("📋 Registering previously skipped transform '{}'", transform_id);
                                
                                // Store the transform in the database
                                if let Err(e) = self.db_ops.store_transform(&transform_id, transform) {
                                    log_feature!(LogFeature::Schema, error, "Failed to store transform {}: {}", transform_id, e);
                                    continue;
                                }
                                
                                // Create field-to-transform mappings
                                for input_field in transform.get_inputs() {
                                    if let Err(e) = self.store_field_to_transform_mapping(input_field, &transform_id) {
                                        log_feature!(LogFeature::Schema, error,
                                            "Failed to store field mapping '{}' → '{}': {}",
                                            input_field, transform_id, e
                                        );
                                    } else {
                                        info!("✅ Stored field mapping: '{}' → '{}' transform", input_field, transform_id);
                                    }
                                }
                                
                                transforms_registered += 1;
                                info!("✅ Registered transform '{}' for newly approved target schema '{}'", transform_id, target_schema_name);
                            }
                            Err(e) => {
                                log_feature!(LogFeature::Schema, error, "Error checking if transform '{}' exists: {}", transform_id, e);
                                continue;
                            }
                        }
                    }
                }
            }
        }
        
        info!("🎉 Re-registered {} transforms targeting newly approved schema '{}'", transforms_registered, target_schema_name);
        Ok(())
    }

    /// Ensures an approved schema is present in the schemas HashMap for field mapping
    /// This is used during initialization to fix the issue where approved schemas
    /// loaded from disk remain in 'available' but map_fields() only looks in 'schemas'
    pub fn ensure_approved_schema_in_schemas(&self, schema_name: &str) -> Result<(), SchemaError> {
        info!("Ensuring approved schema '{}' is available in schemas HashMap", schema_name);

        // Check if schema is already in schemas HashMap
        {
            let schemas = self.schemas.lock().map_err(|_| {
                schema_lock_error()
            })?;
            if schemas.contains_key(schema_name) {
                info!("Schema '{}' already in schemas HashMap", schema_name);
                return Ok(());
            }
        }

        // Get the schema from available HashMap and verify it's approved
        let schema_to_move = {
            let available = self.available.lock().map_err(|_| {
                schema_lock_error()
            })?;
            
            if let Some((schema, state)) = available.get(schema_name) {
                if *state == SchemaState::Approved {
                    Some(schema.clone())
                } else {
                    return Err(SchemaError::InvalidData(
                        format!("Schema '{}' is not in Approved state", schema_name)
                    ));
                }
            } else {
                return Err(SchemaError::NotFound(
                    format!("Schema '{}' not found in available schemas", schema_name)
                ));
            }
        };

        // Move the schema to schemas HashMap
        if let Some(schema) = schema_to_move {
            let mut schemas = self.schemas.lock().map_err(|_| {
                schema_lock_error()
            })?;
            
            schemas.insert(schema_name.to_string(), schema);
            info!("Successfully moved approved schema '{}' to schemas HashMap for field mapping", schema_name);
        }

        Ok(())
    }

    /// Block a schema from queries and mutations
    pub fn block_schema(&self, schema_name: &str) -> Result<(), SchemaError> {
        info!("Blocking schema '{}'", schema_name);

        // Check current state and validate transition
        let current_state = {
            let available = self.available.lock().map_err(|_| {
                schema_lock_error()
            })?;
            available.get(schema_name)
                .map(|(_, state)| *state)
                .ok_or_else(|| SchemaError::NotFound(format!("Schema '{}' not found", schema_name)))?
        };

        // Validate state transition: only Approved schemas can be blocked
        match current_state {
            SchemaState::Approved => {
                info!("✅ Schema '{}' is in Approved state, proceeding with blocking", schema_name);
            }
            SchemaState::Available => {
                return Err(SchemaError::InvalidData(format!(
                    "Schema '{}' is in Available state. Only approved schemas can be blocked. Approve the schema first.",
                    schema_name
                )));
            }
            SchemaState::Blocked => {
                return Err(SchemaError::InvalidData(format!(
                    "Schema '{}' is already blocked.",
                    schema_name
                )));
            }
        }

        // Remove from active schemas but keep in available
        {
            let mut schemas = self.schemas.lock().map_err(|_| {
                schema_lock_error()
            })?;
            schemas.remove(schema_name);
        }

        self.set_schema_state(schema_name, SchemaState::Blocked)?;
        
        // Publish SchemaChanged event for blocking
        use crate::fold_db_core::infrastructure::message_bus::schema_events::SchemaChanged;
        let schema_changed_event = SchemaChanged::new(schema_name);
        if let Err(e) = self.message_bus.publish(schema_changed_event) {
            log_feature!(LogFeature::Schema, warn, "Failed to publish SchemaChanged event for blocking: {}", e);
        }
        
        info!("Schema '{}' blocked successfully", schema_name);
        Ok(())
    }

    /// Add a new schema from JSON to the available_schemas directory with validation
    pub fn add_schema_to_available_directory(
        &self,
        json_content: &str,
        schema_name: Option<String>,
    ) -> Result<String, SchemaError> {
        info!("Adding new schema to available_schemas directory");

        // Parse and validate the JSON schema
        let json_schema = self.parse_and_validate_json_schema(json_content)?;
        let final_name = schema_name.unwrap_or_else(|| json_schema.name.clone());

        // Check for duplicates and conflicts using the dedicated module
        super::duplicate_detection::SchemaDuplicateDetector::check_schema_conflicts(
            &json_schema,
            &final_name,
            "available_schemas",
            |hash, exclude| self.find_schema_by_hash(hash, exclude),
        )?;

        // Write schema to file with hash using the dedicated module
        super::file_operations::SchemaFileOperations::write_schema_to_file(
            &json_schema,
            &final_name,
            "available_schemas",
        )?;

        // Load schema into memory
        let schema = self.interpret_schema(json_schema)?;
        self.load_schema_internal(schema)?;

        info!(
            "Schema '{}' added to available schemas and ready for approval",
            final_name
        );
        Ok(final_name)
    }

    /// Parse and validate JSON schema content
    fn parse_and_validate_json_schema(
        &self,
        json_content: &str,
    ) -> Result<super::types::JsonSchemaDefinition, SchemaError> {
        let json_schema: super::types::JsonSchemaDefinition = serde_json::from_str(json_content)
            .map_err(|e| SchemaError::InvalidField(format!("Invalid JSON schema: {}", e)))?;

        let validator = super::validator::SchemaValidator::new(self);
        validator.validate_json_schema(&json_schema)?;
        info!("JSON schema validation passed for '{}'", json_schema.name);

        Ok(json_schema)
    }

    /// Find a schema with the same hash (for duplicate detection) in the specified directory
    /// Find a schema with the same hash (for duplicate detection)
    fn find_schema_by_hash(
        &self,
        target_hash: &str,
        exclude_name: &str,
    ) -> Result<Option<String>, SchemaError> {
        let available_schemas_dir = std::path::PathBuf::from("available_schemas");

        if let Ok(entries) = std::fs::read_dir(&available_schemas_dir) {
            for entry in entries.flatten() {
                let path = entry.path();
                if path.extension().map(|e| e == "json").unwrap_or(false) {
                    // Skip the file we're trying to create
                    if let Some(file_stem) = path.file_stem() {
                        if file_stem == exclude_name {
                            continue;
                        }
                    }

                    if let Ok(content) = std::fs::read_to_string(&path) {
                        if let Ok(schema_json) = serde_json::from_str::<serde_json::Value>(&content)
                        {
                            // Check if schema has a hash field
                            if let Some(existing_hash) =
                                schema_json.get("hash").and_then(|h| h.as_str())
                            {
                                if existing_hash == target_hash {
                                    if let Some(name) =
                                        schema_json.get("name").and_then(|n| n.as_str())
                                    {
                                        return Ok(Some(name.to_string()));
                                    }
                                }
                            } else {
                                // Calculate hash for schemas without hash field
                                if let Ok(calculated_hash) =
                                    super::hasher::SchemaHasher::calculate_hash(&schema_json)
                                {
                                    if calculated_hash == target_hash {
                                        if let Some(name) =
                                            schema_json.get("name").and_then(|n| n.as_str())
                                        {
                                            return Ok(Some(name.to_string()));
                                        }
                                    }
                                }
                            }
                        }
                    }
                }
            }
        }

        Ok(None)
    }

    /// Maps fields between schemas based on their defined relationships.
    /// Returns a list of Molecules that need to be persisted in FoldDB.
    pub fn map_fields(&self, schema_name: &str) -> Result<Vec<MoleculeVariant>, SchemaError> {
        info!("🔧 Starting field mapping for schema '{}'", schema_name);
        
        let schemas = self
            .schemas
            .lock()
            .map_err(|_| schema_lock_error())?;

        // First collect all the source field molecule_uuids we need
        let mut field_mappings = Vec::new();
        if let Some(schema) = schemas.get(schema_name) {
            for (field_name, field) in &schema.fields {
                for (source_schema_name, source_field_name) in field.field_mappers() {
                    if let Some(source_schema) = schemas.get(source_schema_name) {
                        if let Some(source_field) = source_schema.fields.get(source_field_name) {
                            if let Some(molecule_uuid) = source_field.molecule_uuid() {
                                field_mappings.push((field_name.clone(), molecule_uuid.clone()));
                            }
                        }
                    }
                }
            }
        }
        drop(schemas); // Release the immutable lock

        // Now get a mutable lock to update the fields
        let mut schemas = self
            .schemas
            .lock()
            .map_err(|_| schema_lock_error())?;

        let schema = schemas
            .get_mut(schema_name)
            .ok_or_else(|| SchemaError::InvalidData(format!("Schema {schema_name} not found")))?;

        // Apply the collected mappings
        for (field_name, molecule_uuid) in field_mappings {
            if let Some(field) = schema.fields.get_mut(&field_name) {
                field.set_molecule_uuid(molecule_uuid);
            }
        }

        // Use the field mapping module to create molecules for unmapped fields
        let molecules = map_fields(&self.db_ops, schema)?;

        // Persist the updated schema
        self.persist_schema(schema)?;

        // Also update the available HashMap to keep it in sync
        let updated_schema = schema.clone();
        drop(schemas); // Release the schemas lock

        let mut available = self
            .available
            .lock()
            .map_err(|_| schema_lock_error())?;

        if let Some((_, state)) = available.get(schema_name) {
            let state = *state;
            available.insert(schema_name.to_string(), (updated_schema, state));
        }

        Ok(molecules)
    }

    /// Interprets a JSON schema definition and converts it to a Schema.
    pub fn interpret_schema(
        &self,
        json_schema: crate::schema::types::JsonSchemaDefinition,
    ) -> Result<Schema, SchemaError> {
        interpret_schema(&SchemaValidator::new(self), json_schema)
    }

    /// Interprets a JSON schema from a string and loads it as Available.
    pub fn load_schema_from_json(&self, json_str: &str) -> Result<(), SchemaError> {
        let schema = load_schema_from_json(&SchemaValidator::new(self), json_str)?;
        self.load_schema_internal(schema)
    }

    /// Interprets a JSON schema from a file and loads it as Available.
    pub fn load_schema_from_file(&self, path: &str) -> Result<(), SchemaError> {
        let schema = load_schema_from_file(&SchemaValidator::new(self), path)?;
        self.load_schema_internal(schema)
    }
}
