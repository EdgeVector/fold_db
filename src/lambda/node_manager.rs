use crate::datafold_node::DataFoldNode;
use crate::ingestion::IngestionError;
use crate::lambda::config::{LambdaConfig, LambdaStorage};
use std::collections::HashMap;
use std::sync::{Arc, Mutex};

/// Manages DataFold nodes for different tenants
pub struct NodeManager {
    /// Configuration for creating new nodes
    config: LambdaConfig,
    /// Cache of active nodes (user_id -> Node)
    /// In a real Lambda environment, this might need expiration/eviction
    /// but for now we rely on Lambda execution context reuse
    nodes: Arc<Mutex<HashMap<String, Arc<tokio::sync::Mutex<DataFoldNode>>>>>,
    /// Singleton node for single-tenant mode (optimization)
    single_node: Option<Arc<tokio::sync::Mutex<DataFoldNode>>>,
}

impl NodeManager {
    /// Get the single-tenant node if one exists
    pub fn get_single_node(&self) -> Option<Arc<tokio::sync::Mutex<DataFoldNode>>> {
        self.single_node.clone()
    }

    /// Create a new NodeManager
    pub async fn new(config: LambdaConfig) -> Result<Self, IngestionError> {
        let mut manager = Self {
            config: config.clone(),
            nodes: Arc::new(Mutex::new(HashMap::new())),
            single_node: None,
        };

        // Pre-initialize single node if not in DynamoDB mode (single tenant optimization)
        match &config.storage {
            LambdaStorage::Config(crate::storage::DatabaseConfig::Cloud(_)) => {
                // Multi-tenant mode: Nodes created on demand
            }
            _ => {
                // Single-tenant mode: Create one node now
                let user_id = std::env::var("FOLDB_USER_ID").map_err(|_| {
                    IngestionError::configuration_error(
                        "FOLDB_USER_ID environment variable required for single-tenant mode",
                    )
                })?;
                let node = manager.create_node(&user_id).await?;
                manager.single_node = Some(node);
            }
        }

        Ok(manager)
    }

    /// Get a node for a specific user
    pub async fn get_node(
        &self,
        user_id: &str,
    ) -> Result<Arc<tokio::sync::Mutex<DataFoldNode>>, IngestionError> {
        // If we have a singleton node, return it regardless of user_id
        // This maintains backward compatibility for single-tenant users
        if let Some(node) = &self.single_node {
            return Ok(node.clone());
        }

        // Check cache first
        {
            let nodes = self.nodes.lock().unwrap();
            if let Some(node) = nodes.get(user_id) {
                return Ok(node.clone());
            }
        }

        // Create new node
        let node = self.create_node(user_id).await?;

        // Cache it
        {
            let mut nodes = self.nodes.lock().unwrap();
            nodes.insert(user_id.to_string(), node.clone());
        }

        Ok(node)
    }

