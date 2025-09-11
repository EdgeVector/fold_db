use crate::schema::types::{JsonSchemaDefinition, JsonSchemaField, Schema, SchemaError, SingleField, FieldVariant, field::common::Field};
use crate::schema::field::HashRangeField;
use crate::schema::constants::{HASH_FIELD_NAME, RANGE_FIELD_NAME};
use super::validator::SchemaValidator;

/// Converts a JSON schema field to a FieldVariant.
fn convert_field(json_field: JsonSchemaField, schema_type: &crate::schema::types::schema::SchemaType) -> FieldVariant {
    match schema_type {
        crate::schema::types::schema::SchemaType::HashRange => {
            // For HashRange schemas, create HashRangeField variants
            let hashrange_field = HashRangeField {
                inner: crate::schema::types::field::common::FieldCommon::new(
                    json_field.permission_policy.into(),
                    json_field.payment_config.into(),
                    json_field.field_mappers,
                ),
                // For HashRange schemas, these fields are set from the schema's key configuration
                // Individual fields don't have hash_field/range_field - they inherit from schema
                hash_field: HASH_FIELD_NAME.to_string(), // Will be set from schema key config
                range_field: RANGE_FIELD_NAME.to_string(), // Will be set from schema key config
                atom_uuid: json_field.molecule_uuid.unwrap_or_default(),
                cached_chains: None,
            };
            FieldVariant::HashRange(Box::new(hashrange_field))
        }
        crate::schema::types::schema::SchemaType::Range { .. } => {
            // For Range schemas, create RangeField variants
            let range_field = crate::schema::types::field::range_field::RangeField {
                inner: crate::schema::types::field::common::FieldCommon::new(
                    json_field.permission_policy.into(),
                    json_field.payment_config.into(),
                    json_field.field_mappers,
                ),
                molecule_range: None, // Will be set when the field is actually used
            };
            FieldVariant::Range(range_field)
        }
        _ => {
            // For other schema types, create SingleField variants
            let mut single_field = SingleField::new(
                json_field.permission_policy.into(),
                json_field.payment_config.into(),
                json_field.field_mappers,
            );

            if let Some(molecule_uuid) = json_field.molecule_uuid {
                single_field.set_molecule_uuid(molecule_uuid);
            }

            // Add transform if present
            if let Some(json_transform) = json_field.transform {
                single_field.set_transform(json_transform.into());
                // IMPORTANT: Fields with transforms are derived/computed fields that:
                // 1. Cannot be directly modified through mutations (they're read-only)
                // 2. Are automatically populated by executing the associated transform
                // 3. Depend on other fields as inputs defined in the transform
                // This ensures data consistency and prevents manual override of computed values
                single_field.set_writable(false);
            }

            FieldVariant::Single(single_field)
        }
    }
}

/// Interprets a JSON schema definition and converts it to a Schema.
pub fn interpret_schema(
    validator: &SchemaValidator,
    json_schema: JsonSchemaDefinition,
) -> Result<Schema, SchemaError> {
    // First validate the JSON schema
    validator.validate_json_schema(&json_schema)?;

    // Convert fields
    let mut fields = std::collections::HashMap::new();
    for (field_name, json_field) in json_schema.fields {
        fields.insert(field_name, convert_field(json_field, &json_schema.schema_type));
    }

    // Create the schema
    Ok(Schema {
        name: json_schema.name,
        schema_type: json_schema.schema_type,
        fields,
        payment_config: json_schema.payment_config,
        hash: json_schema.hash,
    })
}

/// Interprets a JSON schema from a string and loads it as Available.
pub fn load_schema_from_json(
    validator: &SchemaValidator,
    json_str: &str,
) -> Result<Schema, SchemaError> {
    log::info!(
        "Parsing JSON schema from string, length: {}",
        json_str.len()
    );
    let json_schema: JsonSchemaDefinition = serde_json::from_str(json_str)
        .map_err(|e| SchemaError::InvalidField(format!("Invalid JSON schema: {e}")))?;

    log::info!(
        "JSON schema parsed successfully, name: {}, fields: {:?}",
        json_schema.name,
        json_schema.fields.keys().collect::<Vec<_>>()
    );
    let schema = interpret_schema(validator, json_schema)?;
    log::info!(
        "Schema interpreted successfully, name: {}, fields: {:?}",
        schema.name,
        schema.fields.keys().collect::<Vec<_>>()
    );
    Ok(schema)
}

/// Interprets a JSON schema from a file and loads it as Available.
pub fn load_schema_from_file(
    validator: &SchemaValidator,
    path: &str,
) -> Result<Schema, SchemaError> {
    let json_str = std::fs::read_to_string(path)
        .map_err(|e| SchemaError::InvalidField(format!("Failed to read schema file: {e}")))?;

    log::info!(
        "Loading schema from file: {}, content length: {}",
        path,
        json_str.len()
    );
    load_schema_from_json(validator, &json_str)
}
