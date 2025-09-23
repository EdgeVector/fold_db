use crate::fold_db_core::services::mutation::{MutationService, NormalizedFieldValueRequest};
use crate::schema::constants::TRANSFORM_SYSTEM_ID;
use crate::schema::types::{SchemaError, Transform};
use log::{info, warn};
use serde_json::Value as JsonValue;
use std::sync::Arc;

/// Handles storing transform results
pub struct ResultStorage;

impl ResultStorage {
    /// Generic result storage for any transform using mutations
    pub fn store_transform_result_generic(
        db_ops: &Arc<crate::db_operations::DbOperations>,
        transform: &Transform,
        result: &JsonValue,
        message_bus: Option<&Arc<crate::fold_db_core::infrastructure::MessageBus>>,
    ) -> Result<(), SchemaError> {
        if let Some(dot_pos) = transform.get_output().find('.') {
            let schema_name = &transform.get_output()[..dot_pos];
            let field_name = &transform.get_output()[dot_pos + 1..];

            // Check if this is a HashRange schema and handle it specially
            if Self::is_hashrange_schema(db_ops, schema_name)? {
                return Self::handle_hashrange_storage(db_ops, schema_name, result, message_bus);
            }

            // For non-HashRange schemas, submit through message bus if available
            Self::handle_regular_storage(db_ops, schema_name, field_name, result, message_bus)
        } else {
            Err(SchemaError::InvalidField(format!(
                "Invalid output field format '{}', expected 'Schema.field'",
                transform.get_output()
            )))
        }
    }

    /// Check if a schema is a HashRange schema
    fn is_hashrange_schema(
        db_ops: &Arc<crate::db_operations::DbOperations>,
        schema_name: &str,
    ) -> Result<bool, SchemaError> {
        if let Ok(Some(schema)) = db_ops.get_schema(schema_name) {
            Ok(matches!(
                schema.schema_type,
                crate::schema::types::SchemaType::HashRange
            ))
        } else {
            Ok(false)
        }
    }

    /// Handle HashRange schema storage
    fn handle_hashrange_storage(
        db_ops: &Arc<crate::db_operations::DbOperations>,
        schema_name: &str,
        result: &JsonValue,
        message_bus: Option<&Arc<crate::fold_db_core::infrastructure::MessageBus>>,
    ) -> Result<(), SchemaError> {
        info!(
            "🔑 Storing HashRange transform result for schema '{}' using message bus",
            schema_name
        );

        if let Some(message_bus) = message_bus {
            crate::fold_db_core::transform_manager::hashrange_processor::HashRangeProcessor::store_hashrange_transform_result_with_message_bus(db_ops, schema_name, result, message_bus)
        } else {
            warn!("⚠️ Message bus not available for HashRange transform result storage");
            Err(SchemaError::InvalidData(
                "Message bus not available for HashRange transform result storage".to_string(),
            ))
        }
    }

    /// Handle regular (non-HashRange) schema storage
    fn handle_regular_storage(
        db_ops: &Arc<crate::db_operations::DbOperations>,
        schema_name: &str,
        field_name: &str,
        result: &JsonValue,
        message_bus: Option<&Arc<crate::fold_db_core::infrastructure::MessageBus>>,
    ) -> Result<(), SchemaError> {
        if let Some(message_bus) = message_bus {
            info!(
                "📝 Submitting field value for {}.{} through message bus",
                schema_name, field_name
            );

            let schema = db_ops.get_schema(schema_name)?.ok_or_else(|| {
                SchemaError::InvalidData(format!("Schema '{}' not found", schema_name))
            })?;

            let mutation_service = MutationService::new(Arc::clone(message_bus));
            let (field_value, hash_value, range_value) =
                Self::extract_field_value_and_keys(schema_name, field_name, result);

            let NormalizedFieldValueRequest {
                mut request,
                context,
            } = mutation_service.normalized_field_value_request(
                &schema,
                field_name,
                &field_value,
                hash_value.as_ref(),
                range_value.as_ref(),
                None,
            )?;

            request.source_pub_key = TRANSFORM_SYSTEM_ID.to_string();
            let correlation_id = request.correlation_id.clone();

            if let Err(e) = message_bus.publish(request) {
                warn!(
                    "⚠️ Failed to publish FieldValueSetRequest for {}.{}: {}",
                    schema_name, field_name, e
                );
                return Err(SchemaError::InvalidData(format!(
                    "Failed to submit field value: {}",
                    e
                )));
            }

            let hash_state = context.hash.as_deref().unwrap_or("∅");
            let range_state = context.range.as_deref().unwrap_or("∅");

            info!(
                "✅ FieldValueSetRequest submitted successfully for {}.{} with correlation_id: {} (hash: {}, range: {})",
                schema_name,
                field_name,
                correlation_id,
                hash_state,
                range_state
            );

            // Fire DataPersisted event for the output schema after mutation is submitted
            let data_persisted = crate::fold_db_core::infrastructure::message_bus::events::schema_events::DataPersisted::with_transform(
                schema_name.to_string(),
                correlation_id,
                "TransformResult".to_string(),
            );
            
            if let Err(e) = message_bus.publish(data_persisted) {
                warn!("⚠️ Failed to publish DataPersisted event for schema '{}': {}", schema_name, e);
            } else {
                info!("📊 DataPersisted event fired for schema '{}' after transform result storage", schema_name);
            }
            
            Ok(())
        } else {
            warn!(
                "⚠️ Message bus not available, cannot submit field value for {}.{}",
                schema_name, field_name
            );
            Err(SchemaError::InvalidData(
                "Message bus not available for field value submission".to_string(),
            ))
        }
    }

    fn extract_field_value_and_keys(
        schema_name: &str,
        field_name: &str,
        result: &JsonValue,
    ) -> (JsonValue, Option<JsonValue>, Option<JsonValue>) {
        if let Some(obj) = result.as_object() {
            let hash_value = obj
                .get("hash")
                .cloned()
                .or_else(|| obj.get("hash_key").cloned());
            let range_value = obj
                .get("range")
                .cloned()
                .or_else(|| obj.get("range_key").cloned());

            if let Some(fields_obj) = obj.get("fields").and_then(|value| value.as_object()) {
                let field_value = fields_obj.get(field_name).cloned().unwrap_or_else(|| {
                    warn!(
                        "⚠️ Normalized payload for {}.{} missing field value in 'fields' map",
                        schema_name, field_name
                    );
                    JsonValue::Null
                });
                (field_value, hash_value, range_value)
            } else {
                let field_value = obj.get(field_name).cloned().unwrap_or_else(|| {
                    warn!(
                        "⚠️ Transform result for {}.{} missing direct field value",
                        schema_name, field_name
                    );
                    JsonValue::Null
                });
                (field_value, hash_value, range_value)
            }
        } else {
            (result.clone(), None, None)
        }
    }
}
