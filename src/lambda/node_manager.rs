use std::collections::HashMap;
use std::sync::{Arc, Mutex};
use crate::datafold_node::{DataFoldNode, NodeConfig};
use crate::ingestion::IngestionError;
use crate::lambda::config::{LambdaConfig, LambdaStorage, TableConfig};
use crate::storage::TableNameResolver;

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
            LambdaStorage::DynamoDb(_) => {
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
    pub async fn get_node(&self, user_id: &str) -> Result<Arc<tokio::sync::Mutex<DataFoldNode>>, IngestionError> {
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
    async fn create_node(&self, user_id: &str) -> Result<Arc<tokio::sync::Mutex<DataFoldNode>>, IngestionError> {
        use crate::db_operations::DbOperationsV2;
        use crate::fold_db_core::FoldDB;
        use crate::storage::{StorageConfig, DynamoDbConfig};

        let (db, storage_path) = match &self.config.storage {
            LambdaStorage::Config(storage_config) => {
                // Legacy path for Local storage
                let (fold_db, path) = match storage_config {
                    StorageConfig::Local { path } => {
                        std::fs::create_dir_all(path)
                            .map_err(|e| IngestionError::StorageError(e.to_string()))?;
                        
                        let path_str = path
                            .to_str()
                            .ok_or_else(|| IngestionError::StorageError("Invalid storage path".to_string()))?;
                        
                        let fold_db = FoldDB::new(path_str).await
                            .map_err(|e| IngestionError::StorageError(e.to_string()))?;
                        (fold_db, path.clone())
                    }
                };
                (Arc::new(Mutex::new(fold_db)), path)
            }
            LambdaStorage::DbOps(db_ops) => {
                // Pre-created ops - usually single tenant
                let db_path = "custom_backend".to_string();
                let fold_db = FoldDB::new_with_db_ops(Arc::clone(db_ops), &db_path, None).await
                    .map_err(|e| IngestionError::StorageError(e.to_string()))?;
                (Arc::new(Mutex::new(fold_db)), std::path::PathBuf::from(db_path))
            }
            LambdaStorage::DynamoDb(dynamo_config) => {
                // Multi-tenant DynamoDB creation
                let config = aws_config::defaults(aws_config::BehaviorVersion::latest())
                    .region(aws_sdk_dynamodb::config::Region::new(dynamo_config.region.clone()))
                    .load()
                    .await;
                
                let client = aws_sdk_dynamodb::Client::new(&config);
                
                let resolver = match &dynamo_config.table_config {
                    TableConfig::Prefix(prefix) => TableNameResolver::Prefix(prefix.clone()),
                    TableConfig::Explicit(tables) => {
                        let mut map = HashMap::new();
                        map.insert("main".to_string(), tables.main.clone());
                        map.insert("metadata".to_string(), tables.metadata.clone());
                        map.insert("node_id_schema_permissions".to_string(), tables.permissions.clone());
                        map.insert("transforms".to_string(), tables.transforms.clone());
                        map.insert("orchestrator_state".to_string(), tables.orchestrator.clone());
                        map.insert("schema_states".to_string(), tables.schema_states.clone());
                        map.insert("schemas".to_string(), tables.schemas.clone());
                        map.insert("public_keys".to_string(), tables.public_keys.clone());
                        map.insert("transform_queue_tree".to_string(), tables.transform_queue.clone());
                        map.insert("native_index".to_string(), tables.native_index.clone());
                        TableNameResolver::Explicit(map)
                    }
                };
                
                let db_ops = Arc::new(
                    DbOperationsV2::from_dynamodb_flexible(
                        client, 
                        resolver, 
                        dynamo_config.auto_create,
                        Some(user_id.to_string())
                    ).await
                        .map_err(|e| IngestionError::StorageError(format!("Failed to initialize DynamoDB backend: {}", e)))?
                );
                
                // Use a derived path for internal consistency (though DB ops handles actual storage)
                let process_table_name = match &dynamo_config.table_config {
                     TableConfig::Prefix(prefix) => Some(format!("{}-process", prefix)),
                     TableConfig::Explicit(tables) => Some(tables.process.clone()),
                };
                
                let db_path = format!("dynamodb_{}", user_id);
                let fold_db = FoldDB::new_with_db_ops(db_ops, &db_path, process_table_name).await
                    .map_err(|e| IngestionError::StorageError(e.to_string()))?;
                
                (Arc::new(Mutex::new(fold_db)), std::path::PathBuf::from(db_path))
            }
        };

        // Initialize node config
        let mut node_config = NodeConfig::new(storage_path);

        // Set schema service URL if provided
        if let Some(schema_url) = &self.config.schema_service_url {
            node_config = node_config.with_schema_service_url(schema_url);
        }

        // Create DataFold node
        let node = DataFoldNode::new_with_db(node_config, db).await
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
        
        // Use LambdaConfig::new with StorageConfig
        let config = LambdaConfig::new(
            crate::storage::StorageConfig::Local {
                path: db_path,
            },
            crate::lambda::config::LambdaLogging::Stdout
        );

        let manager = NodeManager::new(config).await.expect("Failed to create manager");

        // Should return the same singleton node for any user_id
        let node1 = manager.get_node("user1").await.expect("Failed to get node1");
        let node2 = manager.get_node("user2").await.expect("Failed to get node2");

        let id1 = node1.lock().await.get_node_id().to_string();
        let id2 = node2.lock().await.get_node_id().to_string();

        assert_eq!(id1, id2, "In single mode, all users should get the same node");
    }
}
