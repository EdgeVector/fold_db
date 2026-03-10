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
}

impl FoldDB {
    /// Retrieves or generates and persists the node identifier.
    pub async fn get_node_id(&self) -> Result<String, crate::storage::StorageError> {
        self.db_ops
            .get_node_id()
            .await
            .map_err(|e| crate::storage::StorageError::BackendError(e.to_string()))
    }

    /// Properly close and flush the database to release all file locks
    pub fn close(&self) -> Result<(), sled::Error> {
        log_feature!(
            LogFeature::Database,
            info,
            "Closing FoldDB and flushing all data to disk"
        );

        log_feature!(
            LogFeature::Database,
            debug,
            "FoldDB close() - relying on storage backend's own flush mechanisms"
        );

        Ok(())
    }

    /// Graceful async shutdown: flush pending sync, stop background timer, then close.
    pub async fn shutdown(&mut self) -> Result<(), StorageError> {
        if let Err(e) = self.stop_sync().await {
            log::warn!("sync flush on shutdown failed: {e}");
        }
        self.flush().await?;
        self.close().map_err(|e| StorageError::IoError(std::io::Error::other(e.to_string())))
    }

    /// Set the sync engine (called by the factory when sync is configured).
    pub fn set_sync_engine(&mut self, engine: Arc<crate::sync::SyncEngine>) {
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

        let handle = tokio::spawn(async move {
            let interval = tokio::time::Duration::from_millis(interval_ms);
            loop {
                tokio::time::sleep(interval).await;
                if engine.state().await == crate::sync::SyncState::Dirty {
                    if let Err(e) = engine.sync().await {
                        log::warn!("sync cycle failed: {e}");
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

    /// Creates a new FoldDB instance with the specified storage path.
    /// All initializations happen here. This is the main entry point for the FoldDB system.
    /// Do not initialize anywhere else.
    pub async fn new(path: &str) -> Result<Self, StorageError> {
        let db = sled::open(path)
            .map_err(|e| StorageError::IoError(std::io::Error::other(e.to_string())))?;

        Self::initialize_from_db(db, path).await
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
    /// This method initializes all FoldDB components from an already-opened sled database
    async fn initialize_from_db(db: sled::Db, db_path: &str) -> Result<Self, StorageError> {
        log_feature!(
            LogFeature::Database,
            info,
            "🔄 Using DbOperations with storage abstraction layer (Sled backend)"
        );

        let db_ops = Arc::new(DbOperations::from_sled(db.clone()).await?);

        log_feature!(
            LogFeature::Database,
            info,
            "✅ Storage abstraction active - using {} backend",
            "Sled"
        );

        // For local Sled backend, create persistent progress store using a dedicated sled tree
        let progress_tree = db
            .open_tree("progress")
            .map_err(|e| StorageError::IoError(std::io::Error::other(e.to_string())))?;
        let job_store: ProgressTracker = crate::progress::create_tracker_with_sled(progress_tree);
        Self::initialize_from_db_ops(db_ops, db_path, Some(job_store), "local".to_string()).await
    }

    /// Common initialization logic that creates all FoldDB components from DbOperations
    pub async fn initialize_from_db_ops(
        db_ops: Arc<DbOperations>,
        _db_path: &str,
        job_store: Option<Arc<dyn JobStore>>,
        user_id: String,
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
        let event_monitor = Arc::new(
            EventMonitor::new(
                Arc::clone(&message_bus),
            )
            .await,
        );
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
            query_executor,
            message_bus,
            event_monitor,
            mutation_manager,
            pending_tasks,
            progress_tracker,
            sync_engine: None,
            sync_task: None,
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
