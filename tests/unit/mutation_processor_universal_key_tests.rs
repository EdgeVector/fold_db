//! Unit tests for MutationProcessor universal key configuration functionality
//!
//! This module contains comprehensive tests for the mutation processor's
//! universal key configuration support, including HashRange, Range, and Single
//! schema types with various key configurations.

use datafold::db_operations::DbOperations;
use datafold::fees::types::config::FieldPaymentConfig;
use datafold::fold_db_core::infrastructure::message_bus::MessageBus;
use datafold::fold_db_core::mutation::MutationProcessor;
use datafold::fold_db_core::services::mutation::MutationService;
use datafold::permissions::types::policy::PermissionsPolicy;
use datafold::schema::types::field::{FieldVariant, SingleField};
use datafold::schema::types::json_schema::KeyConfig;
use datafold::schema::types::{Mutation, MutationType, Schema, SchemaType};
use datafold::schema::{Field, SchemaCore};
use serde_json::{json, Value};
use sled;
use std::collections::HashMap;
use std::sync::Arc;
use tempfile::TempDir;

/// Test fixture for mutation processor universal key tests
struct MutationProcessorTestFixture {
    mutation_processor: MutationProcessor,
    mutation_service: MutationService,
    temp_dir: TempDir,
}

impl MutationProcessorTestFixture {
    fn new() -> Result<Self, Box<dyn std::error::Error>> {
        let temp_dir = tempfile::tempdir()?;
        let db_path = temp_dir.path().to_str().unwrap();

        // Create sled database
        let sled_db = sled::open(db_path)?;

        // Create database operations
        let db_ops = Arc::new(DbOperations::new(sled_db)?);

        // Create message bus
        let message_bus = Arc::new(MessageBus::new());

        // Create schema core
        let schema_core = Arc::new(SchemaCore::new(
            db_path,
            db_ops.clone(),
            Arc::clone(&message_bus),
        )?);

        // Create mutation processor
        let mutation_processor = MutationProcessor::new(Arc::clone(&schema_core));

        // Create mutation service
        let mutation_service = MutationService::new(message_bus);

        Ok(Self {
            mutation_processor,
            mutation_service,
            temp_dir,
        })
    }

    /// Create a HashRange schema with universal key configuration
    fn create_hashrange_schema_with_universal_key(
        &self,
        hash_field: &str,
        range_field: &str,
    ) -> Schema {
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

        let mut fields = HashMap::new();
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
            payment_config: datafold::fees::payment_config::SchemaPaymentConfig::default(),
            hash: None,
        }
    }

    /// Create a Range schema with universal key configuration
    fn create_range_schema_with_universal_key(&self, range_field: &str) -> Schema {
        let mut range_field_obj = SingleField::new(
            PermissionsPolicy::default(),
            FieldPaymentConfig::default(),
            HashMap::new(),
        );
        range_field_obj.set_molecule_uuid("user.map().timestamp.$atom_uuid".to_string());

        let mut name_field = SingleField::new(
            PermissionsPolicy::default(),
            FieldPaymentConfig::default(),
            HashMap::new(),
        );
        name_field.set_molecule_uuid("user.map().name.$atom_uuid".to_string());

        let mut fields = HashMap::new();
        fields.insert(
            range_field.to_string(),
            FieldVariant::Single(range_field_obj),
        );
        fields.insert("name".to_string(), FieldVariant::Single(name_field));

        Schema {
            name: "TestRangeSchema".to_string(),
            schema_type: SchemaType::Range {
                range_key: range_field.to_string(),
            },
            key: Some(KeyConfig {
                hash_field: "".to_string(),
                range_field: range_field.to_string(),
            }),
            fields,
            payment_config: datafold::fees::payment_config::SchemaPaymentConfig::default(),
            hash: None,
        }
    }

    /// Create a Range schema with legacy range_key (no universal key)
    fn create_range_schema_legacy(&self, range_key: &str) -> Schema {
        let mut range_field_obj = SingleField::new(
            PermissionsPolicy::default(),
            FieldPaymentConfig::default(),
            HashMap::new(),
        );
        range_field_obj.set_molecule_uuid("user.map().timestamp.$atom_uuid".to_string());

        let mut name_field = SingleField::new(
            PermissionsPolicy::default(),
            FieldPaymentConfig::default(),
            HashMap::new(),
        );
        name_field.set_molecule_uuid("user.map().name.$atom_uuid".to_string());

        let mut fields = HashMap::new();
        fields.insert(range_key.to_string(), FieldVariant::Single(range_field_obj));
        fields.insert("name".to_string(), FieldVariant::Single(name_field));

        Schema {
            name: "TestRangeSchemaLegacy".to_string(),
            schema_type: SchemaType::Range {
                range_key: range_key.to_string(),
            },
            key: None, // No universal key configuration
            fields,
            payment_config: datafold::fees::payment_config::SchemaPaymentConfig::default(),
            hash: None,
        }
    }

    /// Create a Single schema with optional universal key
    fn create_single_schema_with_universal_key(
        &self,
        hash_field: &str,
        range_field: &str,
    ) -> Schema {
        let mut id_field = SingleField::new(
            PermissionsPolicy::default(),
            FieldPaymentConfig::default(),
            HashMap::new(),
        );
        id_field.set_molecule_uuid("user.map().id.$atom_uuid".to_string());

        let mut name_field = SingleField::new(
            PermissionsPolicy::default(),
            FieldPaymentConfig::default(),
            HashMap::new(),
        );
        name_field.set_molecule_uuid("user.map().name.$atom_uuid".to_string());

        let mut fields = HashMap::new();
        fields.insert("id".to_string(), FieldVariant::Single(id_field));
        fields.insert("name".to_string(), FieldVariant::Single(name_field));

        Schema {
            name: "TestSingleSchema".to_string(),
            schema_type: SchemaType::Single,
            key: Some(KeyConfig {
                hash_field: hash_field.to_string(),
                range_field: range_field.to_string(),
            }),
            fields,
            payment_config: datafold::fees::payment_config::SchemaPaymentConfig::default(),
            hash: None,
        }
    }

    /// Create a mutation with specified fields and values
    fn create_mutation(
        &self,
        schema_name: &str,
        fields_and_values: HashMap<String, Value>,
    ) -> Mutation {
        Mutation {
            schema_name: schema_name.to_string(),
            mutation_type: MutationType::Update,
            fields_and_values,
            pub_key: "test_key".to_string(),
            synchronous: Some(false),
            trust_distance: 0,
        }
    }
}

