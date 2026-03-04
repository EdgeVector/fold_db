use crate::error::FoldDbResult;
use crate::schema::SchemaWithState;

use super::OperationProcessor;

impl OperationProcessor {
    /// List all schemas with their states.
    pub async fn list_schemas(&self) -> FoldDbResult<Vec<SchemaWithState>> {
        let db = self
            .node
            .get_fold_db()
            .await?;

        Ok(db.schema_manager
            .get_schemas_with_states()?)
    }

    /// Get a specific schema by name with its state.
    pub async fn get_schema(&self, name: &str) -> FoldDbResult<Option<SchemaWithState>> {
        let db = self
            .node
            .get_fold_db()
            .await?;

        let mgr = &db.schema_manager;
        match mgr.get_schema_metadata(name)? {
            Some(schema) => {
                let states = mgr.get_schema_states()?;
                let state = states.get(name).copied().unwrap_or_default();
                Ok(Some(SchemaWithState::new(schema, state)))
            }
            None => Ok(None),
        }
    }

    /// Approve a schema.
    pub async fn approve_schema(&self, schema_name: &str) -> FoldDbResult<()> {
        let db = self
            .node
            .get_fold_db()
            .await?;

        Ok(db.schema_manager
            .approve(schema_name)
            .await?)
    }

    /// Block a schema.
    pub async fn block_schema(&self, schema_name: &str) -> FoldDbResult<()> {
        let db = self
            .node
            .get_fold_db()
            .await?;

        Ok(db.schema_manager
            .block_schema(schema_name)
            .await?)
    }

    /// Load schemas from the schema service.
    /// Returns (available_count, loaded_count, failed_schemas).
    pub async fn load_schemas(&self) -> FoldDbResult<(usize, usize, Vec<String>)> {
        let schemas = {
            self.node
                .fetch_available_schemas()
                .await?
        };

        let schema_count = schemas.len();
        let mut loaded_count = 0;
        let mut failed_schemas = Vec::new();

        for schema in schemas {
            let schema_name = schema.name.clone();
            let result = {
                let db = self
                    .node
                    .get_fold_db()
                    .await?;

                db.schema_manager
                    .load_schema_internal(schema)
                    .await
            };

            match result {
                Ok(_) => loaded_count += 1,
                Err(e) => {
                    log::error!("Failed to load schema {}: {}", schema_name, e);
                    failed_schemas.push(schema_name);
                }
            }
        }

        Ok((schema_count, loaded_count, failed_schemas))
    }
}
