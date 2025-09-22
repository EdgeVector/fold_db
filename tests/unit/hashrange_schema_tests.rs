use datafold::fees::payment_config::SchemaPaymentConfig;
use datafold::fees::types::config::FieldPaymentConfig;
use datafold::permissions::types::policy::PermissionsPolicy;
use datafold::schema::core::SchemaCore;
use datafold::schema::types::field::hash_range_field::HashRangeField;
use datafold::schema::types::field::single_field::SingleField;
use datafold::schema::types::field::FieldVariant;
use datafold::schema::types::json_schema::DeclarativeSchemaDefinition;
use datafold::schema::types::{Schema, SchemaType};
use datafold::schema::Field;
use serde_json::Value;
use std::collections::HashMap;

/// Test fixture for HashRange schema tests
use tempfile::TempDir;

struct HashRangeTestFixture {
    schema_core: SchemaCore,
    _temp_dir: TempDir, // Keep temp directory alive
}

impl HashRangeTestFixture {
    fn new() -> Result<Self, Box<dyn std::error::Error>> {
        // Create unique temporary directory for each test to avoid database lock conflicts
        let temp_dir = tempfile::tempdir()?;
        let db_path = temp_dir.path().join("test_db");
        let schema_core = SchemaCore::new_for_testing(db_path.to_str().unwrap())?;
        Ok(Self {
            schema_core,
            _temp_dir: temp_dir,
        })
    }

    /// Create a HashRange schema with Single field variants (simulating loaded JSON schema)
    fn create_hashrange_schema_with_single_fields(&self) -> Schema {
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

        let mut tags_field = SingleField::new(
            PermissionsPolicy::default(),
            FieldPaymentConfig::default(),
            HashMap::new(),
        );
        tags_field.set_molecule_uuid("blogpost.map().tags.$atom_uuid".to_string());

        let mut fields = HashMap::new();
        fields.insert("blog".to_string(), FieldVariant::Single(blog_field));
        fields.insert("author".to_string(), FieldVariant::Single(author_field));
        fields.insert("title".to_string(), FieldVariant::Single(title_field));
        fields.insert("tags".to_string(), FieldVariant::Single(tags_field));

        Schema {
            name: "BlogPostWordIndex".to_string(),
            schema_type: SchemaType::HashRange,
            key: None,
            fields,
            payment_config: SchemaPaymentConfig::default(),
            hash: None,
        }
    }

    /// Create a HashRange schema with actual HashRange field variants
    fn create_hashrange_schema_with_hashrange_fields(&self) -> Schema {
        let blog_field = HashRangeField {
            inner: datafold::schema::types::field::common::FieldCommon::new(
                PermissionsPolicy::default(),
                FieldPaymentConfig::default(),
                HashMap::new(),
            ),
            hash_field: "blogpost.map().fields.content.split_by_word().map()".to_string(),
            range_field: "blogpost.map().fields.publish_date".to_string(),
            atom_uuid: "blogpost.map().$atom_uuid".to_string(),
            cached_chains: None,
        };

        let author_field = HashRangeField {
            inner: datafold::schema::types::field::common::FieldCommon::new(
                PermissionsPolicy::default(),
                FieldPaymentConfig::default(),
                HashMap::new(),
            ),
            hash_field: "blogpost.map().fields.content.split_by_word().map()".to_string(),
            range_field: "blogpost.map().fields.publish_date".to_string(),
            atom_uuid: "blogpost.map().author.$atom_uuid".to_string(),
            cached_chains: None,
        };

        let mut fields = HashMap::new();
        fields.insert(
            "blog".to_string(),
            FieldVariant::HashRange(Box::new(blog_field)),
        );
        fields.insert(
            "author".to_string(),
            FieldVariant::HashRange(Box::new(author_field)),
        );

        Schema {
            name: "BlogPostWordIndex".to_string(),
            schema_type: SchemaType::HashRange,
            key: None,
            fields,
            payment_config: SchemaPaymentConfig::default(),
            hash: None,
        }
    }

