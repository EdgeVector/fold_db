use super::manager::TransformManager;
use crate::schema::types::{SchemaError, Transform, TransformRegistration};
use log::info;

impl TransformManager {
    // Removed redundant register_transform() wrapper - callers should use register_transform_event_driven() directly
    // This eliminates duplicate registration pathways and simplifies the API

    /// Registers a transform with automatic input dependency detection.
    pub fn register_transform_auto(
        &self,
        transform_id: String,
        transform: Transform,
        output_molecule: String,
        schema_name: String,
        field_name: String,
    ) -> Result<(), SchemaError> {
        let dependencies = transform
            .analyze_dependencies()
            .into_iter()
            .collect::<Vec<String>>();
        let trigger_fields = Vec::new();
        let inputs_len = dependencies.len();
        let output_field = format!("{}.{}", schema_name, field_name);
        let tid = transform_id.clone();
        self.register_transform_event_driven(TransformRegistration {
            transform_id,
            transform,
            input_molecules: dependencies,
            input_names: Vec::new(),
            trigger_fields,
            output_molecule,
            schema_name,
            field_name,
        })?;
        info!(
            "Registered transform {} output {} with {} input references",
            tid, output_field, inputs_len
        );
        Ok(())
    }

    /// Unregisters a transform using direct database operations.
    pub fn unregister_transform(&self, transform_id: &str) -> Result<bool, SchemaError> {
        // Use direct database operations for consistency with other components
        let _existed = self.db_ops.delete_transform(transform_id)?;

        // Remove from in-memory cache
        let found = {
            let mut registered_transforms = self.registered_transforms.write().map_err(|_| {
                SchemaError::InvalidData("Failed to acquire registered_transforms lock".to_string())
            })?;
            registered_transforms.remove(transform_id).is_some()
        };

        if found {
            // Remove from transform outputs
            {
                let mut transform_outputs = self.transform_outputs.write().map_err(|_| {
                    SchemaError::InvalidData("Failed to acquire transform_outputs lock".to_string())
                })?;
                transform_outputs.remove(transform_id);
            }

            // Remove field mappings
            {
                let mut transform_to_fields = self.transform_to_fields.write().map_err(|_| {
                    SchemaError::InvalidData(
                        "Failed to acquire transform_to_fields lock".to_string(),
                    )
                })?;
                let mut field_to_transforms = self.field_to_transforms.write().map_err(|_| {
                    SchemaError::InvalidData(
                        "Failed to acquire field_to_transforms lock".to_string(),
                    )
                })?;

                if let Some(fields) = transform_to_fields.remove(transform_id) {
                    for field in fields {
                        if let Some(set) = field_to_transforms.get_mut(&field) {
                            set.remove(transform_id);
                            if set.is_empty() {
                                field_to_transforms.remove(&field);
                            }
                        }
                    }
                }
            }

            // Get the input molecules for this transform
            let input_molecules = {
                let mut transform_to_molecules =
                    self.transform_to_molecules.write().map_err(|_| {
                        SchemaError::InvalidData(
                            "Failed to acquire transform_to_molecules lock".to_string(),
                        )
                    })?;
                transform_to_molecules
                    .remove(transform_id)
                    .unwrap_or_default()
            };

            // Remove input name mapping
            {
                let mut transform_input_names =
                    self.transform_input_names.write().map_err(|_| {
                        SchemaError::InvalidData(
                            "Failed to acquire transform_input_names lock".to_string(),
                        )
                    })?;
                transform_input_names.remove(transform_id);
            }

            // Update the reverse mapping (molecule -> transforms)
            {
                let mut molecule_to_transforms =
                    self.molecule_to_transforms.write().map_err(|_| {
                        SchemaError::InvalidData(
                            "Failed to acquire molecule_to_transforms lock".to_string(),
                        )
                    })?;

                for molecule_uuid in input_molecules {
                    if let Some(transform_set) = molecule_to_transforms.get_mut(&molecule_uuid) {
                        transform_set.remove(transform_id);

                        // Remove the entry if the set is empty
                        if transform_set.is_empty() {
                            molecule_to_transforms.remove(&molecule_uuid);
                        }
                    }
                }
            }
        }

        if found {
            self.persist_mappings_direct()?;
        }

        Ok(found)
    }
}
