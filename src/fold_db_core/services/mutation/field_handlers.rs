//! Field-specific update logic for different field types.
//!
//! This module contains handlers for updating different types of fields:
//! - Single fields
//! - Range fields  
//! - HashRange fields

use crate::schema::types::{Schema, SchemaError, field::FieldVariant};
use crate::fold_db_core::infrastructure::factory::InfrastructureLogger;
use crate::fold_db_core::services::mutation::{MutationService, NormalizedFieldValueRequest};
use crate::fold_db_core::services::mutation::utilities::summarize_normalized_context;
use serde_json::Value;

impl MutationService {
    /// Handle single field mutation
    pub fn update_single_field(
        &self,
        schema: &Schema,
        field_name: &str,
        _single_field: &crate::schema::types::field::single_field::SingleField,
        value: &Value,
        mutation_hash: &str,
    ) -> Result<(), SchemaError> {
        InfrastructureLogger::log_operation_start(
            "MutationService",
            "Updating single field",
            &format!("{}.{}", schema.name, field_name),
        );

        let NormalizedFieldValueRequest { request, context } = self
            .normalized_field_value_request(
                schema,
                field_name,
                value,
                None,
                None,
                Some(mutation_hash),
            )?;

        let context_summary = summarize_normalized_context(&context);

        InfrastructureLogger::log_debug_info(
            "MutationService",
            &format!(
                "Publishing single field request for {}.{} [{}]",
                schema.name, field_name, context_summary
            ),
        );

        if let Err(e) = self.message_bus.publish(request) {
            InfrastructureLogger::log_operation_error(
                "MutationService",
                "Failed to send field value set request",
                &format!(
                    "{}.{} [{}]: {:?}",
                    schema.name, field_name, context_summary, e
                ),
            );
            return Err(SchemaError::InvalidData(format!(
                "Failed to set field value: {}",
                e
            )));
        }
        InfrastructureLogger::log_operation_success(
            "MutationService",
            "Field value set request sent",
            &format!("{}.{} [{}]", schema.name, field_name, context_summary),
        );

        // Transform triggers are now handled automatically by TransformOrchestrator
        // via direct FieldValueSet event monitoring
        Ok(())
    }

    /// Handle HashRange field mutation
    pub fn update_hashrange_field(
        &self,
        schema: &Schema,
        field_name: &str,
        _value: &Value,
        _mutation_hash: &str,
    ) -> Result<(), SchemaError> {
        InfrastructureLogger::log_operation_start(
            "MutationService",
            "Updating HashRange field",
            &format!("{}.{}", schema.name, field_name),
        );

        // HashRange fields should be processed via the HashRange schema method which has proper hash_key and range_key context
        InfrastructureLogger::log_operation_error(
            "MutationService",
            "Individual HashRange field updates not supported",
            "HashRange fields must be updated via HashRange schema mutation.",
        );
        Err(SchemaError::InvalidData(format!(
            "HashRange field '{}' in schema '{}' cannot be updated individually. Use HashRange schema mutation instead.",
            field_name, schema.name
        )))
    }

    /// Update individual field value (main field update entry point)
    pub fn update_field_value(
        &self,
        schema: &Schema,
        field_name: &str,
        value: &Value,
        mutation_hash: &str,
    ) -> Result<(), SchemaError> {
        InfrastructureLogger::log_operation_start(
            "MutationService",
            "Updating field",
            &format!("{}.{}", schema.name, field_name),
        );

        // Get field definition from schema
        let field_variant = schema.fields.get(field_name).ok_or_else(|| {
            SchemaError::InvalidData(format!(
                "Field '{}' not found in schema '{}'",
                field_name, schema.name
            ))
        })?;

        // Apply field-specific mutation logic
        match field_variant {
            FieldVariant::Single(single_field) => {
                self.update_single_field(schema, field_name, single_field, value, mutation_hash)
            }
            FieldVariant::Range(_range_field) => {
                InfrastructureLogger::log_operation_error(
                    "MutationService",
                    "Individual range field updates not supported",
                    "Range fields must be updated via range schema mutation.",
                );
                Err(SchemaError::InvalidData(format!(
                    "Range field '{}' in schema '{}' cannot be updated individually. Use range schema mutation instead.",
                    field_name, schema.name
                )))
            }
            FieldVariant::HashRange(_hash_range_field) => {
                self.update_hashrange_field(schema, field_name, value, mutation_hash)
            }
        }
    }
}
