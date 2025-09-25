use crate::db_operations::DbOperations;
use crate::fold_db_core::infrastructure::message_bus::MessageBus;
use crate::schema::types::{SchemaError, Transform};
use log::info;
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

        // Create the TransformManager instance
        let manager = Self {
            db_ops: Arc::clone(&db_ops),
            registered_transforms: RwLock::new(registered_transforms),
            schema_field_to_transforms: RwLock::new(schema_field_to_transforms),
            message_bus: Arc::clone(&message_bus),
        };

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

        Ok(result)
    }

    pub fn handle_transform_registration(
        &self,
        registration: &crate::schema::types::TransformRegistration,
    ) -> Result<(), SchemaError> {
        let transform_id = &registration.transform_id;
        let transform = &registration.transform;
        let trigger_fields = &registration.trigger_fields;

        // 1. Store the transform in the database
        self.db_ops.store_transform(transform_id, transform)?;

        // 2. Update field-to-transform mappings
        self.update_field_trigger_mappings(transform_id, trigger_fields)?;

        // 3. Update in-memory state
        self.update_in_memory_registration(transform_id, transform, trigger_fields)?;

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
            let mut registered_transforms = self
                .registered_transforms
                .write()
                .map_err(|_| SchemaError::InvalidData("Failed to acquire write lock".to_string()))?;
            registered_transforms.insert(transform_id.to_string(), transform.clone());
        }

        // Update field-to-transform mappings
        {
            let mut field_to_transforms = self
                .schema_field_to_transforms
                .write()
                .map_err(|_| SchemaError::InvalidData("Failed to acquire write lock".to_string()))?;

            for field in trigger_fields {
                field_to_transforms
                    .entry(field.clone())
                    .or_insert_with(HashSet::new)
                    .insert(transform_id.to_string());
            }
        }

        Ok(())
    }

    /// Helper method to update field trigger mappings
    pub(super) fn update_field_trigger_mappings(
        &self,
        transform_id: &str,
        trigger_fields: &[String],
    ) -> Result<(), SchemaError> {
        let mut field_to_transforms = self.schema_field_to_transforms.write().map_err(|_| {
            SchemaError::InvalidData("Failed to acquire field_to_transforms lock".to_string())
        })?;

        let field_set: HashSet<String> = trigger_fields.iter().cloned().collect();
        info!(
            "🔍 DEBUG: Registering field mappings for transform '{}' with trigger_fields: {:?}",
            transform_id, trigger_fields
        );
        for field_key in trigger_fields {
            let set = field_to_transforms.entry(field_key.clone()).or_default();
            set.insert(transform_id.to_string());
            info!(
                "🔗 DEBUG: Registered field mapping '{}' -> transform '{}'",
                field_key, transform_id
            );
        }
        self.schema_field_to_transforms.write().map_err(|_| {
            SchemaError::InvalidData("Failed to acquire schema_field_to_transforms lock".to_string())
        })?.insert(transform_id.to_string(), field_set);

        // DEBUG: Log current field mappings state
        info!("🔍 DEBUG: Current field_to_transforms state after registration:");
        for (field_key, transforms) in field_to_transforms.iter() {
            info!("  📋 '{}' -> {:?}", field_key, transforms);
        }

        Ok(())
    }
}