#[test]
fn test_hashrange_schema_missing_key_configuration() -> Result<(), Box<dyn std::error::Error>> {
    println!("🧪 Testing HashRange schema mutation with missing key configuration...");

    let fixture = MutationProcessorTestFixture::new()?;

    // Create HashRange schema without key configuration
    let mut schema =
        fixture.create_hashrange_schema_with_universal_key("hash_field", "range_field");
    schema.key = None; // Remove key configuration

    // Create mutation
    let mut fields_and_values = HashMap::new();
    fields_and_values.insert("hash_field".to_string(), json!("user123"));
    fields_and_values.insert("range_field".to_string(), json!("2025-01-01"));
    fields_and_values.insert("blog_id".to_string(), json!("blog456"));

    let mutation = fixture.create_mutation("TestHashRangeSchema", fields_and_values);

    // Test should fail with clear error message
    let result = fixture
        .mutation_processor
        .process_field_mutations_via_service(
            &fixture.mutation_service,
            &schema,
            &mutation,
            "test_hash",
        );

    assert!(result.is_err());
    let error_msg = result.unwrap_err().to_string();
    assert!(error_msg.contains("HashRange schema 'TestHashRangeSchema' requires key configuration"));

    println!("  ✅ HashRange schema mutation with missing key configuration properly fails");
    Ok(())
}

#[test]
fn test_hashrange_schema_empty_key_fields() -> Result<(), Box<dyn std::error::Error>> {
    println!("🧪 Testing HashRange schema mutation with empty key fields...");

    let fixture = MutationProcessorTestFixture::new()?;

    // Create HashRange schema with empty key fields
    let schema = fixture.create_hashrange_schema_with_universal_key("", "");

    // Create mutation
    let mut fields_and_values = HashMap::new();
    fields_and_values.insert("hash_field".to_string(), json!("user123"));
    fields_and_values.insert("range_field".to_string(), json!("2025-01-01"));
    fields_and_values.insert("blog_id".to_string(), json!("blog456"));

    let mutation = fixture.create_mutation("TestHashRangeSchema", fields_and_values);

    // Test should fail with clear error message
    let result = fixture
        .mutation_processor
        .process_field_mutations_via_service(
            &fixture.mutation_service,
            &schema,
            &mutation,
            "test_hash",
        );

    assert!(result.is_err());
    let error_msg = result.unwrap_err().to_string();
    assert!(
        error_msg.contains("HashRange schema 'TestHashRangeSchema' requires non-empty hash_field")
    );

    println!("  ✅ HashRange schema mutation with empty key fields properly fails");
    Ok(())
}

