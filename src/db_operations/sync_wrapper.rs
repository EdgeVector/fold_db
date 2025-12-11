//! Synchronous wrapper for DbOperations
//!
//! This module provides a compatibility layer to use async DbOperations
//! in synchronous contexts by using a Tokio runtime.

use crate::db_operations::DbOperations;
use crate::schema::SchemaError;
use serde::{de::DeserializeOwned, Serialize};
use std::collections::HashMap;
use std::sync::Arc;

/// Synchronous wrapper around DbOperations for backward compatibility
///
/// This wrapper allows using the new async storage abstraction in
/// existing synchronous code by using Tokio's blocking runtime.
#[derive(Clone)]
pub struct DbOperationsSync {
    inner: Arc<DbOperations>,
}

impl DbOperationsSync {
    /// Create a new synchronous wrapper from DbOperations
    pub fn new(db_ops: DbOperations) -> Self {
        Self {
            inner: Arc::new(db_ops),
        }
    }
    
    /// Create from Sled database (blocking)
    /// 
    /// Note: This must be called from within an async context or it will create
    /// a new runtime. For HTTP servers already running Tokio, use from_sled_async instead.
    pub fn from_sled(db: sled::Db) -> Result<Self, crate::storage::StorageError> {
        // Try to use existing runtime, otherwise create new one
        let db_ops = match tokio::runtime::Handle::try_current() {
            Ok(handle) => {
                // We're in an async context, spawn and block
                handle.block_on(DbOperations::from_sled(db))?
            }
            Err(_) => {
                // No runtime, create one
                let runtime = tokio::runtime::Runtime::new()
                    .expect("Failed to create Tokio runtime");
                runtime.block_on(DbOperations::from_sled(db))?
            }
        };
        
        Ok(Self {
            inner: Arc::new(db_ops),
        })
    }
    
    /// Get the inner async DbOperations (for use in async contexts)
    pub fn inner(&self) -> &Arc<DbOperations> {
        &self.inner
    }
    
    /// Helper to run async code in blocking context
    fn block_on<F: std::future::Future>(&self, future: F) -> F::Output {
        match tokio::runtime::Handle::try_current() {
            Ok(handle) => {
                // Use existing runtime
                handle.block_on(future)
            }
            Err(_) => {
                // Create temporary runtime
                let runtime = tokio::runtime::Runtime::new()
                    .expect("Failed to create Tokio runtime");
                runtime.block_on(future)
            }
        }
    }
    
    /// Store an item (blocking)
    pub fn store_item<T: Serialize + Send + Sync>(&self, key: &str, item: &T) -> Result<(), SchemaError> {
        self.block_on(self.inner.store_item(key, item))
    }
    
    /// Get an item (blocking)
    pub fn get_item<T: DeserializeOwned + Send + Sync>(&self, key: &str) -> Result<Option<T>, SchemaError> {
        self.block_on(self.inner.get_item(key))
    }
    
    /// Delete an item (blocking)
    pub fn delete_item(&self, key: &str) -> Result<bool, SchemaError> {
        self.block_on(self.inner.delete_item(key))
    }
    
    /// List items with prefix (blocking)
    pub fn list_items_with_prefix(&self, prefix: &str) -> Result<Vec<String>, SchemaError> {
        self.block_on(self.inner.list_items_with_prefix(prefix))
    }
    
    /// Store in namespace (blocking)
    pub fn store_in_namespace<T: Serialize + Send + Sync>(
        &self,
        namespace: &str,
        key: &str,
        item: &T,
    ) -> Result<(), SchemaError> {
        self.block_on(self.inner.store_in_namespace(namespace, key, item))
    }
    
    /// Get from namespace (blocking)
    pub fn get_from_namespace<T: DeserializeOwned + Send + Sync>(
        &self,
        namespace: &str,
        key: &str,
    ) -> Result<Option<T>, SchemaError> {
        self.block_on(self.inner.get_from_namespace(namespace, key))
    }
    
    /// List keys in namespace (blocking)
    pub fn list_keys_in_namespace(&self, namespace: &str) -> Result<Vec<String>, SchemaError> {
        self.block_on(self.inner.list_keys_in_namespace(namespace))
    }
    
    /// Delete from namespace (blocking)
    pub fn delete_from_namespace(&self, namespace: &str, key: &str) -> Result<bool, SchemaError> {
        self.block_on(self.inner.delete_from_namespace(namespace, key))
    }
    
    /// Batch store items (blocking)
    pub fn batch_store_items<T: Serialize + Send + Sync + Clone>(&self, items: &[(String, T)]) -> Result<(), SchemaError> {
        self.block_on(self.inner.batch_store_items(items))
    }
    
    /// Get stats (blocking)
    pub fn get_stats(&self) -> Result<HashMap<String, u64>, SchemaError> {
        self.block_on(self.inner.get_stats())
    }
}
