use datafold::fees::payment_config::SchemaPaymentConfig;
use datafold::fees::types::config::FieldPaymentConfig;
use datafold::permissions::types::policy::PermissionsPolicy;
use datafold::schema::types::field::single_field::SingleField;
use datafold::schema::types::field::FieldVariant;
use datafold::schema::types::json_schema::KeyConfig;
use datafold::schema::types::{Schema, SchemaType};
use datafold::schema::Field;
use std::collections::HashMap;

/// Test fixture for HashRange schema validation tests
struct HashRangeSchemaTestFixture;

impl HashRangeSchemaTestFixture {
    fn new() -> Self {
        Self
    }

    /// Create a HashRange schema with universal key configuration
    fn create_hashrange_schema_with_universal_key(
        &self,
        hash_field: &str,
        range_field: &str,
    ) -> Schema {
        let mut fields = HashMap::new();

        // Create fields for the schema
        let mut blog_field = SingleField::new(
            PermissionsPolicy::default(),
            FieldPaymentConfig::default(),
            HashMap::new(),
        );
        blog_field.set_molecule_uuid("blogpost.map().$atom_uuid".to_string());

        let mut author_field = SingleField::new(
            PermissionsPolicy::default(),
            FieldPaymentConfig::default(),
            HashMap::new(),
        );
        author_field.set_molecule_uuid("blogpost.map().author.$atom_uuid".to_string());

        let mut title_field = SingleField::new(
            PermissionsPolicy::default(),
            FieldPaymentConfig::default(),
            HashMap::new(),
        );
        title_field.set_molecule_uuid("blogpost.map().title.$atom_uuid".to_string());

        fields.insert("blog_id".to_string(), FieldVariant::Single(blog_field));
        fields.insert("author_id".to_string(), FieldVariant::Single(author_field));
        fields.insert("title".to_string(), FieldVariant::Single(title_field));

        Schema {
            name: "TestHashRangeSchema".to_string(),
            schema_type: SchemaType::HashRange,
            key: Some(KeyConfig {
                hash_field: hash_field.to_string(),
                range_field: range_field.to_string(),
            }),
            fields,
            payment_config: SchemaPaymentConfig::default(),
            hash: None,
        }
    }

    /// Create a HashRange schema without key configuration (should fail)
    fn create_hashrange_schema_without_key(&self) -> Schema {
        let mut fields = HashMap::new();

        let mut blog_field = SingleField::new(
            PermissionsPolicy::default(),
            FieldPaymentConfig::default(),
            HashMap::new(),
        );
        blog_field.set_molecule_uuid("blogpost.map().$atom_uuid".to_string());

        fields.insert("blog_id".to_string(), FieldVariant::Single(blog_field));

        Schema {
            name: "TestHashRangeSchemaNoKey".to_string(),
            schema_type: SchemaType::HashRange,
            key: None, // No key configuration
            fields,
            payment_config: SchemaPaymentConfig::default(),
            hash: None,
        }
    }

    /// Create a HashRange schema with empty key fields (should fail)
    fn create_hashrange_schema_with_empty_key_fields(&self) -> Schema {
        let mut fields = HashMap::new();

        let mut blog_field = SingleField::new(
            PermissionsPolicy::default(),
            FieldPaymentConfig::default(),
            HashMap::new(),
        );
        blog_field.set_molecule_uuid("blogpost.map().$atom_uuid".to_string());

        fields.insert("blog_id".to_string(), FieldVariant::Single(blog_field));

        Schema {
            name: "TestHashRangeSchemaEmptyKey".to_string(),
            schema_type: SchemaType::HashRange,
            key: Some(KeyConfig {
                hash_field: "".to_string(),  // Empty hash field
                range_field: "".to_string(), // Empty range field
            }),
            fields,
            payment_config: SchemaPaymentConfig::default(),
            hash: None,
        }
    }

    /// Validate that a HashRange schema has proper key configuration
    fn validate_hashrange_key_configuration(&self, schema: &Schema) -> Result<(), String> {
        // Check if schema is HashRange type
        if !matches!(schema.schema_type, SchemaType::HashRange) {
            return Err("Schema is not HashRange type".to_string());
        }

        // Check if key configuration exists
        let key_config = schema.key.as_ref().ok_or_else(|| {
            format!(
                "HashRange schema '{}' requires key configuration",
                schema.name
            )
        })?;

        // Check if hash_field is not empty
        if key_config.hash_field.trim().is_empty() {
            return Err(format!(
                "HashRange schema '{}' requires non-empty hash_field in key configuration",
                schema.name
            ));
        }

        // Check if range_field is not empty
        if key_config.range_field.trim().is_empty() {
            return Err(format!(
                "HashRange schema '{}' requires non-empty range_field in key configuration",
                schema.name
            ));
        }

        Ok(())
    }
}

#[test]
fn test_hashrange_schema_missing_key_configuration() -> Result<(), Box<dyn std::error::Error>> {
    println!("🧪 Testing HashRange schema validation with missing key configuration...");

    let fixture = HashRangeSchemaTestFixture::new();

    // Create schema without key configuration
    let schema = fixture.create_hashrange_schema_without_key();

    // Validation should fail with clear error message
    let result = fixture.validate_hashrange_key_configuration(&schema);

    assert!(result.is_err());
    let error_msg = result.unwrap_err();
    assert!(error_msg.contains("requires key configuration"));
    assert!(error_msg.contains("TestHashRangeSchemaNoKey"));

    println!("  ✅ HashRange schema validation with missing key configuration properly fails");
    Ok(())
}

#[test]
fn test_hashrange_schema_empty_key_fields() -> Result<(), Box<dyn std::error::Error>> {
    println!("🧪 Testing HashRange schema validation with empty key fields...");

    let fixture = HashRangeSchemaTestFixture::new();

    // Create schema with empty key fields
    let schema = fixture.create_hashrange_schema_with_empty_key_fields();

    // Validation should fail with clear error message
    let result = fixture.validate_hashrange_key_configuration(&schema);

    assert!(result.is_err());
    let error_msg = result.unwrap_err();
    assert!(error_msg.contains("requires non-empty hash_field"));
    assert!(error_msg.contains("TestHashRangeSchemaEmptyKey"));

    println!("  ✅ HashRange schema validation with empty key fields properly fails");
    Ok(())
}

#[test]
fn test_hashrange_schema_valid_key_configuration() -> Result<(), Box<dyn std::error::Error>> {
    println!("🧪 Testing HashRange schema validation with valid key configuration...");

    let fixture = HashRangeSchemaTestFixture::new();

    // Create schema with valid universal key
    let schema = fixture.create_hashrange_schema_with_universal_key("blog_id", "created_at");

    // Validation should succeed
    let result = fixture.validate_hashrange_key_configuration(&schema);

    assert!(result.is_ok());

    // Verify the key configuration values
    let key_config = schema.key.unwrap();
    assert_eq!(key_config.hash_field, "blog_id");
    assert_eq!(key_config.range_field, "created_at");

    println!("  ✅ HashRange schema validation with valid key configuration succeeds");
    Ok(())
}
