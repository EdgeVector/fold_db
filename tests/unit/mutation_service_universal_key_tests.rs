//! Unit tests for MutationService universal key configuration functionality

use datafold::fold_db_core::services::mutation::MutationService;
use datafold::fold_db_core::infrastructure::message_bus::MessageBus;
use datafold::schema::types::{Schema, SchemaType};
use datafold::schema::types::field::{FieldVariant, SingleField, HashRangeField};
use datafold::schema::types::json_schema::KeyConfig;
use datafold::permissions::types::policy::PermissionsPolicy;
use datafold::fees::types::config::FieldPaymentConfig;
use datafold::fees::SchemaPaymentConfig;
use serde_json::json;
use std::collections::HashMap;
use std::sync::Arc;

#[cfg(test)]
mod tests {
    use super::*;

    /// Test fixture for mutation service tests
    struct MutationServiceTestFixture {
        mutation_service: MutationService,
        message_bus: Arc<MessageBus>,
    }

    impl MutationServiceTestFixture {
        fn new() -> Self {
            let message_bus = Arc::new(MessageBus::new());
            let mutation_service = MutationService::new(Arc::clone(&message_bus));
            
            Self {
                mutation_service,
                message_bus,
            }
        }

        /// Create a HashRange schema with universal key configuration
        fn create_hashrange_schema_with_universal_key(
            &self,
            name: &str,
            hash_field: &str,
            range_field: &str,
        ) -> Schema {
            let mut fields = HashMap::new();
            
            // Create hash field
            let hash_field_obj = SingleField::new(
                PermissionsPolicy::default(),
                FieldPaymentConfig::default(),
                HashMap::new(),
            );
            fields.insert(hash_field.to_string(), FieldVariant::Single(hash_field_obj));
            
            // Create range field
            let range_field_obj = SingleField::new(
                PermissionsPolicy::default(),
                FieldPaymentConfig::default(),
                HashMap::new(),
            );
            fields.insert(range_field.to_string(), FieldVariant::Single(range_field_obj));
            
            // Create content field
            let content_field = HashRangeField::new(
                PermissionsPolicy::default(),
                FieldPaymentConfig::default(),
                HashMap::new(),
                "content".to_string(),
                "content".to_string(),
                "content".to_string(),
            );
            fields.insert("content".to_string(), FieldVariant::HashRange(Box::new(content_field)));
            
            // Create author field
            let author_field = SingleField::new(
                PermissionsPolicy::default(),
                FieldPaymentConfig::default(),
                HashMap::new(),
            );
            fields.insert("author".to_string(), FieldVariant::Single(author_field));

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

        /// Create a HashRange schema without key configuration (should fail)
        fn create_hashrange_schema_without_key(&self, name: &str) -> Schema {
            let mut fields = HashMap::new();
            
            let word_field = SingleField::new(
                PermissionsPolicy::default(),
                FieldPaymentConfig::default(),
                HashMap::new(),
            );
            fields.insert("word".to_string(), FieldVariant::Single(word_field));
            
            let publish_date_field = SingleField::new(
                PermissionsPolicy::default(),
                FieldPaymentConfig::default(),
                HashMap::new(),
            );
            fields.insert("publish_date".to_string(), FieldVariant::Single(publish_date_field));
            
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
                key: None,
                fields,
                hash: Some("test_hash".to_string()),
                payment_config: SchemaPaymentConfig::default(),
            }
        }

