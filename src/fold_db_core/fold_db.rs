//! FoldDB Core - Main database coordinator
//!
//! This module contains the main FoldDB struct that manages schemas, permissions, and data storage.

// Standard library imports
use std::path::Path;
use std::sync::Arc;

// External crate imports
use log::{debug, info};

// Internal crate imports
use crate::db_operations::{DbOperations, IndexResult};
use crate::logging::features::{log_feature, LogFeature};
use crate::schema::{SchemaCore, SchemaError};
use crate::storage::SledPool;
use crate::storage::StorageError;

// Infrastructure components that are used internally
use super::infrastructure::{AsyncMessageBus, EventMonitor};
use super::mutation_manager::MutationManager;
use super::orchestration::index_status::IndexStatusTracker;
use super::query::QueryExecutor;
use crate::progress::ProgressStore as JobStore;
use crate::progress::ProgressTracker;

/// The main database coordinator that manages schemas, permissions, and data storage.
pub struct FoldDB {
    pub schema_manager: Arc<SchemaCore>,
    /// Shared database operations with storage abstraction
    pub db_ops: Arc<DbOperations>,
    /// SledPool for on-demand Sled access (e.g., org operations, config store).
    /// Only present when using the Sled backend.
    sled_pool: Option<Arc<SledPool>>,
    /// Query executor for handling all query operations
    pub query_executor: QueryExecutor,
    /// Message bus for event-driven communication (held for Arc lifetime)
    pub message_bus: Arc<AsyncMessageBus>,
    /// Event monitor for system-wide observability
    pub event_monitor: Arc<EventMonitor>,
    /// Mutation manager for handling all mutation operations
    pub mutation_manager: MutationManager,
    /// Tracker for pending background tasks
    pub pending_tasks: Arc<super::infrastructure::pending_task_tracker::PendingTaskTracker>,
    /// Unified progress tracker for all job types (ingestion, indexing, etc.)
    /// This is the single source of truth for progress - local uses Sled, cloud uses DynamoDB
    pub progress_tracker: ProgressTracker,
    /// Optional sync engine for S3 replication.
    /// Present when sync is configured (local mode only).
    sync_engine: Option<Arc<crate::sync::SyncEngine>>,
    /// Handle for the background sync timer task.
    sync_task: Option<tokio::task::JoinHandle<()>>,
    /// Optional reference to the encrypting store for org crypto registration.
    encrypting_store: Option<Arc<crate::storage::EncryptingNamespacedStore>>,
    /// Optional Sled-backed configuration store for runtime node config.
    config_store: Option<crate::storage::NodeConfigStore>,
}

impl FoldDB {
    /// Retrieves or generates and persists the node identifier.
    pub async fn get_node_id(&self) -> Result<String, crate::storage::StorageError> {
        self.db_ops
            .get_node_id()
            .await
            .map_err(|e| crate::storage::StorageError::BackendError(e.to_string()))
    }

    /// Returns a reference to the SledPool, if available.
    /// This is used by modules that need direct sled tree access (e.g., org operations).
    pub fn sled_pool(&self) -> Option<&Arc<SledPool>> {
        self.sled_pool.as_ref()
    }

    /// Graceful async shutdown: flush pending sync, stop background timer, then flush storage.
    pub async fn shutdown(&mut self) -> Result<(), StorageError> {
        log_feature!(
            LogFeature::Database,
            info,
            "Shutting down FoldDB: flushing sync and storage"
        );
        if let Err(e) = self.stop_sync().await {
            log::warn!("sync flush on shutdown failed: {e}");
        }
        self.flush().await
    }

    /// Set the sync engine (called by the factory when sync is configured).
    /// Also registers the schema reloader callback so SchemaCore's in-memory
    /// cache is refreshed after sync replays schema entries into Sled.
    pub async fn set_sync_engine(&mut self, engine: Arc<crate::sync::SyncEngine>) {
        let schema_mgr = Arc::clone(&self.schema_manager);
        engine
            .set_schema_reloader(Arc::new(move || {
                let mgr = Arc::clone(&schema_mgr);
                Box::pin(async move {
                    mgr.reload_from_store()
                        .await
                        .map_err(|e| format!("SchemaCore reload failed: {e}"))
                })
            }))
            .await;

        // Register embedding reloader so the in-memory EmbeddingIndex is
        // refreshed after sync replays native_index entries into Sled.
        if let Some(nim) = self.db_ops.native_index_manager() {
            let embedding_index = nim.clone();
            engine
                .set_embedding_reloader(Arc::new(move || {
                    let idx = embedding_index.clone();
                    Box::pin(async move {
                        let count = idx.reload_embeddings().await;
                        Ok(count)
                    })
                }))
                .await;
        }

        self.sync_engine = Some(engine);
    }

