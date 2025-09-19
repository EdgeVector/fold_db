//! FoldDB Core - Event-driven database system
//! 
//! This module contains the core components of the FoldDB system organized
//! into logical groups for better maintainability and understanding:
//! 
//! - **managers/**: Core managers for different aspects of data management
//! - **services/**: Service layer components for operations
//! - **infrastructure/**: Foundation components (message bus, initialization, etc.)
//! - **orchestration/**: Coordination and orchestration components
//! - **shared/**: Common utilities and shared components
//! - **transform_manager/**: Transform system (already well-organized)

// Organized module declarations
pub mod managers;
pub mod services;
pub mod infrastructure;
pub mod orchestration;
pub mod shared;
pub mod transform_manager;
pub mod query;
pub mod mutation;

// Core components
pub mod mutation_completion_handler;

// Re-export key components for backwards compatibility
pub use managers::AtomManager; // FieldManager removed (was dead code), CollectionManager removed - collections no longer supported
pub use services::field_retrieval::service::FieldRetrievalService;
pub use infrastructure::{MessageBus, EventMonitor};
pub use orchestration::TransformOrchestrator;
pub use transform_manager::TransformManager;
pub use shared::*;
pub use query::QueryExecutor;
pub use mutation::MutationExecutor;

// Re-export core components
pub use mutation_completion_handler::{MutationCompletionHandler, MutationCompletionError, MutationCompletionResult, MutationCompletionDiagnostics, DEFAULT_COMPLETION_TIMEOUT};

// Import infrastructure components that are used internally
use infrastructure::message_bus::{
    request_events::{
        FieldValueSetResponse, FieldUpdateResponse, SchemaLoadResponse, SchemaApprovalResponse,
        AtomCreateResponse, MoleculeCreateResponse, MoleculeUpdateRequest, SystemInitializationRequest,
    },
};
use crate::fold_db_core::transform_manager::types::TransformRunner;
use infrastructure::init::{init_orchestrator, init_transform_manager};

// External dependencies
use crate::atom::MoleculeBehavior;
use crate::db_operations::DbOperations;
use crate::permissions::PermissionWrapper;
use crate::schema::SchemaState;
use crate::schema::SchemaCore;
use crate::schema::{Schema, SchemaError};
use log::info;
use crate::logging::features::{log_feature, LogFeature};
use serde_json::Value;
use crate::schema::types::{Mutation, Query};
use std::collections::HashMap;
use std::path::Path;
use std::sync::Arc;

// REMOVED: PendingOperationRequest - marked as dead code, never used

/// Unified response type for all operations
#[derive(Debug, Clone)]
pub enum OperationResponse {
    FieldValueSetResponse(FieldValueSetResponse),
    FieldUpdateResponse(FieldUpdateResponse),
    SchemaLoadResponse(SchemaLoadResponse),
    SchemaApprovalResponse(SchemaApprovalResponse),
    AtomCreateResponse(AtomCreateResponse),
    MoleculeCreateResponse(MoleculeCreateResponse),
    Error(String),
    Timeout,
}

/// The main database coordinator that manages schemas, permissions, and data storage.
pub struct FoldDB {
    pub(crate) atom_manager: AtomManager,
    pub(crate) field_retrieval_service: FieldRetrievalService,
    pub(crate) schema_manager: Arc<SchemaCore>,
    pub(crate) transform_manager: Arc<TransformManager>,
    pub(crate) transform_orchestrator: Arc<TransformOrchestrator>,
    /// Shared database operations
    pub(crate) db_ops: Arc<DbOperations>,
    #[allow(dead_code)]
    permission_wrapper: PermissionWrapper,
    /// Query executor for handling all query operations
    query_executor: QueryExecutor,
    /// Mutation executor for handling all mutation operations
    mutation_executor: MutationExecutor,
    /// Message bus for event-driven communication
    pub(crate) message_bus: Arc<MessageBus>,
    /// Event monitor for system-wide observability
    pub(crate) event_monitor: Arc<infrastructure::event_monitor::EventMonitor>,
    /// Mutation completion handler for tracking async mutation completion
    pub(crate) completion_handler: Arc<MutationCompletionHandler>,
}

