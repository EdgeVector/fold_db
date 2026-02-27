use crate::error::FoldDbResult;
use crate::schema::{SchemaState, SchemaWithState};

use super::OperationProcessor;

impl OperationProcessor {
    /// List all schemas with their states.
    pub async fn list_schemas(&self) -> FoldDbResult<Vec<SchemaWithState>> {
        let db = self
            .node
            .get_fold_db()
            .await
            ?;

        Ok(db.schema_manager
            .get_schemas_with_states()?)
    }

    /// Get a specific schema by name with its state.
    pub async fn get_schema(&self, name: &str) -> FoldDbResult<Option<SchemaWithState>> {
        let db = self
            .node
            .get_fold_db()
            .await
            ?;

        let mgr = &db.schema_manager;
        match mgr
            .get_schema_metadata(name)
            ?
        {
            Some(schema) => {
                let states = mgr
                    .get_schema_states()
                    ?;
                let state = states.get(name).copied().unwrap_or_default();
                Ok(Some(SchemaWithState::new(schema, state)))
            }
            None => Ok(None),
        }
    }

    /// Approve a schema.
    pub async fn approve_schema(&self, schema_name: &str) -> FoldDbResult<Option<String>> {
        let db = self
            .node
            .get_fold_db()
            .await
            ?;

        let schema_mgr = &db.schema_manager;
        let transform_mgr = &db.transform_manager;

        // Check if schema is already approved
        let states = schema_mgr
            .get_schema_states()
            ?;
        let current_state = states.get(schema_name).copied().unwrap_or_default();

        if current_state == SchemaState::Approved {
            return Ok(None);
        }

        let is_transform = transform_mgr.transform_exists(schema_name).unwrap_or(false);

        let backfill_hash = if is_transform {
            Self::generate_backfill_hash_for_transform(transform_mgr, schema_name).await
        } else {
            None
        };

        schema_mgr
            .set_schema_state_with_backfill(
                schema_name,
                SchemaState::Approved,
                backfill_hash.clone(),
            )
            .await
            ?;

        Ok(backfill_hash)
    }

    /// Block a schema.
    pub async fn block_schema(&self, schema_name: &str) -> FoldDbResult<()> {
        let db = self
            .node
            .get_fold_db()
            .await
            ?;

        Ok(db.schema_manager
            .block_schema(schema_name)
            .await?)
    }

    /// Load schemas from the schema service (Standard fetch & load logic).
    /// Returns (available_count, loaded_count, failed_schemas).
    pub async fn load_schemas(&self) -> FoldDbResult<(usize, usize, Vec<String>)> {
        let schemas = {
            self.node
                .fetch_available_schemas()
                .await
                ?
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
                    .await
                    ?;

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

    // Helper for approve_schema
    async fn generate_backfill_hash_for_transform(
        transform_manager: &crate::transform::manager::TransformManager,
        schema_name: &str,
    ) -> Option<String> {
        let transforms = match transform_manager.list_transforms() {
            Ok(t) => t,
            Err(e) => {
                log::warn!("Failed to list transforms for {}: {}", schema_name, e);
                return None;
            }
        };

        let transform = match transforms.get(schema_name) {
            Some(t) => t,
            None => {
                log::debug!("Transform {} not found in transform list", schema_name);
                return None;
            }
        };

        // Look up the transform's schema from the database
        let declarative_schema = match transform_manager
            .db_ops
            .get_schema(transform.get_schema_name())
            .await
        {
            Ok(Some(s)) => s,
            Ok(None) => {
                log::warn!("Transform {} schema not found in database", schema_name);
                return None;
            }
            Err(e) => {
                log::warn!("Failed to get schema for transform {}: {}", schema_name, e);
                return None;
            }
        };

        let inputs = declarative_schema.get_inputs();
        let first_input = match inputs.first() {
            Some(i) => i,
            None => {
                log::warn!(
                    "Transform {} has no inputs in declarative schema",
                    schema_name
                );
                return None;
            }
        };

        let source_schema_name = match first_input.split('.').next() {
            Some(s) => s,
            None => {
                log::warn!("Failed to parse source schema from input: {}", first_input);
                return None;
            }
        };

        Some(
            crate::fold_db_core::infrastructure::backfill_tracker::BackfillTracker::generate_hash(
                schema_name,
                source_schema_name,
            ),
        )
    }
}
