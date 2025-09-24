//! Unified field validation utilities.
//!
//! This module consolidates all field validation logic that was previously duplicated
//! across mutation/validation.rs, transform/validation.rs, and schema/validator.rs.
//! It provides a single source of truth for field validation patterns.

use crate::schema::types::{SchemaError, field::FieldVariant, Schema, Mutation};
use crate::schema::types::field::FieldType;
use crate::schema::types::schema::SchemaType;
use serde_json::Value as JsonValue;

/// Unified field value validation based on field variant type.
///
/// This function consolidates the duplicate field value validation logic that was
/// previously scattered across multiple modules.
///
/// # Arguments
///
/// * `field_variant` - The field variant type to validate against
/// * `value` - The JSON value to validate
///
/// # Returns
///
/// Validation result or error
pub fn validate_field_value(
    field_variant: &FieldVariant,
    value: &JsonValue,
) -> Result<(), SchemaError> {
    match field_variant {
        FieldVariant::Single(_) => {
            validate_single_field_value(value)
        }
        FieldVariant::Range(_) => {
            validate_range_field_value(value)
        }
        FieldVariant::HashRange(_) => {
            validate_hashrange_field_value(value)
        }
    }
}

/// Validates a single field value.
fn validate_single_field_value(value: &JsonValue) -> Result<(), SchemaError> {
    if value.is_null() {
        return Err(SchemaError::InvalidData(
            "Single field value cannot be null".to_string(),
        ));
    }
    Ok(())
}

/// Validates a range field value.
fn validate_range_field_value(value: &JsonValue) -> Result<(), SchemaError> {
    if !value.is_object() {
        return Err(SchemaError::InvalidData(
            "Range field value must be an object".to_string(),
        ));
    }
    Ok(())
}

/// Validates a hash-range field value.
fn validate_hashrange_field_value(value: &JsonValue) -> Result<(), SchemaError> {
    if !value.is_object() {
        return Err(SchemaError::InvalidData(
            "HashRange field value must be an object".to_string(),
        ));
    }
    Ok(())
}

/// Unified range field consistency validation.
///
/// This function consolidates the duplicate range field validation logic that was
/// previously duplicated between validate_range_field_consistency and 
/// validate_json_range_field_consistency.
///
/// # Arguments
///
/// * `schema_name` - The name of the schema being validated
/// * `range_key` - The range key field name
/// * `fields` - Iterator over field names and their types
///
/// # Returns
///
/// Validation result or error
pub fn validate_range_field_consistency_unified<T, F>(
    schema_name: &str,
    range_key: &str,
    fields: T,
) -> Result<(), SchemaError>
where
    T: IntoIterator<Item = (String, F)>,
    F: FieldTypeProvider,
{
    let mut fields_iter = fields.into_iter();
    
    // First ensure the range_key field exists and is a Range field
    let range_key_field = fields_iter
        .find(|(name, _)| name == range_key)
        .ok_or_else(|| SchemaError::InvalidField(format!(
            "RangeSchema '{}' range_key field '{}' does not exist in the schema",
            schema_name, range_key
        )))?;

    if !range_key_field.1.is_range_type() {
        return Err(SchemaError::InvalidField(format!(
            "RangeSchema '{}' has range_key field '{}' that is a {} field, but range_key must be a Range field",
            schema_name, range_key, range_key_field.1.field_type_name()
        )));
    }

    // Validate that ALL fields in the RangeSchema are Range fields
    let all_fields: Vec<_> = fields_iter.collect();
    for (field_name, field_type) in all_fields {
        if !field_type.is_range_type() {
            return Err(SchemaError::InvalidField(format!(
                "RangeSchema '{}' contains {} field '{}', but ALL fields must be Range fields. \
                Consider using a regular Schema (not RangeSchema) if you need {} fields, \
                or convert '{}' to a Range field to maintain RangeSchema consistency.",
                schema_name, field_type.field_type_name(), field_name, field_type.field_type_name(), field_name
            )));
        }
    }

    Ok(())
}

/// Trait for providing field type information in a unified way.
pub trait FieldTypeProvider {
    fn is_range_type(&self) -> bool;
    fn field_type_name(&self) -> &'static str;
}

