use crate::schema::constants::TRANSFORM_SYSTEM_ID;
use crate::schema::types::{Mutation, SchemaError, Transform};
use crate::fold_db_core::infrastructure::message_bus::events::MutationRequest;
use serde_json::Value as JsonValue;
use std::collections::HashMap;
use std::sync::Arc;
use crate::schema::types::MutationType;
use uuid::Uuid;

/// Handles storing transform results
pub struct ResultStorage;

impl ResultStorage {
    /// Generic result storage for any transform using mutations
    pub fn store_transform_result_generic(
        transform: &Transform,
        code_hash_to_result: HashMap<String, JsonValue>,
        message_bus: Option<&Arc<crate::fold_db_core::infrastructure::MessageBus>>,
    ) -> Result<(), SchemaError> {
        // TODO: Map Transform's declarative schema's field_to_hash_code to the result of the execution.
        let field_to_hash_code = transform.get_declarative_schema().unwrap().get_field_to_hash_code();
        let key_to_hash_code = transform.get_declarative_schema().unwrap().get_key_to_hash_code();

        let mut fields_and_values = HashMap::new();
        let mut keys_and_values = HashMap::new();

        for (field_name, hash_code) in field_to_hash_code {
            let result = code_hash_to_result.get(&hash_code).unwrap().clone();
            fields_and_values.insert(field_name, result);
        }

        for (key_name, hash_code) in key_to_hash_code {
            let result = code_hash_to_result.get(&hash_code).unwrap().clone();
            keys_and_values.insert(key_name, result);
        }

        // create a mutation through the event bus
        let mutation = Mutation::new(
            transform.get_declarative_schema().unwrap().name.clone(),
            fields_and_values,
            keys_and_values,
            TRANSFORM_SYSTEM_ID.to_string(),
            0,
            MutationType::Update,
        );

        if let Some(message_bus) = message_bus {
            let mutation_request = MutationRequest {
                correlation_id: Uuid::new_v4().to_string(),
                mutation,
            };
            message_bus.publish(mutation_request);
        }

        Ok(())
    }
}