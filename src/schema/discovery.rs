use super::{schema_lock_error, SchemaCore, SchemaState};
use crate::schema::{SchemaLoadingReport, SchemaSource};
use crate::schema::types::{JsonSchemaDefinition, Schema, SchemaError};
use log::info;
use std::collections::HashMap;
use std::path::PathBuf;

impl SchemaCore {
    /// Discover schemas from the schemas directory
    pub fn discover_schemas_from_files(&self) -> Result<Vec<Schema>, SchemaError> {
        let mut discovered_schemas = Vec::new();

        info!("Discovering schemas from {}", self.schemas_dir.display());
        for path in Self::iter_schema_files(&self.schemas_dir)? {
            if let Some(mut schema) = self.parse_schema_file(&path)? {
                self.fix_transform_outputs(&mut schema);
                let schema_name = schema.name.clone();
                discovered_schemas.push(schema);
                info!("Discovered schema '{}' from file", schema_name);
            }
        }

        Ok(discovered_schemas)
    }

    /// Discover schemas from the available_schemas directory
    pub fn discover_available_schemas(&self) -> Result<Vec<Schema>, SchemaError> {
        info!("🔍 DEBUG: Starting discovery from available_schemas directory");
        let mut discovered_schemas = Vec::new();
        let available_schemas_dir = PathBuf::from("available_schemas");

        info!(
            "Discovering available schemas from {}",
            available_schemas_dir.display()
        );
        for path in Self::iter_schema_files(&available_schemas_dir)? {
            if let Some(mut schema) = self.parse_schema_file(&path)? {
                self.fix_transform_outputs(&mut schema);
                let schema_name = schema.name.clone();
                discovered_schemas.push(schema);
                info!("Discovered available schema '{}' from file", schema_name);
            }
        }

        Ok(discovered_schemas)
    }

    /// Load all schemas from the available_schemas directory into SchemaCore
    pub fn load_available_schemas_from_directory(&self) -> Result<(), SchemaError> {
        let discovered_schemas = self.discover_available_schemas()?;

        for schema in discovered_schemas {
            let schema_name = schema.name.clone();
            info!("Loading available schema '{}' into SchemaCore", schema_name);
            self.load_schema_internal(schema)?;
        }

        info!(
            "Loaded {} schemas from available_schemas directory",
            self.list_available_schemas()?.len()
        );
        Ok(())
    }

    /// Single entry point for all schema discovery and loading
    pub fn discover_and_load_all_schemas(&self) -> Result<SchemaLoadingReport, SchemaError> {
        info!("🔍 Starting unified schema discovery and loading");

        let mut discovered_schemas = Vec::new();
        let mut failed_schemas = Vec::new();
        let mut loading_sources = HashMap::new();

        let current_schemas = {
            let available = self.available.lock().map_err(|_| {
                schema_lock_error()
            })?;
            available
                .keys()
                .cloned()
                .collect::<std::collections::HashSet<String>>()
        };

        info!("📋 Loading existing schema states from persistence first");
        let schema_states = self.load_states();
        for schema_name in schema_states.keys() {
            loading_sources.insert(schema_name.clone(), SchemaSource::Persistence);
            info!("Loaded persisted schema state for '{}'", schema_name);
        }

        match self.discover_available_schemas() {
            Ok(schemas) => {
                for schema in schemas {
                    let schema_name = schema.name.clone();
                    discovered_schemas.push(schema_name.clone());

                    if !loading_sources.contains_key(&schema_name) {
                        loading_sources.insert(
                            schema_name.clone(),
                            SchemaSource::AvailableDirectory,
                        );
                    }

                    // Record schema as available but do not load it automatically
                    let state = schema_states
                        .get(&schema_name)
                        .copied()
                        .unwrap_or(SchemaState::Available);
                    {
                        let mut available = self
                            .available
                            .lock()
                            .map_err(|_| schema_lock_error())?;
                        available.insert(schema_name.clone(), (schema, state));
                    }

                    info!(
                        "Discovered available schema '{}' with state {:?} (not auto-loaded)",
                        schema_name,
                        state
                    );
                }
            }
            Err(e) => {
                info!("Failed to discover schemas from available_schemas/: {}", e);
            }
        }

        match self.discover_schemas_from_files() {
            Ok(schemas) => {
                for schema in schemas {
                    let schema_name = schema.name.clone();
                    if !discovered_schemas.contains(&schema_name) {
                        discovered_schemas.push(schema_name.clone());

                        if !loading_sources.contains_key(&schema_name) {
                            loading_sources.insert(schema_name.clone(), SchemaSource::DataDirectory);
                        }

                        if !current_schemas.contains(&schema_name) {
                            info!(
                                "Loading new schema '{}' from data/schemas/ (preserving persisted state)",
                                schema_name
                            );
                            if let Err(e) = self.load_schema_internal(schema) {
                                failed_schemas.push((schema_name, e.to_string()));
                            }
                        } else {
                            info!("Schema '{}' already in memory, skipping reload", schema_name);
                        }
                    }
                }
            }
            Err(e) => {
                info!("Failed to discover schemas from data/schemas/: {}", e);
            }
        }

        let loaded_schemas = self
            .list_schemas_by_state(SchemaState::Approved)
            .unwrap_or_else(|_| Vec::new());

        info!(
            "✅ Schema discovery complete: {} discovered, {} loaded, {} failed",
            discovered_schemas.len(),
            loaded_schemas.len(),
            failed_schemas.len()
        );

        Ok(SchemaLoadingReport {
            discovered_schemas,
            loaded_schemas,
            failed_schemas,
            schema_states,
            loading_sources,
            last_updated: chrono::Utc::now(),
        })
    }

    pub fn initialize_schema_system(&self) -> Result<(), SchemaError> {
        info!("🚀 Initializing schema system");
        self.discover_and_load_all_schemas()?;
        info!("✅ Schema system initialized successfully");
        Ok(())
    }

    pub fn get_schema_status(&self) -> Result<SchemaLoadingReport, SchemaError> {
        info!("📊 Getting schema status");

        let schema_states = self.load_states();
        let loaded_schemas = self
            .list_schemas_by_state(SchemaState::Approved)
            .unwrap_or_else(|_| Vec::new());

        let discovered_schemas: Vec<String> = schema_states.keys().cloned().collect();

        let loading_sources: HashMap<String, SchemaSource> = discovered_schemas
            .iter()
            .map(|name| (name.clone(), SchemaSource::Persistence))
            .collect();

        Ok(SchemaLoadingReport {
            discovered_schemas,
            loaded_schemas,
            failed_schemas: Vec::new(),
            schema_states,
            loading_sources,
            last_updated: chrono::Utc::now(),
        })
    }

    pub fn fetch_available_schemas(&self) -> Result<Vec<String>, SchemaError> {
        let mut all_schemas = Vec::new();

        let discovered_default = self.discover_schemas_from_files()?;
        all_schemas.extend(discovered_default.into_iter().map(|s| s.name));

        let discovered_available = self.discover_available_schemas()?;
        all_schemas.extend(discovered_available.into_iter().map(|s| s.name));

        let mut unique_schemas = Vec::new();
        for schema_name in all_schemas {
            if !unique_schemas.contains(&schema_name) {
                unique_schemas.push(schema_name);
            }
        }

        Ok(unique_schemas)
    }

    pub fn load_schema_state(&self) -> Result<HashMap<String, SchemaState>, SchemaError> {
        let states = self.load_states();
        Ok(states)
    }

    pub fn load_available_schemas(&self) -> Result<(), SchemaError> {
        self.load_schemas_from_disk()
    }
}

