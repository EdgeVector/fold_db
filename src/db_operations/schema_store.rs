//! Schema domain store.
//!
//! Owns the storage namespaces for schemas, schema states, and
//! schema supersede-by mappings. External callers access schema
//! operations through this type via `DbOperations::schemas()`.

use crate::schema::{Schema, SchemaError, SchemaState};
use crate::storage::traits::{KvStore, TypedStore};
use crate::storage::TypedKvStore;
use std::collections::HashMap;
use std::sync::Arc;

/// Domain store for schema-related persistence.
#[derive(Clone)]
pub struct SchemaStore {
    schemas_store: Arc<TypedKvStore<dyn KvStore>>,
    schema_states_store: Arc<TypedKvStore<dyn KvStore>>,
    superseded_by_store: Arc<TypedKvStore<dyn KvStore>>,
}

impl SchemaStore {
    pub(crate) fn new(
        schemas_store: Arc<TypedKvStore<dyn KvStore>>,
        schema_states_store: Arc<TypedKvStore<dyn KvStore>>,
        superseded_by_store: Arc<TypedKvStore<dyn KvStore>>,
    ) -> Self {
        Self {
            schemas_store,
            schema_states_store,
            superseded_by_store,
        }
    }

    /// Access the raw schemas namespace. Restricted to other modules inside
    /// `fold_db` that need generic typed access (e.g. org purge).
    pub(crate) fn raw_schemas(&self) -> &Arc<TypedKvStore<dyn KvStore>> {
        &self.schemas_store
    }

    /// Access the raw schema-states namespace (crate-internal).
    pub(crate) fn raw_schema_states(&self) -> &Arc<TypedKvStore<dyn KvStore>> {
        &self.schema_states_store
    }

    /// Access the raw superseded-by namespace (crate-internal).
    pub(crate) fn raw_superseded_by(&self) -> &Arc<TypedKvStore<dyn KvStore>> {
        &self.superseded_by_store
    }

    /// Get a specific schema by name
    pub async fn get_schema(&self, schema_name: &str) -> Result<Option<Schema>, SchemaError> {
        let mut schema_opt: Option<Schema> = self.schemas_store.get_item(schema_name).await?;

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
        Ok(self.schema_states_store.get_item(schema_name).await?)
    }

    /// Store a schema.
    pub async fn store_schema(
        &self,
        schema_name: &str,
        schema: &Schema,
    ) -> Result<(), SchemaError> {
        self.schemas_store.put_item(schema_name, schema).await?;
        self.schemas_store.inner().flush().await?;
        Ok(())
    }

    /// Store schema state
    pub async fn store_schema_state(
        &self,
        schema_name: &str,
        state: &SchemaState,
    ) -> Result<(), SchemaError> {
        self.schema_states_store
            .put_item(schema_name, state)
            .await?;
        self.schema_states_store.inner().flush().await?;
        Ok(())
    }

    /// Get all schemas
    pub async fn get_all_schemas(&self) -> Result<HashMap<String, Schema>, SchemaError> {
        let items: Vec<(String, Schema)> = self.schemas_store.scan_items_with_prefix("").await?;

        let mut schemas = HashMap::with_capacity(items.len());
        for (key, mut schema) in items {
            schema.populate_runtime_fields()?;
            schemas.insert(key, schema);
        }

        Ok(schemas)
    }

    /// Store a schema superseded-by mapping (old_name → new_name)
    pub async fn store_superseded_by(
        &self,
        old_name: &str,
        new_name: &str,
    ) -> Result<(), SchemaError> {
        self.superseded_by_store
            .put_item(old_name, &new_name.to_string())
            .await?;
        self.superseded_by_store.inner().flush().await?;
        Ok(())
    }

    /// Get all superseded-by mappings
    pub async fn get_all_superseded_by(&self) -> Result<HashMap<String, String>, SchemaError> {
        let items: Vec<(String, String)> =
            self.superseded_by_store.scan_items_with_prefix("").await?;
        Ok(items.into_iter().collect())
    }

    /// Get all schema states
    pub async fn get_all_schema_states(&self) -> Result<HashMap<String, SchemaState>, SchemaError> {
        let items: Vec<(String, SchemaState)> =
            self.schema_states_store.scan_items_with_prefix("").await?;
        Ok(items.into_iter().collect())
    }

    /// Delete a schema entry (name only) from the schemas namespace.
    pub async fn delete_schema(&self, schema_name: &str) -> Result<(), SchemaError> {
        self.schemas_store.delete_item(schema_name).await?;
        Ok(())
    }

    /// Delete a schema state entry.
    pub async fn delete_schema_state(&self, schema_name: &str) -> Result<(), SchemaError> {
        self.schema_states_store.delete_item(schema_name).await?;
        Ok(())
    }
}
