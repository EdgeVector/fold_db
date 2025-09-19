//! Field Retrieval Service Coordinator
//!
//! Provides a unified interface for field value retrieval by delegating to
//! appropriate specialized retrievers based on field type. This replaces the
//! complex branching logic in FieldManager.

use crate::db_operations::DbOperations;
use crate::fold_db_core::infrastructure::message_bus::{
    request_events::FieldValueQueryRequest, MessageBus,
};
use crate::schema::types::field::FieldVariant;
use crate::schema::Schema;
use crate::schema::SchemaError;
use log::info;
use serde_json::Value;
use std::collections::HashMap;
use std::sync::Arc;
use uuid::Uuid;

pub struct FieldRetrievalService {
    message_bus: Arc<MessageBus>,
    db_ops: Option<Arc<DbOperations>>,
}

impl FieldRetrievalService {
    pub fn new(message_bus: Arc<MessageBus>) -> Self {
        Self {
            message_bus,
            db_ops: None,
        }
    }

    pub fn new_with_db_ops(message_bus: Arc<MessageBus>, db_ops: Arc<DbOperations>) -> Self {
        Self {
            message_bus,
            db_ops: Some(db_ops),
        }
    }

    pub fn new_default() -> Self {
        Self {
            message_bus: crate::fold_db_core::infrastructure::factory::InfrastructureFactory::create_message_bus(),
            db_ops: None,
        }
    }

    /// Retrieves a field value without filtering using unified FieldValueResolver
    pub fn get_field_value(&self, schema: &Schema, field: &str) -> Result<Value, SchemaError> {
        info!(
            "🔍 FieldRetrievalService::get_field_value - schema: {}, field: {} (UNIFIED)",
            schema.name, field
        );

        // Use the unified FieldValueResolver instead of event-driven placeholder
        // This ensures consistent field resolution across the application
        match &self.db_ops {
            Some(db_ops) => {
                crate::fold_db_core::transform_manager::utils::TransformUtils::resolve_field_value(
                    db_ops, schema, field, None, None,
                )
            }
            None => {
                // Fallback to event-driven approach if no db_ops available
                let correlation_id = Uuid::new_v4().to_string();
                let query_request = FieldValueQueryRequest {
                    correlation_id: correlation_id.clone(),
                    schema_name: schema.name.clone(),
                    field_name: field.to_string(),
                    filter: None,
                };

                match self.message_bus.publish(query_request) {
                    Ok(_) => {
                        info!(
                            "✅ FieldValueQueryRequest sent successfully for {}.{}",
                            schema.name, field
                        );
                        Ok(Value::String(format!(
                            "EVENT_DRIVEN_PLACEHOLDER_{}_{}",
                            schema.name, field
                        )))
                    }
                    Err(e) => {
                        let error_msg = format!(
                            "Failed to send FieldValueQueryRequest for {}.{}: {:?}",
                            schema.name, field, e
                        );
                        info!("❌ {}", error_msg);
                        Err(SchemaError::InvalidField(error_msg))
                    }
                }
            }
        }
    }

    /// Retrieves a field value with optional filtering using event-driven communication
    pub fn get_field_value_with_filter(
        &self,
        schema: &Schema,
        field: &str,
        filter: &Value,
    ) -> Result<Value, SchemaError> {
        info!(
            "🔄 FieldRetrievalService::get_field_value_with_filter - schema: {}, field: {} (EVENT-DRIVEN)",
            schema.name, field
        );

        // Send FieldValueQueryRequest with filter via message bus
        let correlation_id = Uuid::new_v4().to_string();
        let query_request = FieldValueQueryRequest {
            correlation_id: correlation_id.clone(),
            schema_name: schema.name.clone(),
            field_name: field.to_string(),
            filter: Some(filter.clone()),
        };

        match self.message_bus.publish(query_request) {
            Ok(_) => {
                info!(
                    "✅ FieldValueQueryRequest with filter sent successfully for {}.{}",
                    schema.name, field
                );
                // For now, return a placeholder - in a real event-driven system, this would wait for response
                Ok(Value::String(format!(
                    "EVENT_DRIVEN_FILTERED_PLACEHOLDER_{}_{}",
                    schema.name, field
                )))
            }
            Err(e) => {
                let error_msg = format!(
                    "Failed to send filtered FieldValueQueryRequest for {}.{}: {:?}",
                    schema.name, field, e
                );
                info!("❌ {}", error_msg);
                Err(SchemaError::InvalidField(error_msg))
            }
        }
    }

