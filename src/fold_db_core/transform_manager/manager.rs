use crate::db_operations::DbOperations;
use crate::fold_db_core::infrastructure::message_bus::MessageBus;
use crate::schema::types::{SchemaError, Transform};
use log::{error, info};
use std::collections::{HashMap, HashSet, BTreeMap};
use std::sync::{Arc, RwLock};

pub const SCHEMA_FIELD_TO_TRANSFORMS_KEY: &str = "map_schema_field_to_transforms";

/// TransformManager: Handles transform registration, execution, and field-to-transform mapping
///
/// CURRENT ARCHITECTURE RESPONSIBILITIES:
/// - Transform Registration: Manages loading, storing, and event-driven registration of transforms
/// - Transform Execution: Executes individual transforms with mutation context for incremental processing
/// - Field-to-Transform Mapping: Maintains and persists mappings between schema fields and their dependent transforms
/// - Event-Driven Registration: Listens for TransformRegistrationRequest events and handles registration asynchronously
/// - Result Storage: Stores transform execution results via message bus mutations
/// - Transform Lookup: Provides methods to query transforms by field or schema name
///
/// MODULAR EXECUTION ARCHITECTURE:
/// - InputFetcher: Handles fetching input data with mutation context for incremental processing
/// - TransformRunner: Executes transforms using TransformExecutor with proper context handling
/// - ResultStorage: Stores results as mutations through the message bus system
/// - TransformUtils: Provides shared utilities for field value resolution and default handling
///
/// ORCHESTRATION IS HANDLED BY TransformOrchestrator:
/// - TransformOrchestrator listens for FieldValueSet events directly TODO: change listening even to MutationCompleted
/// - TransformOrchestrator determines which transforms to execute
/// - TransformOrchestrator calls TransformManager for actual execution
///
/// This separation provides clean responsibilities:
/// - TransformOrchestrator: Orchestration and event handling
/// - TransformManager: Registration, execution, mapping, and result storage
pub struct TransformManager {
    pub(super) db_ops: Arc<DbOperations>,
    pub(super) registered_transforms: RwLock<HashMap<String, Transform>>,
    pub(super) schema_field_to_transforms: RwLock<BTreeMap<String, HashSet<String>>>,
    pub(super) message_bus: Arc<MessageBus>,
}

impl TransformManager {
    /// Creates a new TransformManager instance with unified database operations
    pub fn new(
        db_ops: std::sync::Arc<crate::db_operations::DbOperations>,
        message_bus: Arc<MessageBus>,
    ) -> Result<Self, SchemaError> {
        // Load any persisted transforms using direct database operations
        let mut registered_transforms = HashMap::new();

        let transform_ids = db_ops.list_transforms()?;

        for transform_id in transform_ids {
            if let Ok(Some(transform)) = db_ops.get_transform(&transform_id) {
                registered_transforms.insert(transform_id, transform);
            }
        }

        // Load mappings using direct database operations
        let schema_field_to_transforms = db_ops.load_field_to_transforms_mapping(SCHEMA_FIELD_TO_TRANSFORMS_KEY)?;

        // Start the orchestration system to handle TransformTriggered events
        Self::start_orchestration_system(Arc::clone(&db_ops), Arc::clone(&message_bus))?;

        // Create the TransformManager instance first
        let manager = Self {
            db_ops: Arc::clone(&db_ops),
            registered_transforms: RwLock::new(registered_transforms),
            schema_field_to_transforms: RwLock::new(schema_field_to_transforms),
            message_bus: Arc::clone(&message_bus),
        };

        // Start the transform registration listener
        Self::start_transform_registration_listener(Arc::clone(&db_ops), Arc::clone(&message_bus))?;

        Ok(manager)
    }

    /// Returns true if a transform with the given id is registered.
    pub fn transform_exists(&self, transform_id: &str) -> Result<bool, SchemaError> {
        let registered_transforms = self
            .registered_transforms
            .read()
            .map_err(|_| SchemaError::InvalidData("Failed to acquire read lock".to_string()))?;
        Ok(registered_transforms.contains_key(transform_id))
    }

    /// List all registered transforms.
    pub fn list_transforms(&self) -> Result<HashMap<String, Transform>, SchemaError> {
        let registered_transforms = self
            .registered_transforms
            .read()
            .map_err(|_| SchemaError::InvalidData("Failed to acquire read lock".to_string()))?;
        Ok(registered_transforms.clone())
    }

