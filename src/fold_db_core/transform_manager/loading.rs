use super::manager::TransformManager;
use crate::schema::types::{SchemaError, Transform, TransformRegistration};
use crate::transform::TransformExecutor;
use crate::fold_db_core::transform_manager::utils::*;
use log::info;
use crate::logging::features::{log_feature, LogFeature};
use std::collections::{HashMap, HashSet};

impl TransformManager {
    /// Reload transforms from database - called when new transforms are registered
    pub fn reload_transforms(&self) -> Result<(), SchemaError> {
        info!("🔄 TransformManager: Reloading transforms from database");

        // Get fresh list of transform IDs
        let transform_ids = self.db_ops.list_transforms()?;
        
        // Load transforms into memory
        let mut registered_transforms = self.registered_transforms.write()
            .map_err(|_| SchemaError::InvalidData("Failed to acquire write lock".to_string()))?;

        let mut field_to_transforms = self.field_to_transforms.write()
            .map_err(|_| SchemaError::InvalidData("Failed to acquire write lock".to_string()))?;

        let mut transform_to_fields = self.transform_to_fields.write()
            .map_err(|_| SchemaError::InvalidData("Failed to acquire write lock".to_string()))?;

        for transform_id in transform_ids {
            // Skip if transform is already loaded
            if registered_transforms.contains_key(&transform_id) {
                continue;
            }
            
            match self.db_ops.get_transform(&transform_id) {
                Ok(Some(transform)) => {
                    info!(
                        "📋 Loaded new transform '{}' with inputs: {:?}, output: {}",
                        transform_id, transform.get_inputs(), transform.get_output()
                    );
                    registered_transforms.insert(transform_id.clone(), transform.clone());
                    
                    // Register field mappings for the new transform
                    LoggingHelper::log_transform_registration(&transform_id, transform.get_inputs(), transform.get_output());
                    for input in transform.get_inputs() {
                        field_to_transforms.entry(input.clone()).or_insert_with(HashSet::new).insert(transform_id.clone());
                        transform_to_fields.entry(transform_id.clone()).or_insert_with(HashSet::new).insert(input.to_string());
                        LoggingHelper::log_field_mapping_creation(input, &transform_id);
                    }
                }
                Ok(None) => {
                    log_feature!(LogFeature::Transform, warn,
                        "Transform '{}' not found in storage during reload",
                        transform_id
                    );
                }
                Err(e) => {
                    log_feature!(LogFeature::Transform, error,
                        "Failed to load transform '{}' during reload: {}",
                        transform_id,
                        e
                    );
                }
            }
        }

        info!("✅ TransformManager: Transform reload completed");
        Ok(())
    }

    /// Register transform using event-driven database operations only
    pub fn register_transform_event_driven(
        &self,
        registration: TransformRegistration,
    ) -> Result<(), SchemaError> {
        let TransformRegistration {
            transform_id,
            mut transform,
            input_arefs,
            input_names,
            trigger_fields,
            output_aref,
            schema_name,
            field_name,
        } = registration;

        // Validate and prepare transform
        TransformExecutor::validate_transform(&transform)?;
        let output_field = format!("{}.{}", schema_name, field_name);
        let inputs_len = input_arefs.len();
        transform.set_output(output_field.clone());

        // Store transform using direct database operations
        self.db_ops.store_transform(&transform_id, &transform)?;

        // Update in-memory state
        self.update_in_memory_mappings(
            &transform_id,
            transform,
            &input_arefs,
            &input_names,
            &trigger_fields,
            &output_aref,
        )?;

        info!(
            "Registered transform {} output {} with {} input references using unified operations",
            transform_id, output_field, inputs_len
        );

        // Persist mappings using event-driven operations only
        self.persist_mappings_direct()?;
        Ok(())
    }

