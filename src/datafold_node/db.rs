use serde_json::Value;
use std::collections::HashMap;

use crate::error::{FoldDbError, FoldDbResult};
use crate::schema::types::{Mutation, Operation, Query, Transform};
use crate::schema::SchemaError;
use crate::log_feature;
use crate::logging::features::LogFeature;

use super::DataFoldNode;

impl DataFoldNode {
    /// Executes an operation (query or mutation) on the database.
    pub fn execute_operation(&mut self, operation: Operation) -> FoldDbResult<Value> {
        match operation {
            Operation::Query {
                schema,
                fields,
                filter,
            } => {
                let query = Query {
                    schema_name: schema,
                    fields,
                    pub_key: String::new(),
                    trust_distance: 0,
                    filter,
                };
                let results = self.query(query)?;
                let unwrapped: Vec<Value> = results
                    .into_iter()
                    .map(|r| r.unwrap_or_else(|e| serde_json::json!({"error": e.to_string()})))
                    .collect();
                Ok(serde_json::to_value(&unwrapped)?)
            }
            Operation::Mutation {
                schema,
                data,
                mutation_type,
            } => {
                let fields_and_values = match data {
                    Value::Object(map) => map.into_iter().collect(),
                    _ => {
                        return Err(FoldDbError::Config(
                            "Mutation data must be an object".into(),
                        ))
                    }
                };

                let mutation = Mutation {
                    schema_name: schema,
                    fields_and_values,
                    pub_key: String::new(),
                    trust_distance: 0,
                    mutation_type,
                    synchronous: None,
                };
                self.mutate(mutation)?;
                Ok(Value::Null)
            }
        }
    }

    /// Executes a query against the database.
    pub fn query(&mut self, query: Query) -> FoldDbResult<Vec<Result<Value, SchemaError>>> {
        println!("🔍 DEBUG: datafold_node::db::query called for schema: {}", query.schema_name);
        // Check if schema exists first
        let schema_exists = {
            let db = self
                .db
                .lock()
                .map_err(|_| FoldDbError::Config("Cannot lock database mutex".into()))?;
            db.schema_manager
                .schema_exists(&query.schema_name)
                .unwrap_or(false)
        };

        if !schema_exists {
            return Err(FoldDbError::Config(format!(
                "Schema '{}' does not exist. Please create the schema first.",
                query.schema_name
            )));
        }

        // Check if schema is approved for queries
        let can_query = {
            let db = self
                .db
                .lock()
                .map_err(|_| FoldDbError::Config("Cannot lock database mutex".into()))?;
            db.schema_manager.can_query_schema(&query.schema_name)
        };

        if !can_query {
            return Err(FoldDbError::Config(format!(
                "Schema '{}' exists but is not approved for queries. Please approve the schema first using POST /api/schema/{}/approve",
                query.schema_name, query.schema_name
            )));
        }
        if !self.check_schema_permission(&query.schema_name)? {
            let current_perms = self.log_permission_denied(&query.schema_name, "query")?;
            return Err(FoldDbError::Config(format!(
                "Permission denied for schema '{}'. Node '{}' does not have access. Current permissions: {:?}",
                query.schema_name, self.node_id, current_perms
            )));
        }
        // Note: trust_distance 0 is a valid value meaning "maximum trust" for web UI access
        // Do not override explicitly set trust_distance values
        let db = self
            .db
            .lock()
            .map_err(|_| FoldDbError::Config("Cannot lock database mutex".into()))?;
        Ok(db.query_schema(query))
    }

    /// Executes a mutation on the database.
    pub fn mutate(&mut self, mutation: Mutation) -> FoldDbResult<()> {
        // Check if schema exists first
        let schema_exists = {
            let db = self
                .db
                .lock()
                .map_err(|_| FoldDbError::Config("Cannot lock database mutex".into()))?;
            db.schema_manager
                .schema_exists(&mutation.schema_name)
                .unwrap_or(false)
        };

        if !schema_exists {
            return Err(FoldDbError::Config(format!(
                "Schema '{}' does not exist. Please create the schema first.",
                mutation.schema_name
            )));
        }

        // Check if schema is approved for mutations
        let can_mutate = {
            let db = self
                .db
                .lock()
                .map_err(|_| FoldDbError::Config("Cannot lock database mutex".into()))?;
            db.schema_manager.can_mutate_schema(&mutation.schema_name)
        };

        if !can_mutate {
            return Err(FoldDbError::Config(format!(
                "Schema '{}' exists but is not approved for mutations. Please approve the schema first using POST /api/schema/{}/approve",
                mutation.schema_name, mutation.schema_name
            )));
        }
        if !self.check_schema_permission(&mutation.schema_name)? {
            let current_perms = self.log_permission_denied(&mutation.schema_name, "mutation")?;
            return Err(FoldDbError::Config(format!(
                "Permission denied for schema '{}'. Node '{}' does not have access. Current permissions: {:?}",
                mutation.schema_name, self.node_id, current_perms
            )));
        }
        let mut db = self
            .db
            .lock()
            .map_err(|_| FoldDbError::Config("Cannot lock database mutex".into()))?;
        db.write_schema(mutation)?;
        Ok(())
    }

    /// Retrieves the version history for a specific atom reference.
    pub fn get_history(&self, molecule_uuid: &str) -> FoldDbResult<Vec<Value>> {
        let db = self
            .db
            .lock()
            .map_err(|_| FoldDbError::Config("Cannot lock database mutex".into()))?;
        let history = db
            .atom_manager
            .get_atom_history(molecule_uuid)
            .map_err(|e| FoldDbError::Database(e.to_string()))?;
        Ok(history.into_iter().map(|a| a.content().clone()).collect())
    }

    /// List all registered transforms.
    pub fn list_transforms(&self) -> FoldDbResult<HashMap<String, Transform>> {
        let db = self
            .db
            .lock()
            .map_err(|_| FoldDbError::Config("Cannot lock database mutex".into()))?;
        Ok(db.list_transforms()?)
    }

    /// Execute a transform by id and return the result.
    pub fn run_transform(&mut self, transform_id: &str) -> FoldDbResult<Value> {
        let db = self
            .db
            .lock()
            .map_err(|_| FoldDbError::Config("Cannot lock database mutex".into()))?;
        Ok(db.run_transform(transform_id)?)
    }

    /// Process all queued transforms.
    pub fn process_transform_queue(&self) -> FoldDbResult<()> {
        let db = self
            .db
            .lock()
            .map_err(|_| FoldDbError::Config("Cannot lock database mutex".into()))?;
        db.process_transform_queue();
        Ok(())
    }

    /// Reload transforms from the database.
    pub fn reload_transforms(&self) -> FoldDbResult<()> {
        let db = self
            .db
            .lock()
            .map_err(|_| FoldDbError::Config("Cannot lock database mutex".into()))?;
        db.reload_transforms()?;
        Ok(())
    }

    /// Helper method to log and create permission denied errors
    fn log_permission_denied(
        &self,
        schema_name: &str,
        operation_type: &str,
    ) -> FoldDbResult<Vec<String>> {
        let node_id = &self.node_id;
        let current_perms = {
            let db = self.db.lock().map_err(|_| {
                FoldDbError::Config("Cannot lock database mutex for permission details".into())
            })?;
            db.get_schema_permissions(node_id)
        };

        log_feature!(
            LogFeature::Permissions,
            error,
            "Permission denied for {} on schema '{}': Node '{}' permissions: {:?}",
            operation_type,
            schema_name,
            node_id,
            current_perms
        );

        Ok(current_perms)
    }
}