    /// Start the background sync timer.
    ///
    /// Spawns a tokio task that calls `sync()` every `interval_ms` when the
    /// engine is dirty. Does nothing if no sync engine is configured.
    pub fn start_sync(&mut self, interval_ms: u64) {
        let engine = match &self.sync_engine {
            Some(e) => Arc::clone(e),
            None => return,
        };
        let db_ops = Arc::clone(&self.db_ops);
        let sled_pool = self.sled_pool.clone();

        let handle = tokio::spawn(async move {
            let interval = tokio::time::Duration::from_millis(interval_ms);
            loop {
                tokio::time::sleep(interval).await;
                // Always run sync — even without pending writes, we need to
                // download org data from other members.
                let has_pending = engine.state().await == crate::sync::SyncState::Dirty;
                let has_orgs = engine.has_org_sync().await;
                if has_pending || has_orgs {
                    if let Err(e) = engine.sync().await {
                        if let crate::sync::SyncError::OrgMembershipRevoked(ref org_hash) = e {
                            log::warn!("🚨 SYSTEM ALERT: You have been removed from organization (hash: {}) by an administrator. Proceeding to securely purge all locally cached copies of its data and schema to prevent orphans.", org_hash);

                            // 1. Delete membership structure locally (if running on Sled backend)
                            if let Some(pool) = &sled_pool {
                                let _ = crate::org::operations::delete_org(pool, org_hash).map_err(
                                    |err| log::error!("Failed to delete org structure: {}", err),
                                );
                            }

                            // 2. Erase the orphaned physical footprints in local DB
                            let _ = db_ops
                                .purge_org_data(org_hash)
                                .await
                                .map_err(|err| log::error!("Failed to purge org data: {}", err));
                        } else {
                            log::warn!("sync cycle failed: {e}");
                        }
                    }
                }
            }
        });

        self.sync_task = Some(handle);
    }

    /// Force an immediate sync (e.g. on shutdown).
    pub async fn force_sync(&self) -> Result<(), crate::sync::SyncError> {
        if let Some(engine) = &self.sync_engine {
            engine.sync().await?;
        }
        Ok(())
    }

    /// Stop the background sync timer and run a final sync.
    pub async fn stop_sync(&mut self) -> Result<(), crate::sync::SyncError> {
        if let Some(handle) = self.sync_task.take() {
            handle.abort();
        }
        self.force_sync().await
    }

    /// Get the sync engine state, if sync is configured.
    pub async fn sync_state(&self) -> Option<crate::sync::SyncState> {
        match &self.sync_engine {
            Some(engine) => Some(engine.state().await),
            None => None,
        }
    }

    /// Get a full sync status snapshot, if sync is configured.
    pub async fn sync_status(&self) -> Option<crate::sync::SyncStatus> {
        match &self.sync_engine {
            Some(engine) => Some(engine.status().await),
            None => None,
        }
    }

    /// Get the number of pending (unsynced) log entries.
    /// Returns None if sync is not configured.
    pub async fn sync_pending_count(&self) -> Option<usize> {
        match &self.sync_engine {
            Some(engine) => Some(engine.pending_count().await),
            None => None,
        }
    }

    /// Returns true if the sync engine is configured.
    pub fn is_sync_enabled(&self) -> bool {
        self.sync_engine.is_some()
    }

    /// Returns a reference to the sync engine, if configured.
    ///
    /// Used by fold_db_node to call `configure_org_sync()` after node startup.
    pub fn sync_engine(&self) -> Option<&Arc<crate::sync::SyncEngine>> {
        self.sync_engine.as_ref()
    }

    /// Returns a reference to the Sled-backed config store, if available.
    pub fn config_store(&self) -> Option<&crate::storage::NodeConfigStore> {
        self.config_store.as_ref()
    }

