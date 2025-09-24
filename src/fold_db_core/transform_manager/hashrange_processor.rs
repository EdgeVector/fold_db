use crate::fold_db_core::services::mutation::MutationService;
use crate::schema::constants::TRANSFORM_SYSTEM_ID;
use crate::schema::types::{Mutation, MutationType, SchemaError};
use log::{debug, info};
use serde_json::{json, Value as JsonValue};
use std::collections::HashMap;
use std::sync::Arc;


/// HashRange transform result data structure
#[derive(Debug, Clone)]
pub struct HashRangeTransformResult {
    pub hash_keys: Vec<String>,
    pub range_keys: Vec<String>,
    pub field_data: HashMap<String, Vec<JsonValue>>,
}

/// Handles HashRange-specific processing
pub struct HashRangeProcessor;

impl HashRangeProcessor {
    /// Special storage for HashRange schema transform results using message bus
    pub fn store_hashrange_transform_result_with_message_bus(
        db_ops: &Arc<crate::db_operations::DbOperations>,
        schema_name: &str,
        result: &JsonValue,
        message_bus: &Arc<crate::fold_db_core::infrastructure::MessageBus>,
    ) -> Result<(), SchemaError> {
        info!(
            "🔑 Storing HashRange transform result for schema '{}' using message bus",
            schema_name
        );
        

        let schema = Self::get_hashrange_schema(db_ops, schema_name)?;
        let field_names = Self::extract_field_names(&schema);
        let transform_result = Self::parse_transform_result_structured(result, &field_names)?;

        info!(
            "🔍 Processing {} hash keys, {} range keys with {} fields",
            transform_result.hash_keys.len(),
            transform_result.range_keys.len(),
            field_names.len()
        );

        Self::process_hashrange_data_structured(
            &schema,
            schema_name,
            &transform_result,
            &field_names,
            message_bus,
        )
    }

    /// Get HashRange schema and validate it exists
    fn get_hashrange_schema(
        db_ops: &Arc<crate::db_operations::DbOperations>,
        schema_name: &str,
    ) -> Result<crate::schema::types::Schema, SchemaError> {
        let schema = db_ops.get_schema(schema_name)?.ok_or_else(|| {
            SchemaError::InvalidData(format!("Schema '{}' not found", schema_name))
        })?;

        if !matches!(
            schema.schema_type,
            crate::schema::types::SchemaType::HashRange
        ) {
            return Err(SchemaError::InvalidData(format!(
                "Schema '{}' is not a HashRange schema",
                schema_name
            )));
        }

        Ok(schema)
    }

    /// Extract field names from schema, excluding special HashRange fields
    fn extract_field_names(schema: &crate::schema::types::Schema) -> Vec<String> {
        schema
            .fields
            .keys()
            .filter(|field_name| *field_name != "hash_key" && *field_name != "range_key")
            .cloned()
            .collect()
    }

    /// Parse transform result into structured HashRangeTransformResult
    fn parse_transform_result_structured(
        result: &JsonValue,
        field_names: &[String],
    ) -> Result<HashRangeTransformResult, SchemaError> {
        let result_obj = result.as_object().ok_or_else(|| {
            SchemaError::InvalidData(format!(
                "HashRange transform result must be a JSON object, got: {}",
                result
            ))
        })?;


        // Extract hash_key and range_key (handle both string and array formats)
        let hash_keys = Self::extract_string_array(result_obj, "hash_key")?;
        let range_keys = Self::extract_string_array(result_obj, "range_key")?;


        // Extract field data arrays
        let mut field_data = HashMap::new();
        for field_name in field_names {
            if let Some(field_value) = result_obj.get(field_name) {
                match field_value {
                    JsonValue::Array(arr) => {
                        field_data.insert(field_name.clone(), arr.clone());
                    }
                    JsonValue::String(s) => {
                        field_data.insert(field_name.clone(), vec![JsonValue::String(s.clone())]);
                    }
                    JsonValue::Null => {
                        field_data.insert(field_name.clone(), vec![]);
                    }
                    _ => {
                        field_data.insert(field_name.clone(), vec![field_value.clone()]);
                    }
                }
            } else {
                field_data.insert(field_name.clone(), vec![]);
            }
        }

        Ok(HashRangeTransformResult {
            hash_keys,
            range_keys,
            field_data,
        })
    }