    /// Create a new node instance
    async fn create_node(
        &self,
        user_id: &str,
    ) -> Result<Arc<tokio::sync::Mutex<DataFoldNode>>, IngestionError> {
        use crate::datafold_node::config::{DatabaseConfig, NodeConfig};
        use crate::fold_db_core::factory;
        use crate::fold_db_core::FoldDB;

        // Convert LambdaStorage to NodeConfig or handle DbOps case
        let (db, node_config) = match &self.config.storage {
            LambdaStorage::Config(storage_config) => {
                let mut node_config = match storage_config {
                    DatabaseConfig::Local { path } => NodeConfig::new(path.clone()),
                    #[cfg(feature = "aws-backend")]
                    DatabaseConfig::Cloud(cloud_config) => {
                        let mut cfg = NodeConfig::default();
                        let mut d_cfg = cloud_config.clone();
                        d_cfg.user_id = Some(user_id.to_string());
                        cfg.database = DatabaseConfig::Cloud(d_cfg);
                        cfg
                    }
                };

                // If schema service URL is provided in LambdaConfig, apply it to NodeConfig
                if let Some(schema_url) = &self.config.schema_service_url {
                    node_config = node_config.with_schema_service_url(schema_url);
                }

                // Deterministically generate identity keys from user_id
                use sha2::{Digest, Sha256};
                let mut hasher = Sha256::new();
                hasher.update(user_id.as_bytes());
                let result = hasher.finalize();
                let secret_seed = result.as_slice();

                let keypair = crate::security::Ed25519KeyPair::from_secret_key(secret_seed)
                    .map_err(|e| IngestionError::SecurityError(e.to_string()))?;

                // Set identity on config
                node_config = node_config
                    .with_identity(&keypair.public_key_base64(), &keypair.secret_key_base64());

                let db = factory::create_fold_db(&node_config.database)
                    .await
                    .map_err(|e| IngestionError::StorageError(e.to_string()))?;

                (db, node_config)
            }
            LambdaStorage::DbOps(db_ops) => {
                // Pre-created ops - usually single tenant
                let db_path = "custom_backend".to_string();

                // Manually create components, effectively replicating what create_fold_db does for custom ops
                let progress_store = Arc::new(crate::progress::InMemoryProgressStore::new());

                let fold_db = FoldDB::new_with_components(
                    Arc::clone(db_ops),
                    &db_path,
                    Some(progress_store),
                    Some(user_id.to_string()),
                )
                .await
                .map_err(|e| IngestionError::StorageError(e.to_string()))?;

                let node_config = NodeConfig::new(std::path::PathBuf::from(db_path));

                // If schema service URL is provided in LambdaConfig, apply it to node_config FIRST
                // (fix: shadowed variable usage)
                let mut node_config = match &self.config.schema_service_url {
                    Some(schema_url) => node_config.with_schema_service_url(schema_url),
                    None => node_config,
                };

                // Deterministically generate identity keys from user_id
                use sha2::{Digest, Sha256};
                let mut hasher = Sha256::new();
                hasher.update(user_id.as_bytes());
                let result = hasher.finalize();
                let secret_seed = result.as_slice();

                let keypair = crate::security::Ed25519KeyPair::from_secret_key(secret_seed)
                    .map_err(|e| IngestionError::SecurityError(e.to_string()))?;

                // Set identity on config
                node_config = node_config
                    .with_identity(&keypair.public_key_base64(), &keypair.secret_key_base64());

                (Arc::new(tokio::sync::Mutex::new(fold_db)), node_config)
            }
        };

        // Create DataFold node
        let node = DataFoldNode::new_with_db(node_config, db)
            .await
            .map_err(|e| IngestionError::InvalidInput(e.to_string()))?;

        Ok(Arc::new(tokio::sync::Mutex::new(node)))
    }
    /// Invalidate (remove) a node from the cache
    /// This forces a reload/recreation on the next access
    pub fn invalidate_node(&self, user_id: &str) {
        if let Ok(mut nodes) = self.nodes.lock() {
            nodes.remove(user_id);
        }
        // Also clear active single node if it matches or if we want to force reset
        // Note: active_node handling might be tricky if it's the same Arc.
        // But for cache invalidation, creating a new one next time is what matters.
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::lambda::config::{LambdaConfig, LambdaStorage};
    use tempfile::tempdir;

    #[tokio::test]
    async fn test_node_manager_single_mode() {
        std::env::set_var("FOLDB_USER_ID", "test_user");
        let temp_dir = tempdir().unwrap();
        let db_path = temp_dir.path().join("db");

        // Use LambdaConfig::new with DatabaseConfig
        let config = LambdaConfig::new(
            crate::storage::DatabaseConfig::Local { path: db_path },
            crate::lambda::config::LambdaLogging::Stdout,
        );

        let manager = NodeManager::new(config)
            .await
            .expect("Failed to create manager");

        // Should return the same singleton node for any user_id
        let node1 = manager
            .get_node("user1")
            .await
            .expect("Failed to get node1");
        let node2 = manager
            .get_node("user2")
            .await
            .expect("Failed to get node2");

        let id1 = node1.lock().await.get_node_id().to_string();
        let id2 = node2.lock().await.get_node_id().to_string();

        assert_eq!(
            id1, id2,
            "In single mode, all users should get the same node"
        );
    }
}