    /// Set the Sled-backed config store (called by the factory).
    pub fn set_config_store(&mut self, store: crate::storage::NodeConfigStore) {
        self.config_store = Some(store);
    }

    /// Start the sync engine on an existing FoldDB instance at runtime.
    /// Called when cloud credentials are written to Sled and sync needs to activate.
    pub async fn start_sync_engine_runtime(
        &mut self,
        api_url: &str,
        api_key: &str,
        data_dir: &str,
        e2e_keys: &crate::crypto::E2eKeys,
        auth_refresh: Option<crate::sync::AuthRefreshCallback>,
    ) -> crate::error::FoldDbResult<()> {
        use crate::error::FoldDbError;

        if self.sync_engine.is_some() {
            return Ok(()); // already running
        }

        let pool = self
            .sled_pool
            .as_ref()
            .ok_or_else(|| FoldDbError::Config("No sled pool for sync engine".to_string()))?
            .clone();

        let mut setup = crate::sync::SyncSetup::from_exemem(api_url, api_key, data_dir);
        setup.auth_refresh = auth_refresh.clone();

        let sync_config = setup.config.unwrap_or_default();
        let interval_ms = sync_config.sync_interval_ms;

        let sync_crypto: Arc<dyn crate::crypto::CryptoProvider> = Arc::new(
            crate::crypto::LocalCryptoProvider::from_key(e2e_keys.encryption_key()),
        );

        let base_store: Arc<dyn crate::storage::traits::NamespacedStore> =
            Arc::new(crate::storage::SledNamespacedStore::new(pool));

        let http = Arc::new(reqwest::Client::new());
        let s3 = crate::sync::s3::S3Client::new(http.clone());
        let auth_client = crate::sync::auth::AuthClient::new(http, setup.auth_url, setup.auth);

        let mut engine = crate::sync::SyncEngine::new(
            setup.device_id,
            sync_crypto,
            s3,
            auth_client,
            base_store,
            sync_config,
        );
        if let Some(cb) = auth_refresh {
            engine.set_auth_refresh(cb);
        }
        let engine = Arc::new(engine);

        self.set_sync_engine(engine).await;
        self.start_sync(interval_ms);

        Ok(())
    }

    /// Register a crypto provider for org-scoped data encryption.
    ///
    /// After this call, storage keys starting with `{org_hash}:` will be
    /// encrypted/decrypted with this provider instead of the node's personal key.
    /// This enables org members to read each other's shared data.
    pub async fn register_org_crypto(
        &self,
        org_hash: &str,
        crypto: Arc<dyn crate::crypto::CryptoProvider>,
    ) {
        if let Some(enc_store) = &self.encrypting_store {
            enc_store
                .register_org_crypto(org_hash.to_string(), crypto)
                .await;
        }
    }

    /// Creates a new FoldDB instance with the specified storage path.
    /// All initializations happen here. This is the main entry point for the FoldDB system.
    /// Do not initialize anywhere else.
    pub async fn new(path: &str) -> Result<Self, StorageError> {
        let pool = Arc::new(SledPool::new(std::path::PathBuf::from(path)));

        Self::initialize_from_pool(pool, path).await
    }

    /// Creates a new FoldDB instance with fully initialized components.
    ///
    /// This is the most flexible constructor, allowing the injection of
    /// specific implementations for storage, progress tracking, etc.
    pub async fn new_with_components(
        db_ops: Arc<DbOperations>,
        db_path: &str,
        job_store: Option<Arc<dyn JobStore>>,
        user_id: Option<String>,
    ) -> Result<Self, StorageError> {
        let actual_user_id = user_id.unwrap_or_else(|| "global".to_string());
        Self::initialize_from_db_ops(db_ops, db_path, job_store, actual_user_id).await
    }