    /// Extract string array from result, handling both string and array formats
    fn extract_string_array(result_obj: &serde_json::Map<String, JsonValue>, key_name: &str) -> Result<Vec<String>, SchemaError> {
        match result_obj.get(key_name) {
            Some(value) => {
                match value {
                    JsonValue::Array(arr) => {
                        let strings: Result<Vec<String>, _> = arr.iter()
                            .map(|v| v.as_str().map(|s| s.to_string()).ok_or_else(|| {
                                SchemaError::InvalidData(format!("{} array element must be string", key_name))
                            }))
                            .collect();
                        strings
                    }
                    JsonValue::String(s) => {
                        Ok(vec![s.clone()])
                    }
                    JsonValue::Null => {
                        Ok(vec![])
                    }
                    _ => {
                        Ok(vec![value.to_string()])
                    }
                }
            }
            None => {
                Ok(vec![])
            }
        }
    }


    /// Process HashRange data using structured format and submit mutations
    fn process_hashrange_data_structured(
        schema: &crate::schema::types::Schema,
        schema_name: &str,
        transform_result: &HashRangeTransformResult,
        field_names: &[String],
        message_bus: &Arc<crate::fold_db_core::infrastructure::MessageBus>,
    ) -> Result<(), SchemaError> {
        info!(
            "🚀 Processing HashRange data with {} hash keys and {} range keys",
            transform_result.hash_keys.len(),
            transform_result.range_keys.len()
        );
        

        let _content_field_name = Self::find_content_field(field_names);

        // Handle the case where hash_key array contains all words from all content
        // and range_key array contains the publish dates
        if transform_result.hash_keys.len() != transform_result.range_keys.len() {
            info!(
                "🔧 Detected flattened structure: {} hash keys vs {} range keys",
                transform_result.hash_keys.len(),
                transform_result.range_keys.len()
            );

            // Use the words already extracted by the transform execution engine
            let words_from_transform = transform_result.hash_keys.clone();

            // Process each range entry with all words
            for (range_index, range) in transform_result.range_keys.iter().enumerate() {
                let field_values = Self::extract_field_values_for_entry_structured(
                    transform_result,
                    field_names,
                    range_index,
                );

                info!(
                    "🔍 Processing range entry {}: using {} words from transform",
                    range_index,
                    words_from_transform.len()
                );

                
                Self::submit_word_mutations_structured(
                    schema,
                    schema_name,
                    &words_from_transform,
                    &field_values,
                    range,
                    message_bus,
                )?;
            }
        } else {
            // Original logic for matching array lengths
            for data_index in 0..transform_result.hash_keys.len() {
                let field_values = Self::extract_field_values_for_entry_structured(
                    transform_result,
                    field_names,
                    data_index,
                );
                let range = transform_result.range_keys[data_index].clone();

                // Use the word from the hash_key array (already extracted by transform execution engine)
                let word_from_transform = transform_result.hash_keys[data_index].clone();

                debug!(
                    "🔍 Processing data entry {}: using word '{}' from transform",
                    data_index, word_from_transform
                );

                if !word_from_transform.is_empty() {
                    Self::submit_word_mutations_structured(
                        schema,
                        schema_name,
                        &[word_from_transform],
                        &field_values,
                        &range,
                        message_bus,
                    )?;
                }
            }
        }

        Ok(())
    }


    /// Find the content field for word extraction
    fn find_content_field(field_names: &[String]) -> String {
        // Look for common content field names
        let content_indicators = ["content", "text", "body", "description", "title"];

        field_names
            .iter()
            .find(|name| {
                let lower_name = name.to_lowercase();
                content_indicators
                    .iter()
                    .any(|indicator| lower_name.contains(indicator))
            })
            .cloned()
            .unwrap_or_else(|| field_names.first().cloned().unwrap_or_default())
    }


