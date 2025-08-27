use super::SchemaCore;
use crate::schema::types::{field::common::Field, Schema, SchemaError};
use crate::fold_db_core::transform_manager::utils::TransformUtils;
use log::info;
use crate::logging::features::{log_feature, LogFeature};

impl SchemaCore {
    pub(crate) fn fix_transform_outputs(&self, schema: &mut Schema) {
        for (field_name, field) in schema.fields.iter_mut() {
            if let Some(transform) = field.transform() {
                let out_schema = transform.get_output();
                if out_schema.starts_with("test.") {
                    let mut new_transform = (*transform).clone();
                    new_transform.set_output(format!("{}.{}", schema.name, field_name));
                    field.set_transform(new_transform);
                }
            }
        }
    }

    /// Auto-register field transforms with TransformManager during schema loading
    pub(crate) fn register_schema_transforms(&self, schema: &Schema) -> Result<(), SchemaError> {
        info!("🔧 DEBUG: Auto-registering transforms for schema: {}", schema.name);
        info!("🔍 DEBUG: Schema has {} fields to check for transforms", schema.fields.len());

        for (field_name, field) in &schema.fields {
            info!("🔍 DEBUG: Checking field '{}.{}' for transforms", schema.name, field_name);
            if let Some(transform) = field.transform() {
                info!(
                    "📋 Found transform on field {}.{}: inputs={:?}, logic={}, output={}",
                    schema.name,
                    field_name,
                    transform.get_inputs(),
                    transform.get_procedural_logic().unwrap_or("[declarative]"),
                    transform.get_output()
                );

                let transform_id = format!("{}.{}", schema.name, field_name);

                // CRITICAL FIX: Check target schema state before registering transform
                let output_parts: Vec<&str> = transform.get_output().split('.').collect();
                if output_parts.len() == 2 {
                    let target_schema_name = output_parts[0];
                    
                    // Check if target schema exists and is approved
                    match self.db_ops.get_schema_state(target_schema_name) {
                        Ok(Some(crate::schema::SchemaState::Approved | crate::schema::SchemaState::Blocked)) => {
                            info!("✅ Target schema '{}' is approved, registering transform '{}'", target_schema_name, transform_id);
                        }
                        Ok(Some(state)) => {
                            info!("⏸️ Skipping transform '{}' - target schema '{}' state is {:?} (not approved)", transform_id, target_schema_name, state);
                            continue;
                        }
                        Ok(None) => {
                            info!("⏸️ Skipping transform '{}' - target schema '{}' not found", transform_id, target_schema_name);
                            continue;
                        }
                        Err(e) => {
                            log_feature!(LogFeature::Schema, error, "❌ Error checking target schema '{}' state for transform '{}': {}", target_schema_name, transform_id, e);
                            continue;
                        }
                    }
                } else {
                    log_feature!(LogFeature::Schema, error, "❌ Invalid transform output format '{}' for transform '{}' - expected 'Schema.field'", transform.get_output(), transform_id);
                    continue;
                }

                // Store the transform in the database so it can be loaded by TransformManager
                if let Err(e) = self.db_ops.store_transform(&transform_id, transform) {
                    log_feature!(LogFeature::Schema, error, "Failed to store transform {}: {}", transform_id, e);
                    continue;
                }

                info!("✅ Stored transform {} for auto-registration", transform_id);

                // Create field-to-transform mappings for TransformOrchestrator
                for input_field in transform.get_inputs() {
                    info!("🔗 Creating field mapping: '{}' → '{}' transform", input_field, transform_id);

                    // Store field mapping in database for TransformManager to load
                    if let Err(e) = self.store_field_to_transform_mapping(input_field, &transform_id) {
                        log_feature!(LogFeature::Schema, error,
                            "Failed to store field mapping '{}' → '{}': {}",
                            input_field, transform_id, e
                        );
                    } else {
                        info!("✅ Stored field mapping: '{}' → '{}' transform", input_field, transform_id);
                    }
                }
            }
        }

        Ok(())
    }

    /// Store field-to-transform mapping in database for TransformManager to load
    pub(crate) fn store_field_to_transform_mapping(&self, field_key: &str, transform_id: &str) -> Result<(), SchemaError> {
        const FIELD_TO_TRANSFORMS_KEY: &str = "map_field_to_transforms";
        let mut field_mappings: std::collections::HashMap<String, std::collections::HashSet<String>> =
            TransformUtils::read_mapping(&self.db_ops, FIELD_TO_TRANSFORMS_KEY, "field_to_transforms")?;

        TransformUtils::insert_mapping_set(&mut field_mappings, field_key, transform_id);

        let json = serde_json::to_vec(&field_mappings)
            .map_err(|e| TransformUtils::handle_error("Failed to serialize field mappings", e))?;
        self.db_ops.store_transform_mapping(FIELD_TO_TRANSFORMS_KEY, &json)?;

        info!("💾 Updated field mappings in database: {} fields mapped", field_mappings.len());

        Ok(())
    }
}

