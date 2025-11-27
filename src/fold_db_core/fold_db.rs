//! FoldDB Core - Main database coordinator
//!
//! This module contains the main FoldDB struct that manages schemas, permissions, and data storage.

// Standard library imports
use std::path::Path;
use std::sync::Arc;

// External crate imports
use log::{debug, info};

// Internal crate imports
use crate::db_operations::{DbOperationsV2, IndexResult};
use crate::logging::features::{log_feature, LogFeature};
use crate::schema::{SchemaCore, SchemaError};
use crate::storage::{S3Config, S3SyncedStorage, StorageError};
use crate::transform::manager::TransformManager;

// Infrastructure components that are used internally
use super::infrastructure::init::{init_transform_manager};
use super::infrastructure::message_bus::request_events::SystemInitializationRequest;
use super::infrastructure::{EventMonitor, MessageBus};
use super::orchestration::{TransformOrchestrator, IndexEventHandler};
use super::query::QueryExecutor;
use super::mutation_manager::MutationManager;

/// The main database coordinator that manages schemas, permissions, and data storage.
pub struct FoldDB {
    pub(crate) schema_manager: Arc<SchemaCore>,
    pub(crate) transform_manager: Arc<TransformManager>,
    /// Shared database operations with storage abstraction
    pub(crate) db_ops: Arc<DbOperationsV2>,
    /// Query executor for handling all query operations
    pub(crate) query_executor: QueryExecutor,
    /// Message bus for event-driven communication
    pub(crate) message_bus: Arc<MessageBus>,
    /// Event monitor for system-wide observability
    pub(crate) event_monitor: Arc<EventMonitor>,
    /// Transform orchestrator for managing transform execution
    pub(crate) transform_orchestrator: Arc<TransformOrchestrator>,
    /// Mutation manager for handling all mutation operations
    pub(crate) mutation_manager: MutationManager,
    /// Index event handler for background indexing
    pub(crate) index_event_handler: IndexEventHandler,
    /// Optional S3 storage for syncing to cloud
    s3_storage: Option<Arc<S3SyncedStorage>>,
}

impl FoldDB {
    /// Retrieves or generates and persists the node identifier.
    pub async fn get_node_id(&self) -> Result<String, crate::storage::StorageError> {
        self.db_ops
            .get_node_id().await
            .map_err(|e| crate::storage::StorageError::BackendError(e.to_string()))
    }

    /// Retrieves the list of permitted schemas for the given node.
    pub fn get_schema_permissions(&self, node_id: &str) -> Vec<String> {
        tokio::runtime::Handle::current().block_on(self.db_ops
            .get_schema_permissions(node_id))
            .unwrap_or_default()
    }

