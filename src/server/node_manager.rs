//! Multi-tenant Node Manager
//!
//! Manages DataFold nodes for different tenants, caching them for reuse.
//! This enables lazy node initialization - nodes are only created when
//! a user makes their first request, avoiding DynamoDB access during startup.

use crate::datafold_node::config::NodeConfig;
use crate::datafold_node::DataFoldNode;
use crate::fold_db_core::factory;
use crate::security::Ed25519KeyPair;
use crate::storage::config::DatabaseConfig;
use sha2::{Digest, Sha256};
use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::Mutex;

/// Error type for node manager operations
#[derive(Debug, thiserror::Error)]
pub enum NodeManagerError {
    #[error("Configuration error: {0}")]
    ConfigurationError(String),
    #[error("Storage error: {0}")]
    StorageError(String),
    #[error("Security error: {0}")]
    SecurityError(String),
    #[error("Node creation error: {0}")]
    NodeCreationError(String),
}

/// Configuration for creating nodes
#[derive(Clone)]
pub struct NodeManagerConfig {
    /// Base node configuration (user_id will be set per-tenant)
    pub base_config: NodeConfig,
}

/// Manages DataFold nodes for different tenants
pub struct NodeManager {
    /// Configuration for creating new nodes
    config: NodeManagerConfig,
    /// Cache of active nodes (user_id -> Node)
    nodes: Arc<Mutex<HashMap<String, Arc<Mutex<DataFoldNode>>>>>,
}

impl NodeManager {
    /// Create a new NodeManager
    pub fn new(config: NodeManagerConfig) -> Self {
        Self {
            config,
            nodes: Arc::new(Mutex::new(HashMap::new())),
        }
    }

    /// Get a node for a specific user, creating one if it doesn't exist
    pub async fn get_node(
        &self,
        user_id: &str,
    ) -> Result<Arc<Mutex<DataFoldNode>>, NodeManagerError> {
        // Check cache first
        {
            let nodes = self.nodes.lock().await;
            if let Some(node) = nodes.get(user_id) {
                return Ok(node.clone());
            }
        }

        // Create new node
        let node = self.create_node(user_id).await?;

        // Cache it
        {
            let mut nodes = self.nodes.lock().await;
            nodes.insert(user_id.to_string(), node.clone());
        }

        Ok(node)
    }

    /// Create a new node instance for a user
    async fn create_node(
        &self,
        user_id: &str,
    ) -> Result<Arc<Mutex<DataFoldNode>>, NodeManagerError> {
        // Clone the base config and set user_id
        let mut node_config = self.config.base_config.clone();

        // Set user_id in database config
        match &mut node_config.database {
            #[cfg(feature = "aws-backend")]
            DatabaseConfig::Cloud(ref mut cloud_config) => {
                cloud_config.user_id = Some(user_id.to_string());
            }
            DatabaseConfig::Local { .. } => {
                // Local storage doesn't need user_id in config
                // User isolation is handled differently
            }
        }

        // Deterministically generate identity keys from user_id
        let mut hasher = Sha256::new();
        hasher.update(user_id.as_bytes());
        let result = hasher.finalize();
        let secret_seed = result.as_slice();

        let keypair = Ed25519KeyPair::from_secret_key(secret_seed)
            .map_err(|e| NodeManagerError::SecurityError(e.to_string()))?;

        // Set identity on config
        node_config =
            node_config.with_identity(&keypair.public_key_base64(), &keypair.secret_key_base64());

        // Create FoldDB with user context set
        let db = crate::logging::core::run_with_user(user_id, async {
            factory::create_fold_db(&node_config.database).await
        })
        .await
        .map_err(|e| NodeManagerError::StorageError(e.to_string()))?;

        // Create DataFold node with user context set
        let node = crate::logging::core::run_with_user(user_id, async {
            DataFoldNode::new_with_db(node_config, db).await
        })
        .await
        .map_err(|e| NodeManagerError::NodeCreationError(e.to_string()))?;

        Ok(Arc::new(Mutex::new(node)))
    }

    /// Invalidate (remove) a node from the cache
    /// This forces a reload/recreation on the next access
    #[allow(dead_code)]
    pub async fn invalidate_node(&self, user_id: &str) {
        let mut nodes = self.nodes.lock().await;
        nodes.remove(user_id);
    }

    /// Set a pre-existing node in the cache
    /// This is useful for embedded scenarios where the node is created externally
    pub async fn set_node(&self, user_id: &str, node: DataFoldNode) {
        let mut nodes = self.nodes.lock().await;
        nodes.insert(user_id.to_string(), Arc::new(Mutex::new(node)));
    }

    /// Get the base configuration
    pub fn get_base_config(&self) -> &NodeConfig {
        &self.config.base_config
    }
}
