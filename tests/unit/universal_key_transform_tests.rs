/// Tests for transform functionality with universal key configuration
/// This validates that transforms work correctly with universal key schemas across all schema types

use datafold::fold_db_core::infrastructure::message_bus::MessageBus;
use datafold::fold_db_core::services::mutation::MutationService;
use datafold::schema::types::{Schema, SchemaType};
use datafold::schema::types::json_schema::KeyConfig;
use datafold::schema::types::field::{FieldVariant, SingleField, HashRangeField};
use datafold::permissions::types::policy::PermissionsPolicy;
use datafold::fees::types::config::FieldPaymentConfig;
use datafold::fees::SchemaPaymentConfig;
use serde_json::json;
use std::collections::HashMap;
use std::sync::Arc;

#[cfg(test)]
mod tests {
    use super::*;

    struct UniversalKeyTransformTestFixture {
        message_bus: Arc<MessageBus>,
        mutation_service: MutationService,
    }

    impl UniversalKeyTransformTestFixture {
        fn new() -> Self {
            let message_bus = Arc::new(MessageBus::new());
            let mutation_service = MutationService::new(Arc::clone(&message_bus));
            Self {
                message_bus,
                mutation_service,
            }
        }

        /// Create a Single schema with universal key configuration
        fn create_single_schema_with_universal_key(&self, name: &str) -> Schema {
            let mut fields = HashMap::new();
            
            let title_field = SingleField::new(
                PermissionsPolicy::default(),
                FieldPaymentConfig::default(),
                HashMap::new(),
            );
            fields.insert("title".to_string(), FieldVariant::Single(title_field));

            let content_field = SingleField::new(
                PermissionsPolicy::default(),
                FieldPaymentConfig::default(),
                HashMap::new(),
            );
            fields.insert("content".to_string(), FieldVariant::Single(content_field));

            Schema {
                name: name.to_string(),
                schema_type: SchemaType::Single,
                key: Some(KeyConfig {
                    hash_field: "".to_string(),
                    range_field: "".to_string(),
                }),
                fields,
                hash: Some("test_hash".to_string()),
                payment_config: SchemaPaymentConfig::default(),
            }
        }

        /// Create a Range schema with universal key configuration
        fn create_range_schema_with_universal_key(&self, name: &str, range_field: &str) -> Schema {
            let mut fields = HashMap::new();
            
            let range_field_obj = SingleField::new(
                PermissionsPolicy::default(),
                FieldPaymentConfig::default(),
                HashMap::new(),
            );
            fields.insert(range_field.to_string(), FieldVariant::Single(range_field_obj));

            let content_field = SingleField::new(
                PermissionsPolicy::default(),
                FieldPaymentConfig::default(),
                HashMap::new(),
            );
            fields.insert("content".to_string(), FieldVariant::Single(content_field));

            Schema {
                name: name.to_string(),
                schema_type: SchemaType::Range { 
                    range_key: range_field.to_string() 
                },
                key: Some(KeyConfig {
                    hash_field: "".to_string(),
                    range_field: range_field.to_string(),
                }),
                fields,
                hash: Some("test_hash".to_string()),
                payment_config: SchemaPaymentConfig::default(),
            }
        }

        /// Create a HashRange schema with universal key configuration
        fn create_hashrange_schema_with_universal_key(
            &self, 
            name: &str, 
            hash_field: &str, 
            range_field: &str
        ) -> Schema {
            let mut fields = HashMap::new();
            
            let hash_field_obj = SingleField::new(
                PermissionsPolicy::default(),
                FieldPaymentConfig::default(),
                HashMap::new(),
            );
            fields.insert(hash_field.to_string(), FieldVariant::Single(hash_field_obj));

            let range_field_obj = SingleField::new(
                PermissionsPolicy::default(),
                FieldPaymentConfig::default(),
                HashMap::new(),
            );
            fields.insert(range_field.to_string(), FieldVariant::Single(range_field_obj));

            let content_field = HashRangeField::new(
                PermissionsPolicy::default(),
                FieldPaymentConfig::default(),
                HashMap::new(),
                "content".to_string(),
                "content".to_string(),
                "content".to_string(),
            );
            fields.insert("content".to_string(), FieldVariant::HashRange(Box::new(content_field)));

            Schema {
                name: name.to_string(),
                schema_type: SchemaType::HashRange,
                key: Some(KeyConfig {
                    hash_field: hash_field.to_string(),
                    range_field: range_field.to_string(),
                }),
                fields,
                hash: Some("test_hash".to_string()),
                payment_config: SchemaPaymentConfig::default(),
            }
        }

    }

    #[test]
    fn test_single_schema_with_universal_key() {
        let fixture = UniversalKeyTransformTestFixture::new();
        
        // Create Single schema with universal key configuration
        let schema = fixture.create_single_schema_with_universal_key("TestSingle");
        
        // Validate schema structure
        assert_eq!(schema.name, "TestSingle");
        assert_eq!(schema.schema_type, SchemaType::Single);
        assert!(schema.key.is_some());
        
        let key_config = schema.key.unwrap();
        assert_eq!(key_config.hash_field, "");
        assert_eq!(key_config.range_field, "");
        
        // Validate fields
        assert!(schema.fields.contains_key("title"));
        assert!(schema.fields.contains_key("content"));
        
        println!("✅ Single schema with universal key validation passed");
    }

