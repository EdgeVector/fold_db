use super::{schema_lock_error, SchemaCore, SchemaState};
use crate::schema::types::{JsonSchemaDefinition, Schema, SchemaError};
use log::info;
use std::collections::HashMap;
use std::path::{Path, PathBuf};

impl SchemaCore {
    /// Persist all schema load states using DbOperations
    pub(crate) fn persist_states(&self) -> Result<(), SchemaError> {
        let available = self
            .available
            .lock()
            .map_err(|_| schema_lock_error())?;

        for (name, (_, state)) in available.iter() {
            self.db_ops.store_schema_state(name, *state)?;
        }

        Ok(())
    }

    /// Load schema states using DbOperations
    pub fn load_states(&self) -> HashMap<String, SchemaState> {
        self.db_ops.get_all_schema_states().unwrap_or_default()
    }

    /// Persists a schema using DbOperations
    pub(crate) fn persist_schema(&self, schema: &Schema) -> Result<(), SchemaError> {
        self.db_ops.store_schema(&schema.name, schema)
    }

    /// Return all JSON schema files in the given directory
    pub(crate) fn iter_schema_files(dir: &Path) -> Result<Vec<PathBuf>, SchemaError> {
        let mut files = Vec::new();
        if let Ok(entries) = std::fs::read_dir(dir) {
            for entry in entries.flatten() {
                let path = entry.path();
                if path.extension().map(|e| e == "json").unwrap_or(false) {
                    files.push(path);
                }
            }
        }
        Ok(files)
    }

    /// Parse a schema from the given JSON file path
    pub(crate) fn parse_schema_file(&self, path: &Path) -> Result<Option<Schema>, SchemaError> {
        let contents = match std::fs::read_to_string(path) {
            Ok(c) => c,
            Err(e) => {
                return Err(SchemaError::InvalidData(format!(
                    "Failed to read {}: {}",
                    path.display(),
                    e
                )))
            }
        };

        log::info!("🔍 Parsing schema file: {}", path.display());

        let mut schema_opt = serde_json::from_str::<Schema>(&contents).ok();
        if schema_opt.is_some() {
            log::info!("✅ Parsed as Schema: {}", path.display());
        }
        
        if schema_opt.is_none() {
            if let Ok(json_schema) = serde_json::from_str::<JsonSchemaDefinition>(&contents) {
                if let Ok(schema) = self.interpret_schema(json_schema) {
                    schema_opt = Some(schema);
                    log::info!("✅ Parsed as JsonSchemaDefinition: {}", path.display());
                }
            }
        }
        
        if schema_opt.is_none() {
            if let Ok(declarative_schema) = serde_json::from_str::<crate::schema::types::json_schema::DeclarativeSchemaDefinition>(&contents) {
                log::info!("🔍 Attempting to interpret declarative schema: {}", path.display());
                if let Ok(schema) = self.interpret_declarative_schema(declarative_schema) {
                    schema_opt = Some(schema);
                    log::info!("✅ Parsed as DeclarativeSchemaDefinition: {}", path.display());
                } else {
                    log::warn!("❌ Failed to interpret declarative schema: {}", path.display());
                }
            } else {
                log::warn!("❌ Failed to parse as DeclarativeSchemaDefinition: {}", path.display());
            }
        }
        
        if schema_opt.is_none() {
            log::warn!("❌ Could not parse schema file: {}", path.display());
        }
        
        Ok(schema_opt)
    }

    /// Loads schemas from the `schemas` directory and restores their states.
    ///
    /// Schemas found in `available_schemas` are only discovered and added to the
    /// available list but are **not** automatically loaded into memory.
    pub fn load_schemas_from_disk(&self) -> Result<(), SchemaError> {
        let states = self.load_states();

        // Load from the node's schemas directory
        info!("Loading schemas from {}", self.schemas_dir.display());
        self.load_schemas_from_directory(&self.schemas_dir, &states)?;

        // Discover available schemas without loading them
        for mut schema in self.discover_available_schemas()? {
            self.fix_transform_outputs(&mut schema);
            let name = schema.name.clone();
            let state = states.get(&name).copied().unwrap_or(SchemaState::Available);
            let mut available = self
                .available
                .lock()
                .map_err(|_| schema_lock_error())?;
            available.insert(name.clone(), (schema, state));
            info!(
                "Discovered available schema '{}' from available_schemas/ with state: {:?}",
                name,
                state
            );
        }

        // Persist any changes to schema states from newly discovered schemas
        self.persist_states()?;

        Ok(())
    }

