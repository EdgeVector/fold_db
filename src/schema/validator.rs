use super::{
    core::SchemaCore,
    types::{Field, JsonSchemaDefinition, Schema, SchemaError},
};
use crate::schema::types::field::FieldType;
use crate::transform::TransformExecutor;
use crate::validation_utils::ValidationUtils;
use crate::validation::{
    validate_range_field_consistency_unified, validate_payment_config, validate_field_mappers,
    templates
};
use crate::{invalid_field_fmt, invalid_field};

/// Validates a [`Schema`] before it is loaded into the database.
///
/// The validator checks general schema formatting rules and verifies
/// that any transforms reference valid fields in other schemas.
pub struct SchemaValidator<'a> {
    core: &'a SchemaCore,
}

impl<'a> SchemaValidator<'a> {
    /// Create a new validator operating on the provided [`SchemaCore`].
    pub fn new(core: &'a SchemaCore) -> Self {
        Self { core }
    }

    /// Get a reference to the underlying SchemaCore
    pub fn schema_core(&self) -> &SchemaCore {
        self.core
    }

    /// Validate the given [`Schema`].
    pub fn validate(&self, schema: &Schema) -> Result<(), SchemaError> {
        ValidationUtils::require_valid_schema_name(&schema.name)?;

        // For RangeSchema, ensure the range_key is a field in the schema
        if let Some(range_key) = schema.range_key() {
            if !schema.fields.contains_key(range_key) {
                return Err(invalid_field_fmt!(templates::range::RANGE_KEY_NOT_FOUND, range_key));
            }
        }

        // CRITICAL: For RangeSchema, ensure ALL Range fields use the SAME range_key
        // This is a fundamental constraint that ensures data consistency across the schema
        if let Some(range_key) = schema.range_key() {
            validate_range_field_consistency_unified(
                &schema.name,
                range_key,
                schema.fields.iter().map(|(name, variant)| (name.clone(), variant.clone())),
            )?;
        }

        ValidationUtils::require_positive(
            schema.payment_config.base_multiplier,
            "Schema base_multiplier",
        )?;

        for (field_name, field) in &schema.fields {
            validate_payment_config(
                field_name,
                field.payment_config().base_multiplier,
                field.payment_config().min_payment,
            )?;

            if let Some(transform) = field.transform() {
                // Basic syntax validation
                TransformExecutor::validate_transform(transform)?;

                // Validate inputs
                for input in &transform.inputs {
                    let (sname, fname) = input.split_once('.').ok_or_else(|| {
                        SchemaError::InvalidTransform(format!(
                            "Invalid input format {input} for field {field_name}",
                        ))
                    })?;

                    if sname == schema.name {
                        if fname == field_name {
                            return Err(SchemaError::InvalidTransform(format!(
                                "Transform input {input} cannot reference its own field",
                            )));
                        }
                        if !schema.fields.contains_key(fname) {
                            return Err(SchemaError::InvalidTransform(format!(
                                "Input {input} references unknown field",
                            )));
                        }
                    } else {
                        let src_schema = self.core.get_schema(sname)?.ok_or_else(|| {
                            SchemaError::InvalidTransform(format!(
                                "Schema {sname} not found for input {input}",
                            ))
                        })?;

                        if !src_schema.fields.contains_key(fname) {
                            return Err(SchemaError::InvalidTransform(format!(
                                "Input {input} references unknown field",
                            )));
                        }
                    }
                }

                // Validate output
                let (out_schema, out_field) =
                    transform.output.split_once('.').ok_or_else(|| {
                        SchemaError::InvalidTransform(format!(
                            "Invalid output format {} for field {field_name}",
                            transform.output
                        ))
                    })?;

                if out_schema == schema.name {
                    if out_field != field_name {
                        return Err(SchemaError::InvalidTransform(format!(
                            "Transform output {} does not match field name {}",
                            transform.output, field_name
                        )));
                    }
                } else {
                    let target = self.core.get_schema(out_schema)?.ok_or_else(|| {
                        SchemaError::InvalidTransform(format!(
                            "Schema {out_schema} not found for output {out_schema}.{out_field}",
                        ))
                    })?;

                    if !target.fields.contains_key(out_field) {
                        return Err(SchemaError::InvalidTransform(format!(
                            "Output field {} not found in schema {}",
                            out_field, out_schema
                        )));
                    }
                }
            }
        }

        Ok(())
    }

    /// Validate a [`JsonSchemaDefinition`] before interpretation
    pub fn validate_json_schema(&self, schema: &JsonSchemaDefinition) -> Result<(), SchemaError> {
        ValidationUtils::require_valid_schema_name(&schema.name)?;

        // CRITICAL: For JSON RangeSchema definitions, validate range key consistency
        // This ensures that JSON-defined schemas also follow the same constraints as programmatically created ones
        if let crate::schema::types::schema::SchemaType::Range { range_key } = &schema.schema_type {
            validate_range_field_consistency_unified(
                &schema.name,
                range_key,
                schema.fields.iter().map(|(name, field_def)| (name.clone(), field_def.field_type.clone())),
            )?;
        }

        for (field_name, field) in &schema.fields {
            if field_name.is_empty() {
                return Err(invalid_field!(templates::field::EMPTY_FIELD_NAME));
            }

            validate_payment_config(
                field_name,
                field.payment_config.base_multiplier,
                field.payment_config.min_payment,
            )?;

            validate_field_mappers(field_name, field.field_mappers.iter().map(|(k, v)| (k.clone(), v.clone())))?;
        }

        Ok(())
    }

    /// Validate a mutation for a RangeSchema.
    pub fn validate_range_schema_mutation(
        &self,
        schema: &crate::schema::types::Schema,
        mutation: &crate::schema::types::operations::Mutation,
    ) -> Result<(), crate::schema::types::SchemaError> {
        if let Some(range_key) = schema.range_key() {
            // 1. Ensure all fields are rangeFields
            for (field_name, field_def) in &schema.fields {
                if !matches!(
                    field_def,
                    crate::schema::types::field::FieldVariant::Range(_)
                ) {
                    return Err(crate::schema::types::SchemaError::InvalidData(format!(
                        "All fields in a RangeSchema must be rangeFields. Field '{}' is not a rangeField.",
                        field_name
                    )));
                }
            }
            // 2. Ensure all values in fields_and_values contain the same range_key value
            let mut found_range_key_value: Option<&serde_json::Value> = None;
            for (field_name, value) in mutation.fields_and_values.iter() {
                // Value must be an object containing the range_key
                let obj = value.as_object().ok_or_else(|| {
                    crate::schema::types::SchemaError::InvalidData(format!(
                        "Value for field '{}' must be an object containing the range_key '{}'.",
                        field_name, range_key
                    ))
                })?;
                let key_val = obj.get(range_key).ok_or_else(|| {
                    crate::schema::types::SchemaError::InvalidData(format!(
                        "Value for field '{}' must contain the range_key '{}'.",
                        field_name, range_key
                    ))
                })?;
                if let Some(existing) = &found_range_key_value {
                    if existing != &key_val {
                        return Err(crate::schema::types::SchemaError::InvalidData(format!(
                            "All range_key values must match for RangeSchema. Field '{}' has a different value.", field_name
                        )));
                    }
                } else {
                    found_range_key_value = Some(key_val);
                }
            }
        }
        Ok(())
    }
}
