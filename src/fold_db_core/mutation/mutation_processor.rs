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
            println!("🎯 DEBUG: Processing HashRange schema mutation for schema '{}'", schema.name);
            log_feature!(LogFeature::Mutation, info, "🎯 DEBUG: Processing HashRange schema mutation for schema '{}'", schema.name);
            
            // Extract the hash_key and range_key from the mutation data
            let hash_key_value = mutation.fields_and_values.get("hash_key")
                .ok_or_else(|| SchemaError::InvalidData(
                    "HashRange schema mutation missing hash_key field".to_string()
                ))?;
            
            let range_key_value = mutation.fields_and_values.get("range_key")
                .ok_or_else(|| SchemaError::InvalidData(
                    "HashRange schema mutation missing range_key field".to_string()
                ))?;
            
            let hash_key_str = match hash_key_value {
                Value::String(s) => s.clone(),
                _ => hash_key_value.to_string().trim_matches('"').to_string(),
            };
            
            let range_key_str = match range_key_value {
                Value::String(s) => s.clone(),
                _ => range_key_value.to_string().trim_matches('"').to_string(),
            };
            
            log_feature!(LogFeature::Mutation, info, "🎯 DEBUG: HashRange key values - hash_key: '{}', range_key: '{}'", hash_key_str, range_key_str);
            
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
        if let Some(range_key) = schema.range_key() {
            log_feature!(LogFeature::Mutation, info, "🎯 DEBUG: Processing range schema mutation for schema '{}' with range_key '{}'", schema.name, range_key);
            
            // Extract the range key value from the mutation data
            let range_key_value = mutation.fields_and_values.get(range_key)
                .ok_or_else(|| SchemaError::InvalidData(format!(
                    "Range schema mutation missing range_key field '{}'", range_key
                )))?;
            
            let range_key_str = match range_key_value {
                Value::String(s) => s.clone(),
                _ => range_key_value.to_string().trim_matches('"').to_string(),
            };
            
            log_feature!(LogFeature::Mutation, info, "🎯 DEBUG: Range key value: '{}'", range_key_str);
            
            // Use the specialized range schema mutation method
            return mutation_service.update_range_schema_fields(
                schema,
                &mutation.fields_and_values,
                &range_key_str,
                mutation_hash,
            );
        } else {
            log_feature!(LogFeature::Mutation, info, "🔍 DEBUG: Processing regular schema mutation for schema '{}'", schema.name);
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
