//! Mutation generator for creating mutations from AI responses and JSON data

use crate::ingestion::IngestionResult;
use crate::log_feature;
use crate::logging::features::LogFeature;
use crate::schema::types::{Mutation, KeyConfig};
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
            "Generating mutations for schema '{}' with {} mappers",
            schema_name,
            mutation_mappers.len()
        );

        let mut mutations = Vec::new();

        // Process each mutation mapper
        for (json_path, schema_path) in mutation_mappers {
            log_feature!(
                LogFeature::Ingestion,
                debug,
                "Processing mapper: {} -> {}",
                json_path,
                schema_path
            );
        }

        // If we have fields to mutate, create a mutation
        if !fields_and_values.is_empty() {
            // Convert keys_and_values to KeyConfig
            let key_config = KeyConfig::new(
                keys_and_values.get("hash_field").cloned(),
                keys_and_values.get("range_field").cloned(),
            );
            
            let mutation = Mutation::new(
                schema_name.to_string(),
                fields_and_values.clone(),
                key_config,
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




    /// Generate mutations for collection fields (arrays) - DEPRECATED
    pub fn generate_collection_mutations(
        &self,
        _schema_name: &str,
        _json_data: &Value,
        _mutation_mappers: &HashMap<String, String>,
        _trust_distance: u32,
        _pub_key: String,
    ) -> IngestionResult<Vec<Mutation>> {
        log_feature!(
            LogFeature::Ingestion,
            warn,
            "Collection mutations are no longer supported - collections have been removed from the schema system"
        );
        Ok(Vec::new())
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
