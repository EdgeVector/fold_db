use crate::log_feature;
use crate::logging::features::LogFeature;
use serde::Serialize;
use std::collections::HashMap;
use std::sync::{Arc, Mutex};

use crate::datafold_node::config::NodeConfig;
use crate::datafold_node::config::NodeInfo;
use crate::error::{FoldDbError, FoldDbResult};
use crate::fold_db_core::FoldDB;
use crate::network::NetworkCore;
use crate::security::{SecurityManager, EncryptionManager, Ed25519KeyPair};

/// A node in the DataFold distributed database system.
///
/// DataFoldNode combines database storage, schema management, and networking
/// capabilities into a complete node implementation. It can operate independently
/// or as part of a network of nodes, with trust relationships defining data access.
///
/// # Features
///
/// * Schema loading and management
/// * Query and mutation execution
/// * Network communication with other nodes
/// * Permission management for schemas
/// * Request forwarding to trusted nodes
///
/// # Examples
///
/// ```rust,no_run
/// use datafold::datafold_node::{DataFoldNode, NodeConfig};
/// use datafold::schema::{Schema, types::Operation};
/// use datafold::error::FoldDbResult;
/// use std::path::PathBuf;
/// use std::collections::HashMap;
///
/// fn main() -> FoldDbResult<()> {
///     // Create a new node with default configuration
///     let config = NodeConfig::new(PathBuf::from("data"));
///     let mut node = DataFoldNode::new(config)?;
///
///     // Create and load a schema
///     let schema = Schema::new("user_profile".to_string());
///
///     // Load the schema
///     node.load_schema(schema)?;
///
///     // Execute a query
///     let operation = Operation::Query {
///         schema: "user_profile".to_string(),
///         fields: vec!["username".to_string(), "email".to_string()],
///         filter: None,
///     };
///     let result = node.execute_operation(operation)?;
///     Ok(())
/// }
/// ```
#[derive(Clone)]
pub struct DataFoldNode {
    /// The underlying database instance for data storage and operations
    pub(super) db: Arc<Mutex<FoldDB>>,
    /// Configuration settings for this node
    pub(super) config: NodeConfig,
    /// Map of trusted nodes and their trust distances
    pub(super) trusted_nodes: HashMap<String, NodeInfo>,
    /// Unique identifier for this node
    pub(super) node_id: String,
    /// Network layer for P2P communication
    pub(super) network: Option<Arc<tokio::sync::Mutex<NetworkCore>>>,
    /// Security manager for authentication and encryption
    pub(super) security_manager: Arc<SecurityManager>,
    /// The node's private key for signing operations
    pub(super) private_key: String,
    /// The node's public key for verification
    pub(super) public_key: String,
}

/// Basic status information about the network layer
#[derive(Debug, Clone, Serialize)]
pub struct NetworkStatus {
    pub node_id: String,
    pub initialized: bool,
    pub connected_nodes_count: usize,
}

impl DataFoldNode {
    /// Creates a new DataFoldNode with the specified configuration.
    pub fn new(config: NodeConfig) -> FoldDbResult<Self> {
        let db = Arc::new(Mutex::new(FoldDB::new(
            config
                .storage_path
                .to_str()
                .ok_or_else(|| FoldDbError::Config("Invalid storage path".to_string()))?,
        )?));

        // Retrieve or generate the persistent node_id from fold_db
        let node_id = {
            let guard = db
                .lock()
                .map_err(|_| FoldDbError::Config("Cannot lock database mutex".into()))?;
            guard
                .get_node_id()
                .map_err(|e| FoldDbError::Config(format!("Failed to get node_id: {}", e)))?
        };

        // Generate a new keypair for this node
        let keypair = Ed25519KeyPair::generate()
            .map_err(|e| FoldDbError::SecurityError(format!("Failed to generate keypair: {}", e)))?;
        let private_key = keypair.secret_key_base64();
        let public_key = keypair.public_key_base64();
        
        log_feature!(LogFeature::Database, info, "Generated new node keypair");

        // Initialize security manager with node configuration
        let mut security_config = config.security_config.clone();
        
        // Generate master key if encryption is enabled but no key is set
        if security_config.encrypt_at_rest && security_config.master_key.is_none() {
            security_config.master_key = Some(EncryptionManager::generate_master_key());
        }
        
        let security_manager = {
            let guard = db
                .lock()
                .map_err(|_| FoldDbError::Config("Cannot lock database mutex".into()))?;

            let db_ops = guard.db_ops.clone();

            Arc::new(
                SecurityManager::new_with_persistence(security_config, db_ops)
                    .map_err(|e| FoldDbError::SecurityError(e.to_string()))?,
            )
        };

        Ok(Self {
            db,
            config,
            trusted_nodes: HashMap::new(),
            node_id,
            network: None,
            security_manager,
            private_key,
            public_key,
        })
    }

    /// Loads an existing database node from the specified configuration.
pub async fn load(config: NodeConfig) -> FoldDbResult<Self> {
        log_feature!(LogFeature::Database, info, "Loading DataFoldNode from config");
        let node = Self::new(config)?;

        // Delegate to SchemaCore for unified schema discovery and loading
        {
            let db = node
                .db
                .lock()
                .map_err(|_| FoldDbError::Config("Cannot lock database mutex".into()))?;
            // Initialize schema system via SchemaCore
            db.schema_manager.discover_and_load_all_schemas().map_err(|e| {
                FoldDbError::Config(format!("Failed to initialize schema system: {}", e))
            })?;
        }

        log_feature!(
            LogFeature::Database,
            info,
            "DataFoldNode loaded successfully with schema system initialized"
        );
        Ok(node)
    }

    /// Get a reference to the underlying FoldDB instance
    pub fn get_fold_db(&self) -> FoldDbResult<std::sync::MutexGuard<'_, FoldDB>> {
        self.db.lock().map_err(|_| FoldDbError::Config("Cannot lock database mutex".into()))
    }

    /// Gets the unique identifier for this node.
    pub fn get_node_id(&self) -> &str {
        &self.node_id
    }

    /// Gets the node's private key.
    pub fn get_node_private_key(&self) -> &str {
        &self.private_key
    }

    /// Gets the node's public key.
    pub fn get_node_public_key(&self) -> &str {
        &self.public_key
    }

    /// Gets a reference to the security manager.
    pub fn get_security_manager(&self) -> &Arc<SecurityManager> {
        &self.security_manager
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::tempdir;
    use base64::{Engine as _, engine::general_purpose};

    #[test]
    fn test_node_private_key_generation() {
        let temp_dir = tempdir().unwrap();
        let config = NodeConfig::new(temp_dir.path().to_path_buf());
        let node = DataFoldNode::new(config).unwrap();

        // Verify that private and public keys were generated
        let private_key = node.get_node_private_key();
        let public_key = node.get_node_public_key();

        assert!(!private_key.is_empty());
        assert!(!public_key.is_empty());
        assert_ne!(private_key, public_key);

        // Verify that the keys are valid base64
        assert!(general_purpose::STANDARD.decode(private_key).is_ok());
        assert!(general_purpose::STANDARD.decode(public_key).is_ok());
    }
}