impl FieldTypeProvider for FieldVariant {
    fn is_range_type(&self) -> bool {
        matches!(self, FieldVariant::Range(_))
    }

    fn field_type_name(&self) -> &'static str {
        match self {
            FieldVariant::Single(_) => "Single",
            FieldVariant::Range(_) => "Range",
            FieldVariant::HashRange(_) => "HashRange",
        }
    }
}

impl FieldTypeProvider for FieldType {
    fn is_range_type(&self) -> bool {
        matches!(self, FieldType::Range)
    }

    fn field_type_name(&self) -> &'static str {
        match self {
            FieldType::Single => "Single",
            FieldType::Range => "Range",
            FieldType::HashRange => "HashRange",
        }
    }
}

/// Unified range schema mutation validation.
///
/// This function consolidates the duplicate range schema mutation validation logic
/// that was previously scattered across multiple modules.
///
/// # Arguments
///
/// * `schema` - The schema being validated
/// * `mutation` - The mutation being validated
///
/// # Returns
///
/// Validation result or error
pub fn validate_range_schema_mutation_unified(
    schema: &Schema,
    mutation: &Mutation,
) -> Result<(), SchemaError> {
    // Get the range field name using universal key configuration or legacy range_key
    let range_field_name = extract_range_field_name(schema)?;

    // Validate that the range field is present in the mutation
    let range_key_value = mutation.fields_and_values.get(&range_field_name)
        .ok_or_else(|| SchemaError::InvalidData(format!(
            "Range schema mutation for '{}' is missing required range field '{}'. All range schema mutations must provide a value for the range field.",
            schema.name, range_field_name
        )))?;

    // Validate the range field value is not null or empty
    validate_range_field_value_content(&schema.name, &range_field_name, range_key_value)?;

    // Validate all fields in the schema are RangeFields
    validate_all_fields_are_range_fields(schema)?;

    Ok(())
}

/// Extracts the range field name from schema configuration.
fn extract_range_field_name(schema: &Schema) -> Result<String, SchemaError> {
    match &schema.schema_type {
        SchemaType::Range { range_key } => {
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
                "validate_range_schema_mutation_format can only be called on Range schemas, got: {:?}",
                schema.schema_type
            )))
        }
    }
}

/// Validates the content of a range field value.
fn validate_range_field_value_content(
    schema_name: &str,
    range_field_name: &str,
    range_key_value: &JsonValue,
) -> Result<(), SchemaError> {
    // Validate the range field value is not null
    if range_key_value.is_null() {
        return Err(SchemaError::InvalidData(format!(
            "Range schema mutation for '{}' has null value for range field '{}'. Range field must have a valid value.",
            schema_name, range_field_name
        )));
    }

    // If range field value is a string, ensure it's not empty
    if let Some(str_value) = range_key_value.as_str() {
        if str_value.trim().is_empty() {
            return Err(SchemaError::InvalidData(format!(
                "Range schema mutation for '{}' has empty string value for range field '{}'. Range field must have a non-empty value.",
                schema_name, range_field_name
            )));
        }
    }

    Ok(())
}

