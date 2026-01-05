use crate::fold_db_core::infrastructure::message_bus::events::MutationRequest;
use crate::fold_db_core::infrastructure::message_bus::{AsyncMessageBus, Event};
use crate::schema::constants::TRANSFORM_SYSTEM_ID;
use crate::schema::types::key_value::KeyValue;
use crate::schema::types::operations::MutationType;
use crate::schema::types::{Mutation, SchemaError, Transform};
use log::warn;
use serde_json::Value as JsonValue;
use std::collections::HashMap;
use std::sync::Arc;
use uuid::Uuid;

/// Handles storing transform results
pub struct ResultStorage;

impl ResultStorage {
    /// Generic result storage for any transform using mutations
    pub async fn store_transform_result_generic(
        transform: &Transform,
        db_ops: &Arc<crate::db_operations::DbOperations>,
        code_hash_to_result: HashMap<String, JsonValue>,
        key_value: KeyValue,
        message_bus: Option<&Arc<AsyncMessageBus>>,
        backfill_hash: Option<String>,
    ) -> Result<(), SchemaError> {
        // Look up the transform's schema from the database
        let transform_schema = db_ops
            .get_schema(transform.get_schema_name())
            .await?
            .ok_or_else(|| {
                SchemaError::InvalidData(format!(
                    "Transform schema '{}' not found",
                    transform.get_schema_name()
                ))
            })?;

        // Create reverse mapping from hash code to field name
        let field_to_hash_code = transform_schema.get_field_to_hash_code();
        let hash_code_to_field: HashMap<String, String> = field_to_hash_code
            .iter()
            .map(|(field_name, hash_code)| (hash_code.clone(), field_name.clone()))
            .collect();

        let mut fields_and_values = HashMap::new();
        for (code_hash, result) in code_hash_to_result {
            if let Some(field_name) = hash_code_to_field.get(&code_hash) {
                fields_and_values.insert(field_name.clone(), result);
            } else {
                warn!("Field mapping not found for code hash: {}", code_hash);
            }
        }

        let mut mutation = Mutation::new(
            transform_schema.name.clone(),
            fields_and_values,
            key_value,
            TRANSFORM_SYSTEM_ID.to_string(),
            0,
            MutationType::Update,
        );

        // Attach backfill_hash if provided
        if let Some(hash) = backfill_hash {
            mutation = mutation.with_backfill_hash(hash.clone());
        }

        if let Some(message_bus) = message_bus {
            let mutation_request = MutationRequest {
                correlation_id: Uuid::new_v4().to_string(),
                mutation,
            };

            message_bus
                .publish_event(Event::MutationRequest(mutation_request))
                .await
                .map_err(|e| {
                    crate::schema::types::SchemaError::InvalidData(format!(
                        "Failed to publish mutation request to message bus: {}",
                        e
                    ))
                })?;
        }

        Ok(())
    }
}