    /// Gets all transforms that should run when the specified field is updated.
    pub fn get_transforms_for_field(
        &self,
        schema_name: &str,
        field_name: &str,
    ) -> Result<HashSet<String>, SchemaError> {
        let key = format!("{}.{}", schema_name, field_name);
        let field_to_transforms = self
            .schema_field_to_transforms
            .read()
            .map_err(|_| SchemaError::InvalidData("Failed to acquire read lock".to_string()))?;

        let result = field_to_transforms.get(&key).cloned().unwrap_or_default();

        // DEBUG: Log field mapping lookup
        info!(
            "🔍 DEBUG TransformManager: Looking up transforms for '{}' - found {} transforms: {:?}",
            key,
            result.len(),
            result
        );

        Ok(result)
    }

    /// Gets all transforms that should run when the specified schema is updated.
    pub fn get_transforms_for_schema(&self, schema_name: &str) -> Result<HashSet<String>, SchemaError> {
        let field_to_transforms = self
            .schema_field_to_transforms
            .read()
            .map_err(|_| SchemaError::InvalidData("Failed to acquire read lock".to_string()))?;

        // Find all transforms that depend on any field of this schema
        let mut result = HashSet::new();
        for (field_key, transform_ids) in field_to_transforms.iter() {
            if field_key.starts_with(&format!("{}.", schema_name)) {
                result.extend(transform_ids.iter().cloned());
            }
        }

        // DEBUG: Log schema mapping lookup
        info!(
            "🔍 DEBUG TransformManager: Looking up transforms for schema '{}' - found {} transforms: {:?}",
            schema_name,
            result.len(),
            result
        );

        Ok(result)
    }

    /// Register a transform using event-driven architecture
    /// 
    /// This method:
    /// 1. Stores the transform in the database
    /// 2. Updates in-memory registered transforms
    /// 3. Creates field-to-transform mappings for trigger detection
    /// 4. Persists the field mappings to the database
    pub fn register_transform_event_driven(
        &self,
        registration: crate::schema::types::TransformRegistration,
    ) -> Result<(), SchemaError> {
        use log::info;
        
        let transform_id = &registration.transform_id;
        let transform = &registration.transform;
        let trigger_fields = &registration.trigger_fields;

        // 1. Store the transform in the database
        self.db_ops.store_transform(transform_id, transform)?;

        // 2. Update in-memory registered transforms
        {
            let mut registered_transforms = self.registered_transforms.write().map_err(|_| {
                SchemaError::InvalidData("Failed to acquire registered_transforms lock".to_string())
            })?;
            registered_transforms.insert(transform_id.clone(), transform.clone());
        }

        // 3. Update field-to-transform mappings
        self.update_field_trigger_mappings(transform_id, trigger_fields)?;

        // 4. Persist field mappings to database
        let field_to_transforms = self.schema_field_to_transforms.read().map_err(|_| {
            SchemaError::InvalidData("Failed to acquire field_to_transforms lock".to_string())
        })?;
        
        let mapping_data = serde_json::to_vec(&*field_to_transforms).map_err(|e| {
            SchemaError::InvalidData(format!("Failed to serialize field mappings: {}", e))
        })?;
        
        self.db_ops.store_transform_mapping(SCHEMA_FIELD_TO_TRANSFORMS_KEY, &mapping_data)?;

        info!(
            "✅ Successfully registered transform '{}' with {} trigger field mappings",
            transform_id,
            trigger_fields.len()
        );

        Ok(())
    }

    /// Start the orchestration system to handle TransformTriggered events
    fn start_orchestration_system(
        _db_ops: Arc<crate::db_operations::DbOperations>,
        _message_bus: Arc<MessageBus>,
    ) -> Result<(), SchemaError> {
        info!("🚀 Starting orchestration system for TransformTriggered event handling");

        // Create a temporary tree for the orchestration system
        let temp_config = sled::Config::new().temporary(true);
        let temp_db = temp_config.open().map_err(|e| {
            SchemaError::InvalidData(format!(
                "Failed to create temporary database for orchestration: {}",
                e
            ))
        })?;
        let _tree = temp_db.open_tree("orchestration").map_err(|e| {
            SchemaError::InvalidData(format!("Failed to create orchestration tree: {}", e))
        })?;


        // Note: EventMonitor is now created by TransformOrchestrator with proper manager access
        // This ensures EventMonitor uses in-memory field mappings from TransformManager
        info!("✅ Orchestration system initialization completed (EventMonitor will be created by TransformOrchestrator)");

        Ok(())
    }

