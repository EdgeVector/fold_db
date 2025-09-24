use crate::error::FoldDbResult;
use crate::schema;

use super::DataFoldNode;

impl DataFoldNode {
    /// Get comprehensive schema status for UI
    pub fn get_schema_status(&self) -> FoldDbResult<schema::SchemaLoadingReport> {
        let db = self
            .db
            .lock()
            .map_err(|_| crate::error::FoldDbError::Config("Cannot lock database mutex".into()))?;
        db.schema_manager.get_schema_status().map_err(|e| {
            crate::error::FoldDbError::Config(format!("Failed to get schema status: {}", e))
        })
    }

    /// Refresh schemas from all sources
    pub fn refresh_schemas(&self) -> FoldDbResult<schema::SchemaLoadingReport> {
        let db = self
            .db
            .lock()
            .map_err(|_| crate::error::FoldDbError::Config("Cannot lock database mutex".into()))?;
        db.schema_manager
            .discover_and_load_all_schemas()
            .map_err(|e| {
                crate::error::FoldDbError::Config(format!("Failed to refresh schemas: {}", e))
            })
    }

    /// Add a schema to the available schemas list
    pub fn add_schema_available(
        &mut self,
        schema: schema::Schema,
    ) -> crate::error::FoldDbResult<()> {
        let db = self
            .db
            .lock()
            .map_err(|_| crate::error::FoldDbError::Config("Cannot lock database mutex".into()))?;
        db.schema_manager
            .add_schema_available(schema)
            .map_err(|e| crate::error::FoldDbError::Config(format!("Failed to add schema: {}", e)))
    }

    /// List all schemas with their states
    pub fn list_schemas_with_state(
        &self,
    ) -> crate::error::FoldDbResult<std::collections::HashMap<String, schema::SchemaState>> {
        let db = self
            .db
            .lock()
            .map_err(|_| crate::error::FoldDbError::Config("Cannot lock database mutex".into()))?;

        // Get persisted states from database
        let mut states = db.schema_manager.load_states();

        // Add available schemas with their current states
        let available = db.schema_manager.available.lock().map_err(|_| {
            crate::error::FoldDbError::Config("Cannot lock available schemas mutex".into())
        })?;

        for (schema_name, (_, state)) in available.iter() {
            states.insert(schema_name.clone(), *state);
        }

        Ok(states)
    }

    /// Get a schema by name
    pub fn get_schema(
        &self,
        schema_name: &str,
    ) -> crate::error::FoldDbResult<Option<schema::Schema>> {
        let db = self
            .db
            .lock()
            .map_err(|_| crate::error::FoldDbError::Config("Cannot lock database mutex".into()))?;
        db.get_schema(schema_name)
            .map_err(|e| crate::error::FoldDbError::Config(format!("Failed to get schema: {}", e)))
    }

    /// Load a schema (approve it for use)
    pub fn load_schema(&mut self, schema: schema::Schema) -> crate::error::FoldDbResult<()> {
        self.add_schema_available(schema.clone())?;
        self.approve_schema(&schema.name)
    }

    /// Approve a schema for queries and mutations
    pub fn approve_schema(&mut self, schema_name: &str) -> crate::error::FoldDbResult<()> {
        {
            let db = self.db.lock().map_err(|_| {
                crate::error::FoldDbError::Config("Cannot lock database mutex".into())
            })?;
            db.schema_manager.approve_schema(schema_name).map_err(|e| {
                crate::error::FoldDbError::Config(format!("Failed to approve schema: {}", e))
            })?;
        }

        let db = self
            .db
            .lock()
            .map_err(|_| crate::error::FoldDbError::Config("Cannot lock database mutex".into()))?;

        let mut current_perms = db.get_schema_permissions(&self.node_id);
        if !current_perms.contains(&schema_name.to_string()) {
            current_perms.push(schema_name.to_string());
            db.set_schema_permissions(&self.node_id, &current_perms)
                .map_err(|e| {
                    crate::error::FoldDbError::Config(format!("Failed to set permissions: {}", e))
                })?;
        }

        Ok(())
    }

    /// Unload a schema (set to available state)
    pub fn unload_schema(&self, schema_name: &str) -> crate::error::FoldDbResult<()> {
        let db = self
            .db
            .lock()
            .map_err(|_| crate::error::FoldDbError::Config("Cannot lock database mutex".into()))?;
        db.unload_schema(schema_name).map_err(|e| {
            crate::error::FoldDbError::Config(format!("Failed to unload schema: {}", e))
        })
    }

