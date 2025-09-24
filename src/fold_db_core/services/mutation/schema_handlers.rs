//! Schema-specific mutation operations for Range and HashRange schemas.
//!
//! This module contains handlers for updating entire schemas with multiple fields,
//! including Range and HashRange schema mutations with proper key handling.

use crate::schema::types::{Schema, SchemaError};
use crate::fold_db_core::infrastructure::factory::InfrastructureLogger;
use crate::fold_db_core::services::mutation::{MutationService, NormalizedFieldValueRequest};
use crate::fold_db_core::services::mutation::utilities::summarize_normalized_context;
use serde_json::Value;
use std::collections::HashMap;

impl MutationService {
    /// Update atoms for a HashRange schema mutation using universal key configuration
    ///
    /// This method processes HashRange schema mutations by dynamically determining the hash and range
    /// field names from the schema's universal key configuration, rather than using hardcoded field names.
    /// This allows HashRange schemas to use any field names for their hash and range keys.
    pub fn update_hashrange_schema_fields(
        &self,
        schema: &Schema,
        fields_and_values: &HashMap<String, Value>,
        hash_key_value: &str,
        range_key_value: &str,
        mutation_hash: &str,
    ) -> Result<(), SchemaError> {
        InfrastructureLogger::log_debug_info(
            "MutationService",
            &format!(
                "Processing HashRange schema mutation for hash_key_value: {}, range_key_value: {}",
                hash_key_value, range_key_value
            ),
        );

        // Get the actual hash and range field names from the schema's universal key configuration
        let (hash_field_name, range_field_name) = self.get_hashrange_key_field_names(schema)?;

        InfrastructureLogger::log_debug_info(
            "MutationService",
            &format!(
                "HashRange schema '{}' key fields - hash: '{}', range: '{}'",
                schema.name, hash_field_name, range_field_name
            ),
        );

        let normalized_hash_value = Value::String(hash_key_value.to_string());
        let normalized_range_value = Value::String(range_key_value.to_string());

        // Process each field in the mutation, skipping the key fields
        for (field_name, value) in fields_and_values {
            // Skip the hash and range key fields - they are metadata, not data fields
            if field_name == &hash_field_name || field_name == &range_field_name {
                InfrastructureLogger::log_debug_info(
                    "MutationService",
                    &format!(
                        "Skipping key field '{}' in HashRange schema mutation",
                        field_name
                    ),
                );
                continue;
            }

            InfrastructureLogger::log_operation_start(
                "MutationService",
                "Processing HashRange field",
                &format!("{} for hash_key: {}, range_key: {}", field_name, hash_key_value, range_key_value),
            );

            let NormalizedFieldValueRequest { request, context } = self
                .normalized_field_value_request(
                    schema,
                    field_name,
                    value,
                    Some(&normalized_hash_value),
                    Some(&normalized_range_value),
                    Some(mutation_hash),
                )?;

            let context_summary = summarize_normalized_context(&context);

            InfrastructureLogger::log_debug_info(
                "MutationService",
                &format!(
                    "Publishing HashRange field request for {}.{} [{}]",
                    schema.name, field_name, context_summary
                ),
            );

            match self.message_bus.publish(request) {
                Ok(_) => {
                    InfrastructureLogger::log_operation_success(
                        "MutationService",
                        "HashRange field update request sent",
                        &format!("{}.{} [{}]", schema.name, field_name, context_summary),
                    );
                }
                Err(e) => {
                    InfrastructureLogger::log_operation_error(
                        "MutationService",
                        "Failed to send HashRange field update",
                        &format!(
                            "{}.{} [{}]: {:?}",
                            schema.name, field_name, context_summary, e
                        ),
                    );
                    return Err(SchemaError::InvalidData(format!(
                        "Failed to update HashRange field {}: {}",
                        field_name, e
                    )));
                }
            }
        }

        InfrastructureLogger::log_operation_success(
            "MutationService",
            "All HashRange field updates sent successfully",
            "",
        );
        Ok(())
    }