    /// Extract field values for a specific entry using structured format
    fn extract_field_values_for_entry_structured(
        transform_result: &HashRangeTransformResult,
        field_names: &[String],
        data_index: usize,
    ) -> HashMap<String, JsonValue> {
        let mut field_values: HashMap<String, JsonValue> = HashMap::new();

        for field_name in field_names {
            let field_value = transform_result
                .field_data
                .get(field_name)
                .and_then(|arr| arr.get(data_index))
                .cloned()
                .unwrap_or(JsonValue::Null);
            field_values.insert(field_name.clone(), field_value);
        }

        field_values
    }

    /// Submit mutations for all words in a data entry using structured format
    fn submit_word_mutations_structured(
        schema: &crate::schema::types::Schema,
        schema_name: &str,
        words: &[String],
        field_values: &HashMap<String, JsonValue>,
        range: &str,
        message_bus: &Arc<crate::fold_db_core::infrastructure::MessageBus>,
    ) -> Result<(), SchemaError> {
        
        for word in words {
            let mutation = Self::create_hashrange_mutation(schema_name, word, field_values, &JsonValue::String(range.to_string()))?;
            Self::submit_mutation_through_message_bus(schema, schema_name, &mutation, message_bus)?;
        }
        Ok(())
    }


    /// Create a HashRange mutation for a specific word
    fn create_hashrange_mutation(
        schema_name: &str,
        word: &str,
        field_values: &HashMap<String, JsonValue>,
        range: &JsonValue,
    ) -> Result<Mutation, SchemaError> {
        let mut fields_and_values = HashMap::new();

        // Add hash_key and range_key
        fields_and_values.insert("hash_key".to_string(), json!(word));
        fields_and_values.insert("range_key".to_string(), range.clone());

        // Add all field values
        for (field_name, field_value) in field_values {
            fields_and_values.insert(field_name.clone(), field_value.clone());
        }

        Ok(Mutation::new(
            schema_name.to_string(),
            fields_and_values,
            TRANSFORM_SYSTEM_ID.to_string(),
            0, // trust_distance
            MutationType::Create,
        ))
    }

    /// Submit a mutation through the message bus
    fn submit_mutation_through_message_bus(
        schema: &crate::schema::types::Schema,
        schema_name: &str,
        mutation: &Mutation,
        message_bus: &Arc<crate::fold_db_core::infrastructure::MessageBus>,
    ) -> Result<(), SchemaError> {
        let hash_key = mutation
            .fields_and_values
            .get("hash_key")
            .and_then(|v| v.as_str())
            .ok_or_else(|| {
                SchemaError::InvalidData("HashRange mutation missing hash_key".to_string())
            })?;

        let _range_key = mutation
            .fields_and_values
            .get("range_key")
            .and_then(|v| v.as_str())
            .ok_or_else(|| {
                SchemaError::InvalidData("HashRange mutation missing range_key".to_string())
            })?;

        let hash_value = mutation.fields_and_values.get("hash_key").cloned();
        let range_value = mutation.fields_and_values.get("range_key").cloned();
        let mutation_service = MutationService::new(Arc::clone(message_bus));

        // Submit each field value through the message bus
        for (field_name, value) in &mutation.fields_and_values {
            if field_name != "hash_key" && field_name != "range_key" {
                debug!(
                    "🔧 Submitting field '{}' for word '{}' through message bus",
                    field_name, hash_key
                );

                let normalized_request = mutation_service.normalized_field_value_request(
                    schema,
                    field_name,
                    value,
                    hash_value.as_ref(),
                    range_value.as_ref(),
                    None,
                )?;

                let mut request = normalized_request.request;
                let context = normalized_request.context;
                let hash_state = context.hash.as_deref().unwrap_or("∅");
                let range_state = context.range.as_deref().unwrap_or("∅");

                request.source_pub_key = TRANSFORM_SYSTEM_ID.to_string();
                let correlation_id = request.correlation_id.clone();

                message_bus.publish(request).map_err(|e| {
                    SchemaError::InvalidData(format!(
                        "Failed to submit field value for {}.{}: {}",
                        schema_name, field_name, e
                    ))
                })?;

                debug!(
                    "✅ HashRange field value submitted successfully for {}.{} with word '{}' [correlation_id: {}, hash: {}, range: {}]",
                    schema_name,
                    field_name,
                    hash_key,
                    correlation_id,
                    hash_state,
                    range_state
                );
            }
        }

        Ok(())
    }
}