#[test]
fn test_hashrange_schema_valid_key_configuration() -> Result<(), Box<dyn std::error::Error>> {
    println!("🧪 Testing HashRange schema mutation with valid key configuration...");

    let fixture = MutationProcessorTestFixture::new()?;

    // Create HashRange schema with custom field names
    let schema = fixture.create_hashrange_schema_with_universal_key("user_id", "timestamp");

    // Create mutation with custom field names
    let mut fields_and_values = HashMap::new();
    fields_and_values.insert("user_id".to_string(), json!("user123"));
    fields_and_values.insert("timestamp".to_string(), json!("2025-01-01"));
    fields_and_values.insert("blog_id".to_string(), json!("blog456"));

    let mutation = fixture.create_mutation("TestHashRangeSchema", fields_and_values);

    // Test should succeed
    let result = fixture
        .mutation_processor
        .process_field_mutations_via_service(
            &fixture.mutation_service,
            &schema,
            &mutation,
            "test_hash",
        );

    assert!(result.is_ok());

    println!("  ✅ HashRange schema mutation with valid key configuration succeeds");
    Ok(())
}

#[test]
fn test_range_schema_with_universal_key() -> Result<(), Box<dyn std::error::Error>> {
    println!("🧪 Testing Range schema mutation with universal key configuration...");

    let fixture = MutationProcessorTestFixture::new()?;

    // Create Range schema with universal key
    let schema = fixture.create_range_schema_with_universal_key("created_at");

    // Create mutation
    let mut fields_and_values = HashMap::new();
    fields_and_values.insert("created_at".to_string(), json!("2025-01-01"));
    fields_and_values.insert("name".to_string(), json!("John Doe"));

    let mutation = fixture.create_mutation("TestRangeSchema", fields_and_values);

    // Test should succeed
    let result = fixture
        .mutation_processor
        .process_field_mutations_via_service(
            &fixture.mutation_service,
            &schema,
            &mutation,
            "test_hash",
        );

    assert!(result.is_ok());

    println!("  ✅ Range schema mutation with universal key configuration succeeds");
    Ok(())
}

#[test]
fn test_range_schema_legacy_backward_compatibility() -> Result<(), Box<dyn std::error::Error>> {
    println!("🧪 Testing Range schema mutation with legacy range_key (backward compatibility)...");

    let fixture = MutationProcessorTestFixture::new()?;

    // Create Range schema with legacy range_key
    let schema = fixture.create_range_schema_legacy("timestamp");

    // Create mutation
    let mut fields_and_values = HashMap::new();
    fields_and_values.insert("timestamp".to_string(), json!("2025-01-01"));
    fields_and_values.insert("name".to_string(), json!("John Doe"));

    let mutation = fixture.create_mutation("TestRangeSchemaLegacy", fields_and_values);

    // Test should succeed (backward compatibility)
    let result = fixture
        .mutation_processor
        .process_field_mutations_via_service(
            &fixture.mutation_service,
            &schema,
            &mutation,
            "test_hash",
        );

    assert!(result.is_ok());

    println!("  ✅ Range schema mutation with legacy range_key succeeds (backward compatibility)");
    Ok(())
}

#[test]
fn test_single_schema_with_universal_key() -> Result<(), Box<dyn std::error::Error>> {
    println!("🧪 Testing Single schema mutation with universal key configuration...");

    let fixture = MutationProcessorTestFixture::new()?;

    // Create Single schema with universal key
    let schema = fixture.create_single_schema_with_universal_key("user_id", "created_at");

    // Create mutation
    let mut fields_and_values = HashMap::new();
    fields_and_values.insert("id".to_string(), json!("user123"));
    fields_and_values.insert("name".to_string(), json!("John Doe"));

    let mutation = fixture.create_mutation("TestSingleSchema", fields_and_values);

    // Test should succeed
    let result = fixture
        .mutation_processor
        .process_field_mutations_via_service(
            &fixture.mutation_service,
            &schema,
            &mutation,
            "test_hash",
        );

    assert!(result.is_ok());

    println!("  ✅ Single schema mutation with universal key configuration succeeds");
    Ok(())
}

