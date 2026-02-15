//! Multi-tenant Node Manager
//!
//! Manages FoldDB nodes for different tenants, caching them for reuse.
//! This enables lazy node initialization - nodes are only created when
//! a user makes their first request, avoiding DynamoDB access during startup.
//!
//! # Storage Mode Behavior
//!
//! - **Cloud mode (DynamoDB)**: Creates separate nodes per user with user_id isolation
//! - **Local mode (Sled)**: Shares a single node across all users (single-tenant)
//!   This avoids Sled lock conflicts since only one process can hold the lock.

use crate::fold_node::config::NodeConfig;
use crate::fold_node::FoldNode;
use crate::fold_db_core::factory;
use crate::security::Ed25519KeyPair;
use crate::storage::config::DatabaseConfig;
use sha2::{Digest, Sha256};
use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::{Mutex, RwLock};

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

/// Manages FoldDB nodes for different tenants
pub struct NodeManager {
    /// Configuration for creating new nodes
    config: NodeManagerConfig,
    /// Cache of active nodes (user_id -> Node)
    nodes: Arc<Mutex<HashMap<String, Arc<RwLock<FoldNode>>>>>,
    /// Shared node for local mode (single-tenant)
    /// In local Sled mode, we share one node to avoid lock conflicts
    shared_local_node: Arc<Mutex<Option<Arc<RwLock<FoldNode>>>>>,
    /// Whether we're in local mode
    is_local_mode: bool,
}

impl NodeManager {
    /// Create a new NodeManager
    pub fn new(config: NodeManagerConfig) -> Self {
        let is_local_mode = matches!(config.base_config.database, DatabaseConfig::Local { .. });
        Self {
            config,
            nodes: Arc::new(Mutex::new(HashMap::new())),
            shared_local_node: Arc::new(Mutex::new(None)),
            is_local_mode,
        }
    }

    /// Get a node for a specific user, creating one if it doesn't exist
    ///
    /// In local mode (Sled), returns a shared node for all users to avoid lock conflicts.
    /// In cloud mode (DynamoDB), creates/returns a per-user node with user_id isolation.
    pub async fn get_node(
        &self,
        user_id: &str,
    ) -> Result<Arc<RwLock<FoldNode>>, NodeManagerError> {
        // Local mode: use shared single node to avoid Sled lock conflicts
        if self.is_local_mode {
            return self.get_shared_local_node(user_id).await;
        }

        // Cloud mode: per-user nodes with DynamoDB partition isolation
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

    /// Get or create the shared local node (for Sled mode)
    ///
    /// Uses a mutex to ensure only one node is ever created, avoiding race conditions
    /// where multiple concurrent requests could try to create the node simultaneously.
    async fn get_shared_local_node(
        &self,
        user_id: &str,
    ) -> Result<Arc<RwLock<FoldNode>>, NodeManagerError> {
        // Hold the lock for the entire check-and-create operation to avoid races
        let mut shared = self.shared_local_node.lock().await;

        // If we already have a shared node, return it
        if let Some(node) = shared.as_ref() {
            return Ok(node.clone());
        }

        // Create the shared node while still holding the lock
        // This ensures only one thread creates the node
        let node = self.create_node(user_id).await?;

        // Store it as the shared node
        *shared = Some(node.clone());

        Ok(node)
    }

    /// Create a new node instance for a user
    async fn create_node(
        &self,
        user_id: &str,
    ) -> Result<Arc<RwLock<FoldNode>>, NodeManagerError> {
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

        // Load or generate E2E encryption keys
        let home = std::env::var("HOME")
            .map(std::path::PathBuf::from)
            .map_err(|_| NodeManagerError::ConfigurationError("HOME environment variable not set".to_string()))?;
        let e2e_key_path = home.join(".fold_db/e2e.key");
        let e2e_keys = crate::crypto::E2eKeys::load_or_generate(&e2e_key_path)
            .await
            .map_err(|e| NodeManagerError::ConfigurationError(format!("Failed to load E2E keys: {}", e)))?;

        // Create FoldDB with user context set
        let db = crate::logging::core::run_with_user(user_id, async {
            factory::create_fold_db(&node_config.database, &e2e_keys).await
        })
        .await
        .map_err(|e| NodeManagerError::StorageError(e.to_string()))?;

        // Create FoldDB node with user context set
        let node = crate::logging::core::run_with_user(user_id, async {
            FoldNode::new_with_db(node_config, db).await
        })
        .await
        .map_err(|e| NodeManagerError::NodeCreationError(e.to_string()))?;

        Ok(Arc::new(RwLock::new(node)))
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
    pub async fn set_node(&self, user_id: &str, node: FoldNode) {
        let node_arc = Arc::new(RwLock::new(node));

        // In local mode, also set the shared_local_node so get_node finds it
        if self.is_local_mode {
            let mut shared = self.shared_local_node.lock().await;
            *shared = Some(node_arc.clone());
        }

        let mut nodes = self.nodes.lock().await;
        nodes.insert(user_id.to_string(), node_arc);
    }

    /// Get the base configuration
    pub fn get_base_config(&self) -> &NodeConfig {
        &self.config.base_config
    }
}
