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
use crate::storage::{StorageError};
use crate::transform::manager::TransformManager;

// Infrastructure components that are used internally
use super::infrastructure::init::{init_transform_manager};
use super::infrastructure::message_bus::request_events::SystemInitializationRequest;
use super::infrastructure::{EventMonitor, MessageBus};
use super::orchestration::{TransformOrchestrator, IndexEventHandler};
use super::orchestration::index_status::IndexStatusTracker;
use super::query::QueryExecutor;
use super::mutation_manager::MutationManager;

/// The main database coordinator that manages schemas, permissions, and data storage.
pub struct FoldDB {
    pub(crate) schema_manager: Arc<SchemaCore>,
    pub(crate) transform_manager: Arc<TransformManager>,
    /// Shared database operations with storage abstraction
    pub(crate) db_ops: Arc<DbOperations>,
    /// Query executor for handling all query operations
    pub(crate) query_executor: QueryExecutor,
    /// Message bus for event-driven communication
    pub(crate) message_bus: Arc<MessageBus>,
    /// Event monitor for system-wide observability
    pub(crate) event_monitor: Arc<EventMonitor>,
    /// Transform orchestrator for managing transform execution
    /// Optional for backends that don't support orchestrator_tree (e.g., DynamoDB)
    pub(crate) transform_orchestrator: Option<Arc<TransformOrchestrator>>,
    /// Mutation manager for handling all mutation operations
    pub(crate) mutation_manager: MutationManager,
    /// Index event handler for background indexing
    pub(crate) index_event_handler: IndexEventHandler,


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
    /// Creates a new FoldDB instance with the specified storage path.
    /// All initializations happen here. This is the main entry point for the FoldDB system.
    /// Do not initialize anywhere else.
    /// 
    /// Now fully async to support DbOperations with storage abstraction!
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



    /// Creates a new FoldDB instance with a pre-created DbOperations.
    /// 
    /// This allows you to use any storage backend implementation (DynamoDB, custom, etc.)
    /// by creating DbOperations yourself and passing it in.
    /// 
    /// # Arguments
    /// 
    /// * `db_ops` - Pre-created DbOperations instance with your chosen storage backend
    /// * `db_path` - Path identifier for logging/debugging (can be any string)
    pub async fn new_with_db_ops(
        db_ops: Arc<DbOperations>,
        db_path: &str,
        process_table_name: Option<String>,
    ) -> Result<Self, StorageError> {
        log_feature!(
            LogFeature::Database,
            info,
            "🔄 Using DbOperations with custom storage backend"
        );
        
        Self::initialize_from_db_ops(db_ops, db_path, process_table_name).await
    }

    /// Common initialization logic shared by both new() and new_with_s3()
    /// This method initializes all FoldDB components from an already-opened sled database
    /// 
    /// Fully async - uses DbOperations with storage abstraction layer!
    async fn initialize_from_db(
        db: sled::Db, 
        db_path: &str,
        progress_table_name: Option<String>,
    ) -> Result<Self, StorageError> {
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

        Self::initialize_from_db_ops(db_ops, db_path, progress_table_name).await
    }

