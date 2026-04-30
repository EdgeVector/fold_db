//! FoldDB Core - Main database coordinator
//!
//! This module contains the main FoldDB struct that manages schemas, permissions, and data storage.

// Standard library imports
use std::path::Path;
use std::sync::Arc;

// External crate imports
use tracing::{debug, info};

// Internal crate imports
use crate::db_operations::{DbOperations, IndexResult};
use crate::schema::{SchemaCore, SchemaError};
use crate::storage::SledPool;
use crate::storage::StorageError;

// Infrastructure components that are used internally
use super::event_monitor::EventMonitor;
use super::mutation_manager::MutationManager;
use super::orchestration::index_status::IndexStatusTracker;
use super::query::QueryExecutor;
use super::sync_coordinator::SyncCoordinator;
use super::trigger_runner::{
    ArcTriggerDispatcher, MutationManagerFiringWriter, TriggerDispatcher, TriggerRunner,
};
use crate::messaging::AsyncMessageBus;
use crate::progress::ProgressStore as JobStore;
use crate::progress::ProgressTracker;
use crate::triggers::clock::SystemClock;

/// The main database coordinator that manages schemas, permissions, and data storage.
pub struct FoldDB {
    pub(crate) schema_manager: Arc<SchemaCore>,
    /// Shared database operations with storage abstraction
    pub(crate) db_ops: Arc<DbOperations>,
    /// SledPool for on-demand Sled access (e.g., org operations, config store).
    /// Only present when using the Sled backend.
    sled_pool: Option<Arc<SledPool>>,
    /// Query executor for handling all query operations
    pub(crate) query_executor: QueryExecutor,
    /// Message bus for event-driven communication (held for Arc lifetime)
    pub(crate) message_bus: Arc<AsyncMessageBus>,
    /// Event monitor for system-wide observability
    pub(crate) event_monitor: Arc<EventMonitor>,
    /// Mutation manager for handling all mutation operations.
    /// Held in an Arc so the trigger runner can hold a `Weak` reference
    /// back (cycle-breaking: runner writes TriggerFiring rows through
    /// this manager, which in turn notifies the runner on commit).
    pub(crate) mutation_manager: Arc<MutationManager>,
    /// Trigger runner — the single source of truth for firing views.
    /// Held as `dyn TriggerShutdown` so callers can `clear_dispatcher`
    /// during shutdown to break the Arc cycle with MutationManager.
    trigger_runner: Arc<TriggerRunner<SystemClock>>,
    /// Shutdown notifier for the trigger runner's scheduler loop.
    trigger_shutdown: Arc<tokio::sync::Notify>,
    /// Tracker for pending background tasks
    pub(crate) pending_tasks: Arc<super::pending_task_tracker::PendingTaskTracker>,
    /// Unified progress tracker for all job types (ingestion, indexing, etc.)
    /// This is the single source of truth for progress — uses Sled for persistent storage.
    pub(crate) progress_tracker: ProgressTracker,
    /// Coordinates the optional cloud sync engine lifecycle.
    /// In local mode this holds no engine and all sync operations are no-ops.
    sync_coordinator: SyncCoordinator,
    /// Optional reference to the encrypting store for org crypto registration.
    encrypting_store: Option<Arc<crate::storage::EncryptingNamespacedStore>>,
    /// Optional Sled-backed configuration store for runtime node config.
    /// Uses RwLock for interior mutability so FoldDB doesn't need &mut self.
    config_store: std::sync::RwLock<Option<crate::storage::NodeConfigStore>>,
    /// Signing keypair for molecule signatures. Plumbed to MutationManager.
    /// Kept on FoldDB so the node layer can access it for future use cases
    /// (e.g., signing sync uploads, signing query responses).
    #[allow(dead_code)]
    pub(crate) signer: Arc<crate::security::Ed25519KeyPair>,
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
    pub async fn shutdown(&self) -> Result<(), StorageError> {
        tracing::info!(
            target: "fold_node::database",
            "Shutting down FoldDB: flushing sync and storage"
        );
        if let Err(e) = self.stop_sync().await {
            tracing::warn!("sync flush on shutdown failed: {e}");
        }
        self.flush().await
    }