impl FoldDB {
    /// Retrieves or generates and persists the node identifier.
    pub fn get_node_id(&self) -> Result<String, sled::Error> {
        self.db_ops
            .get_node_id()
            .map_err(|e| sled::Error::Unsupported(e.to_string()))
    }

    /// Retrieves the list of permitted schemas for the given node.
    pub fn get_schema_permissions(&self, node_id: &str) -> Vec<String> {
        self.db_ops
            .get_schema_permissions(node_id)
            .unwrap_or_default()
    }

    /// Sets the permitted schemas for the given node.
    pub fn set_schema_permissions(&self, node_id: &str, schemas: &[String]) -> sled::Result<()> {
        self.db_ops
            .set_schema_permissions(node_id, schemas)
            .map_err(|e| sled::Error::Unsupported(e.to_string()))
    }

    /// Properly close and flush the database to release all file locks
    pub fn close(&self) -> Result<(), sled::Error> {
        log_feature!(LogFeature::Database, info, "Closing FoldDB and flushing all data to disk");
        
        // Flush the main database
        if let Err(e) = self.db_ops.db().flush() {
            log_feature!(LogFeature::Database, error, "Failed to flush main database: {}", e);
            return Err(e);
        }
        
        log_feature!(LogFeature::Database, info, "FoldDB closed successfully");
        Ok(())
    }

