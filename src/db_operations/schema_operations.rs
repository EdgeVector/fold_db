// Legacy v1 schema operations - DEPRECATED
// Use schema_operations_v2.rs for new code
#[allow(dead_code)]
use super::core_refactored::DbOperationsV2 as DbOperations;
use crate::schema::Schema;
use crate::schema::SchemaError;
use crate::schema::SchemaState;

impl DbOperations {
    /// Stores a schema state using generic tree operations
    pub fn store_schema_state(
        &self,
        schema_name: &str,
        state: SchemaState,
    ) -> Result<(), SchemaError> {
        self.store_in_tree(&self.schema_states_tree, schema_name, &state)
    }

    /// Gets a schema state using generic tree operations
    pub fn get_schema_state(&self, schema_name: &str) -> Result<Option<SchemaState>, SchemaError> {
        self.get_from_tree(&self.schema_states_tree, schema_name)
    }

    /// Lists all schemas with a specific state
    pub fn list_schemas_by_state(
        &self,
        target_state: SchemaState,
    ) -> Result<Vec<String>, SchemaError> {
        let all_states: Vec<(String, SchemaState)> =
            self.list_items_in_tree(&self.schema_states_tree)?;
        Ok(all_states
            .into_iter()
            .filter(|(_, state)| *state == target_state)
            .map(|(name, _)| name)
            .collect())
    }

    /// Stores a schema definition using generic tree operations
    ///
    /// IMPORTANT: SCHEMA STRUCTURE IS IMMUTABLE
    /// - Schema structure (field names, types, transforms) cannot be modified once stored
    /// - Field assignments (molecule_uuid values) CAN be updated as part of normal operations
    /// - This allows field mapping while preventing breaking structural changes
    ///
    /// Automatically creates placeholder Molecules/Molecules for fields that don't have them.
    /// This ensures all fields are immediately queryable after schema creation.
    pub fn store_schema(&self, schema_name: &str, schema: &Schema) -> Result<(), SchemaError> {
        // Store the immutable schema
        self.store_in_tree(&self.schemas_tree, schema_name, &schema)
    }

    /// Gets a schema definition using generic tree operations
    /// Populates runtime_fields from the declarative schema definition
    pub fn get_schema(&self, schema_name: &str) -> Result<Option<Schema>, SchemaError> {
        let mut schema_opt: Option<Schema> = self.get_from_tree(&self.schemas_tree, schema_name)?;
        
        // Populate runtime_fields if schema exists
        if let Some(schema) = schema_opt.as_mut() {
            schema.populate_runtime_fields()?;
        }
        
        Ok(schema_opt)
    }

    /// Lists all stored schemas using generic tree operations
    pub fn list_all_schemas(&self) -> Result<Vec<String>, SchemaError> {
        self.list_keys_in_tree(&self.schemas_tree)
    }

    /// Deletes a schema definition
    pub fn delete_schema(&self, schema_name: &str) -> Result<bool, SchemaError> {
        self.delete_from_tree(&self.schemas_tree, schema_name)
    }

    /// Deletes a schema state
    pub fn delete_schema_state(&self, schema_name: &str) -> Result<bool, SchemaError> {
        self.delete_from_tree(&self.schema_states_tree, schema_name)
    }

    // NOTE: add_schema_to_available_directory has been removed to eliminate duplication.
    // Use SchemaCore::add_schema_to_available_directory instead, which provides:
    // - Comprehensive validation
    // - Hash-based de-duplication
    // - Conflict resolution
    // - Proper integration with the schema system

    /// Checks if a schema exists
    pub fn schema_exists(&self, schema_name: &str) -> Result<bool, SchemaError> {
        self.exists_in_tree(&self.schemas_tree, schema_name)
    }

    /// Checks if a schema state exists
    pub fn schema_state_exists(&self, schema_name: &str) -> Result<bool, SchemaError> {
        self.exists_in_tree(&self.schema_states_tree, schema_name)
    }

    /// Gets all schema states as a HashMap
    pub fn get_all_schema_states(
        &self,
    ) -> Result<std::collections::HashMap<String, SchemaState>, SchemaError> {
        let items: Vec<(String, SchemaState)> =
            self.list_items_in_tree(&self.schema_states_tree)?;
        Ok(items.into_iter().collect())
    }

    /// Gets all schemas as a HashMap
    /// Populates runtime_fields for all schemas
    pub fn get_all_schemas(&self) -> Result<std::collections::HashMap<String, Schema>, SchemaError> {
        let items: Vec<(String, Schema)> = self.list_items_in_tree(&self.schemas_tree)?;
        let mut schemas = std::collections::HashMap::new();
        
        for (name, mut schema) in items {
            schema.populate_runtime_fields()?;
            schemas.insert(name, schema);
        }
        
        Ok(schemas)
    }
}
