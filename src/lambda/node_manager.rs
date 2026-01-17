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
    /// Create a new NodeManager
    pub async fn new(config: LambdaConfig) -> Result<Self, IngestionError> {
        let mut manager = Self {
            config: config.clone(),
            nodes: Arc::new(Mutex::new(HashMap::new())),
            single_node: None,
        };

        // Pre-initialize single node if not in DynamoDB mode (single tenant optimization)
        match &config.storage {
            LambdaStorage::Config(crate::storage::DatabaseConfig::DynamoDb(_)) => {
                // Multi-tenant mode: Nodes created on demand
            }
            _ => {
                // Single-tenant mode: Create one node now
                let node = manager.create_node("default").await?;
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
        use crate::fold_db_core::FoldDB;
        use crate::fold_db_core::factory;
        use crate::storage::DatabaseConfig as StorageConfig; // Alias for compatibility with code below or just update

        // Convert LambdaStorage to NodeConfig or handle DbOps case
        let (db, node_config) = match &self.config.storage {
            LambdaStorage::Config(storage_config) => {
                let mut node_config = match storage_config {
                    StorageConfig::Local { path } => NodeConfig::new(path.clone()),
                    #[cfg(feature = "aws-backend")]
                    StorageConfig::DynamoDb(dynamo_config) => {
                        let mut cfg = NodeConfig::default();
                        let mut d_cfg = dynamo_config.clone();
                        d_cfg.user_id = Some(user_id.to_string());
                        cfg.database = DatabaseConfig::DynamoDb(d_cfg);
                        cfg
                    }
                };

                // If schema service URL is provided in LambdaConfig, apply it to NodeConfig
                if let Some(schema_url) = &self.config.schema_service_url {
                    node_config = node_config.with_schema_service_url(schema_url);
                }

                let db = factory::create_fold_db(&node_config.database)
                    .await
                    .map_err(|e| IngestionError::StorageError(e.to_string()))?;

                (db, node_config)
            }
            LambdaStorage::DbOps(db_ops) => {
                // Pre-created ops - usually single tenant
                let db_path = "custom_backend".to_string();

                // Manually create components, effectively replicating what create_fold_db does for custom ops
                let progress_store =
                    Arc::new(crate::fold_db_core::orchestration::InMemoryProgressStore::new());

                let fold_db =
                    FoldDB::new_with_components(Arc::clone(db_ops), &db_path, progress_store)
                        .await
                        .map_err(|e| IngestionError::StorageError(e.to_string()))?;

                let node_config = NodeConfig::new(std::path::PathBuf::from(db_path));

                // If schema service URL is provided in LambdaConfig, apply it to NodeConfig
                let node_config = if let Some(schema_url) = &self.config.schema_service_url {
                    node_config.with_schema_service_url(schema_url)
                } else {
                    node_config
                };

                (Arc::new(tokio::sync::Mutex::new(fold_db)), node_config)
            }
        };

        // Create DataFold node
        let node = DataFoldNode::new_with_db(node_config, db)
            .await
            .map_err(|e| IngestionError::InvalidInput(e.to_string()))?;

        Ok(Arc::new(tokio::sync::Mutex::new(node)))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::lambda::config::{LambdaConfig, LambdaStorage};
    use tempfile::tempdir;

    #[tokio::test]
    async fn test_node_manager_single_mode() {
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