    /// Returns a reference to the sync coordinator.
    pub fn sync_coordinator(&self) -> &SyncCoordinator {
        &self.sync_coordinator
    }

    /// Set the sync engine (called by the factory when sync is configured).
    /// Also registers the schema reloader callback so SchemaCore's in-memory
    /// cache is refreshed after sync replays schema entries into Sled, and
    /// the embedding reloader so the in-memory EmbeddingIndex is refreshed
    /// after sync replays native_index entries.
    ///
    /// Registration lives here because it needs access to FoldDB-owned
    /// components (SchemaCore, NativeIndexManager); engine storage is then
    /// delegated to the SyncCoordinator.
    pub async fn set_sync_engine(&self, engine: Arc<crate::sync::SyncEngine>) {
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

        self.sync_coordinator.set_engine(engine);
    }

    /// Start the background sync timer. Delegates to the coordinator.
    pub fn start_sync(&self, interval_ms: u64) {
        self.sync_coordinator.start_background_sync(
            interval_ms,
            Arc::clone(&self.db_ops),
            self.sled_pool.clone(),
        );
    }

    /// Force an immediate sync (e.g. on shutdown).
    pub async fn force_sync(&self) -> Result<(), crate::sync::SyncError> {
        self.sync_coordinator.force_sync().await
    }

    /// Stop the background sync timer and run a final sync.
    pub async fn stop_sync(&self) -> Result<(), crate::sync::SyncError> {
        self.sync_coordinator.stop().await
    }

    /// Get the sync engine state, if sync is configured.
    pub async fn sync_state(&self) -> Option<crate::sync::SyncState> {
        self.sync_coordinator.state().await
    }

    /// Get a full sync status snapshot, if sync is configured.
    pub async fn sync_status(&self) -> Option<crate::sync::SyncStatus> {
        self.sync_coordinator.status().await
    }

    /// Get the number of pending (unsynced) log entries.
    /// Returns None if sync is not configured.
    pub async fn sync_pending_count(&self) -> Option<usize> {
        self.sync_coordinator.pending_count().await
    }

    /// Returns true if the sync engine is configured.
    pub fn is_sync_enabled(&self) -> bool {
        self.sync_coordinator.is_enabled()
    }

    /// Returns a clone of the sync engine Arc, if configured.
    ///
    /// Used by fold_db_node to call `configure_org_sync()` after node startup.
    pub fn sync_engine(&self) -> Option<Arc<crate::sync::SyncEngine>> {
        self.sync_coordinator.engine()
    }

    /// Returns a clone of the Sled-backed config store, if available.
    pub fn config_store(&self) -> Option<crate::storage::NodeConfigStore> {
        self.config_store.read().unwrap().clone()
    }

    /// Set the Sled-backed config store (called by the factory).
    pub fn set_config_store(&self, store: crate::storage::NodeConfigStore) {
        *self.config_store.write().unwrap() = Some(store);
    }

