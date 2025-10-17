use crate::log_feature;
use crate::logging::features::LogFeature;
use serde::Serialize;
use std::sync::{Arc, Mutex, MutexGuard};

use crate::datafold_node::config::NodeConfig;
use crate::error::{FoldDbError, FoldDbResult};
use crate::fold_db_core::FoldDB;
use crate::security::{Ed25519KeyPair, EncryptionManager, SecurityManager};

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
    pub(super) config: NodeConfig,
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
        let keypair = Ed25519KeyPair::generate().map_err(|e| {
            FoldDbError::SecurityError(format!("Failed to generate keypair: {}", e))
        })?;
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

        let node = Self {
            db,
            config: config.clone(),
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
            return Err(FoldDbError::Config(
                "Schema service URL is required. Please configure schema_service_url in NodeConfig.".to_string()
            ));
        }

        log_feature!(
            LogFeature::Database,
            info,
            "DataFoldNode created successfully with schema system initialized"
        );
        Ok(node)
    }

    /// Get a reference to the underlying FoldDB instance
    pub fn get_fold_db(&self) -> FoldDbResult<MutexGuard<'_, FoldDB>> {
        self.db
            .lock()
            .map_err(|_| FoldDbError::Config("Cannot lock database mutex".into()))
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
    pub async fn add_schema_to_service(&self, schema: &crate::schema::types::Schema) -> FoldDbResult<crate::schema::types::Schema> {
        let schema_service_url = self.schema_service_url().ok_or_else(|| {
            FoldDbError::Config("Schema service URL is not configured".to_string())
        })?;

        if schema_service_url.starts_with("test://") || schema_service_url.starts_with("mock://") {
            return Err(FoldDbError::Config(
                "Cannot add schemas to test/mock schema service".to_string(),
            ));
        }

        let client = crate::datafold_node::SchemaServiceClient::new(&schema_service_url);
        client.add_schema(schema, std::collections::HashMap::new()).await.map(|response| response.schema)
    }
}

impl Drop for DataFoldNode {
    fn drop(&mut self) {
        log_feature!(
            LogFeature::Database,
            info,
            "DataFoldNode being dropped, closing database..."
        );

        // Try to close the database gracefully
        if let Ok(db) = self.db.lock() {
            if let Err(e) = db.close() {
                log_feature!(
                    LogFeature::Database,
                    error,
                    "Failed to close database during node drop: {}",
                    e
                );
            } else {
                log_feature!(
                    LogFeature::Database,
                    info,
                    "Database closed successfully during node drop"
                );
            }
        } else {
            log_feature!(
                LogFeature::Database,
                warn,
                "Could not acquire database lock during node drop"
            );
        }
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
}

#[cfg(test)]
mod tests {
    use super::*;
    use base64::{engine::general_purpose, Engine as _};
    use tempfile::tempdir;

    #[test]
    fn test_node_private_key_generation() {
        let temp_dir = tempdir().unwrap();
        let config =
            NodeConfig::new(temp_dir.path().to_path_buf()).with_schema_service_url("test://mock");
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
