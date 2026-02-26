use super::core::DbOperations;
use crate::schema::{Schema, SchemaError, SchemaState};
use std::collections::HashMap;

impl DbOperations {
    /// Get a specific schema by name
    pub async fn get_schema(&self, schema_name: &str) -> Result<Option<Schema>, SchemaError> {
        use crate::storage::traits::TypedStore;

        let mut schema_opt: Option<Schema> = self
            .schemas_store()
            .get_item(schema_name)
            .await
            .map_err(|e| SchemaError::InvalidData(format!("Failed to get schema: {}", e)))?;

        // Populate runtime_fields if schema exists
        if let Some(schema) = &mut schema_opt {
            schema.populate_runtime_fields()?;
        }

        Ok(schema_opt)
    }

    /// Get the state of a specific schema
    pub async fn get_schema_state(
        &self,
        schema_name: &str,
    ) -> Result<Option<SchemaState>, SchemaError> {
        use crate::storage::traits::TypedStore;

        self.schema_states_store()
            .get_item(schema_name)
            .await
            .map_err(|e| SchemaError::InvalidData(format!("Failed to get schema state: {}", e)))
    }

    /// Store a schema
    pub async fn store_schema(
        &self,
        schema_name: &str,
        schema: &Schema,
    ) -> Result<(), SchemaError> {
        use crate::storage::traits::TypedStore;

        log::debug!("💾 store_schema: Storing schema '{}'", schema_name);
        self.schemas_store()
            .put_item(schema_name, schema)
            .await
            .map_err(|e| {
                log::error!("❌ Failed to store schema '{}': {}", schema_name, e);
                SchemaError::InvalidData(format!("Failed to store schema: {}", e))
            })?;

        // Flush to ensure persistence
        // All backends now support async flush - the backend implementation
        // will handle whether flush is a no-op or performs actual persistence
        log::debug!("💾 Flushing schema store");
        self.schemas_store().inner().flush().await.map_err(|e| {
            log::error!("❌ Failed to flush schemas: {}", e);
            SchemaError::InvalidData(format!("Failed to flush schemas: {}", e))
        })?;

        log::debug!("✅ Schema '{}' stored successfully", schema_name);
        Ok(())
    }

    /// Store schema state
    pub async fn store_schema_state(
        &self,
        schema_name: &str,
        state: &SchemaState,
    ) -> Result<(), SchemaError> {
        use crate::storage::traits::TypedStore;

        self.schema_states_store()
            .put_item(schema_name, state)
            .await
            .map_err(|e| {
                SchemaError::InvalidData(format!("Failed to store schema state: {}", e))
            })?;

        // Flush to ensure persistence
        self.schema_states_store()
            .inner()
            .flush()
            .await
            .map_err(|e| {
                SchemaError::InvalidData(format!("Failed to flush schema states: {}", e))
            })?;

        Ok(())
    }

    /// Get all schemas
    pub async fn get_all_schemas(&self) -> Result<HashMap<String, Schema>, SchemaError> {
        use crate::storage::traits::TypedStore;

        let keys = self.schemas_store().list_keys_with_prefix("").await
            .map_err(|e| {
                SchemaError::InvalidData(format!("Failed to list schema keys: {}", e))
            })?;

        let mut schemas = HashMap::new();
        for key in keys {
            if let Some(mut schema) = self
                .schemas_store()
                .get_item::<Schema>(&key)
                .await
                .map_err(|e| {
                    SchemaError::InvalidData(format!("Failed to get schema {}: {}", key, e))
                })?
            {
                schema.populate_runtime_fields()?;
                schemas.insert(key, schema);
            }
        }

        Ok(schemas)
    }

    /// Get all schema states
    pub async fn get_all_schema_states(&self) -> Result<HashMap<String, SchemaState>, SchemaError> {
        use crate::storage::traits::TypedStore;

        let keys = self.schema_states_store().list_keys_with_prefix("").await
            .map_err(|e| {
                SchemaError::InvalidData(format!("Failed to list schema state keys: {}", e))
            })?;

        let mut states = HashMap::new();
        for key in keys {
            if let Some(state) = self
                .schema_states_store()
                .get_item::<SchemaState>(&key)
                .await
                .map_err(|e| {
                    SchemaError::InvalidData(format!("Failed to get schema state {}: {}", key, e))
                })?
            {
                states.insert(key, state);
            }
        }

        Ok(states)
    }

}
