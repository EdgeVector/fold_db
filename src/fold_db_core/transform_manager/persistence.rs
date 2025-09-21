use super::manager::{
    TransformManager, AREF_TO_TRANSFORMS_KEY, FIELD_TO_TRANSFORMS_KEY, TRANSFORM_INPUT_NAMES_KEY,
    TRANSFORM_OUTPUTS_KEY, TRANSFORM_TO_AREFS_KEY, TRANSFORM_TO_FIELDS_KEY,
};
use super::utils::*;
use crate::db_operations::DbOperations;
use crate::schema::types::SchemaError;
use log::info;
use std::collections::{HashMap, HashSet};
use std::sync::Arc;

impl TransformManager {
    /// Persist mappings using event-driven operations only
    pub fn persist_mappings_direct(&self) -> Result<(), SchemaError> {
        // Store molecule_to_transforms mapping
        SerializationHelper::store_mapping(
            &self.db_ops,
            &self.molecule_to_transforms,
            AREF_TO_TRANSFORMS_KEY,
            "molecule_to_transforms",
        )?;

        // Store transform_to_molecules mapping
        SerializationHelper::store_mapping(
            &self.db_ops,
            &self.transform_to_molecules,
            TRANSFORM_TO_AREFS_KEY,
            "transform_to_molecules",
        )?;

        // Store transform_input_names mapping
        SerializationHelper::store_mapping(
            &self.db_ops,
            &self.transform_input_names,
            TRANSFORM_INPUT_NAMES_KEY,
            "transform_input_names",
        )?;

        // Store field_to_transforms mapping (with debug logging)
        {
            let field_mappings = self.field_to_transforms.read().map_err(|_| {
                SchemaError::InvalidData("Failed to acquire field_to_transforms lock".to_string())
            })?;
            info!(
                "🔍 DIAGNOSTIC: About to persist field_to_transforms with {} entries:",
                field_mappings.len()
            );
            for (field_key, transforms) in field_mappings.iter() {
                info!("  📋 Persisting '{}' -> {:?}", field_key, transforms);
            }
        }
        SerializationHelper::store_mapping(
            &self.db_ops,
            &self.field_to_transforms,
            FIELD_TO_TRANSFORMS_KEY,
            "field_to_transforms",
        )?;

        // Store transform_to_fields mapping
        SerializationHelper::store_mapping(
            &self.db_ops,
            &self.transform_to_fields,
            TRANSFORM_TO_FIELDS_KEY,
            "transform_to_fields",
        )?;

        // Store transform_outputs mapping
        SerializationHelper::store_mapping(
            &self.db_ops,
            &self.transform_outputs,
            TRANSFORM_OUTPUTS_KEY,
            "transform_outputs",
        )?;

        Ok(())
    }

    /// Load persisted mappings using direct database operations
    #[allow(clippy::type_complexity)]
    pub(super) fn load_persisted_mappings_direct(
        db_ops: &Arc<DbOperations>,
    ) -> Result<
        (
            HashMap<String, HashSet<String>>,
            HashMap<String, HashSet<String>>,
            HashMap<String, HashMap<String, String>>,
            HashMap<String, HashSet<String>>,
            HashMap<String, HashSet<String>>,
            HashMap<String, String>,
        ),
        SchemaError,
    > {
        // Simplified initialization - aggressive cleanup
        let molecule_to_transforms = HashMap::new();
        let transform_to_molecules = HashMap::new();
        let transform_input_names = HashMap::new();

        // Load field_to_transforms with special debug logging
        let field_to_transforms = match db_ops.get_transform_mapping(FIELD_TO_TRANSFORMS_KEY)? {
            Some(data) => {
                let loaded_map: HashMap<String, HashSet<String>> =
                    SerializationHelper::deserialize_mapping(&data, "field_to_transforms")?;
                info!(
                    "🔍 DEBUG: Loaded field_to_transforms mapping from database with {} entries:",
                    loaded_map.len()
                );
                for (field_key, transforms) in &loaded_map {
                    info!("  📋 Loaded '{}' -> {:?}", field_key, transforms);
                }
                loaded_map
            }
            None => HashMap::new(),
        };

        // Simplified initialization - aggressive cleanup
        let transform_to_fields = HashMap::new();
        let transform_outputs = HashMap::new();

        Ok((
            molecule_to_transforms,
            transform_to_molecules,
            transform_input_names,
            field_to_transforms,
            transform_to_fields,
            transform_outputs,
        ))
    }
}