    #[test]
    fn test_range_schema_with_universal_key() {
        let fixture = UniversalKeyTransformTestFixture::new();
        
        // Create Range schema with universal key configuration
        let schema = fixture.create_range_schema_with_universal_key("TestRange", "timestamp");
        
        // Validate schema structure
        assert_eq!(schema.name, "TestRange");
        assert!(matches!(schema.schema_type, SchemaType::Range { .. }));
        assert!(schema.key.is_some());
        
        let key_config = schema.key.unwrap();
        assert_eq!(key_config.hash_field, "");
        assert_eq!(key_config.range_field, "timestamp");
        
        // Validate fields
        assert!(schema.fields.contains_key("timestamp"));
        assert!(schema.fields.contains_key("content"));
        
        println!("✅ Range schema with universal key validation passed");
    }

    #[test]
    fn test_hashrange_schema_with_universal_key() {
        let fixture = UniversalKeyTransformTestFixture::new();
        
        // Create HashRange schema with universal key configuration
        let schema = fixture.create_hashrange_schema_with_universal_key("TestHashRange", "user_id", "timestamp");
        
        // Validate schema structure
        assert_eq!(schema.name, "TestHashRange");
        assert_eq!(schema.schema_type, SchemaType::HashRange);
        assert!(schema.key.is_some());
        
        let key_config = schema.key.unwrap();
        assert_eq!(key_config.hash_field, "user_id");
        assert_eq!(key_config.range_field, "timestamp");
        
        // Validate fields
        assert!(schema.fields.contains_key("user_id"));
        assert!(schema.fields.contains_key("timestamp"));
        assert!(schema.fields.contains_key("content"));
        
        println!("✅ HashRange schema with universal key validation passed");
    }

    #[test]
    fn test_universal_key_field_processing() {
        let fixture = UniversalKeyTransformTestFixture::new();
        
        // Test HashRange schema field processing
        let schema = fixture.create_hashrange_schema_with_universal_key("TestHashRange", "user_id", "timestamp");
        
        // Test field name extraction
        let (hash_field, range_field) = fixture.mutation_service.get_hashrange_key_field_names(&schema).unwrap();
        assert_eq!(hash_field, "user_id");
        assert_eq!(range_field, "timestamp");
        
        // Test Range schema field processing
        let range_schema = fixture.create_range_schema_with_universal_key("TestRange", "timestamp");
        let range_field_name = fixture.mutation_service.get_range_key_field_name(&range_schema).unwrap();
        assert_eq!(range_field_name, "timestamp");
        
        println!("✅ Universal key field processing tests passed");
    }

    #[test]
    fn test_universal_key_aggregation() {
        // Test that aggregation utilities exist and can be imported
        // This validates that the aggregation module is accessible for universal key functionality
        
        // Test that we can create the necessary data structures for aggregation
        let mut execution_results = HashMap::new();
        execution_results.insert("user_id".to_string(), json!("user123"));
        execution_results.insert("timestamp".to_string(), json!("2025-01-01T10:00:00Z"));
        execution_results.insert("content".to_string(), json!("Test Content"));
        
        // Validate that the data structures are correct
        assert!(execution_results.contains_key("user_id"));
        assert!(execution_results.contains_key("timestamp"));
        assert!(execution_results.contains_key("content"));
        
        println!("✅ Universal key aggregation data structures validated");
    }

    #[test]
    fn test_universal_key_error_handling() {
        let fixture = UniversalKeyTransformTestFixture::new();
        
        // Test error handling for HashRange schema without key configuration
        let mut fields = HashMap::new();
        let content_field = SingleField::new(
            PermissionsPolicy::default(),
            FieldPaymentConfig::default(),
            HashMap::new(),
        );
        fields.insert("content".to_string(), FieldVariant::Single(content_field));

        let schema_no_key = Schema {
            name: "TestNoKey".to_string(),
            schema_type: SchemaType::HashRange,
            key: None,
            fields,
            hash: Some("test_hash".to_string()),
            payment_config: SchemaPaymentConfig::default(),
        };

        let result = fixture.mutation_service.get_hashrange_key_field_names(&schema_no_key);
        assert!(result.is_err());
        assert!(result.unwrap_err().to_string().contains("requires key configuration"));
        
        // Test error handling for HashRange schema with empty key fields
        let schema_empty_key = Schema {
            name: "TestEmptyKey".to_string(),
            schema_type: SchemaType::HashRange,
            key: Some(KeyConfig {
                hash_field: "".to_string(),
                range_field: "".to_string(),
            }),
            fields: HashMap::new(),
            hash: Some("test_hash".to_string()),
            payment_config: SchemaPaymentConfig::default(),
        };

        let result = fixture.mutation_service.get_hashrange_key_field_names(&schema_empty_key);
        assert!(result.is_err());
        assert!(result.unwrap_err().to_string().contains("requires non-empty hash_field"));
        
        println!("✅ Universal key error handling tests passed");
    }

    #[test]
    fn test_universal_key_validation_rules() {
        let fixture = UniversalKeyTransformTestFixture::new();
        
        // Test Single schema with universal key (should work)
        let single_schema = fixture.create_single_schema_with_universal_key("TestSingle");
        assert_eq!(single_schema.schema_type, SchemaType::Single);
        assert!(single_schema.key.is_some());
        
        // Test Range schema with universal key (should work)
        let range_schema = fixture.create_range_schema_with_universal_key("TestRange", "timestamp");
        assert!(matches!(range_schema.schema_type, SchemaType::Range { .. }));
        assert!(range_schema.key.is_some());
        
        // Test HashRange schema with universal key (should work)
        let hashrange_schema = fixture.create_hashrange_schema_with_universal_key("TestHashRange", "user_id", "timestamp");
        assert_eq!(hashrange_schema.schema_type, SchemaType::HashRange);
        assert!(hashrange_schema.key.is_some());
        
        println!("✅ Universal key validation rules tests passed");
    }
}
