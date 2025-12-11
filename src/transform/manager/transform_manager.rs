use std::collections::{BTreeMap, HashMap, HashSet};
use std::sync::{Arc, RwLock};

use crate::fold_db_core::infrastructure::message_bus::MessageBus;
use crate::schema::types::{SchemaError, Transform};

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
/// This separation provides clean responsibilities:
/// - TransformOrchestrator: Orchestration and event handling
/// - TransformManager: Registration, execution, mapping, and result storage
pub struct TransformManager {
    pub db_ops: Arc<crate::db_operations::DbOperations>,
    pub(super) registered_transforms: RwLock<HashMap<String, Transform>>,
    pub(super) schema_field_to_transforms: RwLock<BTreeMap<String, HashSet<String>>>,
    pub(super) message_bus: Arc<MessageBus>,
}

impl TransformManager {
    /// Helper to run async code from sync context, handling both cases where we're
    /// already in a runtime (use block_in_place) or not (create new runtime)
    fn run_async<F, T>(future: F) -> Result<T, SchemaError>
    where
        F: std::future::Future<Output = Result<T, SchemaError>>,
    {
        match tokio::runtime::Handle::try_current() {
            Ok(_handle) => {
                // We're already in a runtime, use block_in_place to avoid nested runtime error
                tokio::task::block_in_place(|| {
                    tokio::runtime::Handle::current().block_on(future)
                })
            }
            Err(_) => {
                // No runtime, create one
                tokio::runtime::Runtime::new()
                    .map_err(|e| SchemaError::InvalidData(format!("Failed to create runtime: {}", e)))?
                    .block_on(future)
            }
        }
    }

    /// Creates a new TransformManager instance with unified database operations
    pub async fn new(
        db_ops: std::sync::Arc<crate::db_operations::DbOperations>,
        message_bus: Arc<MessageBus>,
    ) -> Result<Self, SchemaError> {
        // Load persisted state from storage
        let (registered_transforms, schema_field_to_transforms) = 
            db_ops.load_transform_state().await?;

        // Create the TransformManager instance
        let manager = Self {
            db_ops: Arc::clone(&db_ops),
            registered_transforms: RwLock::new(registered_transforms),
            schema_field_to_transforms: RwLock::new(schema_field_to_transforms),
            message_bus: Arc::clone(&message_bus),
        };

        Ok(manager)
    }


    /// List all registered transforms.
    pub fn list_transforms(&self) -> Result<HashMap<String, Transform>, SchemaError> {
        let transforms = self.registered_transforms.read()
            .map_err(|e| SchemaError::InvalidData(format!("Failed to acquire read lock: {}", e)))?;
        Ok(transforms.clone())
    }

    /// Check if a transform is already registered.
    pub fn transform_exists(&self, transform_id: &str) -> Result<bool, SchemaError> {
        let transforms = self.registered_transforms.read()
            .map_err(|e| SchemaError::InvalidData(format!("Failed to acquire read lock: {}", e)))?;
        Ok(transforms.contains_key(transform_id))
    }

    /// Get the schema state for a given schema/transform
    pub fn get_schema_state(&self, schema_name: &str) -> Result<Option<crate::schema::SchemaState>, SchemaError> {
        Self::run_async(self.db_ops.get_schema_state(schema_name))
    }

    /// Gets all transforms that should run when the specified field is updated.
    pub fn get_transforms_for_field(
        &self,
        schema_name: &str,
        field_name: &str,
    ) -> Result<HashSet<String>, SchemaError> {
        let key = format!("{}.{}", schema_name, field_name);
        let mappings = self.schema_field_to_transforms.read()
            .map_err(|e| SchemaError::InvalidData(format!("Failed to acquire read lock: {}", e)))?;
        let field_to_transforms = mappings
            .get(&key)
            .cloned()
            .unwrap_or_default();

        Ok(field_to_transforms)
    }

    pub fn handle_transform_registration(
        &self,
        registration: &crate::schema::types::TransformRegistration,
    ) -> Result<(), SchemaError> {
        let transform_id = &registration.transform_id;
        let transform = &registration.transform;
        let trigger_fields = &registration.trigger_fields;

        // Update in-memory state
        self.update_in_memory_registration(transform_id, transform, trigger_fields)?;

        // Sync to storage
        let transforms = self.registered_transforms.read()
            .map_err(|e| SchemaError::InvalidData(format!("Failed to acquire read lock: {}", e)))?;
        let mappings = self.schema_field_to_transforms.read()
            .map_err(|e| SchemaError::InvalidData(format!("Failed to acquire read lock: {}", e)))?;
        
        Self::run_async(self.db_ops.sync_transform_state(&transforms, &mappings))?;

        Ok(())
    }


    /// Update in-memory state with new transform registration
    fn update_in_memory_registration(
        &self,
        transform_id: &str,
        transform: &Transform,
        trigger_fields: &[String],
    ) -> Result<(), SchemaError> {
        // Update registered transforms
        {
            let mut registered_transforms = self.registered_transforms.write()
                .map_err(|e| SchemaError::InvalidData(format!("Failed to acquire write lock: {}", e)))?;
            registered_transforms.insert(transform_id.to_string(), transform.clone());
        }

        // Update field-to-transform mappings
        {
            let mut field_to_transforms = self.schema_field_to_transforms.write()
                .map_err(|e| SchemaError::InvalidData(format!("Failed to acquire write lock: {}", e)))?;

            for field in trigger_fields {
                field_to_transforms
                    .entry(field.clone())
                    .or_insert_with(HashSet::new)
                    .insert(transform_id.to_string());
            }
        }

        Ok(())
    }
}