    /// Helper method to update in-memory mappings for transform registration
    pub(super) fn update_in_memory_mappings(
        &self,
        transform_id: &str,
        transform: Transform,
        input_arefs: &[String],
        input_names: &[String],
        trigger_fields: &[String],
        output_aref: &str,
    ) -> Result<(), SchemaError> {
        // Update registered transforms
        {
            let mut registered_transforms = self.registered_transforms.write().map_err(|_| {
                SchemaError::InvalidData("Failed to acquire registered_transforms lock".to_string())
            })?;
            registered_transforms.insert(transform_id.to_string(), transform);
        }

        // Update output mapping
        {
            let mut transform_outputs = self.transform_outputs.write().map_err(|_| {
                SchemaError::InvalidData("Failed to acquire transform_outputs lock".to_string())
            })?;
            transform_outputs.insert(transform_id.to_string(), output_aref.to_string());
        }

        // Update input atom references
        {
            let mut transform_to_arefs = self.transform_to_arefs.write().map_err(|_| {
                SchemaError::InvalidData("Failed to acquire transform_to_arefs lock".to_string())
            })?;
            let aref_set: HashSet<String> = input_arefs.iter().cloned().collect();
            transform_to_arefs.insert(transform_id.to_string(), aref_set);
        }

        // Update input names mapping
        {
            let mut transform_input_names = self.transform_input_names.write().map_err(|_| {
                SchemaError::InvalidData("Failed to acquire transform_input_names lock".to_string())
            })?;
            let name_map: HashMap<String, String> = input_arefs.iter()
                .zip(input_names.iter())
                .map(|(aref, name)| (aref.clone(), name.clone()))
                .collect();
            transform_input_names.insert(transform_id.to_string(), name_map);
        }

        // Update field trigger mappings
        self.update_field_trigger_mappings(transform_id, trigger_fields)?;

        // Update reverse mapping (aref -> transforms)
        self.update_aref_to_transforms_mapping(transform_id, input_arefs)?;

        Ok(())
    }

    /// Helper method to update field trigger mappings
    pub(super) fn update_field_trigger_mappings(
        &self,
        transform_id: &str,
        trigger_fields: &[String],
    ) -> Result<(), SchemaError> {
        let mut transform_to_fields = self.transform_to_fields.write().map_err(|_| {
            SchemaError::InvalidData("Failed to acquire transform_to_fields lock".to_string())
        })?;
        let mut field_to_transforms = self.field_to_transforms.write().map_err(|_| {
            SchemaError::InvalidData("Failed to acquire field_to_transforms lock".to_string())
        })?;

        let field_set: HashSet<String> = trigger_fields.iter().cloned().collect();
        info!("🔍 DEBUG: Registering field mappings for transform '{}' with trigger_fields: {:?}", transform_id, trigger_fields);
        for field_key in trigger_fields {
            let set = field_to_transforms.entry(field_key.clone()).or_default();
            set.insert(transform_id.to_string());
            info!("🔗 DEBUG: Registered field mapping '{}' -> transform '{}'", field_key, transform_id);
        }
        transform_to_fields.insert(transform_id.to_string(), field_set);
        
        // DEBUG: Log current field mappings state
        info!("🔍 DEBUG: Current field_to_transforms state after registration:");
        for (field_key, transforms) in field_to_transforms.iter() {
            info!("  📋 '{}' -> {:?}", field_key, transforms);
        }

        Ok(())
    }

    /// Helper method to update aref to transforms mapping
    pub(super) fn update_aref_to_transforms_mapping(
        &self,
        transform_id: &str,
        input_arefs: &[String],
    ) -> Result<(), SchemaError> {
        let mut aref_to_transforms = self.aref_to_transforms.write().map_err(|_| {
            SchemaError::InvalidData("Failed to acquire aref_to_transforms lock".to_string())
        })?;

        for aref_uuid in input_arefs {
            let transform_set = aref_to_transforms.entry(aref_uuid.clone()).or_default();
            transform_set.insert(transform_id.to_string());
        }

        Ok(())
    }
}