    /// List all loaded (approved) schemas
    pub fn list_schemas(&self) -> crate::error::FoldDbResult<Vec<String>> {
        let db = self
            .db
            .lock()
            .map_err(|_| crate::error::FoldDbError::Config("Cannot lock database mutex".into()))?;
        db.schema_manager.list_loaded_schemas().map_err(|e| {
            crate::error::FoldDbError::Config(format!("Failed to list schemas: {}", e))
        })
    }

    /// List all available schemas (any state)
    pub fn list_available_schemas(&self) -> crate::error::FoldDbResult<Vec<String>> {
        let db = self
            .db
            .lock()
            .map_err(|_| crate::error::FoldDbError::Config("Cannot lock database mutex".into()))?;
        db.schema_manager.list_available_schemas().map_err(|e| {
            crate::error::FoldDbError::Config(format!("Failed to list available schemas: {}", e))
        })
    }

    /// Load schema from file
    pub fn load_schema_from_file(&mut self, path: &str) -> crate::error::FoldDbResult<()> {
        let mut db = self
            .db
            .lock()
            .map_err(|_| crate::error::FoldDbError::Config("Cannot lock database mutex".into()))?;
        db.load_schema_from_file(path).map_err(|e| {
            crate::error::FoldDbError::Config(format!("Failed to load schema from file: {}", e))
        })
    }

    /// Check if a schema is loaded (approved)
    pub fn is_schema_loaded(&self, schema_name: &str) -> crate::error::FoldDbResult<bool> {
        let db = self
            .db
            .lock()
            .map_err(|_| crate::error::FoldDbError::Config("Cannot lock database mutex".into()))?;
        let state = db.schema_manager.get_schema_state(schema_name);
        Ok(matches!(state, Some(schema::SchemaState::Approved)))
    }

    /// List schemas by specific state
    pub fn list_schemas_by_state(
        &self,
        state: schema::SchemaState,
    ) -> crate::error::FoldDbResult<Vec<String>> {
        let db = self
            .db
            .lock()
            .map_err(|_| crate::error::FoldDbError::Config("Cannot lock database mutex".into()))?;
        db.schema_manager.list_schemas_by_state(state).map_err(|e| {
            crate::error::FoldDbError::Config(format!("Failed to list schemas by state: {}", e))
        })
    }

    /// Block a schema from queries and mutations
    pub fn block_schema(&mut self, schema_name: &str) -> crate::error::FoldDbResult<()> {
        if !self.check_schema_permission(schema_name)? {
            return Err(crate::error::FoldDbError::Config(format!(
                "Permission denied for schema {}",
                schema_name
            )));
        }

        let db = self
            .db
            .lock()
            .map_err(|_| crate::error::FoldDbError::Config("Cannot lock database mutex".into()))?;
        db.schema_manager.block_schema(schema_name).map_err(|e| {
            crate::error::FoldDbError::Config(format!("Failed to block schema: {}", e))
        })
    }

    /// Get the current state of a schema
    pub fn get_schema_state(
        &self,
        schema_name: &str,
    ) -> crate::error::FoldDbResult<schema::SchemaState> {
        let db = self
            .db
            .lock()
            .map_err(|_| crate::error::FoldDbError::Config("Cannot lock database mutex".into()))?;

        let exists = db.schema_manager.schema_exists(schema_name).map_err(|e| {
            crate::error::FoldDbError::Config(format!("Failed to check schema existence: {}", e))
        })?;

        if !exists {
            return Err(crate::error::FoldDbError::Config(format!(
                "Schema '{}' not found",
                schema_name
            )));
        }

        let states = db.schema_manager.load_states();

        Ok(states
            .get(schema_name)
            .copied()
            .unwrap_or(schema::SchemaState::Available))
    }

    /// Set schema permissions for a node (for testing)
    pub fn set_schema_permissions(
        &self,
        node_id: &str,
        schemas: &[String],
    ) -> crate::error::FoldDbResult<()> {
        let db = self
            .db
            .lock()
            .map_err(|_| crate::error::FoldDbError::Config("Cannot lock database mutex".into()))?;
        db.set_schema_permissions(node_id, schemas).map_err(|e| {
            crate::error::FoldDbError::Config(format!("Failed to set schema permissions: {}", e))
        })
    }

    /// Add a new schema from JSON content to the available_schemas directory with validation
    pub fn add_schema_to_available_directory(
        &self,
        json_content: &str,
        schema_name: Option<String>,
    ) -> crate::error::FoldDbResult<String> {
        let db = self
            .db
            .lock()
            .map_err(|_| crate::error::FoldDbError::Config("Cannot lock database mutex".into()))?;

        db.schema_manager
            .add_schema_to_available_directory(json_content, schema_name)
            .map_err(|e| crate::error::FoldDbError::Config(format!("Failed to add schema: {}", e)))
    }
}
