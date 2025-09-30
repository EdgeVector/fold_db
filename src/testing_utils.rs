//! Consolidated testing utilities for database setup and common test patterns
//!
//! This module eliminates duplicate database setup code found across 11+ files

use crate::db_operations::DbOperations;
use crate::fold_db_core::infrastructure::message_bus::MessageBus;
use sled::{Db, Tree};
use std::sync::Arc;
use std::collections::HashMap;

/// Consolidated temporary database creation - eliminates 11+ duplicates
pub struct TestDatabaseFactory;

impl TestDatabaseFactory {
    /// Create a temporary sled database for testing
    pub fn create_temp_sled_db() -> Result<Db, sled::Error> {
        sled::Config::new().temporary(true).open()
    }

    /// Create temporary DbOperations for testing - consolidates pattern from multiple files
    pub fn create_temp_db_ops() -> Result<DbOperations, Box<dyn std::error::Error>> {
        let db = Self::create_temp_sled_db()?;
        Ok(DbOperations::new(db)?)
    }

    /// Create temporary tree for testing - consolidates pattern from orchestration files  
    pub fn create_temp_tree() -> Result<Tree, sled::Error> {
        let db = Self::create_temp_sled_db()?;
        db.open_tree("test_tree")
    }

    /// Create complete test environment with db_ops and message bus
    pub fn create_test_environment(
    ) -> Result<(Arc<DbOperations>, Arc<MessageBus>), Box<dyn std::error::Error>> {
        let db_ops = Arc::new(Self::create_temp_db_ops()?);
        let message_bus = Arc::new(MessageBus::new());
        Ok((db_ops, message_bus))
    }

    /// Create test schema (consolidates duplicate create_test_schema functions)
    pub fn create_test_schema(name: &str) -> crate::schema::types::Schema {
        crate::schema::types::Schema::new(name.to_string(), None, HashMap::new(), None)
    }

    /// Create test node config (consolidates create_test_config functions)
    pub fn create_test_node_config() -> crate::datafold_node::config::NodeConfig {
        let dir = tempfile::tempdir().unwrap();
        crate::datafold_node::config::NodeConfig {
            storage_path: dir.path().to_path_buf(),
            default_trust_distance: 1,
            network_listen_address: "/ip4/127.0.0.1/tcp/0".to_string(),
            security_config: crate::security::SecurityConfig::default(),
        }
    }

    /// Create named test tree (consolidates multiple create_test_tree functions)
    pub fn create_named_test_tree(tree_name: &str) -> Tree {
        let db = Self::create_temp_sled_db().expect("Failed to create test database");
        db.open_tree(tree_name).expect("Failed to create test tree")
    }
}