    /// Helper method to load schemas from a specific directory
    pub(crate) fn load_schemas_from_directory(
        &self,
        dir: &Path,
        states: &HashMap<String, SchemaState>,
    ) -> Result<(), SchemaError> {
        for path in Self::iter_schema_files(dir)? {
            if let Some(mut schema) = self.parse_schema_file(&path)? {
                self.fix_transform_outputs(&mut schema);
                let name = schema.name.clone();
                let state = states.get(&name).copied().unwrap_or(SchemaState::Available);
                {
                    let mut available = self
                        .available
                        .lock()
                        .map_err(|_| schema_lock_error())?;
                    available.insert(name.clone(), (schema.clone(), state));
                }
                if state == SchemaState::Approved {
                    let mut loaded = self
                        .schemas
                        .lock()
                        .map_err(|_| schema_lock_error())?;
                    loaded.insert(name.clone(), schema);
                    drop(loaded);
                    let _ = self.map_fields(&name);
                }
                info!(
                    "Loaded schema '{}' from {} with state: {:?}",
                    name,
                    dir.display(),
                    state
                );
            }
        }
        Ok(())
    }

    /// Loads schema states from sled and loads schemas that are marked as Approved.
    #[allow(dead_code)]
    pub(crate) fn load_schema_states_from_disk(&self) -> Result<(), SchemaError> {
        let states = self.load_states();
        info!("Loading schema states from sled: {:?}", states);
        info!(
            "DEBUG: load_schema_states_from_disk called with {} states",
            states.len()
        );
        let mut available = self
            .available
            .lock()
            .map_err(|_| schema_lock_error())?;
        let mut schemas = self
            .schemas
            .lock()
            .map_err(|_| schema_lock_error())?;

        for (name, state) in states {
            info!("DEBUG: Processing schema '{}' with state {:?}", name, state);
            if state == SchemaState::Approved {
                // Load the actual schema from sled database into active memory
                match self.db_ops.get_schema(&name) {
                    Ok(Some(mut schema)) => {
                        info!(
                            "Auto-loading approved schema '{}' from sled with {} fields: {:?}",
                            name,
                            schema.fields.len(),
                            schema.fields.keys().collect::<Vec<_>>()
                        );

                        // 🔄 Log molecule_uuid values during schema loading
                        info!(
                            "🔄 SCHEMA_LOAD - Loading schema '{}' with {} fields",
                            name,
                            schema.fields.len()
                        );
                        for (field_name, field_def) in &schema.fields {
                            use crate::schema::types::Field;
                            match field_def.molecule_uuid() {
                                Some(uuid) => info!(
                                    "📋 Field {}.{} has molecule_uuid: {}",
                                    name, field_name, uuid
                                ),
                                None => info!(
                                    "📋 Field {}.{} has molecule_uuid: None",
                                    name, field_name
                                ),
                            }
                        }

                        self.fix_transform_outputs(&mut schema);
                        info!(
                            "After fix_transform_outputs, auto-loaded schema '{}' has {} fields: {:?}",
                            name,
                            schema.fields.len(),
                            schema.fields.keys().collect::<Vec<_>>()
                        );
                        schemas.insert(name.clone(), schema.clone());
                        available.insert(name.clone(), (schema, state));
                        drop(schemas); // Release the lock before calling map_fields
                        drop(available); // Release the lock before calling map_fields

                        // Ensure fields have proper ARefs assigned
                        let _ = self.map_fields(&name);

                        // Re-acquire locks for the next iteration
                        available = self.available.lock().map_err(|_| {
                            schema_lock_error()
                        })?;
                        schemas = self.schemas.lock().map_err(|_| {
                            schema_lock_error()
                        })?;
                    }
                    Ok(None) => {
                        info!("Schema '{}' not found in sled, creating empty schema", name);
                        available.insert(name.clone(), (Schema::new(name), SchemaState::Available));
                    }
                    Err(e) => {
                        info!("Failed to load schema '{}' from sled: {}", name, e);
                        available.insert(name.clone(), (Schema::new(name), SchemaState::Available));
                    }
                }
            } else {
                // Load the actual schema from sled for non-Approved states too
                match self.db_ops.get_schema(&name) {
                    Ok(Some(mut schema)) => {
                        // 🔄 Log molecule_uuid values during schema loading (non-Approved)
                        info!(
                            "🔄 SCHEMA_LOAD - Loading schema '{}' (state: {:?}) with {} fields",
                            name,
                            state,
                            schema.fields.len()
                        );
                        for (field_name, field_def) in &schema.fields {
                            use crate::schema::types::Field;
                            match field_def.molecule_uuid() {
                                Some(uuid) => info!(
                                    "📋 Field {}.{} has molecule_uuid: {}",
                                    name, field_name, uuid
                                ),
                                None => info!(
                                    "📋 Field {}.{} has molecule_uuid: None",
                                    name, field_name
                                ),
                            }
                        }

                        self.fix_transform_outputs(&mut schema);
                        info!(
                            "Loading schema '{}' from sled with state {:?} and {} fields: {:?}",
                            name,
                            state,
                            schema.fields.len(),
                            schema.fields.keys().collect::<Vec<_>>()
                        );
                        available.insert(name.clone(), (schema, state));
                    }
                    Ok(None) => {
                        info!("Schema '{}' not found in sled, creating empty schema", name);
                        available.insert(name.clone(), (Schema::new(name), state));
                    }
                    Err(e) => {
                        info!(
                            "Failed to load schema '{}' from sled: {}, creating empty schema",
                            name, e
                        );
                        available.insert(name.clone(), (Schema::new(name), state));
                    }
                }
            }
        }
        Ok(())
    }