    /// Create a Single schema for comparison
    fn create_single_schema(&self) -> Schema {
        let mut blog_field = SingleField::new(
            PermissionsPolicy::default(),
            FieldPaymentConfig::default(),
            HashMap::new(),
        );
        blog_field.set_molecule_uuid("blogpost.map().$atom_uuid".to_string());

        let mut fields = HashMap::new();
        fields.insert("blog".to_string(), FieldVariant::Single(blog_field));

        Schema {
            name: "BlogPost".to_string(),
            schema_type: SchemaType::Single,
            key: None,
            fields,
            payment_config: SchemaPaymentConfig::default(),
            hash: None,
        }
    }
}

#[test]
fn test_hashrange_schema_declarative_definition_conversion() {
    let fixture = HashRangeTestFixture::new().expect("Failed to create test fixture");
    let schema = fixture.create_hashrange_schema_with_hashrange_fields();

    println!("🔧 Testing HashRange schema declarative definition conversion");

    // Test the conversion function directly
    let declarative_schema = fixture
        .schema_core
        .convert_schema_to_declarative_definition(&schema);

    match declarative_schema {
        Ok(declarative_schema) => {
            println!("✅ Declarative schema conversion successful");
            println!(
                "📊 Declarative schema has {} fields",
                declarative_schema.fields.len()
            );

            // Verify the declarative schema properties
            assert_eq!(declarative_schema.name, "BlogPostWordIndex");
            assert!(matches!(
                declarative_schema.schema_type,
                SchemaType::HashRange
            ));
            assert!(
                declarative_schema.key.is_some(),
                "HashRange schema should have key config"
            );

            let key = declarative_schema.key.as_ref().unwrap();
            assert_eq!(
                key.hash_field,
                "BlogPost.map().fields.content.split_by_word().map()"
            );
            assert_eq!(key.range_field, "BlogPost.map().fields.publish_date");

            // Verify fields were converted (should be 2 fields: blog and author)
            assert_eq!(declarative_schema.fields.len(), 2);
            assert!(declarative_schema.fields.contains_key("blog"));
            assert!(declarative_schema.fields.contains_key("author"));

            println!("✅ HashRange fields correctly converted to declarative definition");
        }
        Err(e) => {
            panic!("❌ Declarative schema conversion failed: {}", e);
        }
    }
}

#[test]
fn test_hashrange_schema_field_conversion() {
    let fixture = HashRangeTestFixture::new().expect("Failed to create test fixture");
    let schema = fixture.create_hashrange_schema_with_single_fields();

    println!("🔧 Testing HashRange schema field conversion logic");

    // Test that all fields are Single variants
    for (field_name, field) in &schema.fields {
        match field {
            FieldVariant::Single(_) => {
                println!("✅ Field '{}' is correctly a Single variant", field_name);
            }
            _ => {
                panic!(
                    "❌ Field '{}' should be a Single variant but is {:?}",
                    field_name,
                    std::mem::discriminant(field)
                );
            }
        }
    }

    // Test that the schema type is HashRange
    assert!(
        matches!(schema.schema_type, SchemaType::HashRange),
        "Schema type should be HashRange"
    );
    println!("✅ Schema type is correctly HashRange");

    // Test field count
    assert_eq!(schema.fields.len(), 4, "Schema should have 4 fields");
    println!(
        "✅ Schema has correct number of fields: {}",
        schema.fields.len()
    );
}

#[test]
fn test_hashrange_schema_key_config_reading() {
    let fixture = HashRangeTestFixture::new().expect("Failed to create test fixture");

    println!("🔧 Testing HashRange schema key config reading");

    // Test the key config reading function directly
    let key_config = fixture
        .schema_core
        .get_universal_key_config_from_json("BlogPostWordIndex");

    match key_config {
        Ok(Some(config)) => {
            println!(
                "✅ Key config found - hash_field: {}, range_field: {}",
                config.hash_field, config.range_field
            );

            // Verify the key config matches the expected values from BlogPostWordIndex.json
            assert_eq!(
                config.hash_field,
                "BlogPost.map().fields.content.split_by_word().map()"
            );
            assert_eq!(config.range_field, "BlogPost.map().fields.publish_date");
        }
        Ok(None) => {
            panic!("❌ Key config should be found for BlogPostWordIndex");
        }
        Err(e) => {
            panic!("❌ Failed to read key config: {}", e);
        }
    }
}

