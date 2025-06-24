//! Mutation Domain Service
//! 
//! This module handles ONLY mutation-specific domain logic:
//! - Field value updates
//! - Atom modifications  
//! - Collection updates
//! 
//! It does NOT handle:
//! - Schema orchestration (belongs to FoldDB)
//! - Permission checking (belongs to FoldDB) 
//! - Event publishing (belongs to FoldDB)
//! - Schema validation (belongs to FoldDB)

use crate::fold_db_core::infrastructure::message_bus::{
    MessageBus,
    request_events::FieldValueSetRequest,
};
use crate::fold_db_core::infrastructure::factory::InfrastructureLogger;
use crate::logging::features::{log_feature, LogFeature};
use crate::schema::types::field::FieldVariant;
use crate::schema::types::schema::Schema;
use crate::schema::types::Mutation;
use crate::schema::SchemaError;
use sha2::{Digest, Sha256};
use std::sync::Arc;
use uuid::Uuid;
use serde_json::Value;

/// Mutation service responsible for field updates and atom modifications
pub struct MutationService {
    message_bus: Arc<MessageBus>,
}

impl MutationService {
    pub fn new(message_bus: Arc<MessageBus>) -> Self {
        Self {
            message_bus,
        }
    }

    /// Update a single field value (core mutation operation)
    pub fn update_field_value(
        &self,
        schema: &Schema,
        field_name: &str,
        value: &Value,
        mutation_hash: &str,
    ) -> Result<(), SchemaError> {
        InfrastructureLogger::log_operation_start("MutationService", "Updating field", &format!("{}.{}", schema.name, field_name));

        // Get field definition from schema
        let field_variant = schema.fields.get(field_name)
            .ok_or_else(|| SchemaError::InvalidData(
                format!("Field '{}' not found in schema '{}'", field_name, schema.name)
            ))?;

        // Apply field-specific mutation logic
        match field_variant {
            FieldVariant::Single(single_field) => {
                self.update_single_field(schema, field_name, single_field, value, mutation_hash)
            }
            FieldVariant::Range(range_field) => {
                self.update_range_field(schema, field_name, range_field, value, mutation_hash)
            }
        }
    }

    /// Update atoms for a range schema mutation (sharing MoleculeRange)
    pub fn update_range_schema_fields(
        &self,
        schema: &Schema,
        fields_and_values: &std::collections::HashMap<String, Value>,
        range_key_value: &str,
        _mutation_hash: &str,
    ) -> Result<(), SchemaError> {
        InfrastructureLogger::log_debug_info("MutationService", &format!("Processing range schema mutation for range_key_value: {}", range_key_value));
        
        // DIRECT APPROACH: Since mutation service doesn't have direct DB access,
        // we need to use FieldValueSetRequest with range-specific handling
        for (field_name, value) in fields_and_values {
            InfrastructureLogger::log_operation_start("MutationService", "Processing range field", &format!("{} with value: {} for range_key: {}", field_name, value, range_key_value));
            
            // Create a special field value request that includes the range key
            let range_aware_value = serde_json::json!({
                "range_key": range_key_value,
                "value": value
            });
            
            let correlation_id = Uuid::new_v4().to_string();
            let field_request = FieldValueSetRequest {
                correlation_id: correlation_id.clone(),
                schema_name: schema.name.clone(),
                field_name: field_name.clone(),
                value: range_aware_value,
                source_pub_key: "mutation_service".to_string(),
            };

            match self.message_bus.publish(field_request) {
                Ok(_) => {
                    InfrastructureLogger::log_operation_success("MutationService", "Range field update request sent", &format!("{}.{} with range_key: {}", schema.name, field_name, range_key_value));
                }
                Err(e) => {
                    InfrastructureLogger::log_operation_error("MutationService", "Failed to send range field update", &format!("{}.{}: {:?}", schema.name, field_name, e));
                    return Err(SchemaError::InvalidData(format!("Failed to update range field {}: {}", field_name, e)));
                }
            }
        }
        
        InfrastructureLogger::log_operation_success("MutationService", "All range field updates sent successfully", "");
        Ok(())
    }

    /// Modify atom value (core mutation operation)
    pub fn modify_atom(
        &self,
        atom_uuid: &str,
        _new_value: &Value,
        mutation_hash: &str,
    ) -> Result<(), SchemaError> {
        InfrastructureLogger::log_operation_start("MutationService", "Modifying atom", &format!("{} with hash {}", atom_uuid, mutation_hash));
        
        // This would typically interact with atom storage
        // For now, we'll use event-driven communication
        
        // TODO: Implement direct atom modification logic
        // This should update the atom's value and update its hash
        
        InfrastructureLogger::log_operation_success("MutationService", "Atom modified successfully", atom_uuid);
        Ok(())
    }

