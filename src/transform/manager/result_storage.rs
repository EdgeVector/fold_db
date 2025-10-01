use crate::schema::constants::TRANSFORM_SYSTEM_ID;
use crate::schema::types::{Mutation, SchemaError, Transform};
use crate::fold_db_core::infrastructure::message_bus::events::MutationRequest;
use serde_json::Value as JsonValue;
use std::collections::HashMap;
use std::sync::Arc;
use crate::schema::types::operations::MutationType;
use crate::schema::types::key_value::KeyValue;
use uuid::Uuid;

/// Handles storing transform results
pub struct ResultStorage;

impl ResultStorage {
    /// Generic result storage for any transform using mutations
    pub fn store_transform_result_generic(
        transform: &Transform,
        code_hash_to_result: HashMap<String, JsonValue>,
        key_value: KeyValue,
        message_bus: Option<&Arc<crate::fold_db_core::infrastructure::MessageBus>>,
    ) -> Result<(), SchemaError> {
        // TODO: Map Transform's declarative schema's field_to_hash_code to the result of the execution.
        let field_to_hash_code = transform.get_declarative_schema().unwrap().get_field_to_hash_code();

        let mut fields_and_values = HashMap::new();
        for (code_hash, result) in code_hash_to_result {
            let field_name = field_to_hash_code.get(&code_hash).unwrap().clone();
            fields_and_values.insert(field_name.clone(), result);
        }

        let mutation = Mutation::new(
            transform.get_declarative_schema().unwrap().name.clone(),
            fields_and_values,
            key_value,
            TRANSFORM_SYSTEM_ID.to_string(),
            0,
            MutationType::Update,
        );

        if let Some(message_bus) = message_bus {
            let mutation_request = MutationRequest {
                correlation_id: Uuid::new_v4().to_string(),
                mutation,
            };
            message_bus.publish(mutation_request)
                .map_err(|e| crate::schema::types::SchemaError::InvalidData(format!("Failed to publish mutation request to message bus: {}", e)))?;
        }

        Ok(())
    }
}