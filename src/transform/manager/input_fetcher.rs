use crate::fold_db_core::infrastructure::message_bus::atom_events::MutationContext;
use crate::transform::manager::utils::{DefaultValueHelper, TransformUtils};
use crate::schema::types::Transform;
use crate::schema::types::{Schema, SchemaError};
use crate::schema::types::field::HashRangeFilter;
use serde_json::Value as JsonValue;
use std::collections::HashMap;
use std::sync::Arc;

/// Handles fetching input data for transform execution
pub struct InputFetcher;

impl InputFetcher {
    /// Fetch input values with mutation context for incremental processing
    /// @tomtang keep -- main path
    pub fn fetch_input_values_with_context(
        transform: &Transform,
        db_ops: &Arc<crate::db_operations::DbOperations>,
        mutation_context: &Option<MutationContext>,
    ) -> Result<HashMap<String, JsonValue>, SchemaError> {
        let mut input_values = HashMap::new();
        let inputs_to_process = transform.get_declarative_schema().unwrap().get_inputs();

        for input_field in inputs_to_process {
            if let Some(dot_pos) = input_field.find('.') {
                let input_schema = &input_field[..dot_pos];
                let input_field_name = &input_field[dot_pos + 1..];
                let schema = db_ops.get_schema(input_schema)?.ok_or_else(|| {
                    SchemaError::InvalidData(format!("Schema '{}' not found", input_schema))
                })?;
                let value = Self::fetch_input_for_field_with_context(db_ops, &mut schema.clone(), input_field_name, mutation_context)?;
                input_values.insert(input_schema.to_string() + "." + input_field_name, value);
            }  else {
                let input_schema = input_field;
                let schema = db_ops.get_schema(input_schema.as_str())?.ok_or_else(|| {
                    SchemaError::InvalidData(format!("Schema '{}' not found", input_schema))
                })?;
                for field_name in schema.fields.keys() {
                    let value = Self::fetch_input_for_field_with_context(db_ops, &mut schema.clone(), field_name, mutation_context)?;
                    input_values.insert(input_schema.to_string() + "." + field_name, value);
                }
            }
        }
        Ok(input_values)
    }

    /// Fetch input for a field with mutation context for incremental processing
    /// @tomtang keep -- main path
    fn fetch_input_for_field_with_context(
        db_ops: &Arc<crate::db_operations::DbOperations>,
        schema: &mut Schema,
        field_name: &str,
        mutation_context: &Option<MutationContext>,
    ) -> Result<JsonValue, SchemaError> {
        let key_config = mutation_context.as_ref().and_then(|ctx| ctx.key_config.clone());
        let value = TransformUtils::resolve_field_value(db_ops, schema, field_name, HashRangeFilter::from_key_config(key_config.clone()))
            .unwrap_or_else(|_| {
                DefaultValueHelper::get_default_value_for_field(field_name)
            });
        Ok(value)
    }
}
