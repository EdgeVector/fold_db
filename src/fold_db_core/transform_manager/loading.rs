use super::manager::TransformManager;
use crate::schema::types::SchemaError;

use log::info;
use std::collections::HashSet;

impl TransformManager {

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
