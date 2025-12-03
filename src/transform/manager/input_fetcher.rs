use crate::fold_db_core::infrastructure::message_bus::atom_events::MutationContext;
use crate::schema::types::Transform;
use crate::schema::types::{Schema, SchemaError};
use crate::schema::types::field::HashRangeFilter;
use crate::schema::types::field::Field;
use crate::schema::types::key_value::KeyValue;
use crate::schema::types::field::FieldValue;
use std::collections::HashMap;
use std::sync::Arc;

/// Handles fetching input data for transform execution
pub struct InputFetcher;

impl InputFetcher {
    /// Fetch input values with mutation context for incremental processing
    /// @tomtang keep -- main path
    pub async fn fetch_input_values_with_context(
        transform: &Transform,
        db_ops: &Arc<crate::db_operations::DbOperationsV2>,
        mutation_context: &Option<MutationContext>,
    ) -> Result<HashMap<String, HashMap<KeyValue, FieldValue>>, SchemaError> {
        let mut input_values = HashMap::new();
        
        // Look up the transform's schema from the database
        let transform_schema = db_ops.get_schema(transform.get_schema_name()).await?.ok_or_else(|| {
            SchemaError::InvalidData(format!("Transform schema '{}' not found", transform.get_schema_name()))
        })?;
        let inputs_to_process = transform_schema.get_inputs();

        for input_field in inputs_to_process {
            if let Some(dot_pos) = input_field.find('.') {
                let input_schema = &input_field[..dot_pos];
                let input_field_name = &input_field[dot_pos + 1..];
                let schema = db_ops.get_schema(input_schema).await?.ok_or_else(|| {
                    SchemaError::InvalidData(format!("Schema '{}' not found", input_schema))
                })?;
                let value = Self::fetch_input_for_field_with_context(db_ops, &mut schema.clone(), input_field_name, mutation_context).await?;
                input_values.insert(input_schema.to_string() + "." + input_field_name, value);
            }  else {
                let input_schema = input_field;
                let schema = db_ops.get_schema(input_schema.as_str()).await?.ok_or_else(|| {
                    SchemaError::InvalidData(format!("Schema '{}' not found", input_schema))
                })?;
                for field_name in schema.runtime_fields.keys() {
                    let value = Self::fetch_input_for_field_with_context(db_ops, &mut schema.clone(), field_name, mutation_context).await?;
                    input_values.insert(input_schema.to_string() + "." + field_name, value);
                }
            }
        }
        Ok(input_values)
    }

    /// Fetch input for a field with mutation context for incremental processing
    /// @tomtang keep -- main path
    async fn fetch_input_for_field_with_context(
        db_ops: &Arc<crate::db_operations::DbOperationsV2>,
        schema: &mut Schema,
        field_name: &str,
        mutation_context: &Option<MutationContext>,
    ) -> Result<HashMap<KeyValue, FieldValue>, SchemaError> {
        let key_value_opt = mutation_context.as_ref().and_then(|ctx| ctx.key_value.clone());
        
        // Check if field exists before getting mutable reference
        if !schema.runtime_fields.contains_key(field_name) {
            let available_fields: Vec<&String> = schema.runtime_fields.keys().collect();
            return Err(SchemaError::InvalidData(format!(
                "Field '{}' not found in schema '{}'. Available fields: {:?}", 
                field_name, schema.name, available_fields
            )));
        }
        
        let field = schema.runtime_fields.get_mut(field_name).unwrap();
        let filter = match key_value_opt {
            Some(kv) => {
                let hash_opt = kv.hash.clone();
                let range_opt = kv.range.clone();
                match (hash_opt, range_opt) {
                    (Some(hash), Some(range)) => Some(HashRangeFilter::HashRangeKey { hash, range }),
                    (Some(hash), None) => Some(HashRangeFilter::HashKey(hash)),
                    (None, Some(prefix)) => Some(HashRangeFilter::RangePrefix(prefix)),
                    (None, None) => None,
                }
            }
            None => None,
        };
        let value = field.resolve_value(db_ops, filter).await?;
        Ok(value)
    }
}
