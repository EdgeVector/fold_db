use super::types::TransformRunner;
use crate::db_operations::DbOperations;
use crate::fold_db_core::infrastructure::message_bus::MessageBus;
use crate::schema::types::{SchemaError, Transform};
use log::{error, info};
use serde_json::Value as JsonValue;
use std::collections::{HashMap, HashSet, BTreeMap};
use std::sync::{Arc, RwLock};

pub const SCHEMA_FIELD_TO_TRANSFORMS_KEY: &str = "map_schema_field_to_transforms";

/// TransformManager: Handles transform execution and registration
///
/// CURRENT ARCHITECTURE RESPONSIBILITIES:
/// - Transform Registration: Manages loading and storing of transforms
/// - Transform Execution: Executes individual transforms when requested
/// - Dependency Tracking: Maintains mappings between fields and transforms
/// - Schema Monitoring: Reloads transforms when schemas change
///
/// ORCHESTRATION IS HANDLED BY TransformOrchestrator:
/// - TransformOrchestrator listens for FieldValueSet events directly
/// - TransformOrchestrator determines which transforms to execute
/// - TransformOrchestrator calls TransformManager for actual execution
///
/// This separation provides clean responsibilities:
/// - TransformOrchestrator: Orchestration and event handling
/// - TransformManager: Execution and registration
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
            db_ops,
            registered_transforms: RwLock::new(registered_transforms),
            schema_field_to_transforms: RwLock::new(schema_field_to_transforms),
            message_bus,
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

        // Create a simple transform runner wrapper for the manager
        #[allow(dead_code)]
        struct SimpleTransformRunner {
            db_ops: Arc<crate::db_operations::DbOperations>,
            message_bus: Arc<MessageBus>,
        }

        impl crate::fold_db_core::transform_manager::types::TransformRunner for SimpleTransformRunner {
            fn execute_transform_with_context(
                &self,
                transform_id: &str,
                mutation_context: &Option<
                    crate::fold_db_core::infrastructure::message_bus::atom_events::MutationContext,
                >,
            ) -> Result<serde_json::Value, SchemaError> {
                // Load and execute the transform with context
                // TODO: update the transform executor to use the hash_to_code mappings.
                if let Ok(Some(transform)) = self.db_ops.get_transform(transform_id) {
                    let result = TransformManager::execute_single_transform_with_context(
                        transform_id,
                        &transform,
                        &self.db_ops,
                        mutation_context,
                        None // FoldDB not available in this context - will use fallback
                    )?;

                    // Store the result
                    ResultStorage::store_transform_result_generic(
                        &transform,
                        &result.clone(),
                        Some(&self.message_bus)
                    )?;

                    Ok(result)
                } else {
                    Err(SchemaError::InvalidData(format!(
                        "Transform '{}' not found",
                        transform_id
                    )))
                }
            }

            fn transform_exists(&self, transform_id: &str) -> Result<bool, SchemaError> {
                let exists = self.db_ops.get_transform(transform_id)?.is_some();
                info!(
                    "🔍 DIAGNOSTIC: SimpleTransformRunner.transform_exists('{}') = {}",
                    transform_id, exists
                );
                Ok(exists)
            }

            fn get_transforms_for_field(
                &self,
                schema_name: &str,
                field_name: &str,
            ) -> Result<std::collections::HashSet<String>, SchemaError> {
                // Load field-to-transforms mapping from database
                let field_key = format!("{}.{}", schema_name, field_name);

                match self.db_ops.get_transform_mapping(SCHEMA_FIELD_TO_TRANSFORMS_KEY) {
                    Ok(Some(mapping_bytes)) => {
                        if let Ok(field_to_transforms) = serde_json::from_slice::<
                            std::collections::HashMap<String, std::collections::HashSet<String>>,
                        >(&mapping_bytes)
                        {
                            Ok(field_to_transforms
                                .get(&field_key)
                                .cloned()
                                .unwrap_or_default())
                        } else {
                            info!("⚠️ Failed to deserialize field_to_transforms mapping, returning empty set");
                            Ok(std::collections::HashSet::new())
                        }
                    }
                    Ok(None) => {
                        info!("ℹ️ No field_to_transforms mapping found in database");
                        Ok(std::collections::HashSet::new())
                    }
                    Err(e) => {
                        error!("❌ Failed to load field_to_transforms mapping: {}", e);
                        Err(SchemaError::InvalidData(format!(
                            "Failed to load field mapping: {}",
                            e
                        )))
                    }
                }
            }

            fn get_transforms_for_schema(&self, schema_name: &str) -> Result<std::collections::HashSet<String>, SchemaError> {
                // Load field-to-transforms mapping from database
                match self.db_ops.get_transform_mapping(SCHEMA_FIELD_TO_TRANSFORMS_KEY) {
                    Ok(Some(mapping_bytes)) => {
                        if let Ok(field_to_transforms) = serde_json::from_slice::<
                            std::collections::HashMap<String, std::collections::HashSet<String>>,
                        >(&mapping_bytes)
                        {
                            // Find all transforms that depend on any field of this schema
                            let mut result = std::collections::HashSet::new();
                            for (field_key, transform_ids) in field_to_transforms.iter() {
                                if field_key.starts_with(&format!("{}.", schema_name)) {
                                    result.extend(transform_ids.iter().cloned());
                                }
                            }
                            Ok(result)
                        } else {
                            info!("⚠️ Failed to deserialize field_to_transforms mapping, returning empty set");
                            Ok(std::collections::HashSet::new())
                        }
                    }
                    Ok(None) => {
                        info!("ℹ️ No field_to_transforms mapping found in database");
                        Ok(std::collections::HashSet::new())
                    }
                    Err(e) => {
                        error!("❌ Failed to load field_to_transforms mapping: {}", e);
                        Err(SchemaError::InvalidData(format!(
                            "Failed to load field mapping: {}",
                            e
                        )))
                    }
                }
            }
        }

        // Note: EventMonitor is now created by TransformOrchestrator with proper manager access
        // This ensures EventMonitor uses in-memory field mappings from TransformManager
        info!("✅ Orchestration system initialization completed (EventMonitor will be created by TransformOrchestrator)");

        Ok(())
    }
}

