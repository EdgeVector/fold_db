use super::core::DbOperations;
use crate::schema::{Schema, SchemaError, SchemaState};
use std::collections::HashMap;

impl DbOperations {
    /// Get a specific schema by name
    pub async fn get_schema(&self, schema_name: &str) -> Result<Option<Schema>, SchemaError> {
        use crate::storage::traits::TypedStore;

        let mut schema_opt: Option<Schema> = self.schemas_store().get_item(schema_name).await?;

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

        Ok(self.schema_states_store().get_item(schema_name).await?)
    }

    /// Store a schema.
    ///
    /// For org-scoped schemas, also writes the schema under an org-prefixed key
    /// in the "schemas" namespace. The sync partitioner routes org-prefixed keys
    /// to the org R2 prefix, so other org members receive the schema (including
    /// `field_molecule_uuids`) during normal org sync — no special post-sync step.
    pub async fn store_schema(
        &self,
        schema_name: &str,
        schema: &Schema,
    ) -> Result<(), SchemaError> {
        use crate::storage::traits::TypedStore;

        // Local lookup key (always)
        self.schemas_store().put_item(schema_name, schema).await?;

        // Org-prefixed key for sync routing (partitioner sees org prefix → org R2)
        if let Some(org_hash) = &schema.org_hash {
            let org_key = format!("{}:{}", org_hash, schema_name);
            self.schemas_store().put_item(&org_key, schema).await?;
        }

        self.schemas_store().inner().flush().await?;
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
            .await?;
        self.schema_states_store().inner().flush().await?;
        Ok(())
    }

    /// Get all schemas
    pub async fn get_all_schemas(&self) -> Result<HashMap<String, Schema>, SchemaError> {
        use crate::storage::traits::TypedStore;

        let keys = self.schemas_store().list_keys_with_prefix("").await?;

        let mut schemas = HashMap::new();
        for key in keys {
            if let Some(mut schema) = self.schemas_store().get_item::<Schema>(&key).await? {
                schema.populate_runtime_fields()?;
                schemas.insert(key, schema);
            }
        }

        Ok(schemas)
    }

    /// Store a schema superseded-by mapping (old_name → new_name)
    pub async fn store_superseded_by(
        &self,
        old_name: &str,
        new_name: &str,
    ) -> Result<(), SchemaError> {
        use crate::storage::traits::TypedStore;

        self.superseded_by_store()
            .put_item(old_name, &new_name.to_string())
            .await?;
        self.superseded_by_store().inner().flush().await?;
        Ok(())
    }

    /// Get all superseded-by mappings
    pub async fn get_all_superseded_by(&self) -> Result<HashMap<String, String>, SchemaError> {
        use crate::storage::traits::TypedStore;

        let keys = self.superseded_by_store().list_keys_with_prefix("").await?;

        let mut mappings = HashMap::new();
        for key in keys {
            if let Some(new_name) = self.superseded_by_store().get_item::<String>(&key).await? {
                mappings.insert(key, new_name);
            }
        }

        Ok(mappings)
    }

    /// Get all schema states
    pub async fn get_all_schema_states(&self) -> Result<HashMap<String, SchemaState>, SchemaError> {
        use crate::storage::traits::TypedStore;

        let keys = self.schema_states_store().list_keys_with_prefix("").await?;

        let mut states = HashMap::new();
        for key in keys {
            if let Some(state) = self
                .schema_states_store()
                .get_item::<SchemaState>(&key)
                .await?
            {
                states.insert(key, state);
            }
        }

        Ok(states)
    }
}