    /// Start a background listener for TransformRegistrationRequest events
    fn start_transform_registration_listener(
        db_ops: Arc<DbOperations>,
        message_bus: Arc<MessageBus>,
    ) -> Result<(), SchemaError> {
        use crate::fold_db_core::infrastructure::message_bus::events::schema_events::{
            TransformRegistrationRequest, TransformRegistrationResponse,
        };
        use std::thread;

        info!("🔧 Starting TransformRegistrationRequest listener");

        // Create a consumer for TransformRegistrationRequest events
        let mut consumer = message_bus.subscribe::<TransformRegistrationRequest>();

        // Start a background thread to handle registration requests
        thread::spawn(move || {
            info!("📡 TransformRegistrationRequest listener thread started");

            loop {
                match consumer.recv() {
                    Ok(request) => {
                        info!(
                            "📨 Received TransformRegistrationRequest for transform '{}'",
                            request.registration.transform_id
                        );

                        // Handle the registration directly without creating a new manager
                        let result = Self::handle_transform_registration(
                            &db_ops,
                            &request.registration,
                        );

                        // Send response back
                        let response = TransformRegistrationResponse {
                            correlation_id: request.correlation_id,
                            success: result.is_ok(),
                            error: result.as_ref().err().map(|e| e.to_string()),
                        };

                        match message_bus.publish(response) {
                            Ok(_) => {
                                if result.is_ok() {
                                    info!(
                                        "✅ Published TransformRegistrationResponse (success) for transform '{}'",
                                        request.registration.transform_id
                                    );
                                } else {
                                    error!(
                                        "❌ Published TransformRegistrationResponse (failure) for transform '{}'",
                                        request.registration.transform_id
                                    );
                                }
                            }
                            Err(e) => {
                                error!(
                                    "❌ Failed to publish TransformRegistrationResponse for transform '{}': {}",
                                    request.registration.transform_id, e
                                );
                            }
                        }
                    }
                    Err(e) => {
                        error!("❌ TransformRegistrationRequest listener error: {}", e);
                        // Continue listening for more events
                    }
                }
            }
        });

        info!("✅ TransformRegistrationRequest listener started successfully");
        Ok(())
    }

    /// Handle transform registration without creating a new manager instance
    fn handle_transform_registration(
        db_ops: &Arc<DbOperations>,
        registration: &crate::schema::types::TransformRegistration,
    ) -> Result<(), SchemaError> {
        use log::info;
        
        let transform_id = &registration.transform_id;
        let transform = &registration.transform;
        let trigger_fields = &registration.trigger_fields;

        info!("🔧 Handling transform registration for '{}'", transform_id);

        // 1. Store the transform in the database
        db_ops.store_transform(transform_id, transform)?;

        // 2. Update field-to-transform mappings
        Self::update_field_trigger_mappings_static(db_ops, transform_id, trigger_fields)?;

        // 3. Persist field mappings to database
        let field_to_transforms = db_ops.load_field_to_transforms_mapping(SCHEMA_FIELD_TO_TRANSFORMS_KEY)?;
        
        let mapping_data = serde_json::to_vec(&field_to_transforms).map_err(|e| {
            SchemaError::InvalidData(format!("Failed to serialize field mappings: {}", e))
        })?;
        
        db_ops.store_transform_mapping(SCHEMA_FIELD_TO_TRANSFORMS_KEY, &mapping_data)?;

        info!(
            "✅ Successfully registered transform '{}' with {} trigger field mappings",
            transform_id,
            trigger_fields.len()
        );

        Ok(())
    }

    /// Static version of update_field_trigger_mappings for use in event handlers
    fn update_field_trigger_mappings_static(
        db_ops: &Arc<DbOperations>,
        transform_id: &str,
        trigger_fields: &[String],
    ) -> Result<(), SchemaError> {
        // Load current mappings
        let mut field_to_transforms = db_ops.load_field_to_transforms_mapping(SCHEMA_FIELD_TO_TRANSFORMS_KEY)?;

        // Add mappings for each trigger field
        for field in trigger_fields {
            field_to_transforms
                .entry(field.clone())
                .or_insert_with(HashSet::new)
                .insert(transform_id.to_string());
        }

        // Store updated mappings
        let mapping_data = serde_json::to_vec(&field_to_transforms).map_err(|e| {
            SchemaError::InvalidData(format!("Failed to serialize field mappings: {}", e))
        })?;
        
        db_ops.store_transform_mapping(SCHEMA_FIELD_TO_TRANSFORMS_KEY, &mapping_data)?;

        Ok(())
    }
}