impl TransformRunner for TransformManager {
    fn execute_transform_with_context(
        &self,
        transform_id: &str,
        mutation_context: &Option<
            crate::fold_db_core::infrastructure::message_bus::atom_events::MutationContext,
        >,
    ) -> Result<JsonValue, SchemaError> {
        info!(
            "🚀 DIAGNOSTIC: TransformManager executing transform with context: {}",
            transform_id
        );

        // Load the transform from the database
        let transform = match self.db_ops.get_transform(transform_id) {
            Ok(Some(transform)) => {
                transform
            }
            Ok(None) => {
                error!(
                    "❌ DIAGNOSTIC: Transform '{}' not found in database",
                    transform_id
                );
                return Err(SchemaError::InvalidData(format!(
                    "Transform '{}' not found",
                    transform_id
                )));
            }
            Err(e) => {
                error!(
                    "❌ DIAGNOSTIC: Failed to load transform '{}': {}",
                    transform_id, e
                );
                return Err(SchemaError::InvalidData(format!(
                    "Failed to load transform: {}",
                    e
                )));
            }
        };

        // Log mutation context if available
        if let Some(ref context) = mutation_context {
            info!("🎯 DIAGNOSTIC: Transform execution with mutation context - range_key: {:?}, hash_key: {:?}, incremental: {}", 
                  context.range_key, context.hash_key, context.incremental);
        }

        // Execute the transform using the execution module with mutation context
        println!(
            "🔧 About to call execute_single_transform with context for transform: {}",
            transform_id
        );
        let result = TransformManager::execute_single_transform_with_context(
            transform_id,
            &transform,
            &self.db_ops,
            mutation_context,
            None, // FoldDB not available in this context - will use fallback
        )?;
        println!(
            "🔧 execute_single_transform with context completed with result: {}",
            result
        );

        info!(
            "✅ DIAGNOSTIC: Transform '{}' executed successfully with context, result: {}",
            transform_id, result
        );

        // Store the result using message bus
        match crate::fold_db_core::transform_manager::result_storage::ResultStorage::store_transform_result_generic(
            &self.db_ops,
            &transform,
            &result,
            Some(&self.message_bus)
        ) {
            Ok(_) => {
            }
            Err(e) => {
                return Err(e);
            }
        }

        info!(
            "✅ Transform '{}' executed successfully with context: {}",
            transform_id, result
        );
        Ok(result)
    }

    fn transform_exists(&self, transform_id: &str) -> Result<bool, SchemaError> {
        let registered_transforms = self.registered_transforms.read().map_err(|_| {
            SchemaError::InvalidData("Failed to acquire registered_transforms lock".to_string())
        })?;
        let in_memory_exists = registered_transforms.contains_key(transform_id);

        // Cross-check with database
        let db_exists = self.db_ops.get_transform(transform_id)?.is_some();

        info!(
            "🔍 DIAGNOSTIC: TransformManager.transform_exists('{}') - in_memory: {}, database: {}",
            transform_id, in_memory_exists, db_exists
        );

        if in_memory_exists != db_exists {
            error!(
                "🚨 INCONSISTENCY DETECTED: Transform '{}' - in_memory: {}, database: {}",
                transform_id, in_memory_exists, db_exists
            );
        }

        Ok(in_memory_exists)
    }

    fn get_transforms_for_field(
        &self,
        schema_name: &str,
        field_name: &str,
    ) -> Result<HashSet<String>, SchemaError> {
        let key = format!("{}.{}", schema_name, field_name);
        let field_to_transforms = self.schema_field_to_transforms.read().map_err(|_| {
            SchemaError::InvalidData("Failed to acquire field_to_transforms lock".to_string())
        })?;
        Ok(field_to_transforms.get(&key).cloned().unwrap_or_default())
    }

    fn get_transforms_for_schema(&self, schema_name: &str) -> Result<HashSet<String>, SchemaError> {
        // Delegate to the public method implementation
        self.get_transforms_for_schema(schema_name)
    }
}
