use crate::log_feature;
use crate::logging::features::LogFeature;
use crate::storage::traits::TypedStore;
use serde::{Deserialize, Serialize};
use std::sync::Arc;
use tokio::sync::Mutex;

use crate::fold_node::config::NodeConfig;
use crate::fold_node::mutation_preprocessor::MutationPreprocessor;
use crate::error::{FoldDbError, FoldDbResult};
use crate::fold_db_core::FoldDB;
use crate::security::{EncryptionManager, SecurityConfig, SecurityManager};

/// A node in the Fold distributed database system.
///
/// FoldNode combines database storage, schema management, and networking
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
pub struct FoldNode {
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
    /// E2E encryption keys (content encryption + index blinding).
    /// Stored for future passkey integration where the key may need to be refreshed.
    pub(super) e2e_keys: crate::crypto::E2eKeys,
    /// Preprocessor that enriches mutations with keyword index_terms.
    mutation_preprocessor: MutationPreprocessor,
}

impl FoldNode {
    /// Creates a new FoldNode with the specified configuration.
    ///
    /// Now fully async to support storage abstraction!
    pub async fn new(#[allow(unused_mut)] mut config: NodeConfig) -> FoldDbResult<Self> {
        // 1. Try to use identity from config
        // 2. Fallback to loading from file (legacy/local dev)
        // 3. Fail if neither is present (NO AUTO-GENERATION)
        let (private_key, public_key) =
            if let (Some(priv_k), Some(pub_k)) = (&config.private_key, &config.public_key) {
                (priv_k.clone(), pub_k.clone())
            } else {
                match load_persisted_identity() {
                    Ok(Some((priv_k, pub_k))) => (priv_k, pub_k),
                    _ => {
                        return Err(FoldDbError::SecurityError(
                            "Node identity (keys) not configured and no persisted identity found. \
                        Auto-generation is disabled. Please provide identity."
                                .to_string(),
                        ));
                    }
                }
            };

        // Update config with public key as user_id if not set (for DynamoDB)
        #[cfg(feature = "aws-backend")]
        if let crate::fold_node::config::DatabaseConfig::Cloud(ref mut d) = config.database {
            if d.user_id.is_none() {
                d.user_id = Some(public_key.clone());
            }
        }

        // Load or generate E2E encryption keys
        let home = std::env::var("HOME")
            .map(std::path::PathBuf::from)
            .map_err(|_| FoldDbError::Config("HOME environment variable not set".to_string()))?;
        let e2e_key_path = home.join(".fold_db/e2e.key");
        let e2e_keys = crate::crypto::E2eKeys::load_or_generate(&e2e_key_path)
            .await
            .map_err(|e| FoldDbError::Config(format!("Failed to load E2E keys: {}", e)))?;

        let db = crate::fold_db_core::factory::create_fold_db(&config.database, &e2e_keys).await?;

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
            e2e_keys,
            mutation_preprocessor: MutationPreprocessor::new(),
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
            "FoldNode created successfully with schema system initialized"
        );
        Ok(node)
    }

    /// Creates a new FoldNode with a pre-created FoldDB instance.
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
        // 1. Check config for identity
        let (private_key, public_key) =
            if let (Some(priv_k), Some(pub_k)) = (&config.private_key, &config.public_key) {
                (priv_k.clone(), pub_k.clone())
            } else {
                // 2. Fallback to loading from file
                match load_persisted_identity() {
                    Ok(Some((priv_k, pub_k))) => (priv_k, pub_k),
                    _ => {
                        return Err(FoldDbError::SecurityError(
                            "Node identity (keys) not configured and no persisted identity found. \
                        Auto-generation is disabled."
                                .to_string(),
                        ));
                    }
                }
            };

        // Load or generate E2E encryption keys
        let home = std::env::var("HOME")
            .map(std::path::PathBuf::from)
            .map_err(|_| FoldDbError::Config("HOME environment variable not set".to_string()))?;
        let e2e_key_path = home.join(".fold_db/e2e.key");
        let e2e_keys = crate::crypto::E2eKeys::load_or_generate(&e2e_key_path)
            .await
            .map_err(|e| FoldDbError::Config(format!("Failed to load E2E keys: {}", e)))?;

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
            e2e_keys,
            mutation_preprocessor: MutationPreprocessor::new(),
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
            "FoldNode created successfully with pre-created database"
        );
        Ok(node)
    }

    /// Creates a new FoldNode with a pre-created FoldDB and explicit E2E keys.
    ///
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

        let client = crate::fold_node::SchemaServiceClient::new(&schema_service_url);
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

        let client = crate::fold_node::SchemaServiceClient::new(&schema_service_url);
        client
            .add_schema(schema, std::collections::HashMap::new())
            .await
            .map(|response| response.schema)
    }

    /// Execute a batch of mutations.
    ///
    /// Runs the MutationPreprocessor (keyword extraction) on any mutations
    /// that don't already have `index_terms`, then delegates to FoldDB.
    pub async fn mutate_batch(
        &self,
        mut mutations: Vec<crate::schema::types::operations::Mutation>,
    ) -> FoldDbResult<Vec<String>> {
        // Preprocess: extract keywords for mutations missing index_terms
        self.mutation_preprocessor.preprocess(&mut mutations);

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

        Ok((node_id, security_manager, security_config))
    }
}