    /// Update atoms for a Range schema mutation using universal key configuration
    ///
    /// This method processes Range schema mutations by dynamically determining the range
    /// field name from the schema's universal key configuration or falling back to legacy range_key.
    /// This allows Range schemas to use any field name for their range key.
    pub fn update_range_schema_fields(
        &self,
        schema: &Schema,
        fields_and_values: &HashMap<String, Value>,
        range_key_value: &str,
        mutation_hash: &str,
    ) -> Result<(), SchemaError> {
        InfrastructureLogger::log_debug_info(
            "MutationService",
            &format!(
                "Processing range schema mutation for range_key_value: {}",
                range_key_value
            ),
        );

        // Get the actual range field name from the schema's universal key configuration
        let range_field_name = self.get_range_key_field_name(schema)?;

        InfrastructureLogger::log_debug_info(
            "MutationService",
            &format!(
                "Range schema '{}' key field - range: '{}'",
                schema.name, range_field_name
            ),
        );

        let normalized_range_value = Value::String(range_key_value.to_string());

        // Process each field in the mutation
        for (field_name, value) in fields_and_values {
            InfrastructureLogger::log_operation_start(
                "MutationService",
                "Processing range field",
                &format!("{} for range_key: {}", field_name, range_key_value),
            );

            let NormalizedFieldValueRequest { request, context } = self
                .normalized_field_value_request(
                    schema,
                    field_name,
                    value,
                    None,
                    Some(&normalized_range_value),
                    Some(mutation_hash),
                )?;

            let context_summary = summarize_normalized_context(&context);

            InfrastructureLogger::log_debug_info(
                "MutationService",
                &format!(
                    "Publishing range field request for {}.{} [{}]",
                    schema.name, field_name, context_summary
                ),
            );

            match self.message_bus.publish(request) {
                Ok(_) => {
                    InfrastructureLogger::log_operation_success(
                        "MutationService",
                        "Range field update request sent",
                        &format!("{}.{} [{}]", schema.name, field_name, context_summary),
                    );
                }
                Err(e) => {
                    InfrastructureLogger::log_operation_error(
                        "MutationService",
                        "Failed to send range field update",
                        &format!(
                            "{}.{} [{}]: {:?}",
                            schema.name, field_name, context_summary, e
                        ),
                    );
                    return Err(SchemaError::InvalidData(format!(
                        "Failed to update range field {}: {}",
                        field_name, e
                    )));
                }
            }
        }

        InfrastructureLogger::log_operation_success(
            "MutationService",
            "All range field updates sent successfully",
            "",
        );
        Ok(())
    }

    /// Get the hash and range field names from the schema's universal key configuration
    pub fn get_hashrange_key_field_names(
        &self,
        schema: &Schema,
    ) -> Result<(String, String), SchemaError> {
        // For HashRange schemas, both hash_field and range_field are required
        let key_config = schema.key.as_ref().ok_or_else(|| {
            SchemaError::InvalidData(format!(
                "HashRange schema '{}' requires key configuration",
                schema.name
            ))
        })?;

        let hash_field = if key_config.hash_field.trim().is_empty() {
            return Err(SchemaError::InvalidData(format!(
                "HashRange schema '{}' requires non-empty hash_field in key configuration",
                schema.name
            )));
        } else {
            key_config.hash_field.clone()
        };

        let range_field = if key_config.range_field.trim().is_empty() {
            return Err(SchemaError::InvalidData(format!(
                "HashRange schema '{}' requires non-empty range_field in key configuration",
                schema.name
            )));
        } else {
            key_config.range_field.clone()
        };

        InfrastructureLogger::log_debug_info(
            "MutationService",
            &format!(
                "HashRange schema '{}' key fields - hash: '{}', range: '{}'",
                schema.name, hash_field, range_field
            ),
        );

        Ok((hash_field, range_field))
    }

    /// Get the range field name from the schema's universal key configuration or legacy range_key
    pub fn get_range_key_field_name(&self, schema: &Schema) -> Result<String, SchemaError> {
        match &schema.schema_type {
            crate::schema::types::schema::SchemaType::Range { range_key } => {
                if let Some(key_config) = &schema.key {
                    // Universal key configuration takes precedence
                    if key_config.range_field.trim().is_empty() {
                        return Err(SchemaError::InvalidData(format!(
                            "Range schema '{}' with key configuration requires non-empty range_field",
                            schema.name
                        )));
                    }
                    Ok(key_config.range_field.clone())
                } else {
                    // Fall back to legacy range_key for backward compatibility
                    Ok(range_key.clone())
                }
            }
            _ => {
                Err(SchemaError::InvalidData(format!(
                    "get_range_key_field_name can only be called on Range schemas, got: {:?}",
                    schema.schema_type
                )))
            }
        }
    }
}
