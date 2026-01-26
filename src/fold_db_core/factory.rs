#[cfg(feature = "aws-backend")]
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
pub async fn create_fold_db(config: &DatabaseConfig) -> FoldDbResult<Arc<Mutex<FoldDB>>> {
    match config {
        DatabaseConfig::Local { path } => {
            let path_str = path
                .to_str()
                .ok_or_else(|| FoldDbError::Config("Invalid storage path".to_string()))?;

            // For local backend, we use the simple new() constructor which handles
            // Sled initialization and uses InMemoryProgressStore
            Ok(Arc::new(Mutex::new(
                FoldDB::new(path_str)
                    .await
                    .map_err(|e| FoldDbError::Config(e.to_string()))?,
            )))
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
                    "transforms".to_string(),
                    cloud_config.tables.transforms.clone(),
                ),
                (
                    "orchestrator_state".to_string(),
                    cloud_config.tables.orchestrator.clone(),
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
                    "transform_queue_tree".to_string(),
                    cloud_config.tables.transform_queue.clone(),
                ),
                (
                    "native_index".to_string(),
                    cloud_config.tables.native_index.clone(),
                ),
                ("process".to_string(), cloud_config.tables.process.clone()),
            ]);

            let resolver = TableNameResolver::Explicit(map);

            // Require user_id for DynamoDB backend
            let user_id = cloud_config.user_id.clone().ok_or_else(|| {
                FoldDbError::Config("Missing user_id for Cloud config".to_string())
            })?;

            let db_ops = Arc::new(
                DbOperations::from_cloud_flexible(
                    client.clone(),
                    resolver,
                    cloud_config.auto_create,
                    user_id.clone(),
                )
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
