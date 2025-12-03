
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
    pub async fn query(&self, query: Query) -> FoldDbResult<HashMap<String, HashMap<KeyValue, FieldValue>>> {
        let db = self.db.lock()
            .map_err(|_| FoldDbError::Config("Failed to acquire database lock for query".to_string()))?;
        db.query_executor.query(query).await
            .map_err(|e| FoldDbError::Config(format!("Query operation failed: {}", e)))
    }

    /// Executes a mutation on the database.
    /// 
    /// # Deprecated
    /// Use `mutate_batch()` instead for better performance, even for single mutations.
    #[deprecated(since = "0.1.0", note = "Use mutate_batch() instead for better performance")]
    pub fn mutate(&self, mutation: Mutation) -> FoldDbResult<String> {
        self.with_db_mut(
            #[allow(deprecated)]
            |db| db.mutation_manager.write_mutation(mutation),
            "Failed to acquire database lock for mutation",
            "Mutation operation failed"
        )
    }

    /// Executes multiple mutations in a batch for improved performance.
    pub fn mutate_batch(&self, mutations: Vec<Mutation>) -> FoldDbResult<Vec<String>> {
        // Use async version to avoid deadlocks, blocking on the current thread
        // Since DataFoldNode uses std::sync::Mutex, we need to use spawn_blocking
        // to avoid blocking the async runtime if we are in one.
        
        let db = self.db.clone();
        
        // Check if we are in a tokio runtime
        if let Ok(handle) = tokio::runtime::Handle::try_current() {
            // We are in a runtime, use spawn_blocking to bridge sync -> async
            tokio::task::block_in_place(|| {
                handle.block_on(async {
                    let mut db_guard = db.lock()
                        .map_err(|_| FoldDbError::Config("Failed to acquire database lock for batch mutation".to_string()))?;
                    
                    db_guard.mutation_manager.write_mutations_batch_async(mutations).await
                        .map_err(|e| FoldDbError::Config(format!("Batch mutation operation failed: {}", e)))
                })
            })
        } else {
            // Not in a runtime, create a temporary one (fallback)
            let rt = tokio::runtime::Runtime::new()
                .map_err(|e| FoldDbError::Config(format!("Failed to create tokio runtime: {}", e)))?;
                
            rt.block_on(async {
                let mut db_guard = db.lock()
                    .map_err(|_| FoldDbError::Config("Failed to acquire database lock for batch mutation".to_string()))?;
                
                db_guard.mutation_manager.write_mutations_batch_async(mutations).await
                    .map_err(|e| FoldDbError::Config(format!("Batch mutation operation failed: {}", e)))
            })
        }
    }
    
    /// Executes multiple mutations in a batch (async version - preferred for DynamoDB)
    pub async fn mutate_batch_async(&self, mutations: Vec<Mutation>) -> FoldDbResult<Vec<String>> {
        // Since DataFoldNode uses std::sync::Mutex, we need to use spawn_blocking
        // to avoid blocking the async runtime. The entire operation runs in a blocking context.
        let db = self.db.clone(); // Clone Arc, not the Mutex
        
        tokio::task::spawn_blocking(move || {
            let mut db_guard = db.lock()
                .map_err(|_| FoldDbError::Config("Failed to acquire database lock for batch mutation".to_string()))?;
            
            // Get the runtime handle to run async code
            let handle = tokio::runtime::Handle::try_current()
                .map_err(|_| FoldDbError::Config("No tokio runtime available".to_string()))?;
            
            // Run the async mutation operation
            handle.block_on(
                db_guard.mutation_manager.write_mutations_batch_async(mutations)
            )
            .map_err(|e| FoldDbError::Config(format!("Batch mutation operation failed: {}", e)))
        })
        .await
        .map_err(|e| FoldDbError::Config(format!("Failed to execute mutation in blocking context: {}", e)))?
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