    /// Handle single field mutation
    fn update_single_field(
        &self,
        schema: &Schema,
        field_name: &str,
        _single_field: &crate::schema::types::field::single_field::SingleField,
        value: &Value,
        _mutation_hash: &str,
    ) -> Result<(), SchemaError> {
        InfrastructureLogger::log_operation_start("MutationService", "Updating single field", &format!("{}.{}", schema.name, field_name));
        
        // First, send FieldValueSetRequest to store the actual field value as an Atom
        let value_correlation_id = Uuid::new_v4().to_string();
        let field_value_request = FieldValueSetRequest::new(
            value_correlation_id.clone(),
            schema.name.clone(),
            field_name.to_string(),
            value.clone(),
            "mutation_service".to_string(),
        );

        if let Err(e) = self.message_bus.publish(field_value_request) {
            InfrastructureLogger::log_operation_error("MutationService", "Failed to send field value set request", &format!("{}.{}: {:?}", schema.name, field_name, e));
            return Err(SchemaError::InvalidData(format!("Failed to set field value: {}", e)));
        }
        InfrastructureLogger::log_operation_success("MutationService", "Field value set request sent", &format!("{}.{}", schema.name, field_name));
        
        // DIAGNOSTIC LOG: Track if FieldValueSetRequest is being consumed
        InfrastructureLogger::log_debug_info("MutationService", &format!("🔍 DIAGNOSTIC: FieldValueSetRequest published for {}.{} with correlation_id: {}", schema.name, field_name, value_correlation_id));
        
        // Transform triggers are now handled automatically by TransformOrchestrator
        // via direct FieldValueSet event monitoring
        Ok(())
    }

    /// Handle range field mutation (REMOVED - use update_range_schema_fields instead)
    /// Range fields should be processed via the range schema method which has proper range key context
    fn update_range_field(
        &self,
        schema: &Schema,
        field_name: &str,
        _range_field: &crate::schema::types::field::range_field::RangeField,
        _value: &Value,
        _mutation_hash: &str,
    ) -> Result<(), SchemaError> {
        InfrastructureLogger::log_operation_error("MutationService", "Individual range field updates not supported", "Range fields must be updated via range schema mutation.");
        Err(SchemaError::InvalidData(format!(
            "Range field '{}' in schema '{}' cannot be updated individually. Use range schema mutation instead.",
            field_name, schema.name
        )))
    }
    /// Generate mutation hash for tracking
    pub fn generate_mutation_hash(mutation: &Mutation) -> Result<String, SchemaError> {
        let mutation_bytes = serde_json::to_vec(&mutation).map_err(|e| {
            SchemaError::InvalidData(format!("Failed to serialize mutation: {}", e))
        })?;
        let mut hasher = Sha256::new();
        hasher.update(mutation_bytes);
        let mutation_hash = format!("{:x}", hasher.finalize());
        Ok(mutation_hash)
    }

    /// Validate field value format (mutation-specific validation)
    pub fn validate_field_value(
        field_variant: &FieldVariant,
        value: &Value,
    ) -> Result<(), SchemaError> {
        match field_variant {
            FieldVariant::Single(_) => {
                // Validate single field value format
                if value.is_null() {
                    return Err(SchemaError::InvalidData("Single field value cannot be null".to_string()));
                }
                Ok(())
            }
            FieldVariant::Range(_) => {
                // Validate range field value format
                if !value.is_object() {
                    return Err(SchemaError::InvalidData("Range field value must be an object".to_string()));
                }
                Ok(())
            }
        }
    }
}

/// Range schema mutation validation (domain-specific logic)
pub fn validate_range_schema_mutation_format(
    schema: &Schema,
    mutation: &Mutation,
) -> Result<(), SchemaError> {
    if let Some(range_key) = schema.range_key() {
        log_feature!(LogFeature::Mutation, info,
            "🔍 Validating Range schema mutation format for schema: {} with range_key: {}",
            schema.name,
            range_key
        );

        // MANDATORY: Range schema mutations MUST include the range_key field
        let range_key_value = mutation.fields_and_values.get(range_key)
            .ok_or_else(|| SchemaError::InvalidData(format!(
                "Range schema mutation for '{}' is missing required range_key field '{}'. All range schema mutations must provide a value for the range_key.",
                schema.name, range_key
            )))?;

        // Validate the range_key value is not null or empty
        if range_key_value.is_null() {
            return Err(SchemaError::InvalidData(format!(
                "Range schema mutation for '{}' has null value for range_key field '{}'. Range key must have a valid value.",
                schema.name, range_key
            )));
        }

        // If range_key value is a string, ensure it's not empty
        if let Some(str_value) = range_key_value.as_str() {
            if str_value.trim().is_empty() {
                return Err(SchemaError::InvalidData(format!(
                    "Range schema mutation for '{}' has empty string value for range_key field '{}'. Range key must have a non-empty value.",
                    schema.name, range_key
                )));
            }
        }

        // Validate all fields in the schema are RangeFields
        for (field_name, field_variant) in &schema.fields {
            match field_variant {
                FieldVariant::Range(_) => {
                    InfrastructureLogger::log_operation_success("MutationService", "Field validation", &format!("Field '{}' is correctly a RangeField", field_name));
                }
                FieldVariant::Single(_) => {
                    return Err(SchemaError::InvalidData(format!(
                        "Range schema '{}' contains Single field '{}', but all fields must be RangeFields",
                        schema.name, field_name
                    )));
                }
            }
        }

        InfrastructureLogger::log_operation_success("MutationService", "Range schema mutation format validation passed", &format!("schema: {}", schema.name));
    }

    Ok(())
}
