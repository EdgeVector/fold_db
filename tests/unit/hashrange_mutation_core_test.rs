//! Simple unit tests for HashRange mutation system core functionality

use datafold::schema::types::{Mutation, MutationType};
use serde_json::json;
use std::collections::HashMap;

/// Generate a simple hash for mutation tracking
fn generate_mutation_hash(mutation: &Mutation) -> String {
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_hashrange_mutation_structure() {
        // Test that we can create a HashRange mutation with the correct structure
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
        
        // Verify mutation structure
        assert_eq!(mutation.schema_name, "BlogPostWordIndex");
        assert_eq!(mutation.pub_key, "transform_system");
        assert_eq!(mutation.trust_distance, 0);
        
        // Verify required fields are present
        assert!(mutation.fields_and_values.contains_key("hash_key"));
        assert!(mutation.fields_and_values.contains_key("range_key"));
        assert!(mutation.fields_and_values.contains_key("author"));
        assert!(mutation.fields_and_values.contains_key("title"));
        assert!(mutation.fields_and_values.contains_key("content"));
        assert!(mutation.fields_and_values.contains_key("tags"));
        
        // Verify field values
        assert_eq!(mutation.fields_and_values["hash_key"], json!("data"));
        assert_eq!(mutation.fields_and_values["range_key"], json!("2025-08-22T20:32:52Z"));
        assert_eq!(mutation.fields_and_values["author"], json!("Bob Smith"));
        assert_eq!(mutation.fields_and_values["title"], json!("Getting Started with DataFold"));
    }

    #[test]
    fn test_hashrange_mutation_with_null_values() {
        let mut fields_and_values = HashMap::new();
        fields_and_values.insert("hash_key".to_string(), json!("test"));
        fields_and_values.insert("range_key".to_string(), json!("2025-01-01T00:00:00Z"));
        fields_and_values.insert("author".to_string(), json!(null));
        fields_and_values.insert("title".to_string(), json!("Test Title"));
        
        let mutation = Mutation::new(
            "BlogPostWordIndex".to_string(),
            fields_and_values,
            "transform_system".to_string(),
            0,
            MutationType::Create,
        );
        
        // Should handle null values gracefully
        assert_eq!(mutation.fields_and_values["author"], json!(null));
        assert_eq!(mutation.fields_and_values["title"], json!("Test Title"));
    }

    #[test]
    fn test_hashrange_mutation_with_array_values() {
        let mut fields_and_values = HashMap::new();
        fields_and_values.insert("hash_key".to_string(), json!("test"));
        fields_and_values.insert("range_key".to_string(), json!("2025-01-01T00:00:00Z"));
        fields_and_values.insert("tags".to_string(), json!(["tag1", "tag2", "tag3"]));
        
        let mutation = Mutation::new(
            "BlogPostWordIndex".to_string(),
            fields_and_values,
            "transform_system".to_string(),
            0,
            MutationType::Create,
        );
        
        // Should handle array values
        assert_eq!(mutation.fields_and_values["tags"], json!(["tag1", "tag2", "tag3"]));
    }

    #[test]
    fn test_hashrange_mutation_with_complex_values() {
        let mut fields_and_values = HashMap::new();
        fields_and_values.insert("hash_key".to_string(), json!("complex"));
        fields_and_values.insert("range_key".to_string(), json!("2025-01-01T00:00:00Z"));
        fields_and_values.insert("metadata".to_string(), json!({
            "views": 100,
            "likes": 25,
            "published": true
        }));
        
        let mutation = Mutation::new(
            "BlogPostWordIndex".to_string(),
            fields_and_values,
            "transform_system".to_string(),
            0,
            MutationType::Create,
        );
        
        // Should handle complex object values
        let metadata = &mutation.fields_and_values["metadata"];
        assert_eq!(metadata["views"], json!(100));
        assert_eq!(metadata["likes"], json!(25));
        assert_eq!(metadata["published"], json!(true));
    }

    #[test]
    fn test_hashrange_mutation_serialization() {
        let mut fields_and_values = HashMap::new();
        fields_and_values.insert("hash_key".to_string(), json!("serialize"));
        fields_and_values.insert("range_key".to_string(), json!("2025-01-01T00:00:00Z"));
        fields_and_values.insert("author".to_string(), json!("Test Author"));
        
        let mutation = Mutation::new(
            "BlogPostWordIndex".to_string(),
            fields_and_values,
            "transform_system".to_string(),
            0,
            MutationType::Create,
        );
        
        // Test serialization
        let serialized = serde_json::to_value(&mutation).unwrap();
        assert!(serialized.is_object());
        
        // Test deserialization
        let deserialized_mutation: Mutation = serde_json::from_value(serialized).unwrap();
        assert_eq!(deserialized_mutation.schema_name, "BlogPostWordIndex");
        assert_eq!(deserialized_mutation.fields_and_values["hash_key"], json!("serialize"));
        assert_eq!(deserialized_mutation.fields_and_values["range_key"], json!("2025-01-01T00:00:00Z"));
        assert_eq!(deserialized_mutation.fields_and_values["author"], json!("Test Author"));
    }

    #[test]
    fn test_hashrange_mutation_hash_generation() {
        let mut fields_and_values = HashMap::new();
        fields_and_values.insert("hash_key".to_string(), json!("data"));
        fields_and_values.insert("range_key".to_string(), json!("2025-08-22T20:32:52Z"));
        fields_and_values.insert("author".to_string(), json!("Bob Smith"));
        
        let mutation = Mutation::new(
            "BlogPostWordIndex".to_string(),
            fields_and_values,
            "transform_system".to_string(),
            0,
            MutationType::Create,
        );
        
        // Test hash generation
        let hash = generate_mutation_hash(&mutation);
        assert!(!hash.is_empty());
        assert_eq!(hash.len(), 64); // SHA256 hex string length
        
        // Same mutation should generate same hash
        let hash2 = generate_mutation_hash(&mutation);
        assert_eq!(hash, hash2);
    }

    #[test]
    fn test_hashrange_mutation_different_hashes() {
        let mut fields1 = HashMap::new();
        fields1.insert("hash_key".to_string(), json!("word1"));
        fields1.insert("range_key".to_string(), json!("2025-01-01T00:00:00Z"));
        fields1.insert("author".to_string(), json!("Author 1"));
        
        let mut fields2 = HashMap::new();
        fields2.insert("hash_key".to_string(), json!("word2"));
        fields2.insert("range_key".to_string(), json!("2025-01-02T00:00:00Z"));
        fields2.insert("author".to_string(), json!("Author 2"));
        
        let mutation1 = Mutation::new(
            "BlogPostWordIndex".to_string(),
            fields1,
            "transform_system".to_string(),
            0,
            MutationType::Create,
        );
        
        let mutation2 = Mutation::new(
            "BlogPostWordIndex".to_string(),
            fields2,
            "transform_system".to_string(),
            0,
            MutationType::Create,
        );
        
        // Test hash generation
        let hash1 = generate_mutation_hash(&mutation1);
        let hash2 = generate_mutation_hash(&mutation2);
        
        // Different mutations should generate different hashes
        assert_ne!(hash1, hash2);
    }

    #[test]
    fn test_hashrange_mutation_empty_fields() {
        let fields_and_values = HashMap::new();
        
        let mutation = Mutation::new(
            "BlogPostWordIndex".to_string(),
            fields_and_values,
            "transform_system".to_string(),
            0,
            MutationType::Create,
        );
        
        // Should handle empty fields gracefully
        assert!(mutation.fields_and_values.is_empty());
        assert_eq!(mutation.schema_name, "BlogPostWordIndex");
    }

    #[test]
    fn test_hashrange_mutation_with_special_characters() {
        let mut fields_and_values = HashMap::new();
        fields_and_values.insert("hash_key".to_string(), json!("data-fold_system.v2"));
        fields_and_values.insert("range_key".to_string(), json!("2025-08-22T20:32:52.123Z"));
        fields_and_values.insert("author".to_string(), json!("Bob O'Smith"));
        fields_and_values.insert("title".to_string(), json!("Getting Started with DataFold & More!"));
        
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
    }
}
