//! Mutation generator for creating mutations from AI responses and JSON data

use crate::ingestion::IngestionResult;
use crate::log_feature;
use crate::logging::features::LogFeature;
use crate::schema::types::{Mutation, KeyValue};
use crate::MutationType;
use serde_json::Value;
use std::collections::HashMap;

/// Service for generating mutations from AI responses and JSON data
pub struct MutationGenerator;

impl MutationGenerator {
    /// Create a new mutation generator
    pub fn new() -> Self {
        Self
    }

    /// Generate mutations from JSON data and mutation mappers
    pub fn generate_mutations(
        &self,
        schema_name: &str,
        keys_and_values: &HashMap<String, String>,
        fields_and_values: &HashMap<String, Value>,
        mutation_mappers: &HashMap<String, String>,
        trust_distance: u32,
        pub_key: String,
    ) -> IngestionResult<Vec<Mutation>> {
        log_feature!(
            LogFeature::Ingestion,
            info,
            "Generating mutations for schema '{}' with {} mappers, {} input fields",
            schema_name,
            mutation_mappers.len(),
            fields_and_values.len()
        );

        let mut mutations = Vec::new();

        // Apply mutation mappers to transform JSON fields to schema fields
        let mapped_fields = if mutation_mappers.is_empty() {
            // If no mappers provided, use fields as-is (backward compatibility)
            log_feature!(
                LogFeature::Ingestion,
                info,
                "No mutation mappers provided, using all {} fields directly",
                fields_and_values.len()
            );
            fields_and_values.clone()
        } else {
            // Apply mappers to transform JSON field names to schema field names
            let mut result = HashMap::new();
            for (json_field, schema_field) in mutation_mappers {
                if let Some(value) = fields_and_values.get(json_field) {
                    // Extract just the field name from schema path (e.g., "UserSchema.name" -> "name")
                    let field_name = if schema_field.contains('.') {
                        schema_field.rsplit('.').next().unwrap_or(schema_field)
                    } else {
                        schema_field.as_str()
                    };
                    
                    result.insert(field_name.to_string(), value.clone());
                    log_feature!(
                        LogFeature::Ingestion,
                        debug,
                        "Mapped field: {} -> {}",
                        json_field,
                        field_name
                    );
                } else {
                    log_feature!(
                        LogFeature::Ingestion,
                        warn,
                        "Mapper references missing JSON field: {}",
                        json_field
                    );
                }
            }
            
            log_feature!(
                LogFeature::Ingestion,
                info,
                "Applied mutation mappers: {} JSON fields -> {} schema fields",
                fields_and_values.len(),
                result.len()
            );
            result
        };

        // If we have fields to mutate, create a mutation
        if !mapped_fields.is_empty() {
            // Build KeyValue from keys
            let key_value = KeyValue::new(
                keys_and_values.get("hash_field").cloned(),
                keys_and_values.get("range_field").cloned(),
            );
            
            let mutation = Mutation::new(
                schema_name.to_string(),
                mapped_fields,
                key_value,
                pub_key,
                trust_distance,
                MutationType::Create,
            );
            mutations.push(mutation);
            log_feature!(
                LogFeature::Ingestion,
                info,
                "Created mutation with {} fields",
                mutations[0].fields_and_values.len()
            );
        } else {
            log_feature!(
                LogFeature::Ingestion,
                warn,
                "No valid field mappings found, no mutations generated"
            );
        }

        Ok(mutations)
    }




}


impl Default for MutationGenerator {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;


    #[test]
    fn test_generate_mutations() {
        let generator = MutationGenerator::new();

        let mut keys_and_values = HashMap::new();
        keys_and_values.insert("hash_field".to_string(), "hash_key".to_string());
        keys_and_values.insert("range_field".to_string(), "range_key".to_string());
        
        let mut fields_and_values = HashMap::new();
        fields_and_values.insert("name".to_string(), json!("John"));
        fields_and_values.insert("age".to_string(), json!(30));

        let mut mappers = HashMap::new();
        mappers.insert("name".to_string(), "UserSchema.name".to_string());
        mappers.insert("age".to_string(), "UserSchema.age".to_string());

        let result = generator
            .generate_mutations(
                "UserSchema",
                &keys_and_values,
                &fields_and_values,
                &mappers,
                0,
                "test-key".to_string(),
            )
            .unwrap();

        assert_eq!(result.len(), 1);
        assert_eq!(result[0].fields_and_values.len(), 2);
    }
}