    /// Checks if a field supports filtering
    pub fn supports_filtering(&self, schema: &Schema, field: &str) -> Result<bool, SchemaError> {
        let field_def = schema
            .fields
            .get(field)
            .ok_or_else(|| SchemaError::InvalidField(format!("Field {} not found", field)))?;

        let supports = match field_def {
            FieldVariant::Single(_) => false,
            FieldVariant::Range(_) => true,
            FieldVariant::HashRange(_) => true, // HashRange fields support range queries
        };

        Ok(supports)
    }

    /// Query Range schema and group results by range_key value
    /// Returns map of range_key -> field_name -> field_value
    pub fn query_range_schema(
        &self,
        schema: &Schema,
        fields: &[String],
        range_filter: &Value,
    ) -> Result<HashMap<String, HashMap<String, Value>>, SchemaError> {
        info!(
            "🎯 FieldRetrievalService::query_range_schema - schema: {}, fields: {:?}",
            schema.name, fields
        );

        // Validate this is a Range schema
        let range_key = schema.range_key().ok_or_else(|| {
            SchemaError::InvalidData(format!("Schema '{}' is not a Range schema", schema.name))
        })?;

        // Extract range_filter object
        let range_filter_obj = range_filter.as_object().ok_or_else(|| {
            SchemaError::InvalidData("range_filter must be an object".to_string())
        })?;

        // Get the range_key value from the filter
        let range_key_value = range_filter_obj.get(range_key).ok_or_else(|| {
            SchemaError::InvalidData(format!("range_filter missing key '{}'", range_key))
        })?;

        info!(
            "🔍 Range key '{}' filtering for value: {:?}",
            range_key, range_key_value
        );

        // Extract the actual key value from the filter specification
        // e.g., {"Key": "abc"} -> "abc", {"KeyPrefix": "abc"} -> "abc", etc.
        let range_key_str = if let Some(obj) = range_key_value.as_object() {
            if let Some(key_value) = obj.get("Key") {
                key_value.as_str().unwrap_or("").to_string()
            } else if let Some(prefix_value) = obj.get("KeyPrefix") {
                prefix_value.as_str().unwrap_or("").to_string()
            } else if let Some(pattern_value) = obj.get("KeyPattern") {
                pattern_value.as_str().unwrap_or("").to_string()
            } else {
                // For other filter types or fallback, try to extract any string value
                obj.values()
                    .find_map(|v| v.as_str())
                    .unwrap_or("")
                    .to_string()
            }
        } else {
            // If not an object, convert directly to string
            range_key_value.to_string().trim_matches('"').to_string()
        };

        info!(
            "🔍 Extracted range key string for lookup: '{}'",
            range_key_str
        );

        let mut result: HashMap<String, HashMap<String, Value>> = HashMap::new();

        // For each requested field, get its value with the range filter
        for field_name in fields {
            info!("🔄 Processing field: {}", field_name);

            // Validate field exists in schema
            if !schema.fields.contains_key(field_name) {
                return Err(SchemaError::InvalidField(format!(
                    "Field '{}' not found in schema '{}'",
                    field_name, schema.name
                )));
            }

            // Wrap the range filter in the expected format for individual field processing
            let wrapped_filter = serde_json::json!({
                "range_filter": range_filter
            });

            // Get field value with the wrapped range filter
            match self.get_field_value_with_filter(schema, field_name, &wrapped_filter) {
                Ok(field_value) => {
                    // For range fields, extract the actual content from the filtered result
                    let actual_content = if let Some(matches) = field_value.get("matches") {
                        if let Some(range_content) = matches.get(&range_key_str) {
                            range_content.clone()
                        } else {
                            // No content found for this range key, skip adding to result
                            info!(
                                "⚠️  No content found for range key '{}' in field '{}'",
                                range_key_str, field_name
                            );
                            continue;
                        }
                    } else {
                        field_value
                    };

                    // Only create range_key entry if we have actual content
                    let range_entry = result.entry(range_key_str.clone()).or_default();
                    range_entry.insert(field_name.clone(), actual_content);
                    info!(
                        "✅ Added field '{}' to range key '{}'",
                        field_name, range_key_str
                    );
                }
                Err(e) => {
                    info!("⚠️ Failed to get field '{}': {:?}", field_name, e);
                    return Err(e);
                }
            }
        }

        info!(
            "✅ FieldRetrievalService::query_range_schema COMPLETE - schema: {}, result keys: {:?}",
            schema.name,
            result.keys().collect::<Vec<_>>()
        );

        Ok(result)
    }
}
