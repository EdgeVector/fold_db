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
use base64::Engine as _;
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::collections::HashMap;
use std::path::PathBuf;
use std::sync::Arc;
use tokio::sync::{Mutex, RwLock};

/// Persisted node identity keypair
#[derive(Serialize, Deserialize)]
struct NodeIdentity {
    private_key: String,
    public_key: String,
}

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
    /// Configuration for creating new nodes (wrapped in RwLock for live reconfiguration)
    config: RwLock<NodeManagerConfig>,
    /// Cache of active nodes (user_id -> Node)
    nodes: Arc<Mutex<HashMap<String, Arc<RwLock<FoldNode>>>>>,
    /// Shared node for local mode (single-tenant)
    /// In local Sled mode, we share one node to avoid lock conflicts
    shared_local_node: Arc<Mutex<Option<Arc<RwLock<FoldNode>>>>>,
    /// Whether we're in local mode (wrapped in RwLock for live reconfiguration)
    is_local_mode: RwLock<bool>,
}

impl NodeManager {
    /// Create a new NodeManager
    pub fn new(config: NodeManagerConfig) -> Self {
        let is_local_mode = matches!(config.base_config.database, DatabaseConfig::Local { .. });
        Self {
            config: RwLock::new(config),
            nodes: Arc::new(Mutex::new(HashMap::new())),
            shared_local_node: Arc::new(Mutex::new(None)),
            is_local_mode: RwLock::new(is_local_mode),
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
        if *self.is_local_mode.read().await {
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
        let mut node_config = self.config.read().await.base_config.clone();

        // Set user_id in database config
        match &mut node_config.database {
            #[cfg(feature = "aws-backend")]
            DatabaseConfig::Cloud(ref mut cloud_config) => {
                cloud_config.user_id = Some(user_id.to_string());
            }
            DatabaseConfig::Local { .. } | DatabaseConfig::Exemem { .. } => {
                // Local/Exemem storage doesn't need user_id in config
                // User isolation is handled differently
            }
        }

        // Load or generate a random identity keypair (persisted to disk)
        let keypair = Self::load_or_generate_identity(user_id).await?;

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

    /// Load an existing identity keypair from disk, or generate a new random one.
    ///
    /// Key file path: `~/.fold_db/identity/{sha256(user_id)}.json`
    /// The SHA-256 hash is used as the filename to avoid path injection from arbitrary user_ids.
    async fn load_or_generate_identity(
        user_id: &str,
    ) -> Result<Ed25519KeyPair, NodeManagerError> {
        // Build the key file path: ~/.fold_db/identity/{hash}.json
        let home = std::env::var("HOME")
            .map(PathBuf::from)
            .map_err(|_| NodeManagerError::ConfigurationError("HOME environment variable not set".to_string()))?;

        let mut hasher = Sha256::new();
        hasher.update(user_id.as_bytes());
        let hash_hex = format!("{:x}", hasher.finalize());

        let identity_dir = home.join(".fold_db/identity");
        let identity_path = identity_dir.join(format!("{hash_hex}.json"));

        if identity_path.exists() {
            // Load existing keypair
            let content = tokio::fs::read_to_string(&identity_path)
                .await
                .map_err(|e| NodeManagerError::SecurityError(format!("Failed to read identity file: {e}")))?;

            let identity: NodeIdentity = serde_json::from_str(&content)
                .map_err(|e| NodeManagerError::SecurityError(format!("Invalid identity file: {e}")))?;

            let secret_bytes = base64::engine::general_purpose::STANDARD
                .decode(&identity.private_key)
                .map_err(|e| NodeManagerError::SecurityError(format!("Invalid private key encoding: {e}")))?;

            Ed25519KeyPair::from_secret_key(&secret_bytes)
                .map_err(|e| NodeManagerError::SecurityError(e.to_string()))
        } else {
            // Generate a new random keypair
            let keypair = Ed25519KeyPair::generate()
                .map_err(|e| NodeManagerError::SecurityError(e.to_string()))?;

            let identity = NodeIdentity {
                private_key: keypair.secret_key_base64(),
                public_key: keypair.public_key_base64(),
            };

            // Ensure directory exists
            tokio::fs::create_dir_all(&identity_dir)
                .await
                .map_err(|e| NodeManagerError::SecurityError(format!("Failed to create identity directory: {e}")))?;

            // Write the identity file
            let content = serde_json::to_string_pretty(&identity)
                .map_err(|e| NodeManagerError::SecurityError(format!("Failed to serialize identity: {e}")))?;
            tokio::fs::write(&identity_path, &content)
                .await
                .map_err(|e| NodeManagerError::SecurityError(format!("Failed to write identity file: {e}")))?;

            // Restrict permissions to owner-only (Unix)
            #[cfg(unix)]
            {
                use std::os::unix::fs::PermissionsExt;
                let perms = std::fs::Permissions::from_mode(0o600);
                std::fs::set_permissions(&identity_path, perms).map_err(|e| {
                    NodeManagerError::SecurityError(format!("Failed to set identity file permissions: {e}"))
                })?;
            }

            log::info!("Generated new node identity at {}", identity_path.display());

            Ok(keypair)
        }
    }

    /// Invalidate (remove) a node from the cache
    /// This forces a reload/recreation on the next access
    #[allow(dead_code)]
    pub async fn invalidate_node(&self, user_id: &str) {
        let mut nodes = self.nodes.lock().await;
        nodes.remove(user_id);

        // Also clear the shared local node so it gets re-created
        if *self.is_local_mode.read().await {
            let mut shared = self.shared_local_node.lock().await;
            *shared = None;
        }
    }

    /// Invalidate all cached nodes
    /// Used when configuration changes require all nodes to be recreated
    pub async fn invalidate_all_nodes(&self) {
        let mut nodes = self.nodes.lock().await;
        nodes.clear();

        let mut shared = self.shared_local_node.lock().await;
        *shared = None;
    }

    /// Update the configuration and invalidate all cached nodes
    /// The next request will create fresh nodes with the new config
    pub async fn update_config(&self, new_config: NodeManagerConfig) {
        let new_is_local = matches!(new_config.base_config.database, DatabaseConfig::Local { .. });

        {
            let mut config = self.config.write().await;
            *config = new_config;
        }
        {
            let mut is_local = self.is_local_mode.write().await;
            *is_local = new_is_local;
        }

        self.invalidate_all_nodes().await;
    }

    /// Set a pre-existing node in the cache
    /// This is useful for embedded scenarios where the node is created externally
    pub async fn set_node(&self, user_id: &str, node: FoldNode) {
        let node_arc = Arc::new(RwLock::new(node));

        // In local mode, also set the shared_local_node so get_node finds it
        if *self.is_local_mode.read().await {
            let mut shared = self.shared_local_node.lock().await;
            *shared = Some(node_arc.clone());
        }

        let mut nodes = self.nodes.lock().await;
        nodes.insert(user_id.to_string(), node_arc);
    }

    /// Get the base configuration (returns a clone since config is behind RwLock)
    pub async fn get_base_config(&self) -> NodeConfig {
        self.config.read().await.base_config.clone()
    }
}
