
use std::collections::HashMap;

use crate::error::{FoldDbError, FoldDbResult};
use crate::schema::types::SchemaError;
use crate::schema::types::{Mutation, Query, Transform};
use crate::schema::types::key_value::KeyValue;
use crate::schema::types::field::FieldValue;
use super::DataFoldNode;

impl DataFoldNode {
    /// Helper function to execute database operations with proper error handling
    fn with_db<F, R>(&self, f: F, lock_error_msg: &str, db_error_msg: &str) -> FoldDbResult<R>
    where
        F: FnOnce(&crate::fold_db_core::FoldDB) -> Result<R, SchemaError>,
    {
        let db = self
            .db
            .lock()
            .map_err(|_| FoldDbError::Config(lock_error_msg.to_string()))?;
        f(&db).map_err(|e| FoldDbError::Config(format!("{}: {}", db_error_msg, e)))
    }

    /// Helper function to execute mutable database operations with proper error handling
    fn with_db_mut<F, R>(&self, f: F, lock_error_msg: &str, db_error_msg: &str) -> FoldDbResult<R>
    where
        F: FnOnce(&mut crate::fold_db_core::FoldDB) -> Result<R, SchemaError>,
    {
        let mut db = self
            .db
            .lock()
            .map_err(|_| FoldDbError::Config(lock_error_msg.to_string()))?;
        f(&mut db).map_err(|e| FoldDbError::Config(format!("{}: {}", db_error_msg, e)))
    }

    /// Executes a query against the database.
    pub fn query(&self, query: Query) -> FoldDbResult<HashMap<String, HashMap<KeyValue, FieldValue>>> {
        self.with_db(
            |db| db.query_executor.query(query),
            "Failed to acquire database lock for query",
            "Query operation failed"
        )
    }

    /// Executes a mutation on the database.
    pub fn mutate(&self, mutation: Mutation) -> FoldDbResult<String> {
        self.with_db_mut(
            |db| db.mutation_manager.write_mutation(mutation),
            "Failed to acquire database lock for mutation",
            "Mutation operation failed"
        )
    }

    /// Executes multiple mutations in a batch for improved performance.
    pub fn mutate_batch(&self, mutations: Vec<Mutation>) -> FoldDbResult<Vec<String>> {
        self.with_db_mut(
            |db| db.mutation_manager.write_mutations_batch(mutations),
            "Failed to acquire database lock for batch mutation",
            "Batch mutation operation failed"
        )
    }

    /// List all registered transforms.
    pub fn list_transforms(&self) -> FoldDbResult<HashMap<String, Transform>> {
        self.with_db(
            |db| db.transform_manager.list_transforms(),
            "Failed to acquire database lock for listing transforms",
            "Failed to list transforms"
        )
    }
}