    /// Creates a new FoldDB instance with the specified storage path.
    pub fn new(path: &str) -> sled::Result<Self> {
        let db = match sled::open(path) {
            Ok(db) => db,
            Err(e) => {
                if e.to_string().contains("No such file or directory") {
                    sled::open(path)?
                } else {
                    return Err(e);
                }
            }
        };

        let db_ops =
            DbOperations::new(db.clone()).map_err(|e| sled::Error::Unsupported(e.to_string()))?;
        let orchestrator_tree = db_ops.orchestrator_tree.clone();

        // Initialize message bus
        let message_bus = infrastructure::factory::InfrastructureFactory::create_message_bus();

        // Initialize components via event-driven system initialization
        let correlation_id = uuid::Uuid::new_v4().to_string();
        let init_request = SystemInitializationRequest {
            correlation_id: correlation_id.clone(),
            db_path: path.to_string(),
            orchestrator_config: None,
        };

        // Send system initialization request via message bus
        if let Err(e) = message_bus.publish(init_request) {
            return Err(sled::Error::Unsupported(format!(
                "Failed to initialize system via events: {}",
                e
            )));
        }

        // Create managers using event-driven initialization only
        let db_ops_arc = Arc::new(db_ops.clone());
        let atom_manager = AtomManager::new(db_ops.clone(), Arc::clone(&message_bus));
        let schema_manager = Arc::new(
            SchemaCore::new(path, Arc::clone(&db_ops_arc), Arc::clone(&message_bus))
                .map_err(|e| sled::Error::Unsupported(e.to_string()))?,
        );

        // Use standard initialization but with deprecated closures that recommend events
        let transform_manager = init_transform_manager(Arc::new(db_ops.clone()), Arc::clone(&message_bus))?;
        let orchestrator =
            init_orchestrator(&FieldRetrievalService::new(Arc::clone(&message_bus)), transform_manager.clone(), orchestrator_tree, Arc::clone(&message_bus), Arc::new(db_ops.clone()))?;

        info!("Loading schema states from disk during FoldDB initialization");
        if let Err(e) = schema_manager.discover_and_load_all_schemas() {
            info!("Failed to load schema states: {}", e);
        } else {
            // After loading schema states, we need to ensure approved schemas are moved from 'available'
            // to 'schemas' HashMap so that map_fields() can find them
            if let Ok(approved_schemas) =
                schema_manager.list_schemas_by_state(SchemaState::Approved)
            {
                info!("Moving {} approved schemas from 'available' to 'schemas' HashMap for field mapping", approved_schemas.len());
                
                // Move approved schemas from available to schemas HashMap
                for schema_name in &approved_schemas {
                    if let Err(e) = schema_manager.ensure_approved_schema_in_schemas(schema_name) {
                        info!("Failed to move approved schema '{}' to schemas HashMap: {}", schema_name, e);
                    }
                }
                
                // Now proceed with field mapping for all approved schemas
                for schema_name in approved_schemas {
                    if let Ok(molecules) = schema_manager.map_fields(&schema_name) {
                        // Persist each molecule using event-driven communication
                        for molecule in molecules {
                            let molecule_uuid = molecule.uuid().to_string();
                            let atom_uuid = molecule.get_atom_uuid().clone();

                            // Send MoleculeUpdateRequest via message bus
                            let correlation_id = uuid::Uuid::new_v4().to_string();
                            let update_request = MoleculeUpdateRequest {
                                correlation_id: correlation_id.clone(),
                                molecule_uuid: molecule_uuid.clone(),
                                atom_uuid,
                                source_pub_key: "system".to_string(),
                                molecule_type: "Single".to_string(), // Default type for schema initialization
                                additional_data: None,
                            };

                            if let Err(e) = message_bus.publish(update_request) {
                                info!(
                                    "Failed to publish MoleculeUpdateRequest for schema '{}': {}",
                                    schema_name, e
                                );
                            }
                        }
                    }
                    
                }
            }
        }

        // Create and start EventMonitor for system-wide observability
        let event_monitor = Arc::new(infrastructure::event_monitor::EventMonitor::new(&message_bus));
        info!("Started EventMonitor for system-wide event tracking");

        // Create MutationCompletionHandler for tracking async mutation completion
        let completion_handler = Arc::new(MutationCompletionHandler::new(Arc::clone(&message_bus)));
        info!("Created MutationCompletionHandler for mutation completion tracking");

        // Create QueryExecutor for handling all query operations
        let query_executor = QueryExecutor::new(
            Arc::new(db_ops.clone()),
            Arc::clone(&schema_manager),
            PermissionWrapper::new(),
        );
        info!("Created QueryExecutor for query operations");

        // Create MutationExecutor for handling all mutation operations
        let mutation_executor = MutationExecutor::new(
            Arc::new(db_ops.clone()),
            Arc::clone(&schema_manager),
            Arc::clone(&message_bus),
            Arc::clone(&completion_handler),
        );
        info!("Created MutationExecutor for mutation operations");

        // AtomManager operates via direct method calls, not event consumption.
        // Event-driven components:
        // - EventMonitor: System observability and statistics
        // - TransformOrchestrator: Automatic transform triggering based on field changes
        // - MutationCompletionHandler: Tracks async mutation completion

        Ok(Self {
            atom_manager,
            field_retrieval_service: FieldRetrievalService::new(Arc::clone(&message_bus)),
            schema_manager,
            transform_manager,
            transform_orchestrator: orchestrator,
            db_ops: Arc::new(db_ops.clone()),
            permission_wrapper: PermissionWrapper::new(),
            query_executor,
            mutation_executor,
            message_bus,
            event_monitor,
            completion_handler,
        })
    }

    // ========== CONSOLIDATED SCHEMA API - DELEGATES TO SCHEMA_CORE ==========



    /// Load schema from JSON string (creates Available schema)
    pub fn load_schema_from_json(&mut self, json_str: &str) -> Result<(), SchemaError> {
        // Delegate to working schema_manager implementation
        self.schema_manager.load_schema_from_json(json_str)
    }

    /// Load schema from file (creates Available schema)
    pub fn load_schema_from_file<P: AsRef<Path>>(&mut self, path: P) -> Result<(), SchemaError> {
        // Delegate to working schema_manager implementation
        self.schema_manager.load_schema_from_file(path.as_ref().to_str().unwrap())
    }

    /// Add a schema to available schemas (for testing compatibility)
    pub fn add_schema_available(&mut self, schema: Schema) -> Result<(), SchemaError> {
        // Delegate to working schema_manager implementation
        self.schema_manager.add_schema_available(schema)
    }

