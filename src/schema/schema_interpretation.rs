use std::collections::HashMap;

use crate::schema::constants::{HASH_FIELD_NAME, RANGE_FIELD_NAME};
use crate::schema::field::HashRangeField;
use crate::schema::types::{
    field::common::Field, FieldVariant, Schema, SchemaError,
    SingleField, SchemaType,
};
use crate::schema::json_schema::{JsonSchemaDefinition, JsonSchemaField};

/// Converts a JSON schema field to a FieldVariant.
fn convert_field(
    json_field: JsonSchemaField,
    schema_type: &crate::schema::types::schema::SchemaType,
) -> FieldVariant {
    match schema_type {
        crate::schema::types::schema::SchemaType::HashRange { .. } => {
            // For HashRange schemas, create HashRangeField variants
            let hashrange_field = HashRangeField {
                inner: crate::schema::types::field::common::FieldCommon::new(
                    json_field.permission_policy.into(),
                    json_field.payment_config.into(),
                    json_field.field_mappers,
                ),
                molecule_hash_range: None,
            };
            FieldVariant::HashRange(hashrange_field)
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
    json_schema: JsonSchemaDefinition,
) -> Result<Schema, SchemaError> {

    // Convert fields
    let mut fields = std::collections::HashMap::new();
    for (field_name, json_field) in json_schema.fields {
        fields.insert(
            field_name,
            convert_field(json_field, &json_schema.schema_type),
        );
    }

    let key = match json_schema.schema_type {
        SchemaType::HashRange { keyconfig } => Some(keyconfig),
        SchemaType::Range { keyconfig } => Some(keyconfig),
        SchemaType::Single => None,
    };

    // Create the schema
    Ok(Schema {
        name: json_schema.name,
        schema_type: json_schema.schema_type,
        key: None, // Legacy JSON schema interpretation doesn't support universal keys yet
        fields,
        payment_config: json_schema.payment_config,
        hash: json_schema.hash,
    })
}
