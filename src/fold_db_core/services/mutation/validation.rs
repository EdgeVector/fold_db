//! Validation logic for mutations and field values.
//!
//! This module contains validation functions for field values and schema mutations,
//! ensuring data integrity and proper format compliance.

use crate::schema::types::{Schema, SchemaError, Mutation, field::FieldVariant};
use crate::logging::features::{log_feature, LogFeature};
use crate::fold_db_core::infrastructure::factory::InfrastructureLogger;

/// Validates field value format based on field variant type
pub fn validate_field_value(
    field_variant: &FieldVariant,
    value: &serde_json::Value,
) -> Result<(), SchemaError> {
    match field_variant {
        FieldVariant::Single(_) => {
            // Validate single field value format
            if value.is_null() {
                return Err(SchemaError::InvalidData(
                    "Single field value cannot be null".to_string(),
                ));
            }
            Ok(())
        }
        FieldVariant::Range(_) => {
            // Validate range field value format
            if !value.is_object() {
                return Err(SchemaError::InvalidData(
                    "Range field value must be an object".to_string(),
                ));
            }
            Ok(())
        }
        FieldVariant::HashRange(_) => {
            // Validate hash-range field value format
            if !value.is_object() {
                return Err(SchemaError::InvalidData(
                    "HashRange field value must be an object".to_string(),
                ));
            }
            Ok(())
        }
    }
}

/// Range schema mutation validation using universal key configuration
///
/// This function validates Range schema mutations by checking for the presence and validity
/// of the range key field, using universal key configuration when available or falling back
/// to legacy range_key patterns.
pub fn validate_range_schema_mutation_format(
    schema: &Schema,
    mutation: &Mutation,
) -> Result<(), SchemaError> {
    // Get the range field name using universal key configuration or legacy range_key
    let range_field_name = match &schema.schema_type {
        crate::schema::types::schema::SchemaType::Range { range_key } => {
            if let Some(key_config) = &schema.key {
                // Universal key configuration takes precedence
                if key_config.range_field.trim().is_empty() {
                    return Err(SchemaError::InvalidData(format!(
                        "Range schema '{}' with key configuration requires non-empty range_field",
                        schema.name
                    )));
                }
                key_config.range_field.clone()
            } else {
                // Fall back to legacy range_key for backward compatibility
                range_key.clone()
            }
        }
        _ => {
            return Err(SchemaError::InvalidData(format!(
            "validate_range_schema_mutation_format can only be called on Range schemas, got: {:?}",
            schema.schema_type
        )))
        }
    };

    log_feature!(
        LogFeature::Mutation,
        info,
        "🔍 Validating Range schema mutation format for schema: {} with range_field: {}",
        schema.name,
        range_field_name
    );

    // MANDATORY: Range schema mutations MUST include the range field
    let range_key_value = mutation.fields_and_values.get(&range_field_name)
        .ok_or_else(|| SchemaError::InvalidData(format!(
            "Range schema mutation for '{}' is missing required range field '{}'. All range schema mutations must provide a value for the range field.",
            schema.name, range_field_name
        )))?;

    // Validate the range field value is not null or empty
    if range_key_value.is_null() {
        return Err(SchemaError::InvalidData(format!(
            "Range schema mutation for '{}' has null value for range field '{}'. Range field must have a valid value.",
            schema.name, range_field_name
        )));
    }

    // If range field value is a string, ensure it's not empty
    if let Some(str_value) = range_key_value.as_str() {
        if str_value.trim().is_empty() {
            return Err(SchemaError::InvalidData(format!(
                "Range schema mutation for '{}' has empty string value for range field '{}'. Range field must have a non-empty value.",
                schema.name, range_field_name
            )));
        }
    }

    // Validate all fields in the schema are RangeFields
    for (field_name, field_variant) in &schema.fields {
        match field_variant {
            FieldVariant::Range(_) => {
                InfrastructureLogger::log_operation_success(
                    "MutationService",
                    "Field validation",
                    &format!("Field '{}' is correctly a RangeField", field_name),
                );
            }
            FieldVariant::Single(_) => {
                return Err(SchemaError::InvalidData(format!(
                        "Range schema '{}' contains Single field '{}', but all fields must be RangeFields",
                        schema.name, field_name
                    )));
            }
            FieldVariant::HashRange(_) => {
                return Err(SchemaError::InvalidData(format!(
                        "Range schema '{}' contains HashRange field '{}', but all fields must be RangeFields",
                        schema.name, field_name
                    )));
            }
        }
    }

    InfrastructureLogger::log_operation_success(
        "MutationService",
        "Range schema mutation format validation passed",
        &format!("schema: {}", schema.name),
    );

    Ok(())
}
