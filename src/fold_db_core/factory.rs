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
use crate::sync::SyncSetup;
use std::sync::Arc;
use tokio::sync::Mutex;

#[cfg(feature = "aws-backend")]
use crate::log_feature;

/// Creates a fully initialized FoldDB instance based on the database configuration.
///
/// - **Local**: Sled backend with E2E encryption.
/// - **Exemem**: Local Sled + E2E encryption + S3 sync via the Exemem platform.
///   Uses the same `api_url` and `api_key` for sync auth — no extra config needed.
/// - **Cloud** (aws-backend feature): DynamoDB backend with E2E encryption.
pub async fn create_fold_db(
    config: &DatabaseConfig,
    e2e_keys: &E2eKeys,
) -> FoldDbResult<Arc<Mutex<FoldDB>>> {
    match config {
        DatabaseConfig::Local { path } => create_local_fold_db(path, e2e_keys, None).await,
        DatabaseConfig::Exemem {
            api_url, api_key, ..
        } => {
            // Exemem mode: local Sled + S3 sync via the Exemem platform.
            // The sync auth Lambda shares the same API URL and API key.
            let path = std::path::PathBuf::from(
                std::env::var("FOLD_STORAGE_PATH").unwrap_or_else(|_| "data".to_string()),
            );
            let data_dir = path
                .to_str()
                .ok_or_else(|| FoldDbError::Config("Invalid storage path".to_string()))?;
            let sync_setup = SyncSetup::from_exemem(api_url, api_key, data_dir);
            create_local_fold_db(&path, e2e_keys, Some(sync_setup)).await
        }
        #[cfg(feature = "aws-backend")]
        DatabaseConfig::Cloud(cloud_config) => create_cloud_fold_db(cloud_config, e2e_keys).await,
    }
}