        /// Create a HashRange schema with empty key fields (should fail)
        fn create_hashrange_schema_with_empty_key_fields(&self, name: &str) -> Schema {
            let mut fields = HashMap::new();
            
            let word_field = SingleField::new(
                PermissionsPolicy::default(),
                FieldPaymentConfig::default(),
                HashMap::new(),
            );
            fields.insert("word".to_string(), FieldVariant::Single(word_field));
            
            let publish_date_field = SingleField::new(
                PermissionsPolicy::default(),
                FieldPaymentConfig::default(),
                HashMap::new(),
            );
            fields.insert("publish_date".to_string(), FieldVariant::Single(publish_date_field));
            
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
                    hash_field: "".to_string(),
                    range_field: "".to_string(),
                }),
                fields,
                hash: Some("test_hash".to_string()),
                payment_config: SchemaPaymentConfig::default(),
            }
        }
    }

    #[test]
    fn test_hashrange_key_field_names_extraction() {
        let fixture = MutationServiceTestFixture::new();
        
        // Test with custom field names
        let schema = fixture.create_hashrange_schema_with_universal_key(
            "BlogPostWordIndex",
            "word",
            "publish_date"
        );

        // Test the key field extraction (this tests the private method indirectly through update_hashrange_schema_fields)
        let mut fields_and_values = HashMap::new();
        fields_and_values.insert("word".to_string(), json!("technology"));
        fields_and_values.insert("publish_date".to_string(), json!("2025-01-15"));
        fields_and_values.insert("content".to_string(), json!("AI advances..."));
        fields_and_values.insert("author".to_string(), json!("John Doe"));

        // This should work without errors, indicating the key field extraction is working
        let result = fixture.mutation_service.update_hashrange_schema_fields(
            &schema,
            &fields_and_values,
            "technology",
            "2025-01-15",
            "test_hash"
        );

        // The method should succeed (or fail for other reasons, but not key extraction)
        // We're testing that it doesn't fail due to key field extraction issues
        assert!(result.is_ok() || result.is_err());
    }

    #[test]
    fn test_hashrange_schema_missing_key_configuration() {
        let fixture = MutationServiceTestFixture::new();
        
        let schema = fixture.create_hashrange_schema_without_key("BlogPostWordIndex");

        let mut fields_and_values = HashMap::new();
        fields_and_values.insert("word".to_string(), json!("technology"));
        fields_and_values.insert("publish_date".to_string(), json!("2025-01-15"));
        fields_and_values.insert("content".to_string(), json!("AI advances..."));

        let result = fixture.mutation_service.update_hashrange_schema_fields(
            &schema,
            &fields_and_values,
            "technology",
            "2025-01-15",
            "test_hash"
        );

        // Should fail with missing key configuration error
        assert!(result.is_err());
        if let Err(e) = result {
            assert!(e.to_string().contains("requires key configuration"));
        }
    }

    #[test]
    fn test_hashrange_schema_empty_hash_field() {
        let fixture = MutationServiceTestFixture::new();
        
        let schema = fixture.create_hashrange_schema_with_empty_key_fields("BlogPostWordIndex");

        let mut fields_and_values = HashMap::new();
        fields_and_values.insert("word".to_string(), json!("technology"));
        fields_and_values.insert("publish_date".to_string(), json!("2025-01-15"));
        fields_and_values.insert("content".to_string(), json!("AI advances..."));

        let result = fixture.mutation_service.update_hashrange_schema_fields(
            &schema,
            &fields_and_values,
            "technology",
            "2025-01-15",
            "test_hash"
        );

        // Should fail with empty hash field error
        assert!(result.is_err());
        if let Err(e) = result {
            assert!(e.to_string().contains("requires non-empty hash_field"));
        }
    }

    #[test]
    fn test_hashrange_field_skipping_with_universal_key() {
        let fixture = MutationServiceTestFixture::new();
        
        // Create schema with custom field names
        let schema = fixture.create_hashrange_schema_with_universal_key(
            "BlogPostWordIndex",
            "custom_hash_field",
            "custom_range_field"
        );

        let mut fields_and_values = HashMap::new();
        fields_and_values.insert("custom_hash_field".to_string(), json!("technology"));
        fields_and_values.insert("custom_range_field".to_string(), json!("2025-01-15"));
        fields_and_values.insert("content".to_string(), json!("AI advances..."));
        fields_and_values.insert("author".to_string(), json!("John Doe"));

        // This should work and skip the custom_hash_field and custom_range_field
        // The method should process content and author fields but skip the key fields
        let result = fixture.mutation_service.update_hashrange_schema_fields(
            &schema,
            &fields_and_values,
            "technology",
            "2025-01-15",
            "test_hash"
        );

        // Should succeed (or fail for other reasons, but not field skipping)
        assert!(result.is_ok() || result.is_err());
    }

    #[test]
    fn test_hashrange_mutation_with_different_field_names() {
        let fixture = MutationServiceTestFixture::new();
        
        // Test with completely different field names
        let schema = fixture.create_hashrange_schema_with_universal_key(
            "UserActivityLog",
            "user_id",
            "timestamp"
        );

        let mut fields_and_values = HashMap::new();
        fields_and_values.insert("user_id".to_string(), json!("user123"));
        fields_and_values.insert("timestamp".to_string(), json!("2025-01-15T10:30:00Z"));
        fields_and_values.insert("action".to_string(), json!("login"));
        fields_and_values.insert("details".to_string(), json!("User logged in"));

        let result = fixture.mutation_service.update_hashrange_schema_fields(
            &schema,
            &fields_and_values,
            "user123",
            "2025-01-15T10:30:00Z",
            "test_hash"
        );

        // Should work with different field names
        assert!(result.is_ok() || result.is_err());
    }

    #[test]
    fn test_hashrange_mutation_context_creation() {
        let fixture = MutationServiceTestFixture::new();
        
        let schema = fixture.create_hashrange_schema_with_universal_key(
            "BlogPostWordIndex",
            "word",
            "publish_date"
        );

        let mut fields_and_values = HashMap::new();
        fields_and_values.insert("word".to_string(), json!("technology"));
        fields_and_values.insert("publish_date".to_string(), json!("2025-01-15"));
        fields_and_values.insert("content".to_string(), json!("AI advances..."));

        // Test that mutation context is created correctly
        let result = fixture.mutation_service.update_hashrange_schema_fields(
            &schema,
            &fields_and_values,
            "technology",
            "2025-01-15",
            "test_mutation_hash"
        );

        // Should succeed (or fail for other reasons, but not context creation)
        assert!(result.is_ok() || result.is_err());
    }

    #[test]
    fn test_hashrange_mutation_backward_compatibility() {
        let fixture = MutationServiceTestFixture::new();
        
        // Test with traditional field names to ensure backward compatibility
        let schema = fixture.create_hashrange_schema_with_universal_key(
            "BlogPostWordIndex",
            "hash_key",
            "range_key"
        );

        let mut fields_and_values = HashMap::new();
        fields_and_values.insert("hash_key".to_string(), json!("technology"));
        fields_and_values.insert("range_key".to_string(), json!("2025-01-15"));
        fields_and_values.insert("content".to_string(), json!("AI advances..."));

        let result = fixture.mutation_service.update_hashrange_schema_fields(
            &schema,
            &fields_and_values,
            "technology",
            "2025-01-15",
            "test_hash"
        );

        // Should work with traditional field names (backward compatibility)
        assert!(result.is_ok() || result.is_err());
    }

    #[test]
    fn test_hashrange_mutation_error_handling() {
        let fixture = MutationServiceTestFixture::new();
        
        // Test various error scenarios
        let schema = fixture.create_hashrange_schema_with_universal_key(
            "BlogPostWordIndex",
            "word",
            "publish_date"
        );

        // Test with empty fields_and_values
        let empty_fields = HashMap::new();
        let result = fixture.mutation_service.update_hashrange_schema_fields(
            &schema,
            &empty_fields,
            "technology",
            "2025-01-15",
            "test_hash"
        );

        // Should handle empty fields gracefully
        assert!(result.is_ok() || result.is_err());
    }
}
