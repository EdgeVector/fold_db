use crate::schema::types::SchemaError;
use std::sync::{RwLock, RwLockReadGuard, RwLockWriteGuard};

use super::TransformUtils;

impl TransformUtils {
    /// Acquire a read lock with consistent error handling
    pub fn read_lock<'a, T>(
        lock: &'a RwLock<T>,
        lock_name: &str,
    ) -> Result<RwLockReadGuard<'a, T>, SchemaError> {
        lock.read().map_err(|_| {
            SchemaError::InvalidData(format!("Failed to acquire {} read lock", lock_name))
        })
    }

    /// Acquire a write lock with consistent error handling
    pub fn write_lock<'a, T>(
        lock: &'a RwLock<T>,
        lock_name: &str,
    ) -> Result<RwLockWriteGuard<'a, T>, SchemaError> {
        lock.write().map_err(|_| {
            SchemaError::InvalidData(format!("Failed to acquire {} write lock", lock_name))
        })
    }
}
