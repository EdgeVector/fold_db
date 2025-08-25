use super::{
    core::SchemaCore,
    types::{Field, JsonSchemaDefinition, Schema, SchemaError},
};
use crate::schema::types::field::FieldType;
use crate::transform::TransformExecutor;
use crate::validation_utils::ValidationUtils;

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

    /// Validate the given [`Schema`].
    pub fn validate(&self, schema: &Schema) -> Result<(), SchemaError> {
        ValidationUtils::require_non_empty_string(&schema.name, "Schema name")?;

        // For RangeSchema, ensure the range_key is a field in the schema
        if let Some(range_key) = schema.range_key() {
            if !schema.fields.contains_key(range_key) {
                return Err(SchemaError::InvalidField(format!(
                    "RangeSchema range_key '{}' must be one of the schema's fields.",
                    range_key
                )));
            }
        }

        // CRITICAL: For RangeSchema, ensure ALL Range fields use the SAME range_key
        // This is a fundamental constraint that ensures data consistency across the schema
        if let Some(range_key) = schema.range_key() {
            self.validate_range_field_consistency(schema, range_key)?;
        }

        ValidationUtils::require_positive(schema.payment_config.base_multiplier, "Schema base_multiplier")?;

        for (field_name, field) in &schema.fields {
            if field.payment_config().base_multiplier <= 0.0 {
                return Err(SchemaError::InvalidField(format!(
                    "Field {field_name} base_multiplier must be positive",
                )));
            }

            if let Some(min) = field.payment_config().min_payment {
                if min == 0 {
                    return Err(SchemaError::InvalidField(format!(
                        "Field {field_name} min_payment cannot be zero",
                    )));
                }
            }

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

    /// Validates that all fields in a RangeSchema are Range fields and maintain consistency.
    ///
    /// This is a critical DB-level constraint that ensures:
    /// 1. All fields in a RangeSchema MUST be Range fields (no Single or Collection fields)
    /// 2. The schema's range_key field exists and is a Range field
    /// 3. Consistent field structure across the entire RangeSchema
    ///
    /// This validation prevents data corruption and ensures proper range semantics.
    fn validate_range_field_consistency(
        &self,
        schema: &Schema,
        range_key: &str,
    ) -> Result<(), SchemaError> {
        // First ensure the range_key field itself is a Range field
        match schema.fields.get(range_key) {
            Some(crate::schema::types::field::FieldVariant::Range(_)) => {
                // Good - range_key field is a Range field
            }
            Some(field_variant) => {
                let field_type = match field_variant {
                    crate::schema::types::field::FieldVariant::Single(_) => "Single",
                    crate::schema::types::field::FieldVariant::Range(_) => "Range", // Should not reach here
                    crate::schema::types::field::FieldVariant::HashRange(_) => "HashRange", // Should not reach here
                };
                return Err(SchemaError::InvalidField(format!(
                    "RangeSchema '{}' has range_key field '{}' that is a {} field, but range_key must be a Range field",
                    schema.name, range_key, field_type
                )));
            }
            None => {
                // This should already be caught by earlier validation, but being defensive
                return Err(SchemaError::InvalidField(format!(
                    "RangeSchema '{}' range_key field '{}' does not exist in the schema",
                    schema.name, range_key
                )));
            }
        }

        // Validate that ALL fields in the RangeSchema are Range fields
        // This ensures schema consistency and prevents mixing of field types
        for (field_name, field_variant) in &schema.fields {
            match field_variant {
                crate::schema::types::field::FieldVariant::Range(_) => {
                    // Correct - this is a Range field
                    continue;
                }
                crate::schema::types::field::FieldVariant::Single(_) => {
                    return Err(SchemaError::InvalidField(format!(
                        "RangeSchema '{}' contains Single field '{}', but ALL fields must be Range fields. \
                        Consider using a regular Schema (not RangeSchema) if you need Single fields, \
                        or convert '{}' to a Range field to maintain RangeSchema consistency.",
                        schema.name, field_name, field_name
                    )));
                }
                crate::schema::types::field::FieldVariant::HashRange(_) => {
                    return Err(SchemaError::InvalidField(format!(
                        "RangeSchema '{}' contains HashRange field '{}', but ALL fields must be Range fields. \
                        Consider using a regular Schema (not RangeSchema) if you need HashRange fields, \
                        or convert '{}' to a Range field to maintain RangeSchema consistency.",
                        schema.name, field_name, field_name
                    )));
                }
            }
        }

        // Additional validation: Ensure at least one field exists (the range_key field)
        if schema.fields.is_empty() {
            return Err(SchemaError::InvalidField(format!(
                "RangeSchema '{}' must contain at least the range_key field '{}'",
                schema.name, range_key
            )));
        }

        Ok(())
    }

    /// Validates JSON RangeSchema field consistency to ensure all fields are Range type.
    ///
    /// This method validates JSON schema definitions at creation time to enforce:
    /// 1. All fields in a JSON RangeSchema must be Range fields
    /// 2. The range_key field must exist and be a Range field
    /// 3. No mixing of field types within a RangeSchema
    ///
    /// This prevents invalid schemas from being created via JSON definitions.
    fn validate_json_range_field_consistency(
        &self,
        schema: &JsonSchemaDefinition,
        range_key: &str,
    ) -> Result<(), SchemaError> {
        // Ensure the range_key field exists in the schema
        let range_key_field = schema.fields.get(range_key)
            .ok_or_else(|| SchemaError::InvalidField(format!(
                "JSON RangeSchema '{}' is missing the range_key field '{}'. The range_key must be defined as a field in the schema.",
                schema.name, range_key
            )))?;

        // Ensure the range_key field is a Range field
        if !matches!(range_key_field.field_type, FieldType::Range) {
            return Err(SchemaError::InvalidField(format!(
                "JSON RangeSchema '{}' has range_key field '{}' defined as {:?} field, but it must be a Range field",
                schema.name, range_key, range_key_field.field_type
            )));
        }

        // Validate ALL fields in the JSON RangeSchema are Range fields
        for (field_name, field_def) in &schema.fields {
            if !matches!(field_def.field_type, FieldType::Range) {
                return Err(SchemaError::InvalidField(format!(
                    "JSON RangeSchema '{}' contains {} field '{}', but ALL fields must be Range fields. \
                    Consider using a regular Schema (not RangeSchema) if you need {} fields, \
                    or change '{}' to field_type: \"Range\" to maintain RangeSchema consistency.",
                    schema.name,
                    match field_def.field_type {
                        FieldType::Single => "Single",
                        FieldType::Range => "Range", // shouldn't reach here
                        FieldType::HashRange => "HashRange", // shouldn't reach here
                    },
                    field_name,
                    match field_def.field_type {
                        FieldType::Single => "Single",
                        FieldType::Range => "Range",
                        FieldType::HashRange => "HashRange",
                    },
                    field_name
                )));
            }
        }

        // Ensure at least one field exists (the range_key field)
        if schema.fields.is_empty() {
            return Err(SchemaError::InvalidField(format!(
                "JSON RangeSchema '{}' must contain at least the range_key field '{}'",
                schema.name, range_key
            )));
        }

        Ok(())
    }

    /// Validate a [`JsonSchemaDefinition`] before interpretation
    pub fn validate_json_schema(&self, schema: &JsonSchemaDefinition) -> Result<(), SchemaError> {
        if schema.name.is_empty() {
            return Err(SchemaError::InvalidField(
                "Schema name cannot be empty".to_string(),
            ));
        }

        // CRITICAL: For JSON RangeSchema definitions, validate range key consistency
        // This ensures that JSON-defined schemas also follow the same constraints as programmatically created ones
        if let crate::schema::types::schema::SchemaType::Range { range_key } = &schema.schema_type {
            self.validate_json_range_field_consistency(schema, range_key)?;
        }

        for (field_name, field) in &schema.fields {
            if field_name.is_empty() {
                return Err(SchemaError::InvalidField(
                    "Field name cannot be empty".to_string(),
                ));
            }

            if field.payment_config.base_multiplier <= 0.0 {
                return Err(SchemaError::InvalidField(format!(
                    "Field {field_name} base_multiplier must be positive",
                )));
            }

            for (mapper_key, mapper_value) in &field.field_mappers {
                if mapper_key.is_empty() || mapper_value.is_empty() {
                    return Err(SchemaError::InvalidField(format!(
                        "Field {field_name} has invalid field mapper: empty key or value",
                    )));
                }
            }

            if let Some(min_payment) = field.payment_config.min_payment {
                if min_payment == 0 {
                    return Err(SchemaError::InvalidField(format!(
                        "Field {field_name} min_payment cannot be zero",
                    )));
                }
            }
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