    /// Common initialization logic shared by both new() and new_with_s3()
    /// This method initializes all FoldDB components from a SledPool
    async fn initialize_from_pool(
        pool: Arc<SledPool>,
        db_path: &str,
    ) -> Result<Self, StorageError> {
        log_feature!(
            LogFeature::Database,
            info,
            "🔄 Using DbOperations with storage abstraction layer (Sled backend)"
        );

        let store = Arc::new(crate::storage::SledNamespacedStore::new(Arc::clone(&pool)));
        let db_ops = Arc::new(
            DbOperations::from_namespaced_store(
                store as Arc<dyn crate::storage::traits::NamespacedStore>,
            )
            .await?,
        );

        log_feature!(
            LogFeature::Database,
            info,
            "✅ Storage abstraction active - using {} backend",
            "Sled"
        );

        // Initialize face detection processor if the feature is enabled
        #[cfg(feature = "face-detection")]
        {
            let home_path = std::path::Path::new(db_path);
            if let Some(mgr) = db_ops.native_index_manager() {
                let processor = std::sync::Arc::new(
                    crate::db_operations::native_index::face::OnnxFaceProcessor::new(home_path),
                );
                mgr.set_face_processor(processor);
                log::info!("Face detection processor initialized");
            }
        }

        // For local Sled backend, create persistent progress store
        let job_store: ProgressTracker =
            crate::progress::create_tracker_with_sled(Arc::clone(&pool));
        Self::initialize_from_db_ops_with_sled(
            db_ops,
            db_path,
            Some(job_store),
            "local".to_string(),
            Some(pool),
            None,
        )
        .await
    }

    /// Common initialization logic that creates all FoldDB components from DbOperations
    pub async fn initialize_from_db_ops(
        db_ops: Arc<DbOperations>,
        db_path: &str,
        job_store: Option<Arc<dyn JobStore>>,
        user_id: String,
    ) -> Result<Self, StorageError> {
        Self::initialize_from_db_ops_with_sled(db_ops, db_path, job_store, user_id, None, None)
            .await
    }

    /// Internal initializer that optionally retains the SledPool handle.
    /// The pool is needed by org operations and org sync configuration.
    pub async fn initialize_from_db_ops_with_sled(
        db_ops: Arc<DbOperations>,
        _db_path: &str,
        job_store: Option<Arc<dyn JobStore>>,
        user_id: String,
        sled_pool: Option<Arc<SledPool>>,
        encrypting_store: Option<Arc<crate::storage::EncryptingNamespacedStore>>,
    ) -> Result<Self, StorageError> {
        // Initialize message bus
        let message_bus = Arc::new(AsyncMessageBus::new());

        // Initialize pending task tracker
        let pending_tasks =
            Arc::new(super::infrastructure::pending_task_tracker::PendingTaskTracker::new());

        // Use provided progress tracker or create an in-memory one (for testing)
        let progress_tracker: ProgressTracker =
            job_store.unwrap_or_else(|| Arc::new(crate::progress::InMemoryProgressStore::new()));

        let schema_manager = Arc::new(
            SchemaCore::new(Arc::clone(&db_ops), Arc::clone(&message_bus))
                .await
                .map_err(|e| StorageError::IoError(std::io::Error::other(e.to_string())))?,
        );

        // Create and start EventMonitor for system-wide observability
        let event_monitor = Arc::new(EventMonitor::new(Arc::clone(&message_bus)).await);
        info!("Started EventMonitor for system-wide event tracking");

        // Create QueryExecutor for handling all query operations
        let query_executor = QueryExecutor::new(Arc::clone(&db_ops), Arc::clone(&schema_manager));
        info!("Created QueryExecutor for query operations");

        // Create shared IndexStatusTracker for tracking indexing progress
        let index_status_tracker = IndexStatusTracker::new(Some(progress_tracker.clone()));

        // Create MutationManager for handling all mutation operations
        let mutation_manager = MutationManager::new(
            Arc::clone(&db_ops),
            Arc::clone(&schema_manager),
            Arc::clone(&message_bus),
            Some(index_status_tracker.clone()),
        );

        info!("Created MutationManager for mutation operations");

        // Start the MutationManager event listener
        if let Err(e) = mutation_manager.start_event_listener(user_id.clone()).await {
            log_feature!(
                LogFeature::Database,
                error,
                "Failed to start MutationManager event listener: {}. Mutations via event bus will not be processed.",
                e
            );
            return Err(StorageError::BackendError(format!(
                "Failed to start MutationManager event listener: {}",
                e
            )));
        }

        info!("Started MutationManager event listener");

        // Start ProcessResultsSubscriber to capture actual stored keys for ingestion reports
        let process_results_subscriber =
            super::infrastructure::ProcessResultsSubscriber::new(Arc::clone(&db_ops));
        process_results_subscriber
            .start_event_listener(Arc::clone(&message_bus), user_id.clone())
            .await;
        info!("Started ProcessResultsSubscriber for ingestion result tracking");

        Ok(Self {
            schema_manager,
            db_ops,
            sled_pool,
            query_executor,
            message_bus,
            event_monitor,
            mutation_manager,
            pending_tasks,
            progress_tracker,
            sync_engine: None,
            sync_task: None,
            encrypting_store,
            config_store: None,
        })
    }

