//! FoldDB Core - Main database coordinator
//!
//! This module contains the main FoldDB struct that manages schemas, permissions, and data storage.

// Standard library imports
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
use crate::transform::manager::TransformManager;

// Infrastructure components that are used internally
// init_transform_manager removed
// SystemInitializationRequest removed
use super::infrastructure::{AsyncMessageBus, EventMonitor};
use super::mutation_manager::MutationManager;
use super::orchestration::index_status::IndexStatusTracker;
use super::orchestration::TransformOrchestrator;
use super::query::QueryExecutor;
use crate::progress::ProgressStore as JobStore;
use crate::progress::ProgressTracker;

/// The main database coordinator that manages schemas, permissions, and data storage.
pub struct FoldDB {
    pub(crate) schema_manager: Arc<SchemaCore>,
    pub(crate) transform_manager: Arc<TransformManager>,
    /// Shared database operations with storage abstraction
    pub(crate) db_ops: Arc<DbOperations>,
    /// Query executor for handling all query operations
    pub(crate) query_executor: QueryExecutor,
    /// Message bus for event-driven communication
    pub(crate) message_bus: Arc<AsyncMessageBus>,
    /// Event monitor for system-wide observability
    pub(crate) event_monitor: Arc<EventMonitor>,
    /// Transform orchestrator for managing transform execution
    /// Optional for backends that don't support orchestrator_tree (e.g., DynamoDB)
    pub(crate) transform_orchestrator: Option<Arc<TransformOrchestrator>>,
    /// Mutation manager for handling all mutation operations
    pub(crate) mutation_manager: MutationManager,
    /// Tracker for pending background tasks
    pub(crate) pending_tasks: Arc<super::infrastructure::pending_task_tracker::PendingTaskTracker>,
    /// Unified progress tracker for all job types (ingestion, indexing, etc.)
    /// This is the single source of truth for progress - local uses Sled, cloud uses DynamoDB
    pub(crate) progress_tracker: ProgressTracker,
}

impl FoldDB {
    /// Retrieves or generates and persists the node identifier.
    pub async fn get_node_id(&self) -> Result<String, crate::storage::StorageError> {
        self.db_ops
            .get_node_id()
            .await
            .map_err(|e| crate::storage::StorageError::BackendError(e.to_string()))
    }

    /// Retrieves the list of permitted schemas for the given node.
    pub async fn get_schema_permissions(&self, node_id: &str) -> Vec<String> {
        self.db_ops
            .get_schema_permissions(node_id)
            .await
            .unwrap_or_default()
    }

    /// Sets the permitted schemas for the given node.
    pub async fn set_schema_permissions(
        &self,
        node_id: &str,
        schemas: &[String],
    ) -> sled::Result<()> {
        self.db_ops
            .set_schema_permissions(node_id, schemas)
            .await
            .map_err(|e| sled::Error::Unsupported(e.to_string()))
    }

    /// Properly close and flush the database to release all file locks
    pub fn close(&self) -> Result<(), sled::Error> {
        log_feature!(
            LogFeature::Database,
            info,
            "Closing FoldDB and flushing all data to disk"
        );

        // Flush the main database
        // Storage abstraction auto-flushes for cloud backends, manual flush for Sled
        // NOTE: Flush is critical for Sled, but calling it during Drop in an async context
        // can cause "runtime within runtime" panics. For now, we skip explicit flush on close
        // and rely on Sled's own drop/flush mechanisms.
        // In production, call flush() explicitly before dropping FoldDB.
        log_feature!(
            LogFeature::Database,
            debug,
            "FoldDB close() - relying on storage backend's own flush mechanisms"
        );

        Ok(())
    }

