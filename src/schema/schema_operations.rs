use super::{schema_lock_error, validator::SchemaValidator};
use crate::fold_db_core::infrastructure::message_bus::MessageBus;
use crate::logging::features::{log_feature, LogFeature};
use crate::schema::constants::{
    DEFAULT_OUTPUT_FIELD_NAME, DEFAULT_TRANSFORM_ID_SUFFIX, KEY_CONFIG_HASH_FIELD,
    KEY_CONFIG_RANGE_FIELD, KEY_FIELD_NAME,
};
use crate::schema::types::{
    json_schema::DeclarativeSchemaDefinition, Field, FieldVariant, Schema, SchemaError,
};
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
            let assigned_fields = persisted_schema
                .fields
                .values()
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
            info!(
                "📋 No persisted schema found for '{}', using JSON version",
                schema.name
            );
            Ok(schema)
        }
    }

    fn log_field_refs(&self, schema: &Schema) {
        for (field_name, field) in &schema.fields {
            let molecule_uuid = field
                .molecule_uuid()
                .map(|s| s.to_string())
                .unwrap_or_else(|| "None".to_string());
            info!(
                "📋 Field {}.{} has molecule_uuid: {}",
                schema.name, field_name, molecule_uuid
            );
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
            let mut all = self.available.lock().map_err(|_| schema_lock_error())?;
            all.insert(name.clone(), (schema, schema_state));
        }

        if existing_state.is_none() {
            self.set_schema_state(&name, SchemaState::Available)?;
            info!(
                "Schema '{}' loaded and marked as Available (new schema)",
                name
            );
        } else {
            info!(
                "Schema '{}' loaded with preserved state: {:?}",
                name, schema_state
            );
        }

        self.publish_schema_loaded(&name);
        Ok(())
    }

    fn publish_schema_loaded(&self, name: &str) {
        use crate::fold_db_core::infrastructure::message_bus::schema_events::SchemaLoaded;
        let schema_loaded_event = SchemaLoaded::new(name.to_string(), "loaded");
        if let Err(e) = self.message_bus.publish(schema_loaded_event) {
            log_feature!(
                LogFeature::Schema,
                warn,
                "Failed to publish SchemaLoaded event: {}",
                e
            );
        }
    }

    /// Approve a schema for queries and mutations
    pub fn approve_schema(&self, schema_name: &str) -> Result<(), SchemaError> {
        info!("Approving schema '{}'", schema_name);

        // Check if schema exists and validate current state
        let (schema, current_state) = {
            let available = self.available.lock().map_err(|_| schema_lock_error())?;
            available.get(schema_name).cloned().ok_or_else(|| {
                SchemaError::NotFound(format!("Schema '{}' not found", schema_name))
            })?
        };

        // Validate state transition: Available and Blocked schemas can be approved
        match current_state {
            SchemaState::Available => {
                info!(
                    "✅ Schema '{}' is in Available state, proceeding with approval",
                    schema_name
                );
            }
            SchemaState::Approved => {
                return Err(SchemaError::InvalidData(format!(
                    "Schema '{}' is already approved. Use block operation to change to blocked state.",
                    schema_name
                )));
            }
            SchemaState::Blocked => {
                info!(
                    "✅ Schema '{}' is in Blocked state, re-approving",
                    schema_name
                );
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
            let mut schemas = self.schemas.lock().map_err(|_| schema_lock_error())?;
            let mut available = self.available.lock().map_err(|_| schema_lock_error())?;

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
                            log_feature!(
                                LogFeature::Schema,
                                warn,
                                "Failed to persist schema '{}' with field assignments: {}",
                                schema_name,
                                e
                            );
                        } else {
                            info!(
                                "✅ Schema '{}' with field assignments persisted to sled database",
                                schema_name
                            );
                        }
                    }
                    Ok(None) => {
                        log_feature!(
                            LogFeature::Schema,
                            warn,
                            "Schema '{}' not found after field mapping",
                            schema_name
                        );
                    }
                    Err(e) => {
                        log_feature!(
                            LogFeature::Schema,
                            warn,
                            "Failed to retrieve schema '{}' for persistence: {}",
                            schema_name,
                            e
                        );
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

        // CRITICAL: Check if this is a declarative schema and register its transform
        info!(
            "🔍 DEBUG: Checking if schema '{}' is HashRange for declarative transform registration",
            schema_name
        );

        match self.get_schema(schema_name) {
            Ok(Some(approved_schema)) => {
                info!(
                    "🔍 DEBUG: Retrieved schema '{}' with type: {:?}",
                    schema_name, approved_schema.schema_type
                );
                if matches!(
                    approved_schema.schema_type,
                    crate::schema::types::schema::SchemaType::HashRange
                ) {
                    info!("🔍 Detected HashRange declarative schema '{}', registering declarative transform", schema_name);
                    if let Err(e) = self.register_declarative_transform_for_schema(&approved_schema)
                    {
                        log_feature!(
                            LogFeature::Schema,
                            warn,
                            "Failed to register declarative transform for schema '{}': {}",
                            schema_name,
                            e
                        );
                    } else {
                        info!(
                            "✅ Successfully registered declarative transform for schema '{}'",
                            schema_name
                        );
                    }
                } else {
                    info!("⏸️ Schema '{}' is not HashRange type, skipping declarative transform registration", schema_name);
                }
            }
            Ok(None) => {
                log_feature!(
                    LogFeature::Schema,
                    error,
                    "🚨 CRITICAL: Schema '{}' not found after approval!",
                    schema_name
                );
            }
            Err(e) => {
                log_feature!(
                    LogFeature::Schema,
                    error,
                    "🚨 CRITICAL: Failed to retrieve schema '{}' after approval: {}",
                    schema_name,
                    e
                );
            }
        }

        // CRITICAL: Re-register transforms that target this newly approved schema
        // When a schema is approved, transforms in OTHER schemas that were previously
        // skipped due to target schema state validation should now be registered
        info!(
            "🔄 Re-registering transforms that target newly approved schema '{}'",
            schema_name
        );
        if let Err(e) = self.reregister_transforms_for_approved_schema(schema_name) {
            log_feature!(
                LogFeature::Schema,
                warn,
                "Failed to re-register transforms for approved schema '{}': {}",
                schema_name,
                e
            );
        }

        // Publish SchemaLoaded event for approval
        use crate::fold_db_core::infrastructure::message_bus::schema_events::SchemaLoaded;
        let schema_loaded_event = SchemaLoaded::new(schema_name, "approved");
        if let Err(e) = self.message_bus.publish(schema_loaded_event) {
            log_feature!(
                LogFeature::Schema,
                warn,
                "Failed to publish SchemaLoaded event for approval: {}",
                e
            );
        }

        // Publish SchemaChanged event for approval
        use crate::fold_db_core::infrastructure::message_bus::schema_events::SchemaChanged;
        let schema_changed_event = SchemaChanged::new(schema_name);
        if let Err(e) = self.message_bus.publish(schema_changed_event) {
            log_feature!(
                LogFeature::Schema,
                warn,
                "Failed to publish SchemaChanged event for approval: {}",
                e
            );
        }

        info!("Schema '{}' approved successfully", schema_name);
        Ok(())
    }

    /// Register a declarative transform for a HashRange schema
    ///
    /// PURPOSE: This is the PRIMARY registration path for declarative transforms.
    /// Called when a HashRange schema is approved to automatically register the
    /// corresponding declarative transform that will process data for that schema.
    ///
    /// FLOW: Schema approval → HashRange detection → Transform registration
    ///
    /// This method:
    /// 1. Extracts input dependencies from HashRange field atom_uuid expressions
    /// 2. Creates trigger fields for all fields in the input schema
    /// 3. Stores the transform and registration in the database
    /// 4. Creates field-to-transform mappings for trigger detection
    pub fn register_declarative_transform_for_schema(
        &self,
        schema: &Schema,
    ) -> Result<(), SchemaError> {
        use crate::schema::types::{Transform, TransformRegistration};
        use log::info;

        info!(
            "🔧 Registering declarative transform for HashRange schema '{}'",
            schema.name
        );

        // Extract input dependencies from the schema fields
        let mut input_molecules = Vec::new();
        let mut input_names = Vec::new();

        // Parse field expressions to extract input schemas
        for field in schema.fields.values() {
            if let crate::schema::types::field::FieldVariant::HashRange(hashrange_field) = field {
                // Extract schema names from atom_uuid expressions (same logic as analyze_dependencies)
                let atom_uuid = &hashrange_field.atom_uuid;
                // Extract schema names from expressions like "BlogPost.map().content"
                // Take the first part before the first dot
                if let Some(first_dot) = atom_uuid.find('.') {
                    let schema_name = &atom_uuid[..first_dot];
                    if !schema_name.is_empty()
                        && !input_molecules.contains(&schema_name.to_string())
                    {
                        input_molecules.push(schema_name.to_string());
                        input_names.push(schema_name.to_string());
                        info!("📋 Added input dependency: {}", schema_name);
                    }
                }
            }
        }

        // If no input dependencies found, use a default
        if input_molecules.is_empty() {
            input_molecules.push("BlogPost".to_string());
            input_names.push("BlogPost".to_string());
            info!("📋 Using default input dependency: BlogPost");
        }

        // Create trigger fields based on input dependencies
        // For declarative transforms, we need to trigger on ALL fields of the input schema
        let mut trigger_fields = Vec::new();
        for input_schema in &input_molecules {
            // Get the schema to find all its fields
            if let Ok(Some(schema)) = self.db_ops.get_schema(input_schema) {
                for field_name in schema.fields.keys() {
                    let field_key = format!("{}.{}", input_schema, field_name);
                    trigger_fields.push(field_key);
                }
            } else {
                // Fallback: if we can't get the schema, just use the schema name
                trigger_fields.push(input_schema.to_string());
            }
        }

        // Generate transform ID using configurable suffix
        let transform_id = format!("{}.{}", schema.name, DEFAULT_TRANSFORM_ID_SUFFIX);

        // For HashRange schemas, use the first field as output field
        // TODO: In the future, this could be made configurable via schema metadata
        let output_field = if let Some((first_field_name, _)) = schema.fields.iter().next() {
            first_field_name.clone()
        } else {
            return Err(SchemaError::InvalidData(format!(
                "HashRange schema '{}' has no fields",
                schema.name
            )));
        };

        // Create a declarative transform from the schema
        let transform = Transform::from_declarative_schema(
            self.convert_schema_to_declarative_definition(schema)?,
            input_molecules.clone(),
            format!("{}.{}", schema.name, output_field),
        );

        // Create the registration
        let registration = TransformRegistration {
            transform_id: transform_id.clone(),
            transform,
            input_molecules,
            input_names,
            trigger_fields,
            output_molecule: format!("{}.{}", schema.name, output_field),
            schema_name: schema.name.clone(),
            field_name: output_field,
        };

        // Store the transform in the database
        if let Err(e) = self
            .db_ops
            .store_transform(&transform_id, &registration.transform)
        {
            return Err(SchemaError::InvalidData(format!(
                "Failed to store transform {}: {}",
                transform_id, e
            )));
        }

        // Store the transform registration in the database
        if let Err(e) = self.db_ops.store_transform_registration(&registration) {
            return Err(SchemaError::InvalidData(format!(
                "Failed to store transform registration {}: {}",
                transform_id, e
            )));
        }

        // Create field-to-transform mappings using trigger_fields (individual field names)
        for trigger_field in &registration.trigger_fields {
            if let Err(e) = self.store_field_to_transform_mapping(trigger_field, &transform_id) {
                log_feature!(
                    LogFeature::Schema,
                    error,
                    "Failed to store field mapping '{}' → '{}': {}",
                    trigger_field,
                    transform_id,
                    e
                );
            } else {
                info!(
                    "✅ Stored field mapping: '{}' → '{}' transform",
                    trigger_field, transform_id
                );
            }
        }

        info!(
            "✅ Successfully registered declarative transform '{}' for schema '{}'",
            transform_id, schema.name
        );

        // BACKFILL: Trigger the transform to process existing data
        info!(
            "🔄 BACKFILL: Triggering declarative transform '{}' to process existing data",
            transform_id
        );
        self.trigger_transform_backfill(&transform_id)?;

        Ok(())
    }

    /// Trigger a transform to process existing data (backfill mechanism)
    ///
    /// PURPOSE: When a declarative transform is registered, it should automatically
    /// process all existing data in the source schemas to create the initial index.
    ///
    /// FLOW: Transform registration → Backfill trigger → Transform execution
    ///
    /// This method:
    /// 1. Publishes a TransformTriggered event for the transform
    /// 2. The event system will automatically queue and execute the transform
    /// 3. The transform will process all existing data in the source schemas
    fn trigger_transform_backfill(&self, transform_id: &str) -> Result<(), SchemaError> {
        use crate::fold_db_core::infrastructure::message_bus::events::schema_events::TransformTriggered;

        info!(
            "🔄 BACKFILL: Publishing TransformTriggered event for backfill: {}",
            transform_id
        );

        // Create a TransformTriggered event for backfill
        let triggered_event = TransformTriggered::new(transform_id.to_string());

        // Publish the event to trigger transform execution
        match self.message_bus.publish(triggered_event) {
            Ok(()) => {
                info!(
                    "✅ BACKFILL: Successfully published TransformTriggered event for: {}",
                    transform_id
                );
            }
            Err(e) => {
                log_feature!(
                    LogFeature::Schema,
                    error,
                    "❌ BACKFILL: Failed to publish TransformTriggered event for {}: {}",
                    transform_id,
                    e
                );
                return Err(SchemaError::InvalidData(format!(
                    "Failed to publish TransformTriggered event for backfill {}: {}",
                    transform_id, e
                )));
            }
        }

        info!(
            "✅ BACKFILL: Transform '{}' backfill trigger completed",
            transform_id
        );
        Ok(())
    }

    /// Convert a Schema to DeclarativeSchemaDefinition for transform creation
    pub fn convert_schema_to_declarative_definition(
        &self,
        schema: &Schema,
    ) -> Result<crate::schema::types::json_schema::DeclarativeSchemaDefinition, SchemaError> {
        use crate::schema::types::json_schema::{
            DeclarativeSchemaDefinition, FieldDefinition, KeyConfig,
        };
        use std::collections::HashMap;

        let mut fields = HashMap::new();

        info!(
            "🔧 Converting schema '{}' to declarative definition",
            schema.name
        );
        info!("📊 Schema has {} fields", schema.fields.len());
        info!("🔍 Schema type: {:?}", schema.schema_type);

        // For HashRange schemas, handle HashRange field variants
        if matches!(
            schema.schema_type,
            crate::schema::types::schema::SchemaType::HashRange
        ) {
            info!("🔍 Processing HashRange schema with HashRange field variants");

            // Convert all fields to FieldDefinition (they should all be HashRange variants)
            for (field_name, field) in &schema.fields {
                info!(
                    "🔍 Processing field '{}' with variant: {:?}",
                    field_name,
                    std::mem::discriminant(field)
                );

                if let crate::schema::types::field::FieldVariant::HashRange(hashrange_field) = field
                {
                    info!(
                        "✅ Converting HashRange field '{}' to FieldDefinition",
                        field_name
                    );
                    fields.insert(
                        field_name.clone(),
                        FieldDefinition {
                            atom_uuid: Some(hashrange_field.atom_uuid.clone()),
                            field_type: Some("single".to_string()),
                        },
                    );
                } else {
                    info!("⚠️ Skipping non-HashRange field '{}'", field_name);
                }
            }

            info!(
                "📋 Converted {} fields to declarative definition",
                fields.len()
            );

            // For HashRange schemas, we need to get the key configuration from the original JSON file
            // since the schema fields don't contain the hash_field and range_field information
            let key_config = self.get_universal_key_config_from_json(schema.name.as_str())?;

            let declarative_schema = DeclarativeSchemaDefinition {
                name: schema.name.clone(),
                schema_type: schema.schema_type.clone(),
                key: key_config,
                fields,
            };

            info!(
                "✅ Created declarative schema with {} fields",
                declarative_schema.fields.len()
            );
            Ok(declarative_schema)
        } else {
            // For non-HashRange schemas, handle Single field variants
            info!("🔍 Processing non-HashRange schema with Single field variants");

            // Convert schema fields to DeclarativeSchemaDefinition fields
            for (field_name, field) in &schema.fields {
                info!(
                    "🔍 Processing field '{}' with variant: {:?}",
                    field_name,
                    std::mem::discriminant(field)
                );

                if let crate::schema::types::field::FieldVariant::Single(single_field) = field {
                    info!(
                        "✅ Converting Single field '{}' to FieldDefinition",
                        field_name
                    );
                    fields.insert(
                        field_name.clone(),
                        FieldDefinition {
                            atom_uuid: single_field.molecule_uuid().cloned(),
                            field_type: Some("single".to_string()),
                        },
                    );
                } else {
                    info!("⚠️ Skipping non-Single field '{}'", field_name);
                }
            }

            info!(
                "📋 Converted {} fields to declarative definition",
                fields.len()
            );

            // Non-HashRange schemas don't need key configuration
            let declarative_schema = DeclarativeSchemaDefinition {
                name: schema.name.clone(),
                schema_type: schema.schema_type.clone(),
                key: None,
                fields,
            };

            info!(
                "✅ Created declarative schema with {} fields",
                declarative_schema.fields.len()
            );
            Ok(declarative_schema)
        }
    }

    /// Get universal key configuration from the original JSON schema file
    ///
    /// This function reads the key configuration from a schema JSON file and returns it
    /// as a KeyConfig. It handles all schema types (Single, Range, HashRange) and provides
    /// clear error messages for malformed configurations.
    ///
    /// # Arguments
    /// * `schema_name` - The name of the schema file (without .json extension)
    ///
    /// # Returns
    /// * `Ok(Some(KeyConfig))` - If key configuration is found and valid
    /// * `Ok(None)` - If no key configuration is present (valid for Single schemas)
    /// * `Err(SchemaError)` - If the file cannot be read, JSON is malformed, or key config is invalid
    pub fn get_universal_key_config_from_json(
        &self,
        schema_name: &str,
    ) -> Result<Option<crate::schema::types::json_schema::KeyConfig>, SchemaError> {
        use crate::schema::types::json_schema::KeyConfig;
        use serde_json::Value;

        let schema_file_path = format!("available_schemas/{}.json", schema_name);
        info!("🔍 Reading universal key config from: {}", schema_file_path);

        let content = std::fs::read_to_string(&schema_file_path).map_err(|e| {
            SchemaError::InvalidData(format!(
                "Failed to read schema file {}: {}",
                schema_file_path, e
            ))
        })?;

        let json_value: Value = serde_json::from_str(&content).map_err(|e| {
            SchemaError::InvalidData(format!(
                "Failed to parse JSON from {}: {}",
                schema_file_path, e
            ))
        })?;

        if let Some(key_obj) = json_value.get(KEY_FIELD_NAME).and_then(|v| v.as_object()) {
            let hash_field = key_obj
                .get(KEY_CONFIG_HASH_FIELD)
                .and_then(|v| v.as_str())
                .map(|s| s.to_string())
                .unwrap_or_default();

            let range_field = key_obj
                .get(KEY_CONFIG_RANGE_FIELD)
                .and_then(|v| v.as_str())
                .map(|s| s.to_string())
                .unwrap_or_default();

            info!(
                "🔑 Found key config - hash_field: '{}', range_field: '{}'",
                hash_field, range_field
            );

            Ok(Some(KeyConfig {
                hash_field,
                range_field,
            }))
        } else {
            info!("⚠️ No key configuration found in schema file");
            Ok(None)
        }
    }

    /// Re-register transforms that target a newly approved schema
    /// This method is called when a schema is approved to ensure that transforms
    /// in OTHER schemas that were previously skipped due to target schema state
    /// validation are now registered
    fn reregister_transforms_for_approved_schema(
        &self,
        target_schema_name: &str,
    ) -> Result<(), SchemaError> {
        info!(
            "🔍 Checking all schemas for transforms targeting newly approved schema '{}'",
            target_schema_name
        );

        let available_schemas = {
            let available = self.available.lock().map_err(|_| schema_lock_error())?;
            available.clone()
        };

        let mut transforms_registered = 0;

        for (schema_name, (schema, _)) in available_schemas {
            info!(
                "🔍 Checking schema '{}' for transforms targeting '{}'",
                schema_name, target_schema_name
            );

            for (field_name, field) in &schema.fields {
                if let Some(transform) = field.transform() {
                    // Parse the transform output to get the target schema
                    let output_parts: Vec<&str> = transform.get_output().split('.').collect();
                    if output_parts.len() == 2 && output_parts[0] == target_schema_name {
                        let transform_id = format!("{}.{}", schema_name, field_name);

                        info!(
                            "🎯 Found transform '{}' targeting newly approved schema '{}'",
                            transform_id, target_schema_name
                        );

                        // Check if this transform is already registered
                        match self.db_ops.get_transform(&transform_id) {
                            Ok(Some(_)) => {
                                info!(
                                    "✅ Transform '{}' already registered, skipping",
                                    transform_id
                                );
                                continue;
                            }
                            Ok(None) => {
                                info!(
                                    "📋 Registering previously skipped transform '{}'",
                                    transform_id
                                );

                                // Store the transform in the database
                                if let Err(e) =
                                    self.db_ops.store_transform(&transform_id, transform)
                                {
                                    log_feature!(
                                        LogFeature::Schema,
                                        error,
                                        "Failed to store transform {}: {}",
                                        transform_id,
                                        e
                                    );
                                    continue;
                                }

                                // Create field-to-transform mappings
                                for input_field in transform.get_inputs() {
                                    if let Err(e) = self.store_field_to_transform_mapping(
                                        input_field,
                                        &transform_id,
                                    ) {
                                        log_feature!(
                                            LogFeature::Schema,
                                            error,
                                            "Failed to store field mapping '{}' → '{}': {}",
                                            input_field,
                                            transform_id,
                                            e
                                        );
                                    } else {
                                        info!(
                                            "✅ Stored field mapping: '{}' → '{}' transform",
                                            input_field, transform_id
                                        );
                                    }
                                }

                                transforms_registered += 1;
                                info!("✅ Registered transform '{}' for newly approved target schema '{}'", transform_id, target_schema_name);
                            }
                            Err(e) => {
                                log_feature!(
                                    LogFeature::Schema,
                                    error,
                                    "Error checking if transform '{}' exists: {}",
                                    transform_id,
                                    e
                                );
                                continue;
                            }
                        }
                    }
                }
            }
        }

        info!(
            "🎉 Re-registered {} transforms targeting newly approved schema '{}'",
            transforms_registered, target_schema_name
        );
        Ok(())
    }

    /// Ensures an approved schema is present in the schemas HashMap for field mapping
    /// This is used during initialization to fix the issue where approved schemas
    /// loaded from disk remain in 'available' but map_fields() only looks in 'schemas'
    pub fn ensure_approved_schema_in_schemas(&self, schema_name: &str) -> Result<(), SchemaError> {
        info!(
            "Ensuring approved schema '{}' is available in schemas HashMap",
            schema_name
        );

        // Check if schema is already in schemas HashMap
        {
            let schemas = self.schemas.lock().map_err(|_| schema_lock_error())?;
            if schemas.contains_key(schema_name) {
                info!("Schema '{}' already in schemas HashMap", schema_name);
                return Ok(());
            }
        }

        // Get the schema from available HashMap and verify it's approved
        let schema_to_move = {
            let available = self.available.lock().map_err(|_| schema_lock_error())?;

            if let Some((schema, state)) = available.get(schema_name) {
                if *state == SchemaState::Approved {
                    Some(schema.clone())
                } else {
                    return Err(SchemaError::InvalidData(format!(
                        "Schema '{}' is not in Approved state",
                        schema_name
                    )));
                }
            } else {
                return Err(SchemaError::NotFound(format!(
                    "Schema '{}' not found in available schemas",
                    schema_name
                )));
            }
        };

        // Move the schema to schemas HashMap
        if let Some(schema) = schema_to_move {
            let mut schemas = self.schemas.lock().map_err(|_| schema_lock_error())?;

            schemas.insert(schema_name.to_string(), schema);
            info!(
                "Successfully moved approved schema '{}' to schemas HashMap for field mapping",
                schema_name
            );
        }

        Ok(())
    }

    /// Block a schema from queries and mutations
    pub fn block_schema(&self, schema_name: &str) -> Result<(), SchemaError> {
        info!("Blocking schema '{}'", schema_name);

        // Check current state and validate transition
        let current_state = {
            let available = self.available.lock().map_err(|_| schema_lock_error())?;
            available
                .get(schema_name)
                .map(|(_, state)| *state)
                .ok_or_else(|| {
                    SchemaError::NotFound(format!("Schema '{}' not found", schema_name))
                })?
        };

        // Validate state transition: only Approved schemas can be blocked
        match current_state {
            SchemaState::Approved => {
                info!(
                    "✅ Schema '{}' is in Approved state, proceeding with blocking",
                    schema_name
                );
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
            let mut schemas = self.schemas.lock().map_err(|_| schema_lock_error())?;
            schemas.remove(schema_name);
        }

        self.set_schema_state(schema_name, SchemaState::Blocked)?;

        // Publish SchemaChanged event for blocking
        use crate::fold_db_core::infrastructure::message_bus::schema_events::SchemaChanged;
        let schema_changed_event = SchemaChanged::new(schema_name);
        if let Err(e) = self.message_bus.publish(schema_changed_event) {
            log_feature!(
                LogFeature::Schema,
                warn,
                "Failed to publish SchemaChanged event for blocking: {}",
                e
            );
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

        let schemas = self.schemas.lock().map_err(|_| schema_lock_error())?;

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
        let mut schemas = self.schemas.lock().map_err(|_| schema_lock_error())?;

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

        let mut available = self.available.lock().map_err(|_| schema_lock_error())?;

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

/// Unified key extraction helper for all schema types.
///
/// This function provides a single entry point to extract hash and range values
/// from any schema type, supporting both the new universal KeyConfig and legacy
/// range_key patterns.
///
/// # Arguments
///
/// * `schema` - The schema to extract keys from
/// * `data` - The data object containing the actual values
///
/// # Returns
///
/// A tuple of (hash_value, range_value) where both are Option<String>.
/// For Single schemas, both will be None unless key is provided.
/// For Range schemas, hash_value may be None, range_value will be extracted.
/// For HashRange schemas, both will be extracted from key configuration.
pub trait SchemaKeyContext {
    fn schema_name(&self) -> &str;
    fn schema_type(&self) -> &crate::schema::types::schema::SchemaType;
    fn key_config(&self) -> Option<&crate::schema::types::json_schema::KeyConfig>;
}

impl SchemaKeyContext for Schema {
    fn schema_name(&self) -> &str {
        &self.name
    }

    fn schema_type(&self) -> &crate::schema::types::schema::SchemaType {
        &self.schema_type
    }

    fn key_config(&self) -> Option<&crate::schema::types::json_schema::KeyConfig> {
        self.key.as_ref()
    }
}

impl SchemaKeyContext for DeclarativeSchemaDefinition {
    fn schema_name(&self) -> &str {
        &self.name
    }

    fn schema_type(&self) -> &crate::schema::types::schema::SchemaType {
        &self.schema_type
    }

    fn key_config(&self) -> Option<&crate::schema::types::json_schema::KeyConfig> {
        self.key.as_ref()
    }
}

pub fn extract_unified_keys<S>(
    schema: &S,
    data: &serde_json::Value,
) -> Result<(Option<String>, Option<String>), SchemaError>
where
    S: SchemaKeyContext,
{
    match schema.schema_type() {
        crate::schema::types::schema::SchemaType::Single => {
            // For Single schemas, keys are optional and used for indexing hints
            // Check if schema has a key configuration
            if let Some(key_config) = schema.key_config() {
                let hash_value = if !key_config.hash_field.trim().is_empty() {
                    extract_field_value(data, &key_config.hash_field)?
                } else {
                    None
                };

                let range_value = if !key_config.range_field.trim().is_empty() {
                    extract_field_value(data, &key_config.range_field)?
                } else {
                    None
                };

                Ok((hash_value, range_value))
            } else {
                // No key configuration, return None for both
                Ok((None, None))
            }
        }
        crate::schema::types::schema::SchemaType::Range { range_key } => {
            // For Range schemas, use universal key configuration if available, otherwise fall back to legacy range_key
            let range_value = if let Some(key_config) = schema.key_config() {
                // Universal key configuration takes precedence
                let trimmed_field = key_config.range_field.trim();
                if trimmed_field.is_empty() {
                    return Err(SchemaError::InvalidData(
                        "Range schema with key configuration must have range_field".to_string(),
                    ));
                }

                match extract_field_value(data, trimmed_field)? {
                    Some(value) => Some(value),
                    None => {
                        if let Some(value) = extract_field_value(data, "range_key")? {
                            Some(value)
                        } else if let Some(value) = extract_field_value(data, "range")? {
                            Some(value)
                        } else {
                            return Err(SchemaError::InvalidData(format!(
                                "Range schema '{}' requires key.range_field '{}' in payload or normalized range value",
                                schema.schema_name(), trimmed_field
                            )));
                        }
                    }
                }
            } else {
                // Legacy range_key support - this maintains backward compatibility
                // First try to extract using the schema's range_key field name
                let trimmed_range_key = range_key.trim();
                if trimmed_range_key.is_empty() {
                    return Err(SchemaError::InvalidData(format!(
                        "Range schema '{}' is missing range_key configuration",
                        schema.schema_name()
                    )));
                }

                if let Some(value) = extract_field_value(data, trimmed_range_key)? {
                    Some(value)
                } else if let Some(value) = extract_field_value(data, "range_key")? {
                    Some(value)
                } else if let Some(value) = extract_field_value(data, "range")? {
                    Some(value)
                } else {
                    return Err(SchemaError::InvalidData(format!(
                        "Range schema '{}' requires range key field '{}' or normalized range value in payload",
                        schema.schema_name(), trimmed_range_key
                    )));
                }
            };

            let hash_value = if let Some(key_config) = schema.key_config() {
                if !key_config.hash_field.trim().is_empty() {
                    extract_field_value(data, &key_config.hash_field)?
                } else {
                    None
                }
            } else {
                None
            };

            Ok((hash_value, range_value))
        }
        crate::schema::types::schema::SchemaType::HashRange => {
            // For HashRange schemas, both hash and range are required
            let key_config = schema.key_config().ok_or_else(|| {
                SchemaError::InvalidData("HashRange schema requires key configuration".to_string())
            })?;

            if key_config.hash_field.trim().is_empty() {
                return Err(SchemaError::InvalidData(
                    "HashRange schema requires key.hash_field".to_string(),
                ));
            }
            if key_config.range_field.trim().is_empty() {
                return Err(SchemaError::InvalidData(
                    "HashRange schema requires key.range_field".to_string(),
                ));
            }
            let hash_field: &str = &key_config.hash_field;
            let range_field: &str = &key_config.range_field;

            // Prefer direct hash_key/range_key values provided by the caller when available.
            // Declarative HashRange transforms emit these fields explicitly, so we can avoid
            // re-evaluating complex key expressions (e.g., map().split_by_word()).
            let direct_hash = data
                .get("hash_key")
                .and_then(|value| value.as_str())
                .map(|value| value.to_string());
            let direct_range = data
                .get("range_key")
                .and_then(|value| value.as_str())
                .map(|value| value.to_string());

            let hash_value = if let Some(hash) = direct_hash {
                hash
            } else {
                extract_field_value(data, hash_field)?.ok_or_else(|| {
                    SchemaError::InvalidData(format!(
                        "HashRange hash_field '{}' not found in data",
                        hash_field
                    ))
                })?
            };

            let range_value = if let Some(range) = direct_range {
                range
            } else {
                extract_field_value(data, range_field)?.ok_or_else(|| {
                    SchemaError::InvalidData(format!(
                        "HashRange range_field '{}' not found in data",
                        range_field
                    ))
                })?
            };

            Ok((Some(hash_value), Some(range_value)))
        }
    }
}

/// Helper function to extract field value from data using field expression.
///
/// Supports both direct field access and dotted path expressions.
fn extract_field_value(
    data: &serde_json::Value,
    field_expression: &str,
) -> Result<Option<String>, SchemaError> {
    let trimmed_expr = field_expression.trim();
    if trimmed_expr.is_empty() {
        return Ok(None);
    }

    // Handle dotted path expressions (e.g., "data.timestamp", "input.map().id")
    if trimmed_expr.contains('.') {
        // For now, support simple dotted paths like "data.field" or "field.subfield"
        let parts: Vec<&str> = trimmed_expr.split('.').collect();
        let mut current = data;

        for part in parts {
            if let Some(obj) = current.as_object() {
                if let Some(value) = obj.get(part) {
                    current = value;
                } else {
                    return Ok(None);
                }
            } else {
                return Ok(None);
            }
        }

        // Convert the final value to string
        Ok(Some(current.to_string().trim_matches('"').to_string()))
    } else {
        // Direct field access
        if let Some(obj) = data.as_object() {
            if let Some(value) = obj.get(trimmed_expr) {
                Ok(Some(value.to_string().trim_matches('"').to_string()))
            } else {
                Ok(None)
            }
        } else {
            Ok(None)
        }
    }
}

#[inline]
fn last_segment(expression: &str) -> &str {
    expression.rsplit('.').next().unwrap_or("")
}

/// Standardized result shaping helper for all schema types.
///
/// Shapes query/mutation results into a consistent { hash, range, fields } object.
pub fn shape_unified_result<S>(
    schema: &S,
    data: &serde_json::Value,
    hash_value: Option<String>,
    range_value: Option<String>,
) -> Result<serde_json::Value, SchemaError>
where
    S: SchemaKeyContext,
{
    let mut result = serde_json::Map::new();
    result.insert(
        "hash".to_string(),
        serde_json::Value::String(hash_value.unwrap_or_default()),
    );
    result.insert(
        "range".to_string(),
        serde_json::Value::String(range_value.unwrap_or_default()),
    );

    // Determine key field names (last segment of expressions)
    let mut key_field_names: Vec<String> = Vec::new();
    match schema.schema_type() {
        crate::schema::types::schema::SchemaType::Single => {
            if let Some(key) = schema.key_config() {
                if !key.hash_field.trim().is_empty() {
                    key_field_names.push(last_segment(&key.hash_field).to_string());
                }
                if !key.range_field.trim().is_empty() {
                    key_field_names.push(last_segment(&key.range_field).to_string());
                }
            }
        }
        crate::schema::types::schema::SchemaType::Range { range_key } => {
            key_field_names.push(range_key.clone());
            if let Some(key) = schema.key_config() {
                if !key.hash_field.trim().is_empty() {
                    key_field_names.push(last_segment(&key.hash_field).to_string());
                }
                if !key.range_field.trim().is_empty() {
                    key_field_names.push(last_segment(&key.range_field).to_string());
                }
            }
        }
        crate::schema::types::schema::SchemaType::HashRange => {
            if let Some(key) = schema.key_config() {
                if key.hash_field.trim().is_empty() || key.range_field.trim().is_empty() {
                    return Err(SchemaError::InvalidData(
                        "HashRange schema requires key.hash_field and key.range_field".to_string(),
                    ));
                }
                key_field_names.push(last_segment(&key.hash_field).to_string());
                key_field_names.push(last_segment(&key.range_field).to_string());
                // HashRange payloads frequently include explicit hash_key/range_key fields; exclude
                // them from the shaped field set so only derived values remain.
                key_field_names.push("hash_key".to_string());
                key_field_names.push("range_key".to_string());
            }
        }
    }

    // Build fields object excluding key field names
    let mut fields_obj = serde_json::Map::new();
    if let Some(obj) = data.as_object() {
        for (k, v) in obj {
            if !key_field_names.iter().any(|n| n == k) {
                fields_obj.insert(k.clone(), v.clone());
            }
        }
    }
    result.insert("fields".to_string(), serde_json::Value::Object(fields_obj));

    Ok(serde_json::Value::Object(result))
}