/// Validates that all fields in a schema are Range fields.
fn validate_all_fields_are_range_fields(schema: &Schema) -> Result<(), SchemaError> {
    for (field_name, field_variant) in &schema.fields {
        match field_variant {
            FieldVariant::Range(_) => {
                // Correct - this is a Range field
                continue;
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
    Ok(())
}

/// Unified payment configuration validation.
///
/// This function consolidates the duplicate payment configuration validation logic
/// that was previously scattered across multiple modules.
///
/// # Arguments
///
/// * `field_name` - The name of the field being validated
/// * `base_multiplier` - The base multiplier value
/// * `min_payment` - The optional minimum payment value
///
/// # Returns
///
/// Validation result or error
pub fn validate_payment_config(
    field_name: &str,
    base_multiplier: f64,
    min_payment: Option<u64>,
) -> Result<(), SchemaError> {
    if base_multiplier <= 0.0 {
        return Err(SchemaError::InvalidField(format!(
            "Field {field_name} base_multiplier must be positive",
        )));
    }

    if let Some(min) = min_payment {
        if min == 0 {
            return Err(SchemaError::InvalidField(format!(
                "Field {field_name} min_payment cannot be zero",
            )));
        }
    }

    Ok(())
}

/// Unified field mapper validation.
///
/// This function consolidates the duplicate field mapper validation logic.
///
/// # Arguments
///
/// * `field_name` - The name of the field being validated
/// * `field_mappers` - Iterator over field mapper key-value pairs
///
/// # Returns
///
/// Validation result or error
pub fn validate_field_mappers<T>(
    field_name: &str,
    field_mappers: T,
) -> Result<(), SchemaError>
where
    T: IntoIterator<Item = (String, String)>,
{
    for (mapper_key, mapper_value) in field_mappers {
        if mapper_key.is_empty() || mapper_value.is_empty() {
            return Err(SchemaError::InvalidField(format!(
                "Field {field_name} has invalid field mapper: empty key or value",
            )));
        }
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::schema::types::field::{FieldVariant, SingleField, RangeField};
    use crate::permissions::types::policy::PermissionsPolicy;
    use crate::fees::FieldPaymentConfig;
    use serde_json::json;
    use std::collections::HashMap;

    fn create_test_single_field() -> SingleField {
        SingleField::new(
            PermissionsPolicy::default(),
            FieldPaymentConfig::default(),
            HashMap::new(),
        )
    }

    fn create_test_range_field() -> RangeField {
        RangeField::new(
            PermissionsPolicy::default(),
            FieldPaymentConfig::default(),
            HashMap::new(),
        )
    }

    #[test]
    fn test_validate_field_value_single_null() {
        let field_variant = FieldVariant::Single(create_test_single_field());
        let result = validate_field_value(&field_variant, &json!(null));
        assert!(result.is_err());
        assert!(result.unwrap_err().to_string().contains("cannot be null"));
    }

    #[test]
    fn test_validate_field_value_single_valid() {
        let field_variant = FieldVariant::Single(create_test_single_field());
        let result = validate_field_value(&field_variant, &json!("valid"));
        assert!(result.is_ok());
    }

    #[test]
    fn test_validate_field_value_range_object() {
        let field_variant = FieldVariant::Range(create_test_range_field());
        let result = validate_field_value(&field_variant, &json!({"key": "value"}));
        assert!(result.is_ok());
    }

    #[test]
    fn test_validate_field_value_range_not_object() {
        let field_variant = FieldVariant::Range(create_test_range_field());
        let result = validate_field_value(&field_variant, &json!("not_object"));
        assert!(result.is_err());
        assert!(result.unwrap_err().to_string().contains("must be an object"));
    }

    #[test]
    fn test_validate_payment_config_valid() {
        let result = validate_payment_config("test_field", 1.5, Some(10));
        assert!(result.is_ok());
    }

    #[test]
    fn test_validate_payment_config_invalid_multiplier() {
        let result = validate_payment_config("test_field", 0.0, None);
        assert!(result.is_err());
        assert!(result.unwrap_err().to_string().contains("must be positive"));
    }

    #[test]
    fn test_validate_payment_config_invalid_min_payment() {
        let result = validate_payment_config("test_field", 1.0, Some(0));
        assert!(result.is_err());
        assert!(result.unwrap_err().to_string().contains("cannot be zero"));
    }

    #[test]
    fn test_validate_field_mappers_valid() {
        let mappers = vec![("key1".to_string(), "value1".to_string())];
        let result = validate_field_mappers("test_field", mappers);
        assert!(result.is_ok());
    }

    #[test]
    fn test_validate_field_mappers_empty_key() {
        let mappers = vec![("".to_string(), "value1".to_string())];
        let result = validate_field_mappers("test_field", mappers);
        assert!(result.is_err());
        assert!(result.unwrap_err().to_string().contains("empty key or value"));
    }

    #[test]
    fn test_validate_field_mappers_empty_value() {
        let mappers = vec![("key1".to_string(), "".to_string())];
        let result = validate_field_mappers("test_field", mappers);
        assert!(result.is_err());
        assert!(result.unwrap_err().to_string().contains("empty key or value"));
    }
}
