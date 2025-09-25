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
pub mod infrastructure;
pub mod orchestration;
pub mod query;
pub mod shared;
pub mod transform_manager;

// Core components
pub mod mutation_completion_handler;

// Re-export key components for backwards compatibility
pub use infrastructure::{EventMonitor, MessageBus};
pub use orchestration::TransformOrchestrator;
pub use query::QueryExecutor;
pub use shared::*;
pub use transform_manager::TransformManager;

// Re-export core components
pub use mutation_completion_handler::{
    MutationCompletionDiagnostics, MutationCompletionError, MutationCompletionHandler,
    MutationCompletionResult, DEFAULT_COMPLETION_TIMEOUT,
};

// Standard library imports
use std::collections::HashMap;
use std::path::Path;
use std::sync::Arc;

// External crate imports
use log::info;
use serde_json::Value;

// Internal crate imports
use crate::db_operations::DbOperations;
use crate::logging::features::{log_feature, LogFeature};
use crate::permissions::PermissionWrapper;
use crate::schema::types::{Mutation, Query, key_config::KeyConfig};
use crate::schema::types::field::Field;
use crate::schema::{SchemaCore, SchemaError};
use crate::atom::Atom;

// Infrastructure components that are used internally
use infrastructure::init::{init_transform_manager};
use infrastructure::message_bus::request_events::{
    AtomCreateResponse, FieldUpdateResponse, FieldValueSetResponse, MoleculeCreateResponse,
    SchemaApprovalResponse, SchemaLoadResponse, SystemInitializationRequest,
};

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
    pub(crate) schema_manager: Arc<SchemaCore>,
    pub(crate) transform_manager: Arc<TransformManager>,
    /// Shared database operations
    pub(crate) db_ops: Arc<DbOperations>,
    #[allow(dead_code)]
    permission_wrapper: PermissionWrapper,
    /// Query executor for handling all query operations
    query_executor: QueryExecutor,
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
        log_feature!(
            LogFeature::Database,
            info,
            "Closing FoldDB and flushing all data to disk"
        );

        // Flush the main database
        if let Err(e) = self.db_ops.db().flush() {
            log_feature!(
                LogFeature::Database,
                error,
                "Failed to flush main database: {}",
                e
            );
            return Err(e);
        }

        log_feature!(LogFeature::Database, info, "FoldDB closed successfully");
        Ok(())
    }

    /// Creates a new FoldDB instance with the specified storage path.
    /// All initializations happen here. This is the main entry point for the FoldDB system.
    /// Do not initialize anywhere else.
    /// updated by @tomtang2
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
        let message_bus = Arc::new(MessageBus::new());

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
        let schema_manager = Arc::new(
            SchemaCore::new(Arc::clone(&db_ops_arc), Arc::clone(&message_bus))
                .map_err(|e| sled::Error::Unsupported(e.to_string()))?,
        );

        // Use standard initialization but with deprecated closures that recommend events
        let transform_manager =
            init_transform_manager(Arc::new(db_ops.clone()), Arc::clone(&message_bus))?;

        // Create and start EventMonitor for system-wide observability
        let event_monitor = Arc::new(infrastructure::event_monitor::EventMonitor::new(
            &message_bus,
        ));
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

        // AtomManager operates via direct method calls, not event consumption.
        // Event-driven components:
        // - EventMonitor: System observability and statistics
        // - TransformOrchestrator: Automatic transform triggering based on field changes
        // - MutationCompletionHandler: Tracks async mutation completion

        Ok(Self {
            schema_manager,
            transform_manager,
            db_ops: Arc::new(db_ops.clone()),
            permission_wrapper: PermissionWrapper::new(),
            query_executor,
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
        self.schema_manager
            .load_schema_from_file(path.as_ref().to_str().unwrap())
    }

    /// Provides access to the underlying database operations
    pub fn get_db_ops(&self) -> Arc<DbOperations> {
        Arc::clone(&self.db_ops)
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
    pub fn write_mutation(&mut self, mutation: Mutation) -> Result<String, SchemaError> {
        // Get the schema definition
        let mut schema = self.schema_manager.get_schema(&mutation.schema_name)?
            .ok_or_else(|| SchemaError::InvalidData(format!("Schema '{}' not found", mutation.schema_name)))?;
        
        // Extract hash and range key field names from the mutation's key_config
        let hash_key = mutation.key_config.hash_field.as_ref()
            .ok_or_else(|| SchemaError::InvalidData("Hash key field not specified".to_string()))?;
        let range_key = mutation.key_config.range_field.as_ref()
            .ok_or_else(|| SchemaError::InvalidData("Range key field not specified".to_string()))?;
        
        // Get the actual hash and range values from the mutation
        let hash_value = mutation.fields_and_values.get(hash_key)
            .ok_or_else(|| SchemaError::InvalidData(format!("Hash key '{}' not found in mutation", hash_key)))?
            .as_str()
            .ok_or_else(|| SchemaError::InvalidData(format!("Hash key '{}' must be a string", hash_key)))?;
        let range_value = mutation.fields_and_values.get(range_key)
            .ok_or_else(|| SchemaError::InvalidData(format!("Range key '{}' not found in mutation", range_key)))?
            .as_str()
            .ok_or_else(|| SchemaError::InvalidData(format!("Range key '{}' must be a string", range_key)))?;
        
        let key_config = KeyConfig::new(Some(hash_value.to_string()), Some(range_value.to_string()));
        // Generate a unique mutation ID
        let mutation_id = uuid::Uuid::new_v4().to_string();
        
        // Process each field in the mutation
        for (field_name, value) in mutation.fields_and_values {
            if let Some(schema_field) = schema.fields.get_mut(&field_name) {
                schema_field.refresh_from_db(&self.db_ops);
                let new_atom = Atom::new(mutation.schema_name.clone(), mutation.pub_key.clone(), value);
                schema_field.write_mutation(&key_config, new_atom, mutation.pub_key.clone());
            }
        }

        
        
        // Return the mutation ID
        Ok(mutation_id)
    }

    /// Register a transform with the system
    pub fn register_transform(
        &self,
        _transform: crate::schema::types::Transform,
    ) -> Result<(), SchemaError> {
        // For now, return error since TransformRegistration is expected, not Transform
        Err(SchemaError::InvalidData(
            "Transform registration not yet implemented - needs TransformRegistration type"
                .to_string(),
        ))
    }

    /// List all registered transforms
    pub fn list_transforms(
        &self,
    ) -> Result<HashMap<String, crate::schema::types::Transform>, SchemaError> {
        self.transform_manager.list_transforms()
    }

    /// Process any pending transforms in the queue
    pub fn process_transform_queue(&self) {
        // Transform orchestrator processing is handled automatically by events
        // self.transform_orchestrator.process_pending_transforms();
    }
}