    /// Interprets a declarative schema definition and converts it to a Schema.
    pub fn interpret_declarative_schema(
        &self,
        declarative_schema: crate::schema::types::json_schema::DeclarativeSchemaDefinition,
    ) -> Result<Schema, SchemaError> {
        use crate::schema::types::{SingleField, FieldVariant, field::common::Field};
        use crate::fees::payment_config::SchemaPaymentConfig;
        use crate::permissions::types::policy::{PermissionsPolicy, TrustDistance};
        use crate::fees::types::config::FieldPaymentConfig;
        use crate::fees::types::config::TrustDistanceScaling;
        
        // Validate the declarative schema
        declarative_schema.validate()?;
        
        // Convert fields from FieldDefinition to FieldVariant
        let mut fields = std::collections::HashMap::new();
        for (field_name, field_def) in declarative_schema.fields.clone() {
            // Create a basic single field with default permissions and payment config
            let mut single_field = SingleField::new(
                PermissionsPolicy::new(
                    TrustDistance::Distance(0),
                    TrustDistance::Distance(1),
                ),
                FieldPaymentConfig {
                    base_multiplier: 1.0,
                    trust_distance_scaling: TrustDistanceScaling::None,
                    min_payment: None,
                },
                std::collections::HashMap::new(),
            );
            
            // Set molecule UUID if provided
            if let Some(atom_uuid) = field_def.atom_uuid {
                single_field.set_molecule_uuid(atom_uuid);
            }
            
            fields.insert(field_name, FieldVariant::Single(single_field));
        }
        
        // Create the schema with appropriate type
        let schema = Schema {
            name: declarative_schema.name.clone(),
            schema_type: declarative_schema.schema_type.clone(),
            fields,
            payment_config: SchemaPaymentConfig {
                base_multiplier: 1.0,
                min_payment_threshold: 0,
            },
            hash: None,
        };
        
        // Auto-register the declarative transform
        self.register_declarative_transform(&declarative_schema)?;
        
        Ok(schema)
    }

    /// Registers a declarative transform automatically when a declarative schema is loaded
    pub fn register_declarative_transform(
        &self,
        declarative_schema: &crate::schema::types::json_schema::DeclarativeSchemaDefinition,
    ) -> Result<(), SchemaError> {
        use crate::schema::types::{Transform, TransformRegistration};
        use log::info;
        
        // Create a transform from the declarative schema
        let transform = Transform::from_declarative_schema(
            declarative_schema.clone(),
            vec!["blogpost".to_string()], // Input schema name
            format!("{}.key", declarative_schema.name), // Output field
        );
        
        // Generate transform ID
        let transform_id = format!("{}.declarative", declarative_schema.name);
        
        // Extract input dependencies from the schema
        let mut input_molecules = Vec::new();
        let mut input_names = Vec::new();
        
        // Add the main input schema
        input_molecules.push("blogpost".to_string());
        input_names.push("blogpost".to_string());
        
        // Add any additional dependencies from field expressions
        for field_def in declarative_schema.fields.values() {
            if let Some(atom_uuid) = &field_def.atom_uuid {
                // Extract schema name from atom_uuid expression (e.g., "blogpost.map().$atom_uuid")
                if let Some(schema_name) = atom_uuid.split('.').next() {
                    if !input_molecules.contains(&schema_name.to_string()) {
                        input_molecules.push(schema_name.to_string());
                        input_names.push(schema_name.to_string());
                    }
                }
            }
        }
        
        // Create trigger fields based on input dependencies
        let trigger_fields = input_molecules.clone();
        
        // Create the registration
        let registration = TransformRegistration {
            transform_id: transform_id.clone(),
            transform,
            input_molecules,
            input_names,
            trigger_fields,
            output_molecule: format!("{}.key", declarative_schema.name),
            schema_name: declarative_schema.name.clone(),
            field_name: "key".to_string(),
        };
        
        // Store the transform registration in the database for later processing
        self.db_ops.store_transform(&transform_id, &registration.transform)?;
        self.db_ops.store_transform_registration(&registration)?;
        
        info!("✅ Auto-registered declarative transform: {} (stored for later processing)", transform_id);
        
        Ok(())
    }

    /// Gets a transform by ID from the database
    pub fn get_transform(&self, transform_id: &str) -> Result<Option<crate::schema::types::Transform>, SchemaError> {
        self.db_ops.get_transform(transform_id)
    }
}