    /// Approve a schema for queries and mutations (for testing compatibility)
    pub fn approve_schema(&mut self, schema_name: &str) -> Result<(), SchemaError> {
        // Delegate to working schema_manager implementation
        self.schema_manager.approve_schema(schema_name)
    }

    /// Mark a schema as unloaded without removing transforms.
    pub fn unload_schema(&self, schema_name: &str) -> Result<(), SchemaError> {
        self.schema_manager.unload_schema(schema_name)
    }

    /// Get a schema by name - public accessor for testing
    pub fn get_schema(
        &self,
        schema_name: &str,
    ) -> Result<Option<crate::schema::Schema>, SchemaError> {
        self.schema_manager.get_schema(schema_name)
    }

    /// Provides access to the underlying database operations
    pub fn get_db_ops(&self) -> Arc<DbOperations> {
        Arc::clone(&self.db_ops)
    }

    /// Provides access to the field retrieval service for testing
    pub fn field_retrieval_service(&self) -> &FieldRetrievalService {
        &self.field_retrieval_service
    }

    /// Provides access to the atom manager for testing
    pub fn atom_manager(&self) -> &AtomManager {
        &self.atom_manager
    }

    /// Provides access to the event monitor for observability
    pub fn event_monitor(&self) -> Arc<infrastructure::event_monitor::EventMonitor> {
        Arc::clone(&self.event_monitor)
    }

    /// Get current event statistics from the event monitor
    pub fn get_event_statistics(&self) -> infrastructure::event_monitor::EventStatistics {
        self.event_monitor.get_statistics()
    }