/// Creates a local Sled-backed FoldDB with optional S3 sync.
///
/// When `sync_setup` is provided, the storage stack becomes:
/// ```text
/// EncryptingNamespacedStore  (E2E AES-256-GCM)
///       ↓
/// SyncingNamespacedStore     (records ops for S3 sync)
///       ↓
/// SledNamespacedStore        (local persistence)
/// ```
async fn create_local_fold_db(
    path: &std::path::Path,
    e2e_keys: &E2eKeys,
    sync_setup: Option<SyncSetup>,
) -> FoldDbResult<Arc<Mutex<FoldDB>>> {
    let path_str = path
        .to_str()
        .ok_or_else(|| FoldDbError::Config("Invalid storage path".to_string()))?;

    let db = sled::open(path)
        .map_err(|e| FoldDbError::Config(format!("Failed to open sled database: {}", e)))?;
    let progress_tree = db
        .open_tree("progress")
        .map_err(|e| FoldDbError::Config(format!("Failed to open progress tree: {}", e)))?;

    // Retain the raw sled handle so FoldDB::sled_db() can return it.
    // This is needed by org operations (which store memberships directly in
    // Sled trees) and by configure_org_sync_if_needed().
    let raw_sled = db.clone();

    let base_store: Arc<dyn crate::storage::traits::NamespacedStore> =
        Arc::new(crate::storage::SledNamespacedStore::new(db));

    // Build the store stack, optionally inserting sync layer
    let (store, sync_engine, sync_interval_ms): (
        Arc<dyn crate::storage::traits::NamespacedStore>,
        Option<Arc<crate::sync::SyncEngine>>,
        u64,
    ) = if let Some(setup) = sync_setup {
        let sync_config = setup.config.unwrap_or_default();
        let interval_ms = sync_config.sync_interval_ms;

        let sync_crypto: Arc<dyn crate::crypto::CryptoProvider> = Arc::new(
            crate::crypto::LocalCryptoProvider::from_key(e2e_keys.encryption_key()),
        );
        let http = Arc::new(reqwest::Client::new());
        let s3 = crate::sync::s3::S3Client::new(http.clone());
        let auth = crate::sync::auth::AuthClient::new(http, setup.auth_url, setup.auth);

        let engine = Arc::new(crate::sync::SyncEngine::new(
            setup.device_id,
            sync_crypto,
            s3,
            auth,
            base_store.clone(),
            sync_config,
        ));

        // Bootstrap from B2 if the local database is empty (new device connecting
        // to an existing user database — like a password manager on a new device).
        let namespaces = base_store.list_namespaces().await.unwrap_or_default();
        let has_user_data = namespaces.iter().any(|ns| ns != "__sled__default");
        if !has_user_data {
            log::info!("empty local database with sync enabled — bootstrapping from cloud");
            match engine.bootstrap().await {
                Ok(seq) => log::info!("bootstrap complete: restored to seq {seq}"),
                Err(e) => log::warn!("bootstrap failed (starting fresh): {e}"),
            }
        }

        // Sled → SyncingNamespacedStore → EncryptingNamespacedStore
        let syncing_store = crate::storage::SyncingNamespacedStore::new(base_store, engine.clone());
        let mid_store: Arc<dyn crate::storage::traits::NamespacedStore> = Arc::new(syncing_store);

        let crypto = Arc::new(crate::crypto::LocalCryptoProvider::from_key(
            e2e_keys.encryption_key(),
        ));
        let enc_store = crate::storage::EncryptingNamespacedStore::new(mid_store, crypto, true);

        (Arc::new(enc_store), Some(engine), interval_ms)
    } else {
        // No sync — Sled → EncryptingNamespacedStore
        let crypto = Arc::new(crate::crypto::LocalCryptoProvider::from_key(
            e2e_keys.encryption_key(),
        ));
        let enc_store = crate::storage::EncryptingNamespacedStore::new(base_store, crypto, true);
        (Arc::new(enc_store), None, 0)
    };

    let db_ops = DbOperations::from_namespaced_store(store)
        .await
        .map_err(|e| FoldDbError::Config(e.to_string()))?;

    let job_store = crate::progress::create_tracker_with_sled(progress_tree);

    let mut fold_db = FoldDB::initialize_from_db_ops_with_sled(
        Arc::new(db_ops),
        path_str,
        Some(job_store),
        "local".to_string(),
        Some(raw_sled),
    )
    .await
    .map_err(|e| FoldDbError::Config(e.to_string()))?;

    if let Some(engine) = sync_engine {
        fold_db.set_sync_engine(engine);
        fold_db.start_sync(sync_interval_ms);
    }

    Ok(Arc::new(Mutex::new(fold_db)))
}

#[cfg(feature = "aws-backend")]
async fn create_cloud_fold_db(
    cloud_config: &crate::storage::config::CloudConfig,
    e2e_keys: &E2eKeys,
) -> FoldDbResult<Arc<Mutex<FoldDB>>> {
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

    let user_id = cloud_config
        .user_id
        .clone()
        .ok_or_else(|| FoldDbError::Config("Missing user_id for Cloud config".to_string()))?;

    let base_store: Arc<dyn crate::storage::traits::NamespacedStore> =
        Arc::new(crate::storage::CloudNamespacedStore::new(
            client.clone(),
            resolver,
            cloud_config.auto_create,
        ));

    let e2e_crypto = Arc::new(crate::crypto::LocalCryptoProvider::from_key(
        e2e_keys.encryption_key(),
    ));
    let e2e_store = crate::storage::EncryptingNamespacedStore::new(base_store, e2e_crypto, true);
    let final_store = Arc::new(e2e_store) as Arc<dyn crate::storage::traits::NamespacedStore>;

    let db_ops = Arc::new(
        DbOperations::from_namespaced_store(final_store)
            .await
            .map_err(|e| {
                FoldDbError::Config(format!("Failed to initialize DynamoDB backend: {}", e))
            })?,
    );

    let job_store: Option<Arc<dyn JobStore>> = {
        let table_name = cloud_config.tables.process.clone();
        let store = DynamoDbJobStore::new(client.clone(), table_name);
        Some(Arc::new(store))
    };

    Ok(Arc::new(Mutex::new(
        FoldDB::new_with_components(db_ops, "data", job_store, Some(user_id))
            .await
            .map_err(|e| FoldDbError::Config(e.to_string()))?,
    )))
}
