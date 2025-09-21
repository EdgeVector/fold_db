use crate::schema::constants::TRANSFORM_SYSTEM_ID;
use crate::schema::types::{Mutation, MutationType, SchemaError};
use log::{debug, info};
use serde_json::{json, Value as JsonValue};
use std::collections::HashMap;
use std::sync::Arc;
use uuid;

/// Data structure for HashRange transform results
#[derive(Debug, Clone)]
pub struct HashRangeTransformData {
    pub hash_keys: Vec<JsonValue>,
    pub range_keys: Vec<JsonValue>,
    pub field_arrays: HashMap<String, Vec<JsonValue>>,
    pub data_entries: Vec<JsonValue>, // Currently unused but kept for future extensibility
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
        let transform_data = Self::parse_transform_result(result, &field_names)?;

        debug!(
            "🔍 Processing {} data entries with {} fields",
            transform_data.data_entries.len(),
            field_names.len()
        );

        Self::process_hashrange_data(schema_name, &transform_data, &field_names, message_bus)
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

    /// Parse and validate transform result data structure
    fn parse_transform_result(
        result: &JsonValue,
        field_names: &[String],
    ) -> Result<HashRangeTransformData, SchemaError> {
        let result_obj = result.as_object().ok_or_else(|| {
            SchemaError::InvalidData(format!(
                "HashRange transform result must be an object, got: {}",
                result
            ))
        })?;

        debug!(
            "🔍 HashRange transform result structure: {} keys",
            result_obj.len()
        );

        // Extract hash_key and range_key arrays
        let hash_keys = result_obj
            .get("hash_key")
            .and_then(|h| h.as_array())
            .ok_or_else(|| {
                SchemaError::InvalidData("HashRange result must contain hash_key array".to_string())
            })?
            .clone();

        let range_keys = result_obj
            .get("range_key")
            .and_then(|r| r.as_array())
            .ok_or_else(|| {
                SchemaError::InvalidData(
                    "HashRange result must contain range_key array".to_string(),
                )
            })?
            .clone();

        // Note: Array lengths may not match in flattened structures
        // This will be handled in process_hashrange_data
        debug!(
            "🔍 HashRange result: {} hash keys, {} range keys",
            hash_keys.len(),
            range_keys.len()
        );

        // Extract field values dynamically
        let mut field_arrays: HashMap<String, Vec<JsonValue>> = HashMap::new();

        for field_name in field_names {
            let field_values = result_obj
                .get(field_name)
                .map(|f| {
                    if f.is_array() {
                        f.clone()
                    } else {
                        json!([f.clone()])
                    }
                })
                .unwrap_or_default();

            let field_array = field_values.as_array().cloned().unwrap_or_default();
            field_arrays.insert(field_name.clone(), field_array);
        }

        // Note: Field array lengths may not match hash_key length in flattened structures
        // This will be handled in process_hashrange_data
        debug!(
            "🔍 Field arrays: {:?}",
            field_arrays.keys().collect::<Vec<_>>()
        );

        Ok(HashRangeTransformData {
            hash_keys,
            range_keys,
            field_arrays,
            data_entries: Vec::new(), // Will be populated by process_hashrange_data
        })
    }

