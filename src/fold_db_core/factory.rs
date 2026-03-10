use crate::crypto::E2eKeys;
use crate::db_operations::DbOperations;
use crate::error::{FoldDbError, FoldDbResult};
use crate::fold_db_core::FoldDB;
#[cfg(feature = "aws-backend")]
use crate::logging::features::LogFeature;
#[cfg(feature = "aws-backend")]
use crate::progress::{DynamoDbProgressStore as DynamoDbJobStore, ProgressStore as JobStore};
use crate::storage::config::DatabaseConfig;
#[cfg(feature = "aws-backend")]
use crate::storage::TableNameResolver;
use std::sync::Arc;
use tokio::sync::Mutex;

#[cfg(feature = "aws-backend")]
use crate::log_feature;

/// Creates a fully initialized FoldDB instance based on the database configuration.
///
/// This factory handles the creation of backend-specific components like:
/// - Storage operations (DbOperations)
/// - Progress tracking (ProgressStore)
/// - Connection pooling and configuration
/// - E2E encryption (atom content via EncryptingNamespacedStore, index keywords via HMAC)
pub async fn create_fold_db(
    config: &DatabaseConfig,
    e2e_keys: &E2eKeys,
) -> FoldDbResult<Arc<Mutex<FoldDB>>> {
    match config {
        DatabaseConfig::Local { path } => {
            let path_str = path
                .to_str()
                .ok_or_else(|| FoldDbError::Config("Invalid storage path".to_string()))?;

            // Open sled database
            let db = sled::open(path)
                .map_err(|e| FoldDbError::Config(format!("Failed to open sled database: {}", e)))?;
            let progress_tree = db
                .open_tree("progress")
                .map_err(|e| FoldDbError::Config(format!("Failed to open progress tree: {}", e)))?;

            // Build base namespaced store from sled
            let base_store: Arc<dyn crate::storage::traits::NamespacedStore> =
                Arc::new(crate::storage::SledNamespacedStore::new(db));

            // Wrap with E2E encryption (atom content via AES-256-GCM)
            let crypto = Arc::new(
                crate::crypto::LocalCryptoProvider::from_key(e2e_keys.encryption_key()),
            );
            let enc_store = crate::storage::EncryptingNamespacedStore::new(
                base_store,
                crypto,
                true, // migration_mode: tolerate existing plaintext data
            );
            let store = Arc::new(enc_store) as Arc<dyn crate::storage::traits::NamespacedStore>;

            let db_ops = DbOperations::from_namespaced_store(store)
            .await
            .map_err(|e| FoldDbError::Config(e.to_string()))?;

            let job_store = crate::progress::create_tracker_with_sled(progress_tree);

            Ok(Arc::new(Mutex::new(
                FoldDB::initialize_from_db_ops(
                    Arc::new(db_ops),
                    path_str,
                    Some(job_store),
                    "local".to_string(),
                )
                .await
                .map_err(|e| FoldDbError::Config(e.to_string()))?,
            )))
        }
        DatabaseConfig::Exemem { api_url, api_key } => {
            let auth = crate::storage::ExememAuth::ApiKey(api_key.clone());
            let store: Arc<dyn crate::storage::traits::NamespacedStore> = Arc::new(
                crate::storage::ExememNamespacedStore::new(api_url.clone(), auth),
            );
            create_fold_db_from_store(store, e2e_keys).await
        }
        #[cfg(feature = "aws-backend")]
        DatabaseConfig::Cloud(cloud_config) => {
            log_feature!(
                LogFeature::Database,
                info,
                "Initializing Cloud backend: region={}",
                cloud_config.region
            );

            let aws_config = aws_config::defaults(aws_config::BehaviorVersion::latest())
                .region(aws_sdk_dynamodb::config::Region::new(
                    cloud_config.region.clone(),
                ))
                .load()
                .await;

            let client = aws_sdk_dynamodb::Client::new(&aws_config);

            // Convert ExplicitTables to TableNameResolver
            let map = std::collections::HashMap::from([
                ("main".to_string(), cloud_config.tables.main.clone()),
                ("metadata".to_string(), cloud_config.tables.metadata.clone()),
                (
                    "node_id_schema_permissions".to_string(),
                    cloud_config.tables.permissions.clone(),
                ),
                (
                    "schema_states".to_string(),
                    cloud_config.tables.schema_states.clone(),
                ),
                ("schemas".to_string(), cloud_config.tables.schemas.clone()),
                (
                    "public_keys".to_string(),
                    cloud_config.tables.public_keys.clone(),
                ),
                (
                    "native_index".to_string(),
                    cloud_config.tables.native_index.clone(),
                ),
                ("process".to_string(), cloud_config.tables.process.clone()),
                (
                    "idempotency".to_string(),
                    cloud_config.tables.idempotency.clone(),
                ),
            ]);

            let resolver = TableNameResolver::Explicit(map);

            // Require user_id for DynamoDB backend
            let user_id = cloud_config.user_id.clone().ok_or_else(|| {
                FoldDbError::Config("Missing user_id for Cloud config".to_string())
            })?;

            // Build the base namespaced store
            let base_store: Arc<dyn crate::storage::traits::NamespacedStore> =
                Arc::new(crate::storage::CloudNamespacedStore::new(
                    client.clone(),
                    resolver,
                    cloud_config.auto_create,
                ));

            // Wrap with E2E encryption (atom content via AES-256-GCM)
            let e2e_crypto = Arc::new(
                crate::crypto::LocalCryptoProvider::from_key(e2e_keys.encryption_key()),
            );
            let e2e_store = crate::storage::EncryptingNamespacedStore::new(
                base_store,
                e2e_crypto,
                true, // migration_mode: tolerate existing plaintext data
            );
            let final_store =
                Arc::new(e2e_store) as Arc<dyn crate::storage::traits::NamespacedStore>;

            let db_ops = Arc::new(
                DbOperations::from_namespaced_store(final_store)
                    .await
                    .map_err(|e| {
                        FoldDbError::Config(format!("Failed to initialize DynamoDB backend: {}", e))
                    })?,
            );

            // Generate path string for compatibility
            let path_str = "data";

            // Initialize JobStore (Generic)
            let job_store: Option<Arc<dyn JobStore>> = {
                let table_name = cloud_config.tables.process.clone();
                let store = DynamoDbJobStore::new(client.clone(), table_name);
                Some(Arc::new(store))
            };

            // Use the new constructor that accepts components
            Ok(Arc::new(Mutex::new(
                FoldDB::new_with_components(db_ops, path_str, job_store, Some(user_id))
                    .await
                    .map_err(|e| FoldDbError::Config(e.to_string()))?,
            )))
        }
    }
}

/// Creates a FoldDB instance from a pre-built NamespacedStore.
///
/// This is the generic factory for use when the caller already has a
/// NamespacedStore (e.g. `ExememNamespacedStore` backed by the Storage API).
/// It wraps the store with E2E encryption and initialises DbOperations.
pub async fn create_fold_db_from_store(
    store: Arc<dyn crate::storage::traits::NamespacedStore>,
    e2e_keys: &E2eKeys,
) -> FoldDbResult<Arc<Mutex<FoldDB>>> {
    let crypto = Arc::new(crate::crypto::LocalCryptoProvider::from_key(
        e2e_keys.encryption_key(),
    ));
    let enc_store = crate::storage::EncryptingNamespacedStore::new(store, crypto, true);
    let final_store = Arc::new(enc_store) as Arc<dyn crate::storage::traits::NamespacedStore>;

    let db_ops = Arc::new(
        DbOperations::from_namespaced_store(final_store)
            .await
            .map_err(|e| FoldDbError::Config(e.to_string()))?,
    );

    Ok(Arc::new(Mutex::new(
        FoldDB::new_with_components(db_ops, "remote", None, None)
            .await
            .map_err(|e| FoldDbError::Config(e.to_string()))?,
    )))
}
