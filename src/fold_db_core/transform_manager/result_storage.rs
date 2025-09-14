use crate::schema::types::{SchemaError, Transform};
use crate::schema::constants::TRANSFORM_SYSTEM_ID;
use log::{info, warn};
use std::sync::Arc;
use serde_json::{Value as JsonValue};
use uuid;

/// Handles storing transform results
pub struct ResultStorage;

impl ResultStorage {
    /// Generic result storage for any transform using mutations
    pub fn store_transform_result_generic(
        db_ops: &Arc<crate::db_operations::DbOperations>, 
        transform: &Transform, 
        result: &JsonValue, 
        message_bus: Option<&Arc<crate::fold_db_core::infrastructure::MessageBus>>
    ) -> Result<(), SchemaError> {
        if let Some(dot_pos) = transform.get_output().find('.') {
            let schema_name = &transform.get_output()[..dot_pos];
            let field_name = &transform.get_output()[dot_pos + 1..];
            
            // Check if this is a HashRange schema and handle it specially
            if Self::is_hashrange_schema(db_ops, schema_name)? {
                return Self::handle_hashrange_storage(db_ops, schema_name, result, message_bus);
            }
            
            // For non-HashRange schemas, submit through message bus if available
            Self::handle_regular_storage(schema_name, field_name, result, message_bus)
        } else {
            Err(SchemaError::InvalidField(format!("Invalid output field format '{}', expected 'Schema.field'", transform.get_output())))
        }
    }

    /// Check if a schema is a HashRange schema
    fn is_hashrange_schema(
        db_ops: &Arc<crate::db_operations::DbOperations>, 
        schema_name: &str
    ) -> Result<bool, SchemaError> {
        if let Ok(Some(schema)) = db_ops.get_schema(schema_name) {
            Ok(matches!(schema.schema_type, crate::schema::types::SchemaType::HashRange))
        } else {
            Ok(false)
        }
    }

    /// Handle HashRange schema storage
    fn handle_hashrange_storage(
        db_ops: &Arc<crate::db_operations::DbOperations>, 
        schema_name: &str, 
        result: &JsonValue, 
        message_bus: Option<&Arc<crate::fold_db_core::infrastructure::MessageBus>>
    ) -> Result<(), SchemaError> {
        info!("🔑 Storing HashRange transform result for schema '{}' using message bus", schema_name);
        
        if let Some(message_bus) = message_bus {
            crate::fold_db_core::transform_manager::hashrange_processor::HashRangeProcessor::store_hashrange_transform_result_with_message_bus(db_ops, schema_name, result, message_bus)
        } else {
            warn!("⚠️ Message bus not available for HashRange transform result storage");
            Err(SchemaError::InvalidData("Message bus not available for HashRange transform result storage".to_string()))
        }
    }

    /// Handle regular (non-HashRange) schema storage
    fn handle_regular_storage(
        schema_name: &str,
        field_name: &str,
        result: &JsonValue,
        message_bus: Option<&Arc<crate::fold_db_core::infrastructure::MessageBus>>
    ) -> Result<(), SchemaError> {
        if let Some(message_bus) = message_bus {
            info!("📝 Submitting field value for {}.{} through message bus", schema_name, field_name);
            
            // Create FieldValueSetRequest and publish through message bus
            let correlation_id = uuid::Uuid::new_v4().to_string();
            let field_value_request = crate::fold_db_core::infrastructure::message_bus::request_events::FieldValueSetRequest::new(
                correlation_id.clone(),
                schema_name.to_string(),
                field_name.to_string(),
                result.clone(),
                TRANSFORM_SYSTEM_ID.to_string(),
            );
            
            if let Err(e) = message_bus.publish(field_value_request) {
                warn!("⚠️ Failed to publish FieldValueSetRequest for {}.{}: {}", schema_name, field_name, e);
                return Err(SchemaError::InvalidData(format!("Failed to submit field value: {}", e)));
            }
            
            info!("✅ FieldValueSetRequest submitted successfully for {}.{} with correlation_id: {}", schema_name, field_name, correlation_id);
            Ok(())
        } else {
            warn!("⚠️ Message bus not available, cannot submit field value for {}.{}", schema_name, field_name);
            Err(SchemaError::InvalidData("Message bus not available for field value submission".to_string()))
        }
    }
}