impl FoldNode {
    /// Gets the node's private key.
    pub fn get_node_private_key(&self) -> &str {
        &self.private_key
    }

    /// Gets the node's public key.
    pub fn get_node_public_key(&self) -> &str {
        &self.public_key
    }

    /// Get the E2E encryption key for encrypting files before storage.
    pub fn get_encryption_key(&self) -> [u8; 32] {
        self.e2e_keys.encryption_key()
    }

    /// Gets a reference to the security manager.
    pub fn get_security_manager(&self) -> &Arc<SecurityManager> {
        &self.security_manager
    }

    /// Get the unified progress tracker
    /// This is the single source of truth for all job progress (ingestion, indexing, reset, etc.)
    /// Local deployments use Sled storage, cloud deployments use DynamoDB
    pub async fn get_progress_tracker(&self) -> crate::progress::ProgressTracker {
        let db = self.db.lock().await;
        db.get_progress_tracker()
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

    /// Wait for all pending background tasks to complete
    pub async fn wait_for_background_tasks(&self, timeout: std::time::Duration) -> bool {
        let db = self.db.lock().await;
        db.wait_for_background_tasks(timeout).await
    }

    /// Increment pending task count manually
    pub async fn increment_pending_tasks(&self) {
        let db = self.db.lock().await;
        db.increment_pending_tasks();
    }

    /// Decrement pending task count manually
    pub async fn decrement_pending_tasks(&self) {
        let db = self.db.lock().await;
        db.decrement_pending_tasks();
    }

    /// Check if a file has already been ingested by a specific user.
    ///
    /// Returns the ingestion record if found, `None` otherwise.
    pub async fn is_file_ingested(
        &self,
        pub_key: &str,
        file_hash: &str,
    ) -> Option<FileIngestionRecord> {
        let key = format!("file:{}:{}", pub_key, file_hash);
        let db = self.db.lock().await;
        db.db_ops
            .idempotency_store()
            .get_item::<FileIngestionRecord>(&key)
            .await
            .ok()
            .flatten()
    }

    /// Record that a file has been successfully ingested by a user.
    pub async fn mark_file_ingested(
        &self,
        pub_key: &str,
        file_hash: &str,
        record: FileIngestionRecord,
    ) -> FoldDbResult<()> {
        let key = format!("file:{}:{}", pub_key, file_hash);
        let db = self.db.lock().await;
        db.db_ops
            .idempotency_store()
            .put_item(&key, &record)
            .await
            .map_err(|e| FoldDbError::Database(e.to_string()))
    }

    /// Query process results for a given progress_id.
    ///
    /// Returns actual mutation outcomes (schema_name + key_value as stored)
    /// written by the ProcessResultsSubscriber during ingestion.
    pub async fn get_process_results(
        &self,
        progress_id: &str,
    ) -> FoldDbResult<Vec<MutationOutcome>> {
        let db = self.db.lock().await;
        let prefix = format!("{}:mut:", progress_id);
        let items: Vec<(
            String,
            crate::fold_db_core::infrastructure::process_results_subscriber::ProcessMutationResult,
        )> = db
            .db_ops
            .process_results_store()
            .scan_items_with_prefix(&prefix)
            .await
            .map_err(|e| FoldDbError::Database(e.to_string()))?;

        Ok(items
            .into_iter()
            .map(|(key, result)| {
                let mutation_id = key
                    .rsplit(":mut:")
                    .next()
                    .unwrap_or("")
                    .to_string();
                MutationOutcome {
                    mutation_id,
                    schema_name: result.schema_name,
                    key_value: result.key_value,
                }
            })
            .collect())
    }
}

/// A single mutation outcome from the process_results store.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MutationOutcome {
    pub mutation_id: String,
    pub schema_name: String,
    pub key_value: crate::schema::types::KeyValue,
}

/// Metadata stored when a file is successfully ingested by a user.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FileIngestionRecord {
    pub ingested_at: String,
    pub source_folder: Option<String>,
    pub source_file_name: Option<String>,
    pub progress_id: Option<String>,
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::security::Ed25519KeyPair;
    use base64::{engine::general_purpose, Engine as _};
    use tempfile::tempdir;

    #[tokio::test]
    async fn test_node_private_key_generation() {
        let temp_dir = tempdir().unwrap();

        // Generate identity for the test
        let keypair = Ed25519KeyPair::generate().unwrap();
        let pub_key = keypair.public_key_base64();
        let priv_key = keypair.secret_key_base64();

        let config = NodeConfig::new(temp_dir.path().to_path_buf())
            .with_schema_service_url("test://mock")
            .with_identity(&pub_key, &priv_key);

        let node = FoldNode::new(config).await.unwrap();

        // Verify that private and public keys were generated (or rather, loaded correctly)
        let private_key = node.get_node_private_key();
        let public_key = node.get_node_public_key();

        assert!(!private_key.is_empty());
        assert!(!public_key.is_empty());
        assert_ne!(private_key, public_key);

        assert_eq!(private_key, priv_key);
        assert_eq!(public_key, pub_key);

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
                log::warn!(
                    "Failed to parse node_identity.json: {}. Generating new identity.",
                    e
                );
                Ok(None)
            }
        }
    } else {
        Ok(None)
    }
}
