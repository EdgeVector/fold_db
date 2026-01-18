use crate::log_feature;
use crate::logging::features::LogFeature;
use serde::Serialize;
use std::sync::Arc;
use tokio::sync::Mutex;

use crate::datafold_node::config::NodeConfig;
use crate::error::{FoldDbError, FoldDbResult};
use crate::fold_db_core::FoldDB;
use crate::security::{Ed25519KeyPair, EncryptionManager, SecurityConfig, SecurityManager};

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
#[derive(Clone)]
pub struct DataFoldNode {
    /// The underlying database instance for data storage and operations
    pub(super) db: Arc<Mutex<FoldDB>>,
    /// Configuration settings for this node
    pub config: NodeConfig,
    /// Unique identifier for this node
    pub(super) node_id: String,
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
    ///
    /// Now fully async to support storage abstraction!
    pub async fn new(config: NodeConfig) -> FoldDbResult<Self> {
        // Try to load identity from file first, otherwise generate new
        let (private_key, public_key) = match load_persisted_identity() {
            Ok(Some((priv_k, pub_k))) => (priv_k, pub_k),
            _ => {
                // Fallback to generating a new keypair (and optionally persisting it?)
                // For now, we maintain the behavior of generating one if not found,
                // but ideally the environment should be set up by ensure_identity.
                let keypair = Ed25519KeyPair::generate().map_err(|e| {
                    FoldDbError::SecurityError(format!("Failed to generate keypair: {}", e))
                })?;
                (keypair.secret_key_base64(), keypair.public_key_base64())
            }
        };

        // Update config with public key as user_id if not set (for DynamoDB)
        let config = config;
        #[cfg(feature = "aws-backend")]
        if let crate::datafold_node::config::DatabaseConfig::DynamoDb(ref mut d) = config.database {
            if d.user_id.is_none() {
                d.user_id = Some(public_key.clone());
            }
        }

        let db = crate::fold_db_core::factory::create_fold_db(&config.database).await?;

        let (node_id, security_manager, security_config) =
            Self::init_internals(&config, &db).await?;

        let node = Self {
            db,
            config: NodeConfig {
                security_config,
                ..config.clone()
            },
            node_id,
            security_manager,
            private_key,
            public_key,
        };

        // Require schema service to be configured
        if let Some(schema_service_url) = &config.schema_service_url {
            // Check if this is a mock/test schema service
            if schema_service_url.starts_with("test://")
                || schema_service_url.starts_with("mock://")
            {
                log_feature!(
                    LogFeature::Database,
                    info,
                    "Mock schema service configured: {}. Schemas must be loaded manually.",
                    schema_service_url
                );
            } else {
                log_feature!(
                    LogFeature::Database,
                    info,
                    "Schema service URL configured: {}. Schemas will be loaded asynchronously after node startup.",
                    schema_service_url
                );
                // Note: Schema loading from service is deferred to avoid runtime nesting issues
                // It will be performed by the HTTP server after node initialization
            }
        } else {
            // Schema service is optional - log info and continue
            log::info!("No schema service URL configured - using local schema management only");
        }

        log_feature!(
            LogFeature::Database,
            info,
            "DataFoldNode created successfully with schema system initialized"
        );
        Ok(node)
    }

    /// Creates a new DataFoldNode with a pre-created FoldDB instance.
    ///
    /// This is useful when you need to control the storage backend (e.g., S3)
    /// before creating the node.
    ///
    /// # Arguments
    ///
    /// * `config` - Node configuration
    /// * `db` - Pre-created FoldDB instance
    pub async fn new_with_db(config: NodeConfig, db: Arc<Mutex<FoldDB>>) -> FoldDbResult<Self> {
        // Generate a new keypair (we can't update DB config as it's already created)
        let keypair = Ed25519KeyPair::generate().map_err(|e| {
            FoldDbError::SecurityError(format!("Failed to generate keypair: {}", e))
        })?;
        let private_key = keypair.secret_key_base64();
        let public_key = keypair.public_key_base64();

        let (node_id, security_manager, security_config) =
            Self::init_internals(&config, &db).await?;

        let node = Self {
            db,
            config: NodeConfig {
                security_config,
                ..config.clone()
            },
            node_id,
            security_manager,
            private_key,
            public_key,
        };

        // Require schema service to be configured
        if let Some(schema_service_url) = &config.schema_service_url {
            // Check if this is a mock/test schema service
            if schema_service_url.starts_with("test://")
                || schema_service_url.starts_with("mock://")
            {
                log_feature!(
                    LogFeature::Database,
                    info,
                    "Mock schema service configured: {}. Schemas must be loaded manually.",
                    schema_service_url
                );
            } else {
                log_feature!(
                    LogFeature::Database,
                    info,
                    "Schema service URL configured: {}. Schemas will be loaded asynchronously after node startup.",
                    schema_service_url
                );
            }
        } else {
            // Schema service is optional - log info and continue
            log::info!("No schema service URL configured - using local schema management only");
        }

        log_feature!(
            LogFeature::Database,
            info,
            "DataFoldNode created successfully with pre-created database"
        );
        Ok(node)
    }

    /// Get a reference to the underlying FoldDB instance
    pub async fn get_fold_db(&self) -> FoldDbResult<tokio::sync::OwnedMutexGuard<FoldDB>> {
        Ok(self.db.clone().lock_owned().await)
    }

    /// Gets the unique identifier for this node.
    pub fn get_node_id(&self) -> &str {
        &self.node_id
    }

