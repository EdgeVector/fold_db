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

// Core components
pub mod mutation_completion_handler;

// Re-export key components for backwards compatibility
pub use managers::AtomManager; // FieldManager removed (was dead code), CollectionManager removed - collections no longer supported
pub use services::field_retrieval::service::FieldRetrievalService;
pub use infrastructure::{MessageBus, EventMonitor};
pub use orchestration::TransformOrchestrator;
pub use transform_manager::TransformManager;
pub use shared::*;

// Re-export core components
pub use mutation_completion_handler::{MutationCompletionHandler, MutationCompletionError, MutationCompletionResult, MutationCompletionDiagnostics, DEFAULT_COMPLETION_TIMEOUT};

// Import infrastructure components that are used internally
use infrastructure::message_bus::{
    request_events::{
        FieldValueSetResponse, FieldUpdateResponse, SchemaLoadResponse, SchemaApprovalResponse,
        AtomCreateResponse, MoleculeCreateResponse, MoleculeUpdateRequest, SystemInitializationRequest,
    },
    query_events::MutationExecuted,
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
use log::{info, warn};
use crate::logging::features::{log_feature, LogFeature};
use serde_json::Value;
use std::time::Instant;
use crate::schema::types::{Mutation, Query};
use std::collections::HashMap;
use std::path::Path;
use std::sync::Arc;
use sha2::Digest;

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
    permission_wrapper: PermissionWrapper,
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
    pub fn query_range_schema(&self, _query: Query) -> Result<Value, SchemaError> {
        // CONVERTED TO EVENT-DRIVEN: Use SchemaLoadRequest instead of direct schema_manager access
        Err(SchemaError::InvalidData(
            "Method deprecated: Use event-driven SchemaLoadRequest via message bus instead of direct schema_manager access".to_string()
        ))
    }

    /// Query multiple fields from a schema
    pub fn query(&self, query: Query) -> Result<Value, SchemaError> {
        use log::info;
        
        info!("🔍 EVENT-DRIVEN query for schema: {}", query.schema_name);
        
        // Get schema first
        let schema = match self.schema_manager.get_schema(&query.schema_name)? {
            Some(schema) => schema,
            None => {
                return Err(SchemaError::NotFound(format!(
                    "Schema '{}' not found",
                    query.schema_name
                )));
            }
        };
        
        // Check field-level permissions for each field in the query
        for field_name in &query.fields {
            let permission_result = self.permission_wrapper.check_query_field_permission(
                &query,
                field_name,
                &self.schema_manager,
            );
            
            if !permission_result.allowed {
                return Err(permission_result.error.unwrap_or_else(|| {
                    SchemaError::InvalidData(format!(
                        "Permission denied for field '{}' in schema '{}' with trust distance {}",
                        field_name, query.schema_name, query.trust_distance
                    ))
                }));
            }
        }
        
        // Extract range key filter if this is a range schema with a filter
        let range_key_filter = if let (Some(range_key), Some(filter)) = (schema.range_key(), &query.filter) {
            if let Some(range_filter_obj) = filter.get("range_filter") {
                if let Some(range_filter_map) = range_filter_obj.as_object() {
                    if let Some(range_key_value) = range_filter_map.get(range_key) {
                        // Extract the actual filter value - handle different filter types
                        let extracted_value = if let Some(obj) = range_key_value.as_object() {
                            // Handle complex filters like {"Key": "1"}, {"KeyPrefix": "abc"}, etc.
                            if let Some(key_value) = obj.get("Key") {
                                Some(key_value.as_str().unwrap_or("").to_string())
                            } else if let Some(prefix_value) = obj.get("KeyPrefix") {
                                Some(prefix_value.as_str().unwrap_or("").to_string())
                            } else if let Some(pattern_value) = obj.get("KeyPattern") {
                                Some(pattern_value.as_str().unwrap_or("").to_string())
                            } else {
                                // For other filter types, try to extract any string value
                                obj.values()
                                    .find_map(|v| v.as_str())
                                    .map(|s| s.to_string())
                            }
                        } else {
                            // Simple string filter like "1"
                            Some(range_key_value.to_string().trim_matches('"').to_string())
                        };
                        
                        info!("🎯 RANGE FILTER EXTRACTED: range_key='{}', filter_value={:?}", range_key, extracted_value);
                        extracted_value
                    } else {
                        None
                    }
                } else {
                    None
                }
            } else {
                None
            }
        } else {
            None
        };

        // Retrieve actual field values by accessing database directly
        let mut field_values = serde_json::Map::new();
        
        for field_name in &query.fields {
            match self.get_field_value_from_db(&schema, field_name, range_key_filter.clone()) {
                Ok(value) => {
                    field_values.insert(field_name.clone(), value);
                }
                Err(e) => {
                    info!("Failed to retrieve field '{}': {}", field_name, e);
                    field_values.insert(field_name.clone(), serde_json::Value::Null);
                }
            }
        }
        
        // Return actual field values
        Ok(serde_json::Value::Object(field_values))
    }

    /// Get field value directly from database using unified resolver
    fn get_field_value_from_db(&self, schema: &Schema, field_name: &str, range_key_filter: Option<String>) -> Result<Value, SchemaError> {
        // Use the unified FieldValueResolver to eliminate duplicate code
        crate::fold_db_core::transform_manager::utils::TransformUtils::resolve_field_value(&self.db_ops, schema, field_name, range_key_filter)
    }

    /// Query a schema (compatibility method)
    pub fn query_schema(&self, query: Query) -> Vec<Result<Value, SchemaError>> {
        // Delegate to the main query method and wrap in Vec
        vec![self.query(query)]
    }

    /// Write schema operation - main orchestration method for mutations
    pub fn write_schema(&mut self, mutation: Mutation) -> Result<String, SchemaError> {
        let start_time = Instant::now();
        let fields_count = mutation.fields_and_values.len();
        let operation_type = format!("{:?}", mutation.mutation_type);
        let schema_name = mutation.schema_name.clone();
        
        // Generate unique mutation ID
        let mutation_id = uuid::Uuid::new_v4().to_string();
        
        log_feature!(LogFeature::Mutation, info,
            "Starting mutation execution for schema: {} with ID: {}",
            mutation.schema_name, mutation_id
        );
        log_feature!(LogFeature::Mutation, info, "Mutation type: {:?}", mutation.mutation_type);
        log_feature!(LogFeature::Mutation, info,
            "Fields to mutate: {:?}",
            mutation.fields_and_values.keys().collect::<Vec<_>>()
        );

        if mutation.fields_and_values.is_empty() {
            return Err(SchemaError::InvalidData("No fields to write".to_string()));
        }

        // Register mutation with completion handler for tracking
        log_feature!(LogFeature::Mutation, info, "Generated mutation ID {} for completion tracking", mutation_id);

        // 1. Prepare mutation and validate schema
        let (schema, processed_mutation, mutation_hash) = self.prepare_mutation_and_schema(mutation)?;

        // 2. Create mutation service and delegate field updates
        let mutation_service = services::mutation::MutationService::new(Arc::clone(&self.message_bus));
        let result = self.process_field_mutations_via_service(&mutation_service, &schema, &processed_mutation, &mutation_hash);
        
        // 3. Publish MutationExecuted event
        let execution_time_ms = start_time.elapsed().as_millis() as u64;
        let mutation_event = MutationExecuted::new(
            operation_type,
            schema_name,
            execution_time_ms,
            fields_count,
        );

        if let Err(e) = self.message_bus.publish(mutation_event) {
            warn!("Failed to publish MutationExecuted event: {}", e);
        }

        // 4. Signal completion for mutation tracking AFTER database operations are complete
        // This ensures wait_for_mutation doesn't return until atoms are persisted
        if processed_mutation.synchronous.unwrap_or(false) {
            // Synchronous mode: signal completion immediately
            log_feature!(LogFeature::Mutation, info, "Synchronous mode: signaling completion immediately for mutation ID {}", mutation_id);
            if let Err(e) = self.completion_handler.signal_completion_sync(&mutation_id) {
                warn!("Failed to signal synchronous completion for mutation {}: {}", mutation_id, e);
            }
        } else {
            // Asynchronous mode: spawn task to signal completion AFTER database persistence
            let completion_handler = Arc::clone(&self.completion_handler);
            let mutation_id_clone = mutation_id.clone();
            let db_ops = Arc::clone(&self.db_ops);

            tokio::spawn(async move {
                // Wait a bit to ensure database operations are fully persisted
                tokio::time::sleep(tokio::time::Duration::from_millis(100)).await;

                // Flush the database to ensure all writes are persisted
                if let Err(e) = db_ops.db().flush() {
                    log_feature!(LogFeature::Mutation, warn, "Failed to flush database before signaling completion: {}", e);
                } else {
                    log_feature!(LogFeature::Mutation, info, "Database flushed before signaling mutation completion");
                }

                // Now signal completion
                completion_handler.signal_completion(&mutation_id_clone).await;
                log_feature!(LogFeature::Mutation, info, "✅ Mutation {} completion signaled after database persistence", mutation_id_clone);
            });
        }

        // Return the mutation ID regardless of result
        match result {
            Ok(()) => Ok(mutation_id),
            Err(e) => Err(e),
        }
    }

    /// Prepare mutation and schema - extract and validate components
    fn prepare_mutation_and_schema(
        &self,
        mutation: Mutation,
    ) -> Result<(Schema, Mutation, String), SchemaError> {
        // Get schema
        let schema = match self.schema_manager.get_schema(&mutation.schema_name)? {
            Some(schema) => schema,
            None => {
                return Err(SchemaError::InvalidData(format!(
                    "Schema '{}' not found",
                    mutation.schema_name
                )));
            }
        };

        // Check field-level permissions for each field in the mutation
        for field_name in mutation.fields_and_values.keys() {
            let permission_result = self.permission_wrapper.check_mutation_field_permission(
                &mutation,
                field_name,
                &self.schema_manager,
            );
            
            if !permission_result.allowed {
                return Err(permission_result.error.unwrap_or_else(|| {
                    SchemaError::InvalidData(format!(
                        "Permission denied for field '{}' in schema '{}' with trust distance {}",
                        field_name, mutation.schema_name, mutation.trust_distance
                    ))
                }));
            }
        }

        // Generate mutation hash for tracking
        let mut hasher = <sha2::Sha256 as Digest>::new();
        hasher.update(format!("{:?}", mutation).as_bytes());
        let mutation_hash = format!("{:x}", hasher.finalize());

        Ok((schema, mutation, mutation_hash))
    }

    /// Process field mutations via service delegation
    fn process_field_mutations_via_service(
        &self,
        mutation_service: &services::mutation::MutationService,
        schema: &Schema,
        mutation: &Mutation,
        mutation_hash: &str,
    ) -> Result<(), SchemaError> {
        // Check if this is a range schema
        if let Some(range_key) = schema.range_key() {
            log_feature!(LogFeature::Mutation, info, "🎯 DEBUG: Processing range schema mutation for schema '{}' with range_key '{}'", schema.name, range_key);
            
            // Extract the range key value from the mutation data
            let range_key_value = mutation.fields_and_values.get(range_key)
                .ok_or_else(|| SchemaError::InvalidData(format!(
                    "Range schema mutation missing range_key field '{}'", range_key
                )))?;
            
            let range_key_str = match range_key_value {
                Value::String(s) => s.clone(),
                _ => range_key_value.to_string().trim_matches('"').to_string(),
            };
            
            log_feature!(LogFeature::Mutation, info, "🎯 DEBUG: Range key value: '{}'", range_key_str);
            
            // Use the specialized range schema mutation method
            return mutation_service.update_range_schema_fields(
                schema,
                &mutation.fields_and_values,
                &range_key_str,
                mutation_hash,
            );
        } else {
            log_feature!(LogFeature::Mutation, info, "🔍 DEBUG: Processing regular schema mutation for schema '{}'", schema.name);
        }

        // For non-range schemas, process fields individually
        for (field_name, field_value) in &mutation.fields_and_values {
            // Get field definition
            let _field = schema.fields.get(field_name).ok_or_else(|| {
                SchemaError::InvalidData(format!("Field '{}' not found in schema", field_name))
            })?;

            // Delegate to mutation service
            mutation_service.update_field_value(schema, field_name.as_str(), field_value, mutation_hash)?;
        }

        Ok(())
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
        log_feature!(LogFeature::Transform, info, "🔄 run_transform called for {} - using direct execution", transform_id);
        
        // Use direct execution through the transform manager
        match TransformRunner::execute_transform_now(&*self.transform_manager, transform_id) {
            Ok(result) => Ok(result),
            Err(e) => Err(SchemaError::InvalidData(e.to_string())),
        }
    }

    /// Process any pending transforms in the queue
    pub fn process_transform_queue(&self) {
        // Transform orchestrator processing is handled automatically by events
        // self.transform_orchestrator.process_pending_transforms();
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
        log_feature!(LogFeature::Mutation, info,
            "Waiting for completion of mutation: {}", mutation_id);
        
        // Use the completion handler to wait for the mutation with default timeout
        match self.completion_handler.wait_for_completion(mutation_id).await {
            Ok(()) => {
                log_feature!(LogFeature::Mutation, info,
                    "Mutation {} completed successfully", mutation_id);
                Ok(())
            }
            Err(mutation_error) => {
                // Convert MutationCompletionError to SchemaError with appropriate error messages
                let schema_error = match mutation_error {
                    MutationCompletionError::Timeout(id, duration) => {
                        log_feature!(LogFeature::Mutation, warn,
                            "Mutation {} timed out after {:?}", id, duration);
                        SchemaError::InvalidData("Mutation failed".to_string())
                    }
                    MutationCompletionError::MutationNotFound(id) => {
                        log_feature!(LogFeature::Mutation, warn,
                            "Mutation {} not found in completion tracking system", id);
                        SchemaError::InvalidData(format!(
                            "Mutation '{}' not found in tracking system. The mutation may have already completed, never been registered, or the ID is invalid.",
                            id
                        ))
                    }
                    MutationCompletionError::SignalFailed(id, reason) => {
                        log_feature!(LogFeature::Mutation, error,
                            "Failed to receive completion signal for mutation {}: {}", id, reason);
                        SchemaError::InvalidData(format!(
                            "Mutation '{}' completion signal failed: {}. This may indicate a system error or that the mutation process was interrupted.",
                            id, reason
                        ))
                    }
                    MutationCompletionError::LockFailed(reason) => {
                        log_feature!(LogFeature::Mutation, error,
                            "Failed to acquire lock for mutation completion tracking: {}", reason);
                        SchemaError::InvalidData(format!(
                            "Failed to access mutation tracking system: {}. This may indicate high system load or a concurrency issue.",
                            reason
                        ))
                    }
                };
                
                Err(schema_error)
            }
        }
    }
}