#[test]
fn test_hashrange_schema_missing_hash_field() -> Result<(), Box<dyn std::error::Error>> {
    println!("🧪 Testing HashRange schema mutation with missing hash field...");

    let fixture = MutationProcessorTestFixture::new()?;

    // Create HashRange schema with custom field names
    let schema = fixture.create_hashrange_schema_with_universal_key("user_id", "timestamp");

    // Create mutation missing the hash field
    let mut fields_and_values = HashMap::new();
    fields_and_values.insert("timestamp".to_string(), json!("2025-01-01"));
    fields_and_values.insert("blog_id".to_string(), json!("blog456"));

    let mutation = fixture.create_mutation("TestHashRangeSchema", fields_and_values);

    // Test should fail with clear error message
    let result = fixture
        .mutation_processor
        .process_field_mutations_via_service(
            &fixture.mutation_service,
            &schema,
            &mutation,
            "test_hash",
        );

    assert!(result.is_err());
    let error_msg = result.unwrap_err().to_string();
    assert!(error_msg.contains("HashRange schema mutation missing hash field 'user_id'"));

    println!("  ✅ HashRange schema mutation with missing hash field properly fails");
    Ok(())
}

#[test]
fn test_hashrange_schema_missing_range_field() -> Result<(), Box<dyn std::error::Error>> {
    println!("🧪 Testing HashRange schema mutation with missing range field...");

    let fixture = MutationProcessorTestFixture::new()?;

    // Create HashRange schema with custom field names
    let schema = fixture.create_hashrange_schema_with_universal_key("user_id", "timestamp");

    // Create mutation missing the range field
    let mut fields_and_values = HashMap::new();
    fields_and_values.insert("user_id".to_string(), json!("user123"));
    fields_and_values.insert("blog_id".to_string(), json!("blog456"));

    let mutation = fixture.create_mutation("TestHashRangeSchema", fields_and_values);

    // Test should fail with clear error message
    let result = fixture
        .mutation_processor
        .process_field_mutations_via_service(
            &fixture.mutation_service,
            &schema,
            &mutation,
            "test_hash",
        );

    assert!(result.is_err());
    let error_msg = result.unwrap_err().to_string();
    assert!(error_msg.contains("HashRange schema mutation missing range field 'timestamp'"));

    println!("  ✅ HashRange schema mutation with missing range field properly fails");
    Ok(())
}

#[test]
fn test_range_schema_missing_range_field() -> Result<(), Box<dyn std::error::Error>> {
    println!("🧪 Testing Range schema mutation with missing range field...");

    let fixture = MutationProcessorTestFixture::new()?;

    // Create Range schema with universal key
    let schema = fixture.create_range_schema_with_universal_key("created_at");

    // Create mutation missing the range field
    let mut fields_and_values = HashMap::new();
    fields_and_values.insert("name".to_string(), json!("John Doe"));

    let mutation = fixture.create_mutation("TestRangeSchema", fields_and_values);

    // Test should fail with clear error message
    let result = fixture
        .mutation_processor
        .process_field_mutations_via_service(
            &fixture.mutation_service,
            &schema,
            &mutation,
            "test_hash",
        );

    assert!(result.is_err());
    let error_msg = result.unwrap_err().to_string();
    assert!(error_msg.contains("Range schema mutation missing range field 'created_at'"));

    println!("  ✅ Range schema mutation with missing range field properly fails");
    Ok(())
}

#[test]
fn test_value_extraction_different_types() -> Result<(), Box<dyn std::error::Error>> {
    println!("🧪 Testing value extraction with different JSON types...");

    let fixture = MutationProcessorTestFixture::new()?;

    // Create HashRange schema
    let schema = fixture.create_hashrange_schema_with_universal_key("user_id", "timestamp");

    // Test with different value types
    let test_cases = vec![
        ("String", json!("user123")),
        ("Number", json!(12345)),
        ("Boolean", json!(true)),
        ("Null", json!(null)),
    ];

    for (value_type, value) in test_cases {
        let mut fields_and_values = HashMap::new();
        fields_and_values.insert("user_id".to_string(), value.clone());
        fields_and_values.insert("timestamp".to_string(), json!("2025-01-01"));
        fields_and_values.insert("blog_id".to_string(), json!("blog456"));

        let mutation = fixture.create_mutation("TestHashRangeSchema", fields_and_values);

        // Test should succeed with different value types
        let result = fixture
            .mutation_processor
            .process_field_mutations_via_service(
                &fixture.mutation_service,
                &schema,
                &mutation,
                "test_hash",
            );

        assert!(result.is_ok(), "Failed for {} value type", value_type);
    }

    println!("  ✅ Value extraction with different JSON types succeeds");
    Ok(())
}