#[test]
fn test_hashrange_schema_with_hashrange_fields_declarative_definition() {
    let fixture = HashRangeTestFixture::new().expect("Failed to create test fixture");
    let schema = fixture.create_hashrange_schema_with_hashrange_fields();

    println!("🔧 Testing HashRange schema with HashRange field variants - declarative definition conversion");
    println!("📊 Schema has {} fields", schema.fields.len());
    println!("🔍 Schema type: {:?}", schema.schema_type);

    // Test the conversion function directly
    let declarative_schema = fixture
        .schema_core
        .convert_schema_to_declarative_definition(&schema);

    match declarative_schema {
        Ok(declarative_schema) => {
            println!("✅ Declarative schema conversion successful");
            println!(
                "📊 Declarative schema has {} fields",
                declarative_schema.fields.len()
            );

            // Verify the declarative schema properties
            assert_eq!(declarative_schema.name, "BlogPostWordIndex");
            assert!(matches!(
                declarative_schema.schema_type,
                SchemaType::HashRange
            ));
            assert!(
                declarative_schema.key.is_some(),
                "HashRange schema should have key config"
            );

            let key = declarative_schema.key.as_ref().unwrap();
            assert_eq!(
                key.hash_field,
                "BlogPost.map().fields.content.split_by_word().map()"
            );
            assert_eq!(key.range_field, "BlogPost.map().fields.publish_date");

            // Verify fields were converted
            assert_eq!(declarative_schema.fields.len(), 2);
            assert!(declarative_schema.fields.contains_key("blog"));
            assert!(declarative_schema.fields.contains_key("author"));

            println!("✅ HashRange fields correctly converted to declarative definition");
        }
        Err(e) => {
            panic!("❌ Declarative schema conversion failed: {}", e);
        }
    }
}

