//! Mutation generator for creating mutations from AI responses and JSON data

use crate::ingestion::IngestionResult;
use crate::log_feature;
use crate::logging::features::LogFeature;
use crate::schema::types::{KeyValue, Mutation};
use crate::MutationType;
use serde_json::Value;
use sha2::{Digest, Sha256};
use std::collections::HashMap;

/// Generate mutations from JSON data and mutation mappers
pub fn generate_mutations(
    schema_name: &str,
    keys_and_values: &HashMap<String, String>,
    fields_and_values: &HashMap<String, Value>,
    mutation_mappers: &HashMap<String, String>,
    pub_key: String,
    source_file_name: Option<String>,
    metadata: Option<HashMap<String, String>>,
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
        let mut key_value = KeyValue::new(
            keys_and_values.get("hash_field").cloned(),
            keys_and_values.get("range_field").cloned(),
        );

        // When key fields are missing from the data (e.g., some array items lack the
        // designated key field), generate a deterministic content-hash fallback so the
        // mutation is still stored and retrievable.
        if key_value.hash.is_none() && key_value.range.is_none() {
            let fallback = content_hash_key(&mapped_fields);
            log_feature!(
                LogFeature::Ingestion,
                warn,
                "Key fields not found in data for schema '{}', using content hash '{}' as fallback",
                schema_name, fallback
            );
            key_value.hash = Some(fallback);
        }

        let mut mutation = Mutation::new(
            schema_name.to_string(),
            mapped_fields,
            key_value,
            pub_key,
            MutationType::Create,
        );

        if let Some(filename) = source_file_name {
            mutation = mutation.with_source_file_name(filename);
        }

        if let Some(meta) = metadata {
            mutation = mutation.with_metadata(meta);
        }

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

/// Compute a deterministic hash from field values to use as a fallback key
/// when the schema's designated key fields are missing from the data.
fn content_hash_key(fields: &HashMap<String, Value>) -> String {
    let mut hasher = Sha256::new();
    let mut sorted: Vec<_> = fields.iter().collect();
    sorted.sort_by(|(a, _), (b, _)| a.cmp(b));
    for (k, v) in sorted {
        hasher.update(k.as_bytes());
        hasher.update(v.to_string().as_bytes());
    }
    let digest = hasher.finalize();
    format!("{:x}", digest)[..16].to_string()
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn test_generate_mutations() {
        let mut keys_and_values = HashMap::new();
        keys_and_values.insert("hash_field".to_string(), "hash_key".to_string());
        keys_and_values.insert("range_field".to_string(), "range_key".to_string());

        let mut fields_and_values = HashMap::new();
        fields_and_values.insert("name".to_string(), json!("John"));
        fields_and_values.insert("age".to_string(), json!(30));

        let mut mappers = HashMap::new();
        mappers.insert("name".to_string(), "UserSchema.name".to_string());
        mappers.insert("age".to_string(), "UserSchema.age".to_string());

        let result = generate_mutations(
            "UserSchema",
            &keys_and_values,
            &fields_and_values,
            &mappers,
            "test-key".to_string(),
            None,
            None,
        )
        .unwrap();

        assert_eq!(result.len(), 1);
        assert_eq!(result[0].fields_and_values.len(), 2);
    }

    #[test]
    fn test_content_hash_key_deterministic() {
        let mut fields = HashMap::new();
        fields.insert("name".to_string(), json!("Alice"));
        fields.insert("age".to_string(), json!(30));

        let hash1 = content_hash_key(&fields);
        let hash2 = content_hash_key(&fields);
        assert_eq!(hash1, hash2, "Same fields should produce same hash");
        assert_eq!(hash1.len(), 16, "Hash should be 16 hex chars");
    }

    #[test]
    fn test_content_hash_key_differs_for_different_data() {
        let mut fields_a = HashMap::new();
        fields_a.insert("name".to_string(), json!("Alice"));

        let mut fields_b = HashMap::new();
        fields_b.insert("name".to_string(), json!("Bob"));

        assert_ne!(
            content_hash_key(&fields_a),
            content_hash_key(&fields_b),
            "Different data should produce different hashes"
        );
    }

    #[test]
    fn test_generate_mutations_fallback_key_when_keys_missing() {
        // Empty keys_and_values simulates missing key fields in data
        let keys_and_values = HashMap::new();

        let mut fields_and_values = HashMap::new();
        fields_and_values.insert("name".to_string(), json!("Alice"));
        fields_and_values.insert("phone".to_string(), json!("555-0101"));

        let mut mappers = HashMap::new();
        mappers.insert("name".to_string(), "ContactSchema.name".to_string());
        mappers.insert("phone".to_string(), "ContactSchema.phone".to_string());

        let result = generate_mutations(
            "ContactSchema",
            &keys_and_values,
            &fields_and_values,
            &mappers,
            "test-key".to_string(),
            None,
            None,
        )
        .unwrap();

        assert_eq!(result.len(), 1);
        assert!(
            result[0].key_value.hash.is_some(),
            "Should have a fallback hash key when key fields are missing"
        );
    }
}