    /// Sets the permitted schemas for the given node.
    pub fn set_schema_permissions(&self, node_id: &str, schemas: &[String]) -> sled::Result<()> {
        tokio::runtime::Handle::current().block_on(self.db_ops
            .set_schema_permissions(node_id, schemas))
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
    /// Now fully async to support DbOperationsV2 with storage abstraction!
    pub async fn new(path: &str) -> Result<Self, StorageError> {
        let db = match sled::open(path) {
            Ok(db) => db,
            Err(e) => {
                if e.to_string().contains("No such file or directory") {
                    sled::open(path)
                        .map_err(|e| StorageError::IoError(std::io::Error::other(e.to_string())))?
                } else {
                    return Err(StorageError::IoError(std::io::Error::other(e.to_string())));
                }
            }
        };

        Self::initialize_from_db(db, path, None).await
    }

    /// Creates a new FoldDB instance with S3-backed storage.
    /// The database is downloaded from S3 on initialization and can be synced back with flush_to_s3().
    pub async fn new_with_s3(config: S3Config) -> Result<Self, StorageError> {
        // Initialize S3 storage (downloads from S3 if exists)
        let s3_storage = Arc::new(S3SyncedStorage::new(config).await?);
        
        // Get local path as a String to avoid borrowing issues
        let local_path_string = s3_storage.local_path().to_str()
            .ok_or_else(|| StorageError::InvalidPath("Invalid local path".to_string()))?
            .to_string();
        
        let db = sled::open(&local_path_string)
            .map_err(|e| StorageError::IoError(std::io::Error::other(e.to_string())))?;

        Self::initialize_from_db(db, &local_path_string, Some(s3_storage)).await
    }

    /// Common initialization logic shared by both new() and new_with_s3()
    /// This method initializes all FoldDB components from an already-opened sled database
    /// 
    /// Fully async - uses DbOperationsV2 with storage abstraction layer!
    async fn initialize_from_db(
        db: sled::Db, 
        db_path: &str,
        s3_storage: Option<Arc<S3SyncedStorage>>
    ) -> Result<Self, StorageError> {
        log_feature!(
            LogFeature::Database,
            info,
            "🔄 Using DbOperationsV2 with storage abstraction layer (Sled backend)"
        );
        
        // Use the new async storage abstraction!
        let db_ops = Arc::new(DbOperationsV2::from_sled(db.clone()).await?);
        
        log_feature!(
            LogFeature::Database,
            info,
            "✅ Storage abstraction active - using {} backend",
            "Sled"
        );

        // Initialize message bus
        let message_bus = Arc::new(MessageBus::new());

        // Initialize components via event-driven system initialization
        let correlation_id = uuid::Uuid::new_v4().to_string();
        let init_request = SystemInitializationRequest {
            correlation_id: correlation_id.clone(),
            db_path: db_path.to_string(),
            orchestrator_config: None,
        };

        // Send system initialization request via message bus
        message_bus.publish(init_request)
            .map_err(|e| StorageError::IoError(std::io::Error::other(e.to_string())))?;

        // Create managers using event-driven initialization only
        let transform_manager = init_transform_manager(Arc::clone(&db_ops), Arc::clone(&message_bus)).await
            .map_err(|e| StorageError::IoError(std::io::Error::other(e.to_string())))?;

        let schema_manager = Arc::new(
            SchemaCore::new(Arc::clone(&db_ops), Arc::clone(&message_bus)).await
                .map_err(|e| StorageError::IoError(std::io::Error::other(e.to_string())))?
        );

        // Create and start EventMonitor for system-wide observability
        let event_monitor = Arc::new(EventMonitor::new(Arc::clone(&message_bus), Arc::clone(&transform_manager)));
        info!("Started EventMonitor for system-wide event tracking");

        // Create QueryExecutor for handling all query operations
        let query_executor = QueryExecutor::new(
            Arc::clone(&db_ops),
            Arc::clone(&schema_manager),
        );
        info!("Created QueryExecutor for query operations");

        // Create TransformOrchestrator for managing transform execution
        let orchestrator_tree = db_ops.orchestrator_tree.clone()
            .ok_or_else(|| StorageError::BackendError("Orchestrator tree not available (only supported with Sled backend)".to_string()))?;
        
        let transform_orchestrator = Arc::new(TransformOrchestrator::new(
            Arc::clone(&transform_manager),
            orchestrator_tree,
            Arc::clone(&message_bus),
            Arc::clone(&db_ops),
        ));
        info!("Created TransformOrchestrator for transform execution");

        // Create MutationManager for handling all mutation operations
        let mutation_manager = MutationManager::new(
            Arc::clone(&db_ops),
            Arc::clone(&schema_manager),
            Arc::clone(&message_bus),
        );
        
        info!("Created MutationManager for mutation operations");

        // Start the MutationManager event listener
        mutation_manager.start_event_listener()
            .map_err(|e| StorageError::IoError(std::io::Error::other(e.to_string())))?;
        info!("Started MutationManager event listener");
        
        // Create and start IndexEventHandler for background indexing
        let index_event_handler = IndexEventHandler::new(
            Arc::clone(&message_bus),
            Arc::clone(&db_ops),
        );
        info!("Started IndexEventHandler for background indexing");
        
        // AtomManager operates via direct method calls, not event consumption.
        // Event-driven components:
        // - EventMonitor: System observability and statistics
        // - TransformOrchestrator: Automatic transform triggering based on field changes
        // - IndexEventHandler: Background indexing for improved mutation performance

        Ok(Self {
            schema_manager,
            transform_manager,
            db_ops,
            query_executor,
            message_bus,
            event_monitor,
            transform_orchestrator,
            mutation_manager,
            index_event_handler,
            s3_storage,
        })
    }

    /// Flushes local Sled database and syncs to S3 (if S3 storage is configured)
    pub async fn flush_to_s3(&self) -> Result<(), StorageError> {
        // First flush storage to ensure all data is persisted
        self.db_ops.flush().await
            .map_err(|e| StorageError::IoError(std::io::Error::other(e.to_string())))?;

        // Then sync to S3 if configured
        if let Some(s3_storage) = &self.s3_storage {
            s3_storage.sync_to_s3().await?;
            info!("Successfully synced database to S3");
        } else {
            return Err(StorageError::S3Error("S3 storage not configured".to_string()));
        }

        Ok(())
    }
    
    /// Returns true if this FoldDB instance is configured with S3 storage
    pub fn has_s3_storage(&self) -> bool {
        self.s3_storage.is_some()
    }
    
    // ========== INDEXING STATUS API ==========
    
    /// Get the current indexing status
    pub fn get_indexing_status(&self) -> super::orchestration::IndexingStatus {
        self.index_event_handler.get_status()
    }
    
    /// Check if indexing is currently in progress
    pub fn is_indexing(&self) -> bool {
        self.index_event_handler.is_indexing()
    }

    // ========== CONSOLIDATED SCHEMA API - DELEGATES TO SCHEMA_CORE ==========

    /// Load schema from JSON string (creates Available schema)
    pub async fn load_schema_from_json(&mut self, json_str: &str) -> Result<(), SchemaError> {
        // Delegate to SchemaCore implementation
        self.schema_manager.load_schema_from_json(json_str).await
    }

    /// Load schema from file (creates Available schema)
    pub async fn load_schema_from_file<P: AsRef<Path>>(&mut self, path: P) -> Result<(), SchemaError> {
        // Delegate to SchemaCore implementation
        self.schema_manager.load_schema_from_file(path).await
    }

    /// Provides access to the underlying database operations
    pub fn get_db_ops(&self) -> Arc<DbOperationsV2> {
        Arc::clone(&self.db_ops)
    }

    /// Get current event statistics from the event monitor
    pub fn get_event_statistics(&self) -> super::infrastructure::event_monitor::EventStatistics {
        self.event_monitor.get_statistics()
    }

    /// Get the backfill tracker
    pub fn get_backfill_tracker(&self) -> Arc<super::infrastructure::backfill_tracker::BackfillTracker> {
        self.event_monitor.get_backfill_tracker()
    }

    /// Get all backfill information
    pub fn get_all_backfills(&self) -> Vec<super::infrastructure::backfill_tracker::BackfillInfo> {
        self.event_monitor.get_all_backfills()
    }

    /// Get active (in-progress) backfills
    pub fn get_active_backfills(&self) -> Vec<super::infrastructure::backfill_tracker::BackfillInfo> {
        self.event_monitor.get_active_backfills()
    }

    /// Get specific backfill info
    pub fn get_backfill(&self, transform_id: &str) -> Option<super::infrastructure::backfill_tracker::BackfillInfo> {
        self.event_monitor.get_backfill(transform_id)
    }

    /// Get the message bus for publishing events (for testing)
    pub fn message_bus(&self) -> Arc<MessageBus> {
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
    /// Search the native word index for a specific term
    pub fn native_word_search(&self, term: &str) -> Result<Vec<IndexResult>, SchemaError> {
        self.db_ops.native_index_manager()
            .ok_or_else(|| SchemaError::InvalidData("Native index manager not available".to_string()))?
            .search_word(term)
    }

    /// Search native index across all classification types and aggregate results
    /// This now includes field name matches via search_word
    pub fn native_search_all_classifications(&self, term: &str) -> Result<Vec<IndexResult>, SchemaError> {
        debug!("FoldDB: native_search_all_classifications called for term: '{}'", term);
        
        // Delegate to NativeIndexManager which has the full implementation
        self.db_ops.native_index_manager()
            .ok_or_else(|| SchemaError::InvalidData("Native index manager not available".to_string()))?
            .search_all_classifications(term)
    }

    /// Get the transform orchestrator for managing transform execution
    pub fn transform_orchestrator(&self) -> Arc<TransformOrchestrator> {
        Arc::clone(&self.transform_orchestrator)
    }
}
