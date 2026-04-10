//! Consolidated testing utilities for database setup and common test patterns
//!
//! This module eliminates duplicate database setup code found across 11+ files

use crate::db_operations::DbOperations;
use crate::fold_db_core::infrastructure::message_bus::AsyncMessageBus;
use crate::storage::SledPool;
use std::sync::Arc;

/// Consolidated temporary database creation - eliminates 11+ duplicates
pub struct TestDatabaseFactory;

impl TestDatabaseFactory {
    /// Create a temporary SledPool for testing.
    /// Note: the temp directory is intentionally leaked so the path remains valid.
    #[allow(deprecated)]
    pub fn create_temp_sled_pool() -> Arc<SledPool> {
        let dir = tempfile::TempDir::new().expect("Failed to create temp dir");
        Arc::new(SledPool::new(dir.into_path()))
    }

    /// Create temporary DbOperations for testing - consolidates pattern from multiple files
    pub async fn create_temp_db_ops() -> Result<DbOperations, Box<dyn std::error::Error>> {
        let pool = Self::create_temp_sled_pool();
        Ok(DbOperations::from_sled(pool).await?)
    }

    /// Create complete test environment with db_ops and message bus
    pub async fn create_test_environment(
    ) -> Result<(Arc<DbOperations>, Arc<AsyncMessageBus>), Box<dyn std::error::Error>> {
        let db_ops = Arc::new(Self::create_temp_db_ops().await?);
        let message_bus = Arc::new(AsyncMessageBus::new());
        Ok((db_ops, message_bus))
    }
}