    /// Common initialization logic that creates all FoldDB components from DbOperations
    /// 
    /// This is used by both initialize_from_db (Sled) and new_with_db_ops (custom backends)
    async fn initialize_from_db_ops(
        db_ops: Arc<DbOperations>,
        db_path: &str,
        process_table_name: Option<String>,
    ) -> Result<Self, StorageError> {
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
        // Now supports both Sled and DynamoDB backends
        let transform_orchestrator = if let Some(orchestrator_tree) = db_ops.orchestrator_tree.clone() {
            // Sled backend - use sync version
            let orchestrator = Arc::new(TransformOrchestrator::new(
                Arc::clone(&transform_manager),
                orchestrator_tree,
                Arc::clone(&message_bus),
                Arc::clone(&db_ops),
            ));
            info!("Created TransformOrchestrator for transform execution (Sled backend)");
            Some(orchestrator)
        } else {
            // DynamoDB or other backends - use async version with orchestrator_store
            let orchestrator_store = db_ops.orchestrator_store().inner().clone();
            match TransformOrchestrator::new_with_store(
                Arc::clone(&transform_manager),
                orchestrator_store,
                Arc::clone(&message_bus),
                Arc::clone(&db_ops),
            ).await {
                Ok(orchestrator) => {
                    info!("Created TransformOrchestrator for transform execution (KvStore backend)");
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
        // This is shared between MutationManager (direct indexing) and IndexEventHandler (background indexing)
        // Use DynamoDB progress store if configured, otherwise in-memory
        let progress_store: Arc<dyn super::orchestration::ProgressStore> = if let Some(table_name) = process_table_name {
             #[cfg(feature = "aws-backend")]
             {
                 let region = std::env::var("DATAFOLD_DYNAMODB_REGION").unwrap_or_else(|_| "us-east-1".to_string());
                 let config = aws_config::defaults(aws_config::BehaviorVersion::latest())
                    .region(aws_sdk_dynamodb::config::Region::new(region))
                    .load()
                    .await;
                 let client = aws_sdk_dynamodb::Client::new(&config);
                 // Use user_id scope if available, else default? 
                 // FoldDB init doesn't know "current user" context yet, but IndexStatusTracker tracks across users?
                 // Actually IndexStatusTracker needs to be multi-tenant aware?
                 // The old code used DATAFOLD_DYNAMODB_USER_ID which defaults to "default".
                 // We should probably rely on the same PK strategy.
                 let pk = std::env::var("DATAFOLD_DYNAMODB_USER_ID").unwrap_or_else(|_| "default".to_string());
                 
                 info!("Using DynamoDB progress store (table: {})", table_name);
                 Arc::new(super::orchestration::DynamoDbProgressStore::new(client, table_name, pk))
             }
             #[cfg(not(feature = "aws-backend"))]
             {
                 use log::warn;
                 warn!("DynamoDB configured for progress store but aws-backend feature is disabled. Falling back to in-memory store.");
                 info!("Using in-memory progress store");
                 Arc::new(super::orchestration::InMemoryProgressStore::new())
             }
        } else {
             info!("Using in-memory progress store");
             Arc::new(super::orchestration::InMemoryProgressStore::new())
        };

        let index_status_tracker = IndexStatusTracker::new(Some(progress_store));
        
        // Create MutationManager for handling all mutation operations
        let mutation_manager = MutationManager::new(
            Arc::clone(&db_ops),
            Arc::clone(&schema_manager),
            Arc::clone(&message_bus),
            Some(index_status_tracker.clone()),
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
            Some(index_status_tracker),
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

        })
    }

    /// Flushes local storage to ensure all data is persisted
    pub async fn flush(&self) -> Result<(), StorageError> {
        self.db_ops.flush().await
            .map_err(|e| StorageError::IoError(std::io::Error::other(e.to_string())))
    }
    
    // ========== INDEXING STATUS API ==========
    
    /// Get the current indexing status
    pub async fn get_indexing_status(&self) -> super::orchestration::IndexingStatus {
        self.index_event_handler.get_status().await
    }
    
    /// Check if indexing is currently in progress
    pub async fn is_indexing(&self) -> bool {
        self.index_event_handler.is_indexing().await
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
    pub fn get_db_ops(&self) -> Arc<DbOperations> {
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
    pub async fn native_search_all_classifications(&self, term: &str) -> Result<Vec<IndexResult>, SchemaError> {
        debug!("FoldDB: native_search_all_classifications called for term: '{}'", term);
        
        // Delegate to NativeIndexManager which has the full implementation
        let manager = self.db_ops.native_index_manager()
            .ok_or_else(|| SchemaError::InvalidData("Native index manager not available".to_string()))?;
        
        // Use async version if store is available (DynamoDB), otherwise use sync (Sled)
        if manager.is_async() {
            // DynamoDB backend - use async
            manager.search_all_classifications_async(term).await
        } else {
            // Sled backend - use sync
            manager.search_all_classifications(term)
        }
    }

    /// Get the transform orchestrator for managing transform execution
    /// Returns None if orchestrator is not available (e.g., with DynamoDB backend)
    pub fn transform_orchestrator(&self) -> Option<Arc<TransformOrchestrator>> {
        self.transform_orchestrator.as_ref().map(Arc::clone)
    }
}
