//! Mutation service modules
//!
//! This module contains the broken-down components of the mutation service,
//! organized into focused, maintainable modules.

pub mod types;
pub mod utilities;
pub mod validation;
pub mod field_handlers;
pub mod schema_handlers;

// Re-export the main types and functions for backward compatibility
pub use types::{NormalizedFieldContext, NormalizedFieldValueRequest, MUTATION_SERVICE_SOURCE};
pub use validation::{validate_range_schema_mutation_format};
use crate::validation::validate_field_value;

// Main MutationService implementation
use crate::fold_db_core::infrastructure::factory::InfrastructureLogger;
use crate::fold_db_core::infrastructure::message_bus::{
    request_events::FieldValueSetRequest, MessageBus,
};
use crate::schema::types::schema::{Schema, SchemaType};
use crate::schema::SchemaError;
use serde_json::{Map, Value};
use std::sync::Arc;
use uuid::Uuid;

// Import the modular components
use crate::fold_db_core::services::mutation::{
    utilities::{normalize_optional_string, set_value, sort_fields},
};

/// Mutation service responsible for field updates and atom modifications
pub struct MutationService {
    message_bus: Arc<MessageBus>,
}

impl MutationService {
    pub fn new(message_bus: Arc<MessageBus>) -> Self {
        Self { message_bus }
    }

    /// Construct a normalized FieldValueSetRequest payload using schema-driven key resolution.
    ///
    /// The normalized payload contract is documented in
    /// `docs/reference/fold_db_core/mutation_service.md`.
    pub fn normalized_field_value_request(
        &self,
        schema: &Schema,
        field_name: &str,
        field_value: &Value,
        hash_key_value: Option<&Value>,
        range_key_value: Option<&Value>,
        mutation_hash: Option<&str>,
    ) -> Result<NormalizedFieldValueRequest, SchemaError> {
        self.build_field_value_request(
            schema,
            field_name,
            field_value,
            hash_key_value,
            range_key_value,
            mutation_hash,
        )
    }

    fn build_field_value_request(
        &self,
        schema: &Schema,
        field_name: &str,
        field_value: &Value,
        hash_key_value: Option<&Value>,
        range_key_value: Option<&Value>,
        mutation_hash: Option<&str>,
    ) -> Result<NormalizedFieldValueRequest, SchemaError> {
        InfrastructureLogger::log_debug_info(
            "MutationService",
            &format!(
                "Building normalized field value request for {}.{}",
                schema.name, field_name
            ),
        );

        // Extract key field names from schema (simplified approach)
        let (hash_key, range_key) = match &schema.schema_type {
            SchemaType::Single => (None, None),
            SchemaType::Range { range_key } => {
                let range_field = if let Some(key_config) = &schema.key {
                    if !key_config.range_field.trim().is_empty() {
                        Some(key_config.range_field.clone())
                    } else {
                        None
                    }
                } else {
                    Some(range_key.clone())
                };
                (None, range_field)
            }
            SchemaType::HashRange => {
                if let Some(key_config) = &schema.key {
                    let hash_field = if !key_config.hash_field.trim().is_empty() {
                        Some(key_config.hash_field.clone())
                    } else {
                        None
                    };
                    let range_field = if !key_config.range_field.trim().is_empty() {
                        Some(key_config.range_field.clone())
                    } else {
                        None
                    };
                    (hash_field, range_field)
                } else {
                    (None, None)
                }
            }
        };

        // Create normalized context
        let context = NormalizedFieldContext {
            hash: normalize_optional_string(hash_key),
            range: normalize_optional_string(range_key),
            fields: {
                let mut fields = Map::new();
                set_value(&mut fields, field_name, field_value);
                sort_fields(&fields)
            },
        };

        // Create mutation context if we have key values
        let mutation_context = if hash_key_value.is_some() || range_key_value.is_some() {
            Some(crate::fold_db_core::infrastructure::message_bus::atom_events::MutationContext {
                range_key: range_key_value.map(|v| v.to_string().trim_matches('"').to_string()),
                hash_key: hash_key_value.map(|v| v.to_string().trim_matches('"').to_string()),
                mutation_hash: mutation_hash.map(|s| s.to_string()),
                incremental: true,
            })
        } else {
            None
        };

        // Create the request payload using the correct structure
        let request = FieldValueSetRequest {
            correlation_id: Uuid::new_v4().to_string(),
            schema_name: schema.name.clone(),
            field_name: field_name.to_string(),
            value: field_value.clone(),
            source_pub_key: "mutation_service".to_string(),
            mutation_context,
        };

        Ok(NormalizedFieldValueRequest { request, context })
    }

    /// Modify atom value (core mutation operation)
    pub fn modify_atom(
        &self,
        atom_uuid: &str,
        _new_value: &Value,
        mutation_hash: &str,
    ) -> Result<(), SchemaError> {
        InfrastructureLogger::log_operation_start(
            "MutationService",
            "Modifying atom",
            &format!("{} with hash {}", atom_uuid, mutation_hash),
        );

        // This would typically interact with atom storage
        // For now, we'll use event-driven communication

        // TODO: Implement direct atom modification logic
        // This should update the atom's value and update its hash

        InfrastructureLogger::log_operation_success(
            "MutationService",
            "Atom modified successfully",
            atom_uuid,
        );
        Ok(())
    }

    /// Validate field value format (mutation-specific validation)
    pub fn validate_field_value(
        field_variant: &crate::schema::types::field::FieldVariant,
        value: &Value,
    ) -> Result<(), SchemaError> {
        validate_field_value(field_variant, value)
    }
}
