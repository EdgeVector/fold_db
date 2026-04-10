use crate::crypto::E2eKeys;
use crate::db_operations::DbOperations;
use crate::error::{FoldDbError, FoldDbResult};
use crate::fold_db_core::FoldDB;
use crate::storage::config::DatabaseConfig;
use crate::storage::node_config_store::{CloudCredentials, NodeConfigStore};
use crate::storage::SledPool;
use crate::sync::SyncSetup;
use std::sync::Arc;
use tokio::sync::Mutex;

/// Creates a fully initialized FoldDB instance based on the database configuration.
///
/// Always uses local Sled storage. When `cloud_sync` is configured, layers on
/// encrypted S3 sync via the Exemem platform.
pub async fn create_fold_db(
    config: &DatabaseConfig,
    e2e_keys: &E2eKeys,
) -> FoldDbResult<Arc<Mutex<FoldDB>>> {
    create_fold_db_with_auth_refresh(config, e2e_keys, None).await
}

/// Creates a FoldDB instance with an optional auth-refresh callback for the sync engine.
///
/// When cloud sync is enabled, the callback is invoked on 401 errors to obtain
/// fresh credentials (e.g., by re-registering with the Exemem API using the node's
/// Ed25519 keypair). The sync engine retries once after a successful refresh.
pub async fn create_fold_db_with_auth_refresh(
    config: &DatabaseConfig,
    e2e_keys: &E2eKeys,
    auth_refresh: Option<crate::sync::AuthRefreshCallback>,
) -> FoldDbResult<Arc<Mutex<FoldDB>>> {
    let sync_setup = if let Some(cloud) = &config.cloud_sync {
        let path = std::env::var("FOLD_STORAGE_PATH").unwrap_or_else(|_| "data".to_string());
        let mut setup = SyncSetup::from_exemem(&cloud.api_url, &cloud.api_key, &path);
        setup.auth_refresh = auth_refresh;
        Some(setup)
    } else {
        None
    };

    let db = create_local_fold_db(&config.path, e2e_keys, sync_setup).await?;

    // If cloud sync is configured, persist credentials into the Sled config store
    // so future startups can auto-enable sync even from a minimal config.
    if let Some(cloud) = &config.cloud_sync {
        let locked = db.lock().await;
        if let Some(cs) = locked.config_store() {
            let creds = CloudCredentials {
                api_url: cloud.api_url.clone(),
                api_key: cloud.api_key.clone(),
                session_token: None,
                user_hash: None,
            };
            if let Err(e) = cs.set_cloud_config(&creds) {
                log::warn!("failed to persist cloud config to Sled: {e}");
            }
        }
    }

    Ok(db)
}

/// Creates a local Sled-backed FoldDB with optional S3 sync.
///
/// When `sync_setup` is provided, the storage stack becomes:
/// ```text
/// EncryptingNamespacedStore  (E2E AES-256-GCM)
///       |
/// SyncingNamespacedStore     (records ops for S3 sync)
///       |
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

    let pool = Arc::new(SledPool::new(path.to_path_buf()));

    // Create the config store for runtime node configuration
    let config_store = NodeConfigStore::new(Arc::clone(&pool))
        .map_err(|e| FoldDbError::Config(format!("Failed to open config store: {}", e)))?;

    // If no sync_setup provided but Sled has cloud credentials, build sync from Sled
    let sync_setup = if sync_setup.is_none() {
        if let Some(cloud_creds) = config_store.get_cloud_config() {
            let data_dir = path_str;
            log::info!("found cloud credentials in Sled config store — enabling sync");
            Some(SyncSetup::from_exemem(
                &cloud_creds.api_url,
                &cloud_creds.api_key,
                data_dir,
            ))
        } else {
            None
        }
    } else {
        sync_setup
    };

    let base_store: Arc<dyn crate::storage::traits::NamespacedStore> =
        Arc::new(crate::storage::SledNamespacedStore::new(Arc::clone(&pool)));

    // Build the store stack, optionally inserting sync layer
    #[allow(clippy::type_complexity)]
    let (store, sync_engine, sync_interval_ms, enc_store_ref): (
        Arc<dyn crate::storage::traits::NamespacedStore>,
        Option<Arc<crate::sync::SyncEngine>>,
        u64,
        Option<Arc<crate::storage::EncryptingNamespacedStore>>,
    ) = if let Some(setup) = sync_setup {
        let sync_config = setup.config.unwrap_or_default();
        let interval_ms = sync_config.sync_interval_ms;

        let sync_crypto: Arc<dyn crate::crypto::CryptoProvider> = Arc::new(
            crate::crypto::LocalCryptoProvider::from_key(e2e_keys.encryption_key()),
        );
        let http = Arc::new(reqwest::Client::new());
        let s3 = crate::sync::s3::S3Client::new(http.clone());
        let auth = crate::sync::auth::AuthClient::new(http, setup.auth_url, setup.auth);

        let mut engine = crate::sync::SyncEngine::new(
            setup.device_id,
            sync_crypto,
            s3,
            auth,
            base_store.clone(),
            sync_config,
        );
        if let Some(cb) = setup.auth_refresh {
            engine.set_auth_refresh(cb);
        }
        let engine = Arc::new(engine);

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

        // Sled -> SyncingNamespacedStore -> EncryptingNamespacedStore
        let syncing_store = crate::storage::SyncingNamespacedStore::new(base_store, engine.clone());
        let mid_store: Arc<dyn crate::storage::traits::NamespacedStore> = Arc::new(syncing_store);

        let crypto = Arc::new(crate::crypto::LocalCryptoProvider::from_key(
            e2e_keys.encryption_key(),
        ));
        let enc_store = Arc::new(crate::storage::EncryptingNamespacedStore::new(
            mid_store, crypto, true,
        ));

        (
            enc_store.clone() as Arc<dyn crate::storage::traits::NamespacedStore>,
            Some(engine),
            interval_ms,
            Some(enc_store),
        )
    } else {
        // No sync — Sled -> EncryptingNamespacedStore
        let crypto = Arc::new(crate::crypto::LocalCryptoProvider::from_key(
            e2e_keys.encryption_key(),
        ));
        let enc_store = Arc::new(crate::storage::EncryptingNamespacedStore::new(
            base_store, crypto, true,
        ));
        (
            enc_store.clone() as Arc<dyn crate::storage::traits::NamespacedStore>,
            None,
            0,
            Some(enc_store),
        )
    };

    let db_ops = DbOperations::from_namespaced_store(store)
        .await
        .map_err(|e| FoldDbError::Config(e.to_string()))?;

    let job_store = crate::progress::create_tracker_with_sled(Arc::clone(&pool));

    let mut fold_db = FoldDB::initialize_from_db_ops_with_sled(
        Arc::new(db_ops),
        path_str,
        Some(job_store),
        "local".to_string(),
        Some(pool),
        enc_store_ref,
    )
    .await
    .map_err(|e| FoldDbError::Config(e.to_string()))?;

    fold_db.set_config_store(config_store);

    if let Some(engine) = sync_engine {
        fold_db.set_sync_engine(engine).await;
        fold_db.start_sync(sync_interval_ms);
    }

    Ok(Arc::new(Mutex::new(fold_db)))
}
