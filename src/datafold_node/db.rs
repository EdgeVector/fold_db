use std::collections::HashMap;

use super::DataFoldNode;
use crate::error::{FoldDbError, FoldDbResult};
use crate::schema::types::field::FieldValue;
use crate::schema::types::key_value::KeyValue;
use crate::schema::types::SchemaError;
use crate::schema::types::{Mutation, Query, Transform};

impl DataFoldNode {
    /// Helper function to execute database operations with proper error handling
    async fn with_db<F, R>(
        &self,
        f: F,
        _lock_error_msg: &str,
        db_error_msg: &str,
    ) -> FoldDbResult<R>
    where
        F: FnOnce(&crate::fold_db_core::FoldDB) -> Result<R, SchemaError>,
    {
        let db = self.db.lock().await;
        f(&db).map_err(|e| FoldDbError::Config(format!("{}: {}", db_error_msg, e)))
    }

    /// Helper function to execute mutable database operations with proper error handling
    async fn with_db_mut<F, R>(
        &self,
        f: F,
        _lock_error_msg: &str,
        db_error_msg: &str,
    ) -> FoldDbResult<R>
    where
        F: FnOnce(&mut crate::fold_db_core::FoldDB) -> Result<R, SchemaError>,
    {
        let mut db = self.db.lock().await;
        f(&mut db).map_err(|e| FoldDbError::Config(format!("{}: {}", db_error_msg, e)))
    }

    /// Executes a query against the database.
    pub async fn query(
        &self,
        query: Query,
    ) -> FoldDbResult<HashMap<String, HashMap<KeyValue, FieldValue>>> {
        let db = self.db.lock().await;
        db.query_executor
            .query(query)
            .await
            .map_err(|e| FoldDbError::Config(format!("Query failed: {}", e)))
    }

    /// Executes a mutation on the database.
    ///
    /// # Deprecated
    /// Use `mutate_batch()` instead for better performance, even for single mutations.
    #[deprecated(
        since = "0.1.0",
        note = "Use mutate_batch() instead for better performance"
    )]
    pub async fn mutate(&self, mutation: Mutation) -> FoldDbResult<String> {
        let mut db = self.db.lock().await;
        #[allow(deprecated)]
        db.mutation_manager
            .write_mutation(mutation)
            .map_err(|e| FoldDbError::Config(format!("Mutation operation failed: {}", e)))
    }

    /// Executes multiple mutations in a batch for improved performance.
    pub async fn mutate_batch(&self, mutations: Vec<Mutation>) -> FoldDbResult<Vec<String>> {
        let mut db = self.db.lock().await;
        db.mutation_manager
            .write_mutations_batch_async(mutations)
            .await
            .map_err(|e| FoldDbError::Config(format!("Batch mutation failed: {}", e)))
    }

    /// Executes multiple mutations in a batch (async version - preferred for DynamoDB)
    pub async fn mutate_batch_async(&self, mutations: Vec<Mutation>) -> FoldDbResult<Vec<String>> {
        let mut db = self.db.lock().await;
        db.mutation_manager
            .write_mutations_batch_async(mutations)
            .await
            .map_err(|e| FoldDbError::Config(format!("Async batch mutation failed: {}", e)))
    }

    /// List all registered transforms.
    pub async fn list_transforms(&self) -> FoldDbResult<HashMap<String, Transform>> {
        self.with_db(
            |db| db.transform_manager.list_transforms(),
            "Failed to acquire database lock for listing transforms",
            "Failed to list transforms",
        )
        .await
    }
}