    /// Start the sync engine on an existing FoldDB instance at runtime.
    /// Called when cloud credentials are written to Sled and sync needs to activate.
    pub async fn start_sync_engine_runtime(
        &self,
        api_url: &str,
        api_key: &str,
        data_dir: &str,
        e2e_keys: &crate::crypto::E2eKeys,
        auth_refresh: Option<crate::sync::AuthRefreshCallback>,
    ) -> crate::error::FoldDbResult<()> {
        use crate::error::FoldDbError;

        if self.sync_coordinator.is_enabled() {
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

        // trace-egress: propagate (shared with skip-s3 — see docs/observability/egress-classification-notes.md)
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
            Arc::clone(&self.signer),
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
    ///
    /// This is a convenience path for tests and other in-process callers
    /// that do not have a persistent node identity (e.g., ephemeral
    /// `tempdir` storage). It generates a fresh Ed25519 signing keypair
    /// on the fly. **Production callers must go through
    /// [`crate::fold_db_core::factory::create_fold_db`] and pass in a
    /// signer loaded from the node's persistent identity** — otherwise
    /// every boot produces a different signing key and molecule
    /// signatures will not match the node's public identity.
    pub async fn new(path: &str) -> Result<Self, StorageError> {
        let pool = Arc::new(SledPool::new(std::path::PathBuf::from(path)));

        Self::initialize_from_pool(pool, path).await
    }

    /// Creates a new FoldDB instance with fully initialized components.
    ///
    /// This is the most flexible in-process constructor; it generates a
    /// fresh signing keypair on the fly. See [`FoldDB::new`] for why
    /// production callers must use the factory instead.
    pub async fn new_with_components(
        db_ops: Arc<DbOperations>,
        db_path: &str,
        job_store: Option<Arc<dyn JobStore>>,
        user_id: Option<String>,
    ) -> Result<Self, StorageError> {
        let actual_user_id = user_id.unwrap_or_else(|| "global".to_string());
        Self::initialize_from_db_ops(db_ops, db_path, job_store, actual_user_id).await
    }

    /// Generate a signing keypair for in-process / test callers that do
    /// not have a persistent node identity. Wraps the error so callers
    /// get a single failure mode.
    fn generate_ephemeral_signer() -> Result<Arc<crate::security::Ed25519KeyPair>, StorageError> {
        let keypair = crate::security::Ed25519KeyPair::generate().map_err(|e| {
            StorageError::BackendError(format!("ephemeral signer generation failed: {}", e))
        })?;
        Ok(Arc::new(keypair))
    }

    /// Common initialization logic shared by both new() and new_with_s3()
    /// This method initializes all FoldDB components from a SledPool
    async fn initialize_from_pool(
        pool: Arc<SledPool>,
        db_path: &str,
    ) -> Result<Self, StorageError> {
        tracing::info!(
            target: "fold_node::database",
            "🔄 Using DbOperations with storage abstraction layer (Sled backend)"
        );

        let store = Arc::new(crate::storage::SledNamespacedStore::new(Arc::clone(&pool)));
        let db_ops = Arc::new(
            DbOperations::from_namespaced_store(
                store as Arc<dyn crate::storage::traits::NamespacedStore>,
            )
            .await?,
        );

        tracing::info!(
            target: "fold_node::database",
            "✅ Storage abstraction active - using {} backend",
            "Sled"
        );

        // Initialize face detection processor if the feature is enabled
        #[cfg(feature = "face-detection")]
        {
            let home_path = std::env::var("FOLDDB_HOME")
                .map(std::path::PathBuf::from)
                .unwrap_or_else(|_| {
                    std::path::Path::new(db_path)
                        .parent()
                        .unwrap_or(std::path::Path::new(db_path))
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

        // For local Sled backend, create persistent progress store
        let job_store: ProgressTracker =
            crate::progress::create_tracker_with_sled(Arc::clone(&pool));
        let signer = Self::generate_ephemeral_signer()?;
        Self::initialize_from_db_ops_with_sled(
            db_ops,
            db_path,
            Some(job_store),
            "local".to_string(),
            Some(pool),
            None,
            signer,
        )
        .await
    }

    /// Common initialization logic that creates all FoldDB components from DbOperations.
    /// Generates an ephemeral signing keypair — see [`FoldDB::new`].
    pub async fn initialize_from_db_ops(
        db_ops: Arc<DbOperations>,
        db_path: &str,
        job_store: Option<Arc<dyn JobStore>>,
        user_id: String,
    ) -> Result<Self, StorageError> {
        let signer = Self::generate_ephemeral_signer()?;
        Self::initialize_from_db_ops_with_sled(
            db_ops, db_path, job_store, user_id, None, None, signer,
        )
        .await
    }

    /// Internal initializer that optionally retains the SledPool handle.
    /// The pool is needed by org operations and org sync configuration.
    ///
    /// `signer` is the Ed25519 keypair used to sign molecule mutations.
    /// It is shared with the sync engine so merged-molecule writes during
    /// replay carry the same node identity as direct writes via
    /// `MutationManager`. Production callers (via the factory) must load
    /// this from the node's persistent identity so signatures match the
    /// node's public key — see the module docs on [`FoldDB::new`] for why.
    pub async fn initialize_from_db_ops_with_sled(
        db_ops: Arc<DbOperations>,
        _db_path: &str,
        job_store: Option<Arc<dyn JobStore>>,
        user_id: String,
        sled_pool: Option<Arc<SledPool>>,
        encrypting_store: Option<Arc<crate::storage::EncryptingNamespacedStore>>,
        signer: Arc<crate::security::Ed25519KeyPair>,
    ) -> Result<Self, StorageError> {
        // Initialize message bus
        let message_bus = Arc::new(AsyncMessageBus::new());

        // Initialize pending task tracker
        let pending_tasks = Arc::new(super::pending_task_tracker::PendingTaskTracker::new());

        // Use provided progress tracker or create an in-memory one (for testing)
        let progress_tracker: ProgressTracker =
            job_store.unwrap_or_else(|| Arc::new(crate::progress::InMemoryProgressStore::new()));

        let schema_manager = Arc::new(
            SchemaCore::new(Arc::clone(&db_ops), Arc::clone(&message_bus))
                .await
                .map_err(|e| StorageError::IoError(std::io::Error::other(e.to_string())))?,
        );

        // Register internal TriggerFiring schema so the TriggerRunner
        // (Phase 1 task 3) has somewhere to log every view firing. The
        // call is idempotent — subsequent boots refresh the cache and
        // re-approve a no-op.
        crate::triggers::register_trigger_firing_schema(&schema_manager)
            .await
            .map_err(|e| StorageError::IoError(std::io::Error::other(e.to_string())))?;

        // Create and start EventMonitor for system-wide observability
        let event_monitor = Arc::new(EventMonitor::new(Arc::clone(&message_bus)).await);
        info!("Started EventMonitor for system-wide event tracking");

        // Create QueryExecutor for handling all query operations
        let query_executor = QueryExecutor::new(
            Arc::clone(&db_ops),
            Arc::clone(&schema_manager),
            sled_pool.clone(),
        );
        info!("Created QueryExecutor for query operations");

        // Create shared IndexStatusTracker for tracking indexing progress
        let index_status_tracker = IndexStatusTracker::new(Some(progress_tracker.clone()));

        // Create ViewOrchestrator that MutationManager uses for
        // view lifecycle events (redirect writes, invalidate caches,
        // precompute dependents).
        let view_orchestrator = Arc::new(super::view_orchestrator::ViewOrchestrator::new(
            Arc::clone(&schema_manager),
            Arc::clone(&db_ops),
        ));

        // Create MutationManager for handling all mutation operations.
        // The sled_pool is plumbed through so the manager can consult the
        // org memberships tree and reject mutations against org-scoped
        // schemas the node is not a member of.
        //
        // The signer was loaded and validated by the caller (in
        // production, from the node's persistent identity). It is shared
        // with the sync engine so merged-molecule writes trace to the
        // same node identity as direct writes via `MutationManager`.
        let mutation_manager = Arc::new(MutationManager::new(
            Arc::clone(&db_ops),
            Arc::clone(&schema_manager),
            Arc::clone(&message_bus),
            Arc::clone(&view_orchestrator),
            Some(index_status_tracker.clone()),
            sled_pool.clone(),
            Arc::clone(&signer),
        ));

        info!("Created MutationManager for mutation operations");

        // Build the TriggerRunner. The runner holds a Weak ref to the
        // mutation manager (for writing TriggerFiring audit rows) and
        // the mutation manager holds an Arc ref to the runner (for the
        // dispatcher trait). The Weak back-edge breaks the cycle so
        // dropping FoldDB releases both.
        let firing_writer = Arc::new(MutationManagerFiringWriter::new(
            Arc::downgrade(&mutation_manager),
            signer.public_key_base64(),
        ));
        let trigger_runner = Arc::new(TriggerRunner::new_with_orchestrator(
            Arc::clone(&schema_manager),
            Arc::clone(&view_orchestrator),
            sled_pool.clone(),
            Arc::new(SystemClock::new()),
            firing_writer,
            signer.public_key_base64(),
        ));
        mutation_manager.set_trigger_dispatcher(Arc::new(ArcTriggerDispatcher::new(Arc::clone(
            &trigger_runner,
        ))) as Arc<dyn TriggerDispatcher>);

        // Wire the mutation manager into the view orchestrator as the
        // derived-mutation writer. Same post-construction late-binding
        // pattern as `set_trigger_dispatcher` — both directions of the
        // `ViewOrchestrator` ↔ `MutationManager` edge need to exist, and
        // construction can only build one at a time.
        //
        // `projects/view-compute-as-mutations` PR 2: the fire path
        // dual-writes derived mutations through this writer.
        view_orchestrator
            .set_derived_mutation_writer(Arc::clone(&mutation_manager)
                as Arc<dyn super::view_orchestrator::DerivedMutationWriter>)
            .await;

        // Wire the orchestrator into the query executor so cold reads
        // (atom store empty for the requested fields) can fire the view
        // inline. Same late-binding pattern as the writer above.
        query_executor
            .set_view_orchestrator(Arc::clone(&view_orchestrator))
            .await;

        let trigger_shutdown = Arc::new(tokio::sync::Notify::new());
        {
            let runner = Arc::clone(&trigger_runner);
            let shutdown = Arc::clone(&trigger_shutdown);
            // lint:spawn-bare-ok boot-time scheduler loop — perpetual worker, no per-request parent span.
            tokio::spawn(async move {
                runner.run_scheduler_loop(shutdown).await;
            });
        }
        info!("Started TriggerRunner scheduler loop");

        // Start the MutationManager event listener
        if let Err(e) = mutation_manager.start_event_listener(user_id.clone()).await {
            tracing::error!(
                target: "fold_node::database",
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
            super::process_results_subscriber::ProcessResultsSubscriber::new(Arc::clone(&db_ops));
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
            trigger_runner,
            trigger_shutdown,
            pending_tasks,
            progress_tracker,
            sync_coordinator: SyncCoordinator::new(),
            encrypting_store,
            config_store: std::sync::RwLock::new(None),
            signer,
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
    pub async fn load_schema_from_json(&self, json_str: &str) -> Result<(), SchemaError> {
        self.schema_manager.load_schema_from_json(json_str).await
    }

    /// Load schema from file (creates Available schema)
    pub async fn load_schema_from_file<P: AsRef<Path>>(&self, path: P) -> Result<(), SchemaError> {
        self.schema_manager.load_schema_from_file(path).await
    }

    /// Provides access to the underlying database operations (cloned Arc)
    pub fn get_db_ops(&self) -> Arc<DbOperations> {
        Arc::clone(&self.db_ops)
    }

    /// Returns a reference to the database operations Arc
    pub fn db_ops(&self) -> &Arc<DbOperations> {
        &self.db_ops
    }

    /// Returns a reference to the query executor
    pub fn query_executor(&self) -> &QueryExecutor {
        &self.query_executor
    }

    /// Returns a reference to the event monitor
    pub fn event_monitor(&self) -> &EventMonitor {
        &self.event_monitor
    }

    /// Returns a reference to the pending task tracker
    pub fn pending_tasks(&self) -> &Arc<super::pending_task_tracker::PendingTaskTracker> {
        &self.pending_tasks
    }

    /// Get current event statistics from the event monitor
    pub fn get_event_statistics(&self) -> super::event_monitor::EventStatistics {
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

    /// Get the trigger runner — primarily used by integration tests and
    /// admin endpoints that want to inspect pending/quarantined state.
    pub fn trigger_runner(&self) -> &Arc<TriggerRunner<SystemClock>> {
        &self.trigger_runner
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
        self.sync_coordinator.abort_task();
        // Signal the trigger scheduler loop to exit.
        self.trigger_shutdown.notify_waiters();
        // Break the Arc cycle between the trigger runner and the
        // mutation manager so both actually drop.
        self.mutation_manager.clear_trigger_dispatcher();
    }
}