    /// Creates a new FoldDB instance with the specified storage path.
    /// All initializations happen here. This is the main entry point for the FoldDB system.
    /// Do not initialize anywhere else.
    ///
    /// Creates a new FoldDB instance with the specified storage path.
    /// All initializations happen here. This is the main entry point for the FoldDB system.
    /// Do not initialize anywhere else.
    ///
    /// Now fully async to support DbOperations with storage abstraction!
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
    ///
    /// Fully async - uses DbOperations with storage abstraction layer!
    async fn initialize_from_db(db: sled::Db, db_path: &str) -> Result<Self, StorageError> {
        log_feature!(
            LogFeature::Database,
            info,
            "🔄 Using DbOperations with storage abstraction layer (Sled backend)"
        );

        // Use the new async storage abstraction!
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
    ///
    /// This is used by both initialize_from_db (Sled) and new_with_db_ops (custom backends)
    pub(crate) async fn initialize_from_db_ops(
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

        // Initialize components via event-driven system initialization
        // SystemInitializationRequest removed - dead code

        // Create managers using direct initialization
        let transform_manager = Arc::new(
            TransformManager::new(Arc::clone(&db_ops), Arc::clone(&message_bus))
                .await
                .map_err(|e| StorageError::IoError(std::io::Error::other(e.to_string())))?,
        );

        let schema_manager = Arc::new(
            SchemaCore::new(Arc::clone(&db_ops), Arc::clone(&message_bus))
                .await
                .map_err(|e| StorageError::IoError(std::io::Error::other(e.to_string())))?,
        );

        // Create and start EventMonitor for system-wide observability
        let event_monitor = Arc::new(
            EventMonitor::new(
                Arc::clone(&message_bus),
                Arc::clone(&transform_manager),
                Some(progress_tracker.clone()),
                user_id.clone(),
            )
            .await,
        );
        info!("Started EventMonitor for system-wide event tracking");

        // Create QueryExecutor for handling all query operations
        let query_executor = QueryExecutor::new(Arc::clone(&db_ops), Arc::clone(&schema_manager));
        info!("Created QueryExecutor for query operations");

        // Create and start BackfillManager (Event-Driven Backfill Tracking)
        use super::orchestration::backfill_manager::BackfillManager;
        let backfill_tracker = event_monitor.get_backfill_tracker();
        let backfill_manager = BackfillManager::new(backfill_tracker);
        backfill_manager
            .start_event_listener(Arc::clone(&message_bus))
            .await;
        info!("Started BackfillManager for event-driven backfill tracking");

        // Create TransformOrchestrator for managing transform execution
        // Now supports both Sled and DynamoDB backends
        let transform_orchestrator = if let Some(orchestrator_tree) =
            db_ops.orchestrator_tree.clone()
        {
            // Sled backend - use sync version
            let orchestrator = Arc::new(TransformOrchestrator::new(
                Arc::clone(&transform_manager),
                orchestrator_tree,
                Arc::clone(&message_bus),
                Arc::clone(&db_ops),
            ));

            // Start the event listener to drive transforms
            orchestrator
                .start_event_listener(Arc::clone(&message_bus))
                .await;

            info!("Created and started TransformOrchestrator (Sled backend)");
            Some(orchestrator)
        } else {
            // DynamoDB or other backends - use async version with orchestrator_store
            let orchestrator_store = db_ops.orchestrator_store().inner().clone();
            match TransformOrchestrator::new_with_store(
                Arc::clone(&transform_manager),
                orchestrator_store,
                Arc::clone(&message_bus),
                Arc::clone(&db_ops),
            )
            .await
            {
                Ok(orchestrator) => {
                    // Start the event listener to drive transforms
                    orchestrator
                        .start_event_listener(Arc::clone(&message_bus))
                        .await;

                    info!("Created and started TransformOrchestrator (KvStore backend)");
                    Some(Arc::new(orchestrator))
                }
                Err(e) => {
                    log_feature!(
                        LogFeature::Database,
                        warn,
                        "⚠️  Failed to create TransformOrchestrator: {}. Transforms will have limited functionality.",
                        e
                    );
                    None
                }
            }
        };

        // Create shared IndexStatusTracker for tracking indexing progress
        // This is shared between MutationManager (read status) and IndexOrchestrator (write status)
        let index_status_tracker = IndexStatusTracker::new(Some(progress_tracker.clone()));

        // Create keyword extractor from ingestion service (LLM)
        use super::orchestration::keyword_extractor::KeywordExtractor;

        #[cfg(not(test))]
        let keyword_extractor = {
            let svc = crate::ingestion::ingestion_service::IngestionService::from_env()
                .map_err(|e| StorageError::ConfigurationError(format!(
                    "AI provider not configured. Set FOLD_OPENROUTER_API_KEY (or OPENROUTER_API_KEY) environment variable for indexing. Error: {}",
                    e
                )))?;
            info!("KeywordExtractor initialized - LLM-powered indexing enabled");
            Some(Arc::new(KeywordExtractor::new(Arc::new(svc))))
        };

        #[cfg(test)]
        let keyword_extractor = crate::ingestion::ingestion_service::IngestionService::from_env()
            .ok()
            .map(|svc| Arc::new(KeywordExtractor::new(Arc::new(svc))));

        // Create and start IndexOrchestrator for event-driven native indexing
        use super::orchestration::index_orchestrator::IndexOrchestrator;
        let index_orchestrator = Arc::new(IndexOrchestrator::new(
            Arc::clone(&db_ops),
            Some(index_status_tracker.clone()),
            Arc::clone(&pending_tasks),
            keyword_extractor,
        ));
        index_orchestrator
            .start_event_listener(Arc::clone(&message_bus))
            .await;
        info!("Started IndexOrchestrator for event-driven native indexing");

        // Create MutationManager for handling all mutation operations
        let mutation_manager = MutationManager::new(
            Arc::clone(&db_ops),
            Arc::clone(&schema_manager),
            Arc::clone(&message_bus),
            Some(index_status_tracker.clone()),
        );

        info!("Created MutationManager for mutation operations");

        // Start the MutationManager event listener
        let _ = mutation_manager.start_event_listener().await;

        info!("Started MutationManager event listener");

        // AtomManager operates via direct method calls, not event consumption.
        // Event-driven components:
        // - EventMonitor: System observability and statistics
        // - TransformOrchestrator: Automatic transform triggering based on field changes
        // - IndexEventHandler: Background indexing for improved mutation performance
        // - MutationManager: handles MutationRequest events

        Ok(Self {
            schema_manager,
            transform_manager,
            db_ops,
            query_executor,
            message_bus,
            event_monitor,
            transform_orchestrator,
            mutation_manager,
            pending_tasks,
            progress_tracker,
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
    /// This is the single source of truth for all job progress (ingestion, indexing, etc.)
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
        // Delegate to SchemaCore implementation
        self.schema_manager.load_schema_from_json(json_str).await
    }

    /// Load schema from file (creates Available schema)
    pub async fn load_schema_from_file<P: AsRef<Path>>(
        &mut self,
        path: P,
    ) -> Result<(), SchemaError> {
        // Delegate to SchemaCore implementation
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

    /// Get the backfill tracker
    pub fn get_backfill_tracker(
        &self,
    ) -> Arc<super::infrastructure::backfill_tracker::BackfillTracker> {
        self.event_monitor.get_backfill_tracker()
    }

    /// Get all backfill information
    pub fn get_all_backfills(&self) -> Vec<super::infrastructure::backfill_tracker::BackfillInfo> {
        self.event_monitor.get_all_backfills()
    }

    /// Get active (in-progress) backfills
    pub fn get_active_backfills(
        &self,
    ) -> Vec<super::infrastructure::backfill_tracker::BackfillInfo> {
        self.event_monitor.get_active_backfills()
    }

    /// Get specific backfill info
    pub fn get_backfill(
        &self,
        transform_id: &str,
    ) -> Option<super::infrastructure::backfill_tracker::BackfillInfo> {
        self.event_monitor.get_backfill(transform_id)
    }

    /// Get the message bus for publishing events (for testing)
    pub fn message_bus(&self) -> Arc<AsyncMessageBus> {
        Arc::clone(&self.message_bus)
    }

    /// Get the transform manager for testing transform functionality
    pub fn transform_manager(&self) -> Arc<TransformManager> {
        Arc::clone(&self.transform_manager)
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

        // Use append-only search for all classifications
        let entries = manager.search_all(term).await?;
        Ok(manager.entries_to_results(entries))
    }

    /// Get the transform orchestrator for managing transform execution
    /// Returns None if orchestrator is not available (e.g., with DynamoDB backend)
    pub fn transform_orchestrator(&self) -> Option<Arc<TransformOrchestrator>> {
        self.transform_orchestrator.as_ref().map(Arc::clone)
    }

    /// Get the mutation manager for testing mutation functionality
    pub fn mutation_manager(&self) -> &MutationManager {
        &self.mutation_manager
    }

    /// Get the mutable mutation manager for testing mutation functionality
    pub fn mutation_manager_mut(&mut self) -> &mut MutationManager {
        &mut self.mutation_manager
    }
}