    /// Process HashRange data and submit through message bus
    fn process_hashrange_data(
        schema_name: &str,
        transform_data: &HashRangeTransformData,
        field_names: &[String],
        message_bus: &Arc<crate::fold_db_core::infrastructure::MessageBus>,
    ) -> Result<(), SchemaError> {
        info!(
            "🚀 Processing HashRange data with {} hash keys and {} range keys",
            transform_data.hash_keys.len(),
            transform_data.range_keys.len()
        );

        let _content_field_name = Self::find_content_field(field_names);

        // Handle the case where hash_key array contains all words from all content
        // and range_key array contains the publish dates
        if transform_data.hash_keys.len() != transform_data.range_keys.len() {
            info!(
                "🔧 Detected flattened structure: {} hash keys vs {} range keys",
                transform_data.hash_keys.len(),
                transform_data.range_keys.len()
            );

            // Use the words already extracted by the transform execution engine
            let words_from_transform = transform_data
                .hash_keys
                .iter()
                .filter_map(|key| key.as_str())
                .map(|s| s.to_string())
                .collect::<Vec<String>>();

            info!(
                "🔍 Using {} words from transform execution engine: {:?}",
                words_from_transform.len(),
                words_from_transform
            );

            // Process each range key (content entry) and create word mutations
            for range_index in 0..transform_data.range_keys.len() {
                let field_values =
                    Self::extract_field_values_for_entry(transform_data, field_names, range_index);
                let range = transform_data.range_keys[range_index].clone();

                debug!(
                    "🔍 Processing range entry {}: using {} words from transform",
                    range_index,
                    words_from_transform.len()
                );

                Self::submit_word_mutations(
                    schema_name,
                    &words_from_transform,
                    &field_values,
                    &range,
                    message_bus,
                )?;
            }
        } else {
            // Original logic for matching array lengths
            for data_index in 0..transform_data.hash_keys.len() {
                let field_values =
                    Self::extract_field_values_for_entry(transform_data, field_names, data_index);
                let range = transform_data.range_keys[data_index].clone();

                // Use the word from the hash_key array (already extracted by transform execution engine)
                let word_from_transform = transform_data.hash_keys[data_index]
                    .as_str()
                    .map(|s| s.to_string())
                    .unwrap_or_default();

                debug!(
                    "🔍 Processing data entry {}: using word '{}' from transform",
                    data_index, word_from_transform
                );

                if !word_from_transform.is_empty() {
                    Self::submit_word_mutations(
                        schema_name,
                        &[word_from_transform],
                        &field_values,
                        &range,
                        message_bus,
                    )?;
                }
            }
        }

        info!("🎯 All HashRange field values submitted successfully through message bus");
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

    /// Extract field values for a specific data entry
    fn extract_field_values_for_entry(
        transform_data: &HashRangeTransformData,
        field_names: &[String],
        data_index: usize,
    ) -> HashMap<String, JsonValue> {
        let mut field_values: HashMap<String, JsonValue> = HashMap::new();

        for field_name in field_names {
            let field_value = transform_data
                .field_arrays
                .get(field_name)
                .and_then(|arr| arr.get(data_index))
                .cloned()
                .unwrap_or(JsonValue::Null);
            field_values.insert(field_name.clone(), field_value);
        }

        field_values
    }

    /// Submit mutations for all words in a data entry
    fn submit_word_mutations(
        schema_name: &str,
        words: &[String],
        field_values: &HashMap<String, JsonValue>,
        range: &JsonValue,
        message_bus: &Arc<crate::fold_db_core::infrastructure::MessageBus>,
    ) -> Result<(), SchemaError> {
        for word in words {
            let mutation = Self::create_hashrange_mutation(schema_name, word, field_values, range)?;
            Self::submit_mutation_through_message_bus(schema_name, &mutation, message_bus)?;
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

        let range_key = mutation
            .fields_and_values
            .get("range_key")
            .and_then(|v| v.as_str())
            .ok_or_else(|| {
                SchemaError::InvalidData("HashRange mutation missing range_key".to_string())
            })?;

        // Submit each field value through the message bus
        for (field_name, value) in &mutation.fields_and_values {
            if field_name != "hash_key" && field_name != "range_key" {
                debug!(
                    "🔧 Submitting field '{}' for word '{}' through message bus",
                    field_name, hash_key
                );

                let hashrange_aware_value = serde_json::json!({
                    "hash_key": hash_key,
                    "range_key": range_key,
                    "value": value
                });

                let correlation_id = uuid::Uuid::new_v4().to_string();
                let field_value_request = crate::fold_db_core::infrastructure::message_bus::request_events::FieldValueSetRequest::new(
                    correlation_id,
                    schema_name.to_string(),
                    field_name.clone(),
                    hashrange_aware_value,
                    TRANSFORM_SYSTEM_ID.to_string(),
                );

                message_bus.publish(field_value_request).map_err(|e| {
                    SchemaError::InvalidData(format!(
                        "Failed to submit field value for {}.{}: {}",
                        schema_name, field_name, e
                    ))
                })?;

                debug!(
                    "✅ HashRange field value submitted successfully for {}.{} with word '{}'",
                    schema_name, field_name, hash_key
                );
            }
        }

        Ok(())
    }
}
