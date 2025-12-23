use std::collections::HashMap;

use super::DataFoldNode;
use crate::error::{FoldDbError, FoldDbResult};
use crate::schema::types::field::FieldValue;
use crate::schema::types::key_value::KeyValue;
use crate::schema::types::SchemaError;
use crate::schema::types::{Mutation, Query, Transform};

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

    /// Helper to execute an async operation blocking the current thread.
    /// This handles the complexity of bridging sync -> async, creating a runtime if needed.
    fn run_future_blocking<F, Fut, R>(&self, f: F) -> FoldDbResult<R>
    where
        F: FnOnce() -> Fut,
        Fut: std::future::Future<Output = Result<R, SchemaError>>,
    {
        // Check if we are in a tokio runtime
        let result = if let Ok(handle) = tokio::runtime::Handle::try_current() {
            // We are in a runtime, use block_in_place to allow blocking
            tokio::task::block_in_place(|| handle.block_on(f()))
        } else {
            // Not in a runtime, create a temporary one
            let rt = tokio::runtime::Runtime::new().map_err(|e| {
                FoldDbError::Config(format!("Failed to create tokio runtime: {}", e))
            })?;
            rt.block_on(f())
        };

        result.map_err(|e| FoldDbError::Config(format!("Operation failed: {}", e)))
    }

    /// Helper to execute an async operation in a blocking task.
    /// This ensures the operation runs in a context where it can safely block (e.g. holding a lock).
    async fn run_future_in_blocking_task<F, Fut, R>(&self, f: F) -> FoldDbResult<R>
    where
        F: FnOnce() -> Fut + Send + 'static,
        Fut: std::future::Future<Output = Result<R, SchemaError>>,
        R: Send + 'static,
    {
        tokio::task::spawn_blocking(move || {
            let handle = tokio::runtime::Handle::try_current()
                .map_err(|_| FoldDbError::Config("No tokio runtime available".to_string()))?;

            handle
                .block_on(f())
                .map_err(|e| FoldDbError::Config(format!("Operation failed: {}", e)))
        })
        .await
        .map_err(|e| {
            FoldDbError::Config(format!(
                "Failed to execute operation in blocking context: {}",
                e
            ))
        })?
    }

    /// Executes a query against the database.
    pub async fn query(
        &self,
        query: Query,
    ) -> FoldDbResult<HashMap<String, HashMap<KeyValue, FieldValue>>> {
        let db = self.db.clone();
        self.run_future_in_blocking_task(move || async move {
            let db_guard = db.lock().map_err(|_| {
                SchemaError::InvalidData("Failed to acquire database lock for query".to_string())
            })?;
            db_guard.query_executor.query(query).await
        })
        .await
    }

    /// Executes a mutation on the database.
    ///
    /// # Deprecated
    /// Use `mutate_batch()` instead for better performance, even for single mutations.
    #[deprecated(
        since = "0.1.0",
        note = "Use mutate_batch() instead for better performance"
    )]
    pub fn mutate(&self, mutation: Mutation) -> FoldDbResult<String> {
        self.with_db_mut(
            #[allow(deprecated)]
            |db| db.mutation_manager.write_mutation(mutation),
            "Failed to acquire database lock for mutation",
            "Mutation operation failed",
        )
    }

    /// Executes multiple mutations in a batch for improved performance.
    pub fn mutate_batch(&self, mutations: Vec<Mutation>) -> FoldDbResult<Vec<String>> {
        let db = self.db.clone();
        self.run_future_blocking(move || async move {
            let mut db_guard = db.lock().map_err(|_| {
                SchemaError::InvalidData(
                    "Failed to acquire database lock for batch mutation".to_string(),
                )
            })?;
            db_guard
                .mutation_manager
                .write_mutations_batch_async(mutations)
                .await
        })
    }

    /// Executes multiple mutations in a batch (async version - preferred for DynamoDB)
    pub async fn mutate_batch_async(&self, mutations: Vec<Mutation>) -> FoldDbResult<Vec<String>> {
        let db = self.db.clone();
        self.run_future_in_blocking_task(move || async move {
            let mut db_guard = db.lock().map_err(|_| {
                SchemaError::InvalidData(
                    "Failed to acquire database lock for batch mutation".to_string(),
                )
            })?;
            db_guard
                .mutation_manager
                .write_mutations_batch_async(mutations)
                .await
        })
        .await
    }

    /// List all registered transforms.
    pub fn list_transforms(&self) -> FoldDbResult<HashMap<String, Transform>> {
        self.with_db(
            |db| db.transform_manager.list_transforms(),
            "Failed to acquire database lock for listing transforms",
            "Failed to list transforms",
        )
    }
}
