use crate::crypto::E2eKeys;
use crate::db_operations::DbOperations;
use crate::error::{FoldDbError, FoldDbResult};
use crate::fold_db_core::FoldDB;
use crate::security::Ed25519KeyPair;
use crate::storage::config::DatabaseConfig;
use crate::storage::node_config_store::NodeConfigStore;
use crate::storage::SledPool;
use crate::sync::SyncSetup;
use std::sync::Arc;

/// Creates a fully initialized FoldDB instance based on the database configuration.
///
/// Always uses local Sled storage. When `cloud_sync` is configured, layers on
/// encrypted S3 sync via the Exemem platform.
///
/// `signer` is the Ed25519 keypair used to sign molecule mutations. Callers are
/// responsible for loading and validating it from the node's persistent identity
/// before calling this factory — passing a freshly-generated keypair on every
/// boot would produce signatures that do not match the node's public key.
pub async fn create_fold_db(
    config: &DatabaseConfig,
    e2e_keys: &E2eKeys,
    signer: Arc<Ed25519KeyPair>,
) -> FoldDbResult<Arc<FoldDB>> {
    create_fold_db_with_auth_refresh(config, e2e_keys, signer, None).await
}

/// Creates a FoldDB instance with an optional auth-refresh callback for the sync engine.
///
/// When cloud sync is enabled, the callback is invoked on 401 errors to obtain
/// fresh credentials (e.g., by re-registering with the Exemem API using the node's
/// Ed25519 keypair). The sync engine retries once after a successful refresh.
pub async fn create_fold_db_with_auth_refresh(
    config: &DatabaseConfig,
    e2e_keys: &E2eKeys,
    signer: Arc<Ed25519KeyPair>,
    auth_refresh: Option<crate::sync::AuthRefreshCallback>,
) -> FoldDbResult<Arc<FoldDB>> {
    create_fold_db_with_pool_and_auth_refresh(config, e2e_keys, signer, auth_refresh, None).await
}