#[test]
fn test_blogpost_word_index_transform_population() {
    let _fixture = HashRangeTestFixture::new().expect("Failed to create test fixture");

    println!("🔧 Testing BlogPostWordIndex transform data population");

    // First, create some test blog post data
    let blog_post_data = r#"{
        "BlogPost": [
            {
                "fields": {
                    "author": "Alice",
                    "title": "First Blog Post",
                    "content": "This is the first blog post content with some interesting words",
                    "publish_date": "2025-01-01T10:00:00Z",
                    "tags": ["tech", "programming"]
                },
                "hash": null,
                "range": null
            },
            {
                "fields": {
                    "author": "Bob",
                    "title": "Second Blog Post",
                    "content": "Another blog post with different content and more words",
                    "publish_date": "2025-01-02T10:00:00Z",
                    "tags": ["design", "ui"]
                },
                "hash": null,
                "range": null
            }
        ]
    }"#;

    let input_values: std::collections::HashMap<String, serde_json::Value> =
        serde_json::from_str(blog_post_data).expect("Failed to parse blog post data");

    // Load the BlogPostWordIndex schema
    let declarative_content = r#"{
  "name": "BlogPostWordIndex",
  "schema_type": "HashRange",
  "key": {
    "hash_field": "BlogPost.map().fields.content.split_by_word().map()",
    "range_field": "BlogPost.map().fields.publish_date"
  },
  "fields": {
    "content": { "atom_uuid": "BlogPost.map().fields.content" },
    "author": { "atom_uuid": "BlogPost.map().fields.author" },
    "title": { "atom_uuid": "BlogPost.map().fields.title" },
    "tags": { "atom_uuid": "BlogPost.map().fields.tags" }
  }
}"#;

    let declarative_schema: DeclarativeSchemaDefinition =
        serde_json::from_str(declarative_content).expect("Failed to parse declarative schema");

    // Create a Transform from the declarative schema
    let transform = datafold::schema::types::Transform::from_declarative_schema(
        declarative_schema,
        vec!["BlogPost".to_string()],            // Input schema name
        "BlogPostWordIndex.content".to_string(), // Output field
    );

    // Execute the transform
    let result = datafold::transform::executor::TransformExecutor::execute_transform(
        &transform,
        input_values,
    )
    .expect("Failed to execute transform");

    println!("📊 Transform execution result: {}", result);

    // Parse the result
    let result_obj = result.as_object().expect("Result should be an object");

    // Verify that the result contains the expected fields
    assert!(
        result_obj.contains_key("hash_key"),
        "Result should contain hash_key"
    );
    assert!(
        result_obj.contains_key("range_key"),
        "Result should contain range_key"
    );
    assert!(
        result_obj.contains_key("content"),
        "Result should contain content field"
    );
    assert!(
        result_obj.contains_key("author"),
        "Result should contain author field"
    );
    assert!(
        result_obj.contains_key("title"),
        "Result should contain title field"
    );
    assert!(
        result_obj.contains_key("tags"),
        "Result should contain tags field"
    );

    // Check that hash_key emits the expected word tokens
    let hash_key = result_obj.get("hash_key").expect("hash_key should exist");
    let hash_key_array = hash_key.as_array().expect("hash_key should be an array");

    println!("🔍 Hash key words: {:?}", hash_key_array);

    assert!(
        !hash_key_array.is_empty(),
        "hash_key should contain the split words from blog content"
    );

    let expected_words: Vec<&str> =
        "This is the first blog post content with some interesting words"
            .split_whitespace()
            .collect();
    let actual_words: Vec<String> = hash_key_array
        .iter()
        .filter_map(|value| value.as_str().map(|word| word.to_string()))
        .collect();

    for word in expected_words {
        assert!(
            actual_words.contains(&word.to_string()),
            "Expected hash_key to contain word '{}'",
            word
        );
    }

    // Check that range_key contains the publish dates
    let range_key = result_obj.get("range_key").expect("range_key should exist");
    let range_key_array = range_key.as_array().expect("range_key should be an array");

    println!("📅 Range key dates: {:?}", range_key_array);

    let range_key_strings: Vec<String> = range_key_array
        .iter()
        .filter_map(|v| v.as_str().map(|s| s.to_string()))
        .collect();

    assert!(
        range_key_strings.contains(&"2025-01-01T10:00:00Z".to_string()),
        "Should contain first publish date"
    );
    assert!(
        range_key_strings.contains(&"2025-01-02T10:00:00Z".to_string()),
        "Should contain second publish date"
    );

    // Check that other fields contain the expected data
    let content_array = result_obj
        .get("content")
        .expect("content should exist")
        .as_array()
        .expect("content should be an array");
    assert_eq!(
        content_array,
        &[
            Value::String(
                "This is the first blog post content with some interesting words".to_string(),
            ),
            Value::String("Another blog post with different content and more words".to_string(),),
        ]
    );

    let author = result_obj.get("author").expect("author should exist");
    let author_array = author.as_array().expect("author should be an array");
    assert_eq!(
        author_array,
        &[
            Value::String("Alice".to_string()),
            Value::String("Bob".to_string()),
        ]
    );

    let title = result_obj.get("title").expect("title should exist");
    let title_array = title.as_array().expect("title should be an array");
    assert_eq!(
        title_array,
        &[
            Value::String("First Blog Post".to_string()),
            Value::String("Second Blog Post".to_string()),
        ]
    );

    let tags = result_obj.get("tags").expect("tags should exist");
    let tags_array = tags.as_array().expect("tags should be an array");
    assert_eq!(tags_array.len(), 2, "Tags array should have 2 entries");

    println!("✅ BlogPostWordIndex transform populated normalized fields correctly");
    println!("✅ Range key contains publish dates");
}