    /// Flushes local storage to ensure all data is persisted
    pub async fn flush(&self) -> Result<(), StorageError> {
        self.db_ops
            .flush()
            .await
            .map_err(|e| StorageError::IoError(std::io::Error::other(e.to_string())))
    }

    /// Get the unified progress tracker
    pub fn get_progress_tracker(&self) -> ProgressTracker {
        self.progress_tracker.clone()
    }

    // ========== INDEXING STATUS API ==========

    /// Get the current indexing status
    pub async fn get_indexing_status(&self) -> super::orchestration::IndexingStatus {
        self.mutation_manager.get_indexing_status().await
    }

    /// Check if indexing is currently in progress
    pub async fn is_indexing(&self) -> bool {
        self.mutation_manager.is_indexing().await
    }

    /// Wait for all pending background tasks to complete
    pub async fn wait_for_background_tasks(&self, timeout: std::time::Duration) -> bool {
        self.pending_tasks.wait_for_completion(timeout).await
    }

    /// Increment pending task count manually
    pub fn increment_pending_tasks(&self) {
        self.pending_tasks.increment();
    }

    /// Decrement pending task count manually
    pub fn decrement_pending_tasks(&self) {
        self.pending_tasks.decrement();
    }

    // ========== CONSOLIDATED SCHEMA API - DELEGATES TO SCHEMA_CORE ==========

    /// Load schema from JSON string (creates Available schema)
    pub async fn load_schema_from_json(&mut self, json_str: &str) -> Result<(), SchemaError> {
        self.schema_manager.load_schema_from_json(json_str).await
    }

    /// Load schema from file (creates Available schema)
    pub async fn load_schema_from_file<P: AsRef<Path>>(
        &mut self,
        path: P,
    ) -> Result<(), SchemaError> {
        self.schema_manager.load_schema_from_file(path).await
    }

    /// Provides access to the underlying database operations
    pub fn get_db_ops(&self) -> Arc<DbOperations> {
        Arc::clone(&self.db_ops)
    }

    /// Get current event statistics from the event monitor
    pub fn get_event_statistics(&self) -> super::infrastructure::event_monitor::EventStatistics {
        self.event_monitor.get_statistics()
    }

    /// Get the schema manager for testing schema functionality
    pub fn schema_manager(&self) -> Arc<SchemaCore> {
        Arc::clone(&self.schema_manager)
    }

    /// Search native index across all classification types
    pub async fn native_search_all_classifications(
        &self,
        term: &str,
    ) -> Result<Vec<IndexResult>, SchemaError> {
        debug!(
            "FoldDB: native_search_all_classifications called for term: '{}'",
            term
        );

        let manager = self.db_ops.native_index_manager().ok_or_else(|| {
            SchemaError::InvalidData("Native index manager not available".to_string())
        })?;

        manager.search_all_classifications(term).await
    }

    /// Get the mutation manager for testing mutation functionality
    pub fn mutation_manager(&self) -> &MutationManager {
        &self.mutation_manager
    }

    /// Get the mutable mutation manager for testing mutation functionality
    pub fn mutation_manager_mut(&mut self) -> &mut MutationManager {
        &mut self.mutation_manager
    }

    /// Get the message bus for publishing events
    pub fn message_bus(&self) -> Arc<AsyncMessageBus> {
        Arc::clone(&self.message_bus)
    }
}

impl Drop for FoldDB {
    fn drop(&mut self) {
        // Abort the background sync task to prevent tokio panic:
        // "Cannot drop a runtime in a context where blocking is not allowed"
        if let Some(handle) = self.sync_task.take() {
            handle.abort();
        }
    }
}
