//! Mutation Processor
//! 
//! Handles the core mutation processing logic including preparation, validation,
//! and field mutation processing via service delegation.

use crate::schema::{Schema, SchemaError};
use crate::schema::types::Mutation;
use crate::schema::SchemaCore;
use crate::fold_db_core::services::mutation::MutationService;
use crate::logging::features::{log_feature, LogFeature};
use serde_json::Value;
use std::sync::Arc;

/// Processor for handling mutation preparation and field processing
pub struct MutationProcessor {
    schema_manager: Arc<SchemaCore>,
}

impl MutationProcessor {
    /// Create a new mutation processor
    pub fn new(
        schema_manager: Arc<SchemaCore>,
    ) -> Self {
        Self {
            schema_manager,
        }
    }

    /// Prepare mutation and schema - extract and validate components
    pub fn prepare_mutation_and_schema(
        &self,
        mutation: Mutation,
    ) -> Result<(Schema, Mutation, String), SchemaError> {
        // Get schema
        let schema = match self.schema_manager.get_schema(&mutation.schema_name)? {
            Some(schema) => schema,
            None => {
                return Err(SchemaError::InvalidData(format!(
                    "Schema '{}' not found",
                    mutation.schema_name
                )));
            }
        };

        // Calculate mutation hash for tracking
        let mutation_hash = self.calculate_mutation_hash(&mutation);

        Ok((schema, mutation, mutation_hash))
    }

    /// Process field mutations via service delegation
    pub fn process_field_mutations_via_service(
        &self,
        mutation_service: &MutationService,
        schema: &Schema,
        mutation: &Mutation,
        mutation_hash: &str,
    ) -> Result<(), SchemaError> {
        // Check if this is a HashRange schema
        if matches!(schema.schema_type, crate::schema::types::SchemaType::HashRange) {
            log_feature!(LogFeature::Mutation, info, "🎯 Processing HashRange schema mutation for schema '{}'", schema.name);
            
            // Extract hash and range field names from universal key configuration
            let (hash_field_name, range_field_name) = self.extract_key_field_names(schema)?;
            
            // Extract the hash_key and range_key values from the mutation data using universal field names
            let hash_key_value = mutation.fields_and_values.get(&hash_field_name)
                .ok_or_else(|| SchemaError::InvalidData(format!(
                    "HashRange schema mutation missing hash field '{}'", hash_field_name
                )))?;
            
            let range_key_value = mutation.fields_and_values.get(&range_field_name)
                .ok_or_else(|| SchemaError::InvalidData(format!(
                    "HashRange schema mutation missing range field '{}'", range_field_name
                )))?;
            
            let hash_key_str = self.extract_string_value(hash_key_value)?;
            let range_key_str = self.extract_string_value(range_key_value)?;
            
            log_feature!(LogFeature::Mutation, info, "🎯 HashRange key values - hash_field: '{}', range_field: '{}', hash_value: '{}', range_value: '{}'", 
                        hash_field_name, range_field_name, hash_key_str, range_key_str);
            
            // Use the specialized HashRange schema mutation method
            return mutation_service.update_hashrange_schema_fields(
                schema,
                &mutation.fields_and_values,
                &hash_key_str,
                &range_key_str,
                mutation_hash,
            );
        }
        
        // Check if this is a range schema
        if let Some(range_field_name) = self.extract_range_field_name(schema)? {
            log_feature!(LogFeature::Mutation, info, "🎯 Processing range schema mutation for schema '{}' with range_field '{}'", schema.name, range_field_name);
            
            // Extract the range key value from the mutation data using universal field name
            let range_key_value = mutation.fields_and_values.get(&range_field_name)
                .ok_or_else(|| SchemaError::InvalidData(format!(
                    "Range schema mutation missing range field '{}'", range_field_name
                )))?;
            
            let range_key_str = self.extract_string_value(range_key_value)?;
            
            log_feature!(LogFeature::Mutation, info, "🎯 Range key value: '{}'", range_key_str);
            
            // Use the specialized range schema mutation method
            return mutation_service.update_range_schema_fields(
                schema,
                &mutation.fields_and_values,
                &range_key_str,
                mutation_hash,
            );
        } else {
            log_feature!(LogFeature::Mutation, info, "🔍 Processing regular schema mutation for schema '{}'", schema.name);
        }

        // For non-range schemas, process fields individually
        for (field_name, field_value) in &mutation.fields_and_values {
            // Get field definition
            let _field = schema.fields.get(field_name).ok_or_else(|| {
                SchemaError::InvalidData(format!("Field '{}' not found in schema", field_name))
            })?;

            // Delegate to mutation service
            mutation_service.update_field_value(schema, field_name.as_str(), field_value, mutation_hash)?;
        }

        Ok(())
    }

    /// Extract hash and range field names from schema's universal key configuration
    fn extract_key_field_names(&self, schema: &Schema) -> Result<(String, String), SchemaError> {
        
        // For HashRange schemas, both hash_field and range_field are required
        let key_config = schema.key.as_ref().ok_or_else(|| {
            SchemaError::InvalidData(format!("HashRange schema '{}' requires key configuration", schema.name))
        })?;

        let hash_field = if key_config.hash_field.trim().is_empty() {
            return Err(SchemaError::InvalidData(format!(
                "HashRange schema '{}' requires non-empty hash_field in key configuration", 
                schema.name
            )));
        } else {
            key_config.hash_field.clone()
        };

        let range_field = if key_config.range_field.trim().is_empty() {
            return Err(SchemaError::InvalidData(format!(
                "HashRange schema '{}' requires non-empty range_field in key configuration", 
                schema.name
            )));
        } else {
            key_config.range_field.clone()
        };

        Ok((hash_field, range_field))
    }

    /// Extract range field name from schema's universal key configuration or legacy range_key
    fn extract_range_field_name(&self, schema: &Schema) -> Result<Option<String>, SchemaError> {
        match &schema.schema_type {
            crate::schema::types::SchemaType::Range { range_key } => {
                // Use universal key configuration if available, otherwise fall back to legacy range_key
                if let Some(key_config) = &schema.key {
                    if !key_config.range_field.trim().is_empty() {
                        Ok(Some(key_config.range_field.clone()))
                    } else {
                        Err(SchemaError::InvalidData(format!(
                            "Range schema '{}' with key configuration must have range_field", 
                            schema.name
                        )))
                    }
                } else {
                    // Legacy range_key support
                    Ok(Some(range_key.clone()))
                }
            },
            _ => Ok(None)
        }
    }

    /// Extract string value from JSON Value, handling different types
    fn extract_string_value(&self, value: &Value) -> Result<String, SchemaError> {
        match value {
            Value::String(s) => Ok(s.clone()),
            Value::Number(n) => Ok(n.to_string()),
            Value::Bool(b) => Ok(b.to_string()),
            _ => Ok(value.to_string().trim_matches('"').to_string()),
        }
    }

    /// Calculate a hash for the mutation to use for tracking
    fn calculate_mutation_hash(&self, mutation: &Mutation) -> String {
        use sha2::{Sha256, Digest};
        
        let mut hasher = Sha256::new();
        hasher.update(mutation.schema_name.as_bytes());
        hasher.update(format!("{:?}", mutation.mutation_type).as_bytes());
        
        // Add field names and values to hash
        let mut field_entries: Vec<_> = mutation.fields_and_values.iter().collect();
        field_entries.sort_by_key(|(key, _)| *key);
        
        for (field_name, field_value) in field_entries {
            hasher.update(field_name.as_bytes());
            hasher.update(field_value.to_string().as_bytes());
        }
        
        format!("{:x}", hasher.finalize())
    }
}