    /// Gets the configured schema service URL, if present.
    pub fn schema_service_url(&self) -> Option<String> {
        self.config.schema_service_url.clone()
    }

    /// Fetch available schemas from the schema service.
    /// Returns an error if the schema service URL is not configured or if the fetch fails.
    pub async fn fetch_available_schemas(&self) -> FoldDbResult<Vec<crate::schema::types::Schema>> {
        let schema_service_url = self.schema_service_url().ok_or_else(|| {
            FoldDbError::Config("Schema service URL is not configured".to_string())
        })?;

        if schema_service_url.starts_with("test://") || schema_service_url.starts_with("mock://") {
            return Err(FoldDbError::Config(
                "Cannot fetch schemas from test/mock schema service".to_string(),
            ));
        }

        let client = crate::datafold_node::SchemaServiceClient::new(&schema_service_url);
        client.get_available_schemas().await
    }

    /// Add a new schema to the schema service.
    /// Returns an error if the schema service URL is not configured or if the operation fails.
    pub async fn add_schema_to_service(
        &self,
        schema: &crate::schema::types::Schema,
    ) -> FoldDbResult<crate::schema::types::Schema> {
        let schema_service_url = self.schema_service_url().ok_or_else(|| {
            FoldDbError::Config("Schema service URL is not configured".to_string())
        })?;

        if schema_service_url.starts_with("test://") || schema_service_url.starts_with("mock://") {
            return Err(FoldDbError::Config(
                "Cannot add schemas to test/mock schema service".to_string(),
            ));
        }

        let client = crate::datafold_node::SchemaServiceClient::new(&schema_service_url);
        client
            .add_schema(schema, std::collections::HashMap::new())
            .await
            .map(|response| response.schema)
    }

    /// Execute a batch of mutations.
    ///
    /// This is a convenience method that delegates to the underlying FoldDB.
    /// It is primarily used by tests and internal components that need direct
    /// access without going through the OperationProcessor.
    pub async fn mutate_batch(
        &self,
        mutations: Vec<crate::schema::types::operations::Mutation>,
    ) -> FoldDbResult<Vec<String>> {
        let mut db = self.db.lock().await;
        db.mutation_manager
            .write_mutations_batch_async(mutations)
            .await
            .map_err(|e| FoldDbError::Database(e.to_string()))
    }
    async fn init_internals(
        config: &NodeConfig,
        db: &Arc<Mutex<FoldDB>>,
    ) -> FoldDbResult<(String, Arc<SecurityManager>, SecurityConfig)> {
        // Retrieve or generate the persistent node_id from fold_db
        let node_id = {
            let guard = db.lock().await;
            guard
                .get_node_id()
                .await
                .map_err(|e| FoldDbError::Config(format!("Failed to get node_id: {}", e)))?
        };

        // Initialize security manager with node configuration
        let mut security_config = config.security_config.clone();

        // Generate master key if encryption is enabled but no key is set
        if security_config.encrypt_at_rest && security_config.master_key.is_none() {
            security_config.master_key = Some(EncryptionManager::generate_master_key());
        }

        let security_manager = {
            let guard = db.lock().await;

            let db_ops = guard.db_ops.clone();

            Arc::new(
                SecurityManager::new_with_persistence(
                    config.security_config.clone(),
                    Arc::clone(&db_ops),
                )
                .await
                .map_err(|e| FoldDbError::SecurityError(e.to_string()))?,
            )
        };

        Ok((
            node_id,
            security_manager,
            security_config,
        ))
    }
}



impl DataFoldNode {
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

    /// Get a schema service client for communicating with the schema service
    pub fn get_schema_client(&self) -> crate::datafold_node::schema_client::SchemaServiceClient {
        let url = self
            .config
            .schema_service_url
            .as_deref()
            .unwrap_or("http://localhost:9002");
        crate::datafold_node::schema_client::SchemaServiceClient::new(url)
    }

    /// Get the current indexing status
    pub async fn get_indexing_status(&self) -> crate::fold_db_core::orchestration::IndexingStatus {
        let db = self.db.lock().await;
        db.get_indexing_status().await
    }

    /// Check if indexing is currently in progress
    pub async fn is_indexing(&self) -> bool {
        let db = self.db.lock().await;
        db.is_indexing().await
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use base64::{engine::general_purpose, Engine as _};
    use tempfile::tempdir;

    #[tokio::test]
    async fn test_node_private_key_generation() {
        let temp_dir = tempdir().unwrap();
        let config =
            NodeConfig::new(temp_dir.path().to_path_buf()).with_schema_service_url("test://mock");
        let node = DataFoldNode::new(config).await.unwrap();

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

#[derive(serde::Deserialize)]
struct NodeIdentity {
    private_key: String,
    public_key: String,
}

fn load_persisted_identity() -> FoldDbResult<Option<(String, String)>> {
    let config_path = std::path::Path::new("config/node_identity.json");
    if config_path.exists() {
        let content = std::fs::read_to_string(config_path).map_err(|e| {
            FoldDbError::Config(format!("Failed to read node_identity.json: {}", e))
        })?;
        
        match serde_json::from_str::<NodeIdentity>(&content) {
            Ok(identity) => Ok(Some((identity.private_key, identity.public_key))),
            Err(e) => {
                log::warn!("Failed to parse node_identity.json: {}. Generating new identity.", e);
                Ok(None)
            }
        }
    } else {
        Ok(None)
    }
}
