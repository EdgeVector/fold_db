//! Consolidated testing utilities for database setup and common test patterns
//!
//! This module eliminates duplicate database setup code found across 11+ files

use crate::db_operations::DbOperations;
use crate::fold_db_core::infrastructure::message_bus::AsyncMessageBus;
use sled::{Db, Tree};
use std::sync::Arc;

/// Consolidated temporary database creation - eliminates 11+ duplicates
pub struct TestDatabaseFactory;

impl TestDatabaseFactory {
    /// Create a temporary sled database for testing
    pub fn create_temp_sled_db() -> Result<Db, sled::Error> {
        sled::Config::new().temporary(true).open()
    }

    /// Create temporary DbOperations for testing - consolidates pattern from multiple files
    pub async fn create_temp_db_ops() -> Result<DbOperations, Box<dyn std::error::Error>> {
        let db = Self::create_temp_sled_db()?;
        Ok(DbOperations::from_sled(db).await?)
    }

    /// Create complete test environment with db_ops and message bus
    pub async fn create_test_environment(
    ) -> Result<(Arc<DbOperations>, Arc<AsyncMessageBus>), Box<dyn std::error::Error>> {
        let db_ops = Arc::new(Self::create_temp_db_ops().await?);
        let message_bus = Arc::new(AsyncMessageBus::new());
        Ok((db_ops, message_bus))
    }

    /// Create named test tree (consolidates multiple create_test_tree functions)
    pub fn create_named_test_tree(tree_name: &str) -> Tree {
        let db = Self::create_temp_sled_db().expect("Failed to create test database");
        db.open_tree(tree_name).expect("Failed to create test tree")
    }
}