/// Like [`create_fold_db_with_auth_refresh`], but accepts an optional pre-existing
/// [`SledPool`] to reuse across FoldDB lifetimes.
///
/// Why reuse matters: each `SledPool` holds an exclusive OS file lock on the Sled
/// database directory. Two pools pointing at the same path cannot both be open at
/// the same time. When a caller (e.g. NodeManager) invalidates and recreates the
/// FoldDB for the same path — for instance, after `/api/auth/register` activates
/// cloud sync — handing the same pool into the new instance avoids a
/// `WouldBlock` race where the old pool's Sled handle is still closing while the
/// new instance tries to open the same path.
pub async fn create_fold_db_with_pool_and_auth_refresh(
    config: &DatabaseConfig,
    e2e_keys: &E2eKeys,
    signer: Arc<Ed25519KeyPair>,
    auth_refresh: Option<crate::sync::AuthRefreshCallback>,
    pool: Option<Arc<SledPool>>,
) -> FoldDbResult<Arc<FoldDB>> {
    let sync_setup = if let Some(cloud) = &config.cloud_sync {
        let path = std::env::var("FOLD_STORAGE_PATH").unwrap_or_else(|_| "data".to_string());
        let mut setup = SyncSetup::from_exemem(&cloud.api_url, &cloud.api_key, &path);
        setup.auth_refresh = auth_refresh;
        Some(setup)
    } else {
        None
    };

    let db = create_local_fold_db(&config.path, e2e_keys, signer, sync_setup, pool).await?;

    // If cloud sync is configured, persist ONLY api_url and user_hash to Sled.
    // API keys and session tokens are per-device secrets stored in credentials.json
    // by the fold_db_node layer — they must NOT be written to Sled (which syncs).
    if let Some(cloud) = &config.cloud_sync {
        if let Some(cs) = db.config_store() {
            if let Err(e) = cs.set("cloud:api_url", &cloud.api_url) {
                tracing::warn!("failed to persist cloud api_url to Sled: {e}");
            }
            if let Some(ref uh) = cloud.user_hash {
                if let Err(e) = cs.set("cloud:user_hash", uh) {
                    tracing::warn!("failed to persist cloud user_hash to Sled: {e}");
                }
            }
            if let Err(e) = cs.set("cloud:enabled", "true") {
                tracing::warn!("failed to persist cloud:enabled to Sled: {e}");
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
    signer: Arc<Ed25519KeyPair>,
    sync_setup: Option<SyncSetup>,
    injected_pool: Option<Arc<SledPool>>,
) -> FoldDbResult<Arc<FoldDB>> {
    let path_str = path
        .to_str()
        .ok_or_else(|| FoldDbError::Config("Invalid storage path".to_string()))?;

    let pool = match injected_pool {
        Some(pool) => pool,
        None => {
            let pool = Arc::new(SledPool::new(path.to_path_buf()));
            pool.start_idle_reaper(std::time::Duration::from_secs(30));
            pool
        }
    };

    // Create the config store for runtime node configuration.
    // Pass the E2E encryption key so sensitive fields (node identity
    // private key) are encrypted at rest via AES-256-GCM.
    let config_store =
        NodeConfigStore::with_crypto_key(Arc::clone(&pool), Some(e2e_keys.encryption_key()))
            .map_err(|e| FoldDbError::Config(format!("Failed to open config store: {}", e)))?;

    // Use the sync_setup provided by the caller. The caller (fold_db_node) is
    // responsible for loading the API key from the per-device credentials file.
    // We do NOT read API keys from Sled (which syncs across devices).

    let base_store: Arc<dyn crate::storage::traits::NamespacedStore> =
        Arc::new(crate::storage::SledNamespacedStore::new(Arc::clone(&pool)));

    // The signer was loaded and validated by the caller (in production,
    // from the node's persistent identity). We share the same Arc with
    // both SyncEngine (for signing merged molecules during replay) and
    // FoldDB (used by MutationManager for signing local writes) so
    // merged writes trace to the same node identity as direct writes.

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
        // trace-egress: propagate (shared with skip-s3 — see docs/observability/egress-classification-notes.md)
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
            Arc::clone(&signer),
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
            tracing::info!("empty local database with sync enabled — bootstrapping from cloud");
            match engine.bootstrap().await {
                Ok(seq) => tracing::info!("bootstrap complete: restored to seq {seq}"),
                Err(e) => tracing::warn!("bootstrap failed (starting fresh): {e}"),
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

    // Initialize face detection processor if the feature is enabled
    #[cfg(feature = "face-detection")]
    {
        // FOLDDB_HOME is the node's root dir; models go in {FOLDDB_HOME}/models/
        // path_str is the Sled data path ({FOLDDB_HOME}/data), so go up one level
        let home_path = std::env::var("FOLDDB_HOME")
            .map(std::path::PathBuf::from)
            .unwrap_or_else(|_| {
                std::path::Path::new(path_str)
                    .parent()
                    .unwrap_or(std::path::Path::new(path_str))
                    .to_path_buf()
            });
        if let Some(mgr) = db_ops.native_index_manager() {
            let processor = std::sync::Arc::new(
                crate::db_operations::native_index::face::OnnxFaceProcessor::new(&home_path),
            );
            mgr.set_face_processor(processor);
            tracing::info!(
                "Face detection processor initialized (models at {}/models/)",
                home_path.display()
            );
        }
    }

    let job_store = crate::progress::create_tracker_with_sled(Arc::clone(&pool));

    let fold_db = FoldDB::initialize_from_db_ops_with_sled(
        Arc::new(db_ops),
        path_str,
        Some(job_store),
        "local".to_string(),
        Some(pool),
        enc_store_ref,
        signer,
    )
    .await
    .map_err(|e| FoldDbError::Config(e.to_string()))?;

    fold_db.set_config_store(config_store);

    if let Some(engine) = sync_engine {
        fold_db.set_sync_engine(engine).await;
        fold_db.start_sync(sync_interval_ms);
    }

    Ok(Arc::new(fold_db))
}
