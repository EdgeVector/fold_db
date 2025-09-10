//! Integration tests for HashRange mutation system

use datafold::schema::types::{Mutation, MutationType};
use datafold::fold_db_core::services::mutation::MutationService;
use serde_json::json;
use std::collections::HashMap;

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_complete_hashrange_mutation_flow() {
        // Test the complete flow of creating and processing a HashRange mutation
        
        // Step 1: Create a HashRange mutation (as would be created by transform)
        let mut fields_and_values = HashMap::new();
        fields_and_values.insert("hash_key".to_string(), json!("data"));
        fields_and_values.insert("range_key".to_string(), json!("2025-08-22T20:32:52Z"));
        fields_and_values.insert("author".to_string(), json!("Bob Smith"));
        fields_and_values.insert("title".to_string(), json!("Getting Started with DataFold"));
        fields_and_values.insert("content".to_string(), json!("DataFold is a powerful distributed database system"));
        fields_and_values.insert("tags".to_string(), json!(["tutorial", "beginners", "datafold"]));
        
        let mutation = Mutation::new(
            "BlogPostWordIndex".to_string(),
            fields_and_values,
            "transform_system".to_string(),
            0,
            MutationType::Create,
        );
        
        // Step 2: Verify mutation structure
        assert_eq!(mutation.schema_name, "BlogPostWordIndex");
        assert_eq!(mutation.pub_key, "transform_system");
        assert_eq!(mutation.trust_distance, 0);
        
        // Step 3: Verify required fields are present
        assert!(mutation.fields_and_values.contains_key("hash_key"));
        assert!(mutation.fields_and_values.contains_key("range_key"));
        assert!(mutation.fields_and_values.contains_key("author"));
        assert!(mutation.fields_and_values.contains_key("title"));
        assert!(mutation.fields_and_values.contains_key("content"));
        assert!(mutation.fields_and_values.contains_key("tags"));
        
        // Step 4: Verify field values
        assert_eq!(mutation.fields_and_values["hash_key"], json!("data"));
        assert_eq!(mutation.fields_and_values["range_key"], json!("2025-08-22T20:32:52Z"));
        assert_eq!(mutation.fields_and_values["author"], json!("Bob Smith"));
        assert_eq!(mutation.fields_and_values["title"], json!("Getting Started with DataFold"));
        
        // Step 5: Test mutation hash generation
        let hash = MutationService::generate_mutation_hash(&mutation).unwrap();
        assert!(!hash.is_empty());
        assert_eq!(hash.len(), 64); // SHA256 hex string length
    }

    #[test]
    fn test_multiple_hashrange_mutations_same_word() {
        // Test aggregation of multiple mutations for the same word
        
        // Create first mutation for word "data"
        let mut fields1 = HashMap::new();
        fields1.insert("hash_key".to_string(), json!("data"));
        fields1.insert("range_key".to_string(), json!("2025-08-22T20:32:52Z"));
        fields1.insert("author".to_string(), json!("Bob Smith"));
        fields1.insert("title".to_string(), json!("Getting Started with DataFold"));
        
        let mutation1 = Mutation::new(
            "BlogPostWordIndex".to_string(),
            fields1,
            "transform_system".to_string(),
            0,
            MutationType::Create,
        );
        
        // Create second mutation for same word "data" but different range
        let mut fields2 = HashMap::new();
        fields2.insert("hash_key".to_string(), json!("data"));
        fields2.insert("range_key".to_string(), json!("2025-08-23T20:32:52Z"));
        fields2.insert("author".to_string(), json!("Alice Johnson"));
        fields2.insert("title".to_string(), json!("Advanced DataFold Techniques"));
        
        let mutation2 = Mutation::new(
            "BlogPostWordIndex".to_string(),
            fields2,
            "transform_system".to_string(),
            0,
            MutationType::Create,
        );
        
        // Test hash generation for both mutations
        let hash1 = MutationService::generate_mutation_hash(&mutation1).unwrap();
        let hash2 = MutationService::generate_mutation_hash(&mutation2).unwrap();
        
        // Different mutations should generate different hashes
        assert_ne!(hash1, hash2);
        
        // Verify both mutations have the same hash_key but different range_keys
        assert_eq!(mutation1.fields_and_values["hash_key"], mutation2.fields_and_values["hash_key"]);
        assert_ne!(mutation1.fields_and_values["range_key"], mutation2.fields_and_values["range_key"]);
    }

    #[test]
    fn test_hashrange_mutation_with_different_field_types() {
        // Test mutation with various field types
        let mut fields_and_values = HashMap::new();
        fields_and_values.insert("hash_key".to_string(), json!("test"));
        fields_and_values.insert("range_key".to_string(), json!("2025-01-01T00:00:00Z"));
        fields_and_values.insert("author".to_string(), json!("Test Author")); // String
        fields_and_values.insert("title".to_string(), json!("Test Title")); // String
        fields_and_values.insert("content".to_string(), json!("Test content with numbers 123 and symbols !@#")); // String
        fields_and_values.insert("tags".to_string(), json!(["tag1", "tag2", "tag3"])); // Array
        fields_and_values.insert("metadata".to_string(), json!({
            "views": 100,
            "likes": 25,
            "published": true
        })); // Object
        
        let mutation = Mutation::new(
            "BlogPostWordIndex".to_string(),
            fields_and_values,
            "transform_system".to_string(),
            0,
            MutationType::Create,
        );
        
        // Verify all field types are preserved
        assert_eq!(mutation.fields_and_values["author"], json!("Test Author"));
        assert_eq!(mutation.fields_and_values["title"], json!("Test Title"));
        assert_eq!(mutation.fields_and_values["tags"], json!(["tag1", "tag2", "tag3"]));
        
        let metadata = &mutation.fields_and_values["metadata"];
        assert_eq!(metadata["views"], json!(100));
        assert_eq!(metadata["likes"], json!(25));
        assert_eq!(metadata["published"], json!(true));
    }

    #[test]
    fn test_hashrange_mutation_serialization_roundtrip() {
        let mut fields_and_values = HashMap::new();
        fields_and_values.insert("hash_key".to_string(), json!("serialize"));
        fields_and_values.insert("range_key".to_string(), json!("2025-01-01T00:00:00Z"));
        fields_and_values.insert("author".to_string(), json!("Test Author"));
        fields_and_values.insert("title".to_string(), json!("Test Title"));
        fields_and_values.insert("content".to_string(), json!("Test content"));
        fields_and_values.insert("tags".to_string(), json!(["tag1", "tag2"]));
        
        let original_mutation = Mutation::new(
            "BlogPostWordIndex".to_string(),
            fields_and_values,
            "transform_system".to_string(),
            0,
            MutationType::Create,
        );
        
        // Serialize to JSON
        let serialized = serde_json::to_value(&original_mutation).unwrap();
        
        // Deserialize back to Mutation
        let deserialized_mutation: Mutation = serde_json::from_value(serialized).unwrap();
        
        // Verify roundtrip
        assert_eq!(original_mutation.schema_name, deserialized_mutation.schema_name);
        assert_eq!(original_mutation.pub_key, deserialized_mutation.pub_key);
        assert_eq!(original_mutation.trust_distance, deserialized_mutation.trust_distance);
        assert_eq!(original_mutation.fields_and_values, deserialized_mutation.fields_and_values);
    }

    #[test]
    fn test_hashrange_mutation_hash_consistency_across_components() {
        let mut fields_and_values = HashMap::new();
        fields_and_values.insert("hash_key".to_string(), json!("consistency"));
        fields_and_values.insert("range_key".to_string(), json!("2025-01-01T00:00:00Z"));
        fields_and_values.insert("author".to_string(), json!("Test Author"));
        
        let mutation = Mutation::new(
            "BlogPostWordIndex".to_string(),
            fields_and_values,
            "transform_system".to_string(),
            0,
            MutationType::Create,
        );
        
        // Test hash generation multiple times
        let hash1 = MutationService::generate_mutation_hash(&mutation).unwrap();
        let hash2 = MutationService::generate_mutation_hash(&mutation).unwrap();
        
        // Hashes should be consistent
        assert_eq!(hash1, hash2);
    }

    #[test]
    fn test_hashrange_mutation_with_special_characters() {
        // Test with special characters in hash_key and range_key
        let mut fields_and_values = HashMap::new();
        fields_and_values.insert("hash_key".to_string(), json!("data-fold_system.v2"));
        fields_and_values.insert("range_key".to_string(), json!("2025-08-22T20:32:52.123Z"));
        fields_and_values.insert("author".to_string(), json!("Bob O'Smith"));
        fields_and_values.insert("title".to_string(), json!("Getting Started with DataFold & More!"));
        fields_and_values.insert("content".to_string(), json!("DataFold is a powerful distributed database system that supports various query patterns including filtering, sorting, and aggregation operations."));
        
        let mutation = Mutation::new(
            "BlogPostWordIndex".to_string(),
            fields_and_values,
            "transform_system".to_string(),
            0,
            MutationType::Create,
        );
        
        // Verify special characters are preserved
        assert_eq!(mutation.fields_and_values["hash_key"], json!("data-fold_system.v2"));
        assert_eq!(mutation.fields_and_values["range_key"], json!("2025-08-22T20:32:52.123Z"));
        assert_eq!(mutation.fields_and_values["author"], json!("Bob O'Smith"));
        assert_eq!(mutation.fields_and_values["title"], json!("Getting Started with DataFold & More!"));
        
        // Test hash generation with special characters
        let hash = MutationService::generate_mutation_hash(&mutation).unwrap();
        assert!(!hash.is_empty());
        assert_eq!(hash.len(), 64);
    }
}