    /// Log a summary of all system activity since FoldDB was created
    pub fn log_event_summary(&self) {
        self.event_monitor.log_summary()
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

    /// Provides access to the mutation completion handler for tracking async mutation completion.
    ///
    /// This method returns a shared reference to the MutationCompletionHandler, allowing other
    /// parts of the system to register mutations for completion tracking and wait for their completion.
    ///
    /// # Returns
    ///
    /// A shared reference to the MutationCompletionHandler wrapped in Arc
    ///
    /// # Example
    ///
    /// ```rust
    /// # use datafold::fold_db_core::FoldDB;
    /// # let mut db = FoldDB::new("test_db").unwrap();
    /// let completion_handler = db.get_completion_handler();
    /// // Use the completion handler to track mutations
    /// ```
    pub fn get_completion_handler(&self) -> Arc<MutationCompletionHandler> {
        Arc::clone(&self.completion_handler)
    }

    /// Query a Range schema and return grouped results by range_key
    pub fn query_range_schema(&self, query: Query) -> Result<Value, SchemaError> {
        self.query_executor.query(query)
    }

    /// Query multiple fields from a schema
    pub fn query(&self, query: Query) -> Result<Value, SchemaError> {
        self.query_executor.query(query)
    }

    /// Query a schema (compatibility method)
    pub fn query_schema(&self, query: Query) -> Vec<Result<Value, SchemaError>> {
        self.query_executor.query_schema(query)
    }

    /// Write schema operation - main orchestration method for mutations
    pub fn write_schema(&mut self, mutation: Mutation) -> Result<String, SchemaError> {
        self.mutation_executor.write_schema(mutation)
    }

    /// Register a transform with the system
    pub fn register_transform(
        &self,
        _transform: crate::schema::types::Transform,
    ) -> Result<(), SchemaError> {
        // For now, return error since TransformRegistration is expected, not Transform
        Err(SchemaError::InvalidData(
            "Transform registration not yet implemented - needs TransformRegistration type".to_string()
        ))
    }

    /// List all registered transforms
    pub fn list_transforms(&self) -> Result<HashMap<String, crate::schema::types::Transform>, SchemaError> {
        self.transform_manager.list_transforms()
    }

    /// Execute a transform by ID using direct execution
    /// This executes the transform immediately and returns the result
    pub fn run_transform(&self, transform_id: &str) -> Result<Value, SchemaError> {
        println!("🔄 run_transform called for {} - using direct execution", transform_id);
        
        // Use direct execution through the transform manager
        println!("🔄 About to call TransformRunner::execute_transform_now for transform: {}", transform_id);
        match TransformRunner::execute_transform_now(&*self.transform_manager, transform_id) {
            Ok(result) => {
                println!("🔄 TransformRunner::execute_transform_now completed successfully with result: {}", result);
                Ok(result)
            },
            Err(e) => {
                println!("🔄 TransformRunner::execute_transform_now failed with error: {}", e);
                Err(SchemaError::InvalidData(e.to_string()))
            },
        }
    }

    /// Process any pending transforms in the queue
    pub fn process_transform_queue(&self) {
        // Transform orchestrator processing is handled automatically by events
        // self.transform_orchestrator.process_pending_transforms();
    }

    /// Reload transforms from the database
    pub fn reload_transforms(&self) -> Result<(), SchemaError> {
        self.transform_manager.reload_transforms()
    }

    /// Waits for a specific mutation to complete processing.
    ///
    /// This method allows queries and other operations to wait for specific mutations to finish
    /// processing before executing, solving the race condition where queries try to access data
    /// before mutations finish processing. This is the core functionality that eliminates
    /// "Atom not found" errors.
    ///
    /// # Arguments
    ///
    /// * `mutation_id` - The unique identifier of the mutation to wait for completion
    ///
    /// # Returns
    ///
    /// Returns `Ok(())` if the mutation completed successfully within the timeout period,
    /// or a `SchemaError` if the operation failed.
    ///
    /// # Timeout Behavior
    ///
    /// Uses the default 5-second timeout as defined in `MutationCompletionHandler`.
    /// If the mutation does not complete within this timeframe, a timeout error is returned.
    ///
    /// # Error Handling
    ///
    /// - **Timeout**: Returns `SchemaError::InvalidData("Mutation failed")` when the mutation
    ///   does not complete within the 5-second timeout
    /// - **Invalid ID**: Returns `SchemaError::InvalidData` with details when the mutation ID
    ///   is not found or was never registered
    /// - **System Error**: Returns appropriate `SchemaError` for lock failures or channel errors
    ///
    /// # Usage Examples
    ///
    /// ## Basic Usage
    /// ```no_run
    /// use datafold::fold_db_core::FoldDB;
    /// use datafold::schema::types::{Mutation, MutationType};
    /// use std::collections::HashMap;
    /// use serde_json::Value;
    ///
    /// # async fn example() -> Result<(), Box<dyn std::error::Error>> {
    /// let mut db = FoldDB::new("test_db")?;
    ///
    /// // Execute a mutation and get the mutation ID
    /// let fields = HashMap::new();
    /// let mutation = Mutation::new(
    ///     "schema_name".to_string(),
    ///     fields,
    ///     "pub_key".to_string(),
    ///     0,
    ///     MutationType::Update
    /// );
    /// let mutation_id = db.write_schema(mutation)?;
    ///
    /// // Wait for the mutation to complete before querying
    /// db.wait_for_mutation(&mutation_id).await?;
    /// # Ok(())
    /// # }
    /// ```
    ///
    /// ## Error Handling Example
    /// ```no_run
    /// use datafold::fold_db_core::FoldDB;
    ///
    /// # async fn example() -> Result<(), Box<dyn std::error::Error>> {
    /// let db = FoldDB::new("test_db")?;
    ///
    /// match db.wait_for_mutation("invalid-mutation-id").await {
    ///     Ok(()) => println!("Mutation completed successfully"),
    ///     Err(e) => println!("Mutation failed or timed out: {}", e),
    /// }
    /// # Ok(())
    /// # }
    /// ```
    ///
    /// # Implementation Notes
    ///
    /// This method integrates with the `MutationCompletionHandler` to provide efficient
    /// completion tracking. The mutation must be registered with the completion handler
    /// (typically done by `write_schema`) before calling this method.
    ///
    /// The method is async and non-blocking, allowing other operations to continue while
    /// waiting for mutation completion.
    pub async fn wait_for_mutation(&self, mutation_id: &str) -> Result<(), SchemaError> {
        self.mutation_executor.wait_for_mutation(mutation_id).await
    }
}
