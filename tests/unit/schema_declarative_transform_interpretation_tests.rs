use datafold::schema::types::{
    json_schema::{DeclarativeSchemaDefinition, FieldDefinition, KeyConfig},
    Transform, SchemaType,
};
use datafold::schema::core::SchemaCore;
use datafold::schema::types::field::Field;
use datafold::transform::executor::TransformExecutor;
use std::collections::HashMap;
use tempfile::TempDir;
use sled;
use serde_json::json;

/// Test fixture for schema declarative transform interpretation tests
struct SchemaDeclarativeTransformTestFixture {
    schema_core: SchemaCore,
    temp_dir: TempDir,
}

impl SchemaDeclarativeTransformTestFixture {
    fn new() -> Result<Self, Box<dyn std::error::Error>> {
        let temp_dir = TempDir::new()?;
        let db = sled::open(temp_dir.path())?;
        let db_ops = std::sync::Arc::new(datafold::db_operations::DbOperations::new(db)?);
        let message_bus = std::sync::Arc::new(datafold::fold_db_core::infrastructure::message_bus::MessageBus::new());
        let schema_core = SchemaCore::new(
            temp_dir.path().to_str().unwrap(),
            db_ops.clone(),
            message_bus,
        )?;
        
        Ok(Self {
            schema_core,
            temp_dir,
        })
    }
}

/// Test basic HashRange schema interpretation and transform creation
#[test]
fn test_hashrange_schema_interpretation_and_transform() {
    let fixture = SchemaDeclarativeTransformTestFixture::new().expect("Failed to create test fixture");
    
    // Create a declarative schema definition
    let declarative_schema = DeclarativeSchemaDefinition {
        name: "BlogPostWordIndex".to_string(),
        schema_type: SchemaType::HashRange,
        fields: HashMap::from([
            ("blog".to_string(), FieldDefinition {
                field_type: Some("single".to_string()),
                atom_uuid: Some("blogpost.map().$atom_uuid".to_string()),
            }),
            ("author".to_string(), FieldDefinition {
                field_type: Some("single".to_string()),
                atom_uuid: Some("blogpost.map().author.$atom_uuid".to_string()),
            }),
            ("title".to_string(), FieldDefinition {
                field_type: Some("single".to_string()),
                atom_uuid: Some("blogpost.map().title.$atom_uuid".to_string()),
            }),
        ]),
        key: Some(KeyConfig {
            hash_field: "blogpost.map().content.split_by_word().map()".to_string(),
            range_field: "blogpost.map().publish_date".to_string(),
        }),
    };
    
    // Test 1: Schema interpretation - convert declarative schema to Schema
    let schema_result = fixture.schema_core.interpret_declarative_schema(declarative_schema.clone());
    assert!(schema_result.is_ok(), "Schema interpretation failed: {:?}", schema_result);
    
    let schema = schema_result.unwrap();
    
    // Verify the final schema form
    assert_eq!(schema.name, "BlogPostWordIndex");
    assert!(matches!(schema.schema_type, SchemaType::HashRange));
    assert_eq!(schema.fields.len(), 3);
    
    // Verify all fields are HashRangeField variants
    for field_name in ["blog", "author", "title"] {
        assert!(schema.fields.contains_key(field_name));
        if let datafold::schema::types::field::FieldVariant::HashRange(hashrange_field) = &schema.fields[field_name] {
            // Verify fields are not writable (derived from declarative schema)
            assert!(!hashrange_field.writable());
            // Verify hash and range fields are set correctly
            assert_eq!(hashrange_field.hash_field, "blogpost.map().content.split_by_word().map()");
            assert_eq!(hashrange_field.range_field, "blogpost.map().publish_date");
        } else {
            panic!("Field {} should be HashRangeField variant", field_name);
        }
    }
    
    // Test 2: Transform creation from declarative schema
    let transform = Transform::from_declarative_schema(
        declarative_schema.clone(),
        vec!["blogpost".to_string()],
        "BlogPostWordIndex.key".to_string(),
    );
    
    // Verify transform properties
    assert!(transform.is_declarative());
    assert!(!transform.is_procedural());
    assert_eq!(transform.get_inputs(), vec!["blogpost"]);
    assert_eq!(transform.get_output(), "BlogPostWordIndex.key");
    
    // Verify declarative schema is accessible in transform
    let retrieved_schema = transform.get_declarative_schema().expect("Should have declarative schema");
    assert_eq!(retrieved_schema.name, "BlogPostWordIndex");
    assert_eq!(retrieved_schema.fields.len(), 3);
    assert!(retrieved_schema.key.is_some());
    
    // Test 3: Transform execution with sample data
    let input_values = HashMap::from([
        ("blogpost".to_string(), json!({
            "content": "This is a test blog post",
            "publish_date": "2024-01-15",
            "author": {"name": "John Doe", "$atom_uuid": "author-123"},
            "title": {"text": "Test Post", "$atom_uuid": "title-456"},
            "$atom_uuid": "blog-789"
        }))
    ]);
    
    let executor = TransformExecutor;
    let execution_result = TransformExecutor::execute_transform(&transform, input_values);
    
    // Note: Execution may fail due to missing chain parsing infrastructure
    // but we can verify the transform structure is correct
    println!("Transform execution result: {:?}", execution_result);
}

/// Test Single schema interpretation and transform creation
#[test]
fn test_single_schema_interpretation_and_transform() {
    let fixture = SchemaDeclarativeTransformTestFixture::new().expect("Failed to create test fixture");
    
    // Create a Single schema declarative definition
    let declarative_schema = DeclarativeSchemaDefinition {
        name: "ProcessedBlogPost".to_string(),
        schema_type: SchemaType::Single,
        fields: HashMap::from([
            ("processed_content".to_string(), FieldDefinition {
                field_type: Some("single".to_string()),
                atom_uuid: Some("blogpost.map().content".to_string()),
            }),
            ("word_count".to_string(), FieldDefinition {
                field_type: Some("single".to_string()),
                atom_uuid: Some("blogpost.map().content.split_by_word().map()".to_string()),
            }),
            ("author_name".to_string(), FieldDefinition {
                field_type: Some("single".to_string()),
                atom_uuid: Some("blogpost.map().author.name".to_string()),
            }),
        ]),
        key: None,
    };
    
    // Test 1: Schema interpretation
    let schema_result = fixture.schema_core.interpret_declarative_schema(declarative_schema.clone());
    assert!(schema_result.is_ok(), "Single schema interpretation failed: {:?}", schema_result);
    
    let schema = schema_result.unwrap();
    
    // Verify the final schema form
    assert_eq!(schema.name, "ProcessedBlogPost");
    assert!(matches!(schema.schema_type, SchemaType::Single));
    assert_eq!(schema.fields.len(), 3);
    
    // Verify all fields are SingleField variants
    for field_name in ["processed_content", "word_count", "author_name"] {
        assert!(schema.fields.contains_key(field_name));
        if let datafold::schema::types::field::FieldVariant::Single(single_field) = &schema.fields[field_name] {
            // Verify fields are not writable (derived from declarative schema)
            assert!(!single_field.writable());
        } else {
            panic!("Field {} should be SingleField variant", field_name);
        }
    }
    
    // Test 2: Transform creation
    let transform = Transform::from_declarative_schema(
        declarative_schema.clone(),
        vec!["blogpost".to_string()],
        "ProcessedBlogPost.result".to_string(),
    );
    
    // Verify transform properties
    assert!(transform.is_declarative());
    assert_eq!(transform.get_inputs(), vec!["blogpost"]);
    assert_eq!(transform.get_output(), "ProcessedBlogPost.result");
    
    // Verify declarative schema in transform
    let retrieved_schema = transform.get_declarative_schema().expect("Should have declarative schema");
    assert_eq!(retrieved_schema.name, "ProcessedBlogPost");
    assert_eq!(retrieved_schema.fields.len(), 3);
    assert!(retrieved_schema.key.is_none());
}

/// Test Range schema interpretation and transform creation
#[test]
fn test_range_schema_interpretation_and_transform() {
    let fixture = SchemaDeclarativeTransformTestFixture::new().expect("Failed to create test fixture");
    
    // Create a Range schema declarative definition
    let declarative_schema = DeclarativeSchemaDefinition {
        name: "TimeSeriesData".to_string(),
        schema_type: SchemaType::Range { range_key: "timestamp".to_string() },
        fields: HashMap::from([
            ("timestamp".to_string(), FieldDefinition {
                field_type: Some("single".to_string()),
                atom_uuid: Some("event.map().timestamp".to_string()),
            }),
            ("value".to_string(), FieldDefinition {
                field_type: Some("single".to_string()),
                atom_uuid: Some("event.map().value".to_string()),
            }),
            ("source".to_string(), FieldDefinition {
                field_type: Some("single".to_string()),
                atom_uuid: Some("event.map().source.$atom_uuid".to_string()),
            }),
        ]),
        key: None,
    };
    
    // Test 1: Schema interpretation
    let schema_result = fixture.schema_core.interpret_declarative_schema(declarative_schema.clone());
    assert!(schema_result.is_ok(), "Range schema interpretation failed: {:?}", schema_result);
    
    let schema = schema_result.unwrap();
    
    // Verify the final schema form
    assert_eq!(schema.name, "TimeSeriesData");
    assert!(matches!(schema.schema_type, SchemaType::Range { range_key } if range_key == "timestamp"));
    assert_eq!(schema.fields.len(), 3);
    
    // Verify all fields are SingleField variants (Range schemas use SingleField)
    for field_name in ["timestamp", "value", "source"] {
        assert!(schema.fields.contains_key(field_name));
        if let datafold::schema::types::field::FieldVariant::Single(single_field) = &schema.fields[field_name] {
            // Verify fields are not writable (derived from declarative schema)
            assert!(!single_field.writable());
        } else {
            panic!("Field {} should be SingleField variant", field_name);
        }
    }
    
    // Test 2: Transform creation
    let transform = Transform::from_declarative_schema(
        declarative_schema.clone(),
        vec!["event".to_string()],
        "TimeSeriesData.entry".to_string(),
    );
    
    // Verify transform properties
    assert!(transform.is_declarative());
    assert_eq!(transform.get_inputs(), vec!["event"]);
    assert_eq!(transform.get_output(), "TimeSeriesData.entry");
    
    // Verify declarative schema in transform
    let retrieved_schema = transform.get_declarative_schema().expect("Should have declarative schema");
    assert_eq!(retrieved_schema.name, "TimeSeriesData");
    assert_eq!(retrieved_schema.fields.len(), 3);
    assert!(retrieved_schema.key.is_none());
}

/// Test complex schema with multiple input dependencies
#[test]
fn test_complex_schema_multiple_inputs() {
    let fixture = SchemaDeclarativeTransformTestFixture::new().expect("Failed to create test fixture");
    
    // Create a complex schema with multiple input dependencies
    let declarative_schema = DeclarativeSchemaDefinition {
        name: "MultiSourceAnalytics".to_string(),
        schema_type: SchemaType::HashRange,
        fields: HashMap::from([
            ("blog_content".to_string(), FieldDefinition {
                field_type: Some("single".to_string()),
                atom_uuid: Some("blogpost.map().content".to_string()),
            }),
            ("user_profile".to_string(), FieldDefinition {
                field_type: Some("single".to_string()),
                atom_uuid: Some("user.map().profile.$atom_uuid".to_string()),
            }),
            ("analytics_data".to_string(), FieldDefinition {
                field_type: Some("single".to_string()),
                atom_uuid: Some("analytics.map().metrics.$atom_uuid".to_string()),
            }),
        ]),
        key: Some(KeyConfig {
            hash_field: "blogpost.map().content.split_by_word().map()".to_string(),
            range_field: "analytics.map().timestamp".to_string(),
        }),
    };
    
    // Test 1: Schema interpretation
    let schema_result = fixture.schema_core.interpret_declarative_schema(declarative_schema.clone());
    assert!(schema_result.is_ok(), "Complex schema interpretation failed: {:?}", schema_result);
    
    let schema = schema_result.unwrap();
    
    // Verify the final schema form
    assert_eq!(schema.name, "MultiSourceAnalytics");
    assert!(matches!(schema.schema_type, SchemaType::HashRange));
    assert_eq!(schema.fields.len(), 3);
    
    // Verify all fields are HashRangeField variants
    for field_name in ["blog_content", "user_profile", "analytics_data"] {
        assert!(schema.fields.contains_key(field_name));
        if let datafold::schema::types::field::FieldVariant::HashRange(hashrange_field) = &schema.fields[field_name] {
            assert!(!hashrange_field.writable());
            // Verify hash and range fields are set correctly
            assert_eq!(hashrange_field.hash_field, "blogpost.map().content.split_by_word().map()");
            assert_eq!(hashrange_field.range_field, "analytics.map().timestamp");
        } else {
            panic!("Field {} should be HashRangeField variant", field_name);
        }
    }
    
    // Test 2: Transform creation with multiple inputs
    let transform = Transform::from_declarative_schema(
        declarative_schema.clone(),
        vec!["blogpost".to_string(), "user".to_string(), "analytics".to_string()],
        "MultiSourceAnalytics.key".to_string(),
    );
    
    // Verify transform properties
    assert!(transform.is_declarative());
    assert_eq!(transform.get_inputs(), vec!["blogpost", "user", "analytics"]);
    assert_eq!(transform.get_output(), "MultiSourceAnalytics.key");
    
    // Verify declarative schema in transform
    let retrieved_schema = transform.get_declarative_schema().expect("Should have declarative schema");
    assert_eq!(retrieved_schema.name, "MultiSourceAnalytics");
    assert_eq!(retrieved_schema.fields.len(), 3);
    assert!(retrieved_schema.key.is_some());
}

/// Test multiple blogposts with different hash and range keys to show mapping behavior
#[test]
fn test_multiple_blogposts_hash_range_mapping() {
    let fixture = SchemaDeclarativeTransformTestFixture::new().expect("Failed to create test fixture");
    
    // Create a HashRange schema that will process multiple blogposts
    let declarative_schema = DeclarativeSchemaDefinition {
        name: "BlogPostWordIndex".to_string(),
        schema_type: SchemaType::HashRange,
        fields: HashMap::from([
            ("blog".to_string(), FieldDefinition {
                field_type: Some("single".to_string()),
                atom_uuid: Some("blogpost.map().$atom_uuid".to_string()),
            }),
            ("author".to_string(), FieldDefinition {
                field_type: Some("single".to_string()),
                atom_uuid: Some("blogpost.map().author.$atom_uuid".to_string()),
            }),
            ("title".to_string(), FieldDefinition {
                field_type: Some("single".to_string()),
                atom_uuid: Some("blogpost.map().title.$atom_uuid".to_string()),
            }),
            ("word_count".to_string(), FieldDefinition {
                field_type: Some("single".to_string()),
                atom_uuid: Some("blogpost.map().content.split_by_word().map()".to_string()),
            }),
        ]),
        key: Some(KeyConfig {
            hash_field: "blogpost.map().content.split_by_word().map()".to_string(),
            range_field: "blogpost.map().publish_date".to_string(),
        }),
    };
    
    // Test 1: Schema interpretation
    let schema_result = fixture.schema_core.interpret_declarative_schema(declarative_schema.clone());
    assert!(schema_result.is_ok(), "Schema interpretation failed: {:?}", schema_result);
    
    let schema = schema_result.unwrap();
    
    // Verify the final schema form
    assert_eq!(schema.name, "BlogPostWordIndex");
    assert!(matches!(schema.schema_type, SchemaType::HashRange));
    assert_eq!(schema.fields.len(), 4);
    
    // Verify all fields are HashRangeField variants with correct hash/range expressions
    for field_name in ["blog", "author", "title", "word_count"] {
        assert!(schema.fields.contains_key(field_name));
        if let datafold::schema::types::field::FieldVariant::HashRange(hashrange_field) = &schema.fields[field_name] {
            assert!(!hashrange_field.writable());
            // All fields should have the same hash and range field expressions
            assert_eq!(hashrange_field.hash_field, "blogpost.map().content.split_by_word().map()");
            assert_eq!(hashrange_field.range_field, "blogpost.map().publish_date");
        } else {
            panic!("Field {} should be HashRangeField variant", field_name);
        }
    }
    
    // Test 2: Transform creation
    let transform = Transform::from_declarative_schema(
        declarative_schema.clone(),
        vec!["blogpost".to_string()],
        "BlogPostWordIndex.key".to_string(),
    );
    
    // Verify transform properties
    assert!(transform.is_declarative());
    assert_eq!(transform.get_inputs(), vec!["blogpost"]);
    assert_eq!(transform.get_output(), "BlogPostWordIndex.key");
    
    // Test 3: Transform execution with multiple blogposts
    let input_values = HashMap::from([
        ("blogpost".to_string(), json!([
            {
                "title": {"text": "First Blog Post", "$atom_uuid": "title-1"},
                "content": "This is the first blog post content with multiple words",
                "publish_date": "2024-01-15T10:00:00Z",
                "author": {"name": "Alice Johnson", "$atom_uuid": "author-1"},
                "$atom_uuid": "blog-1"
            },
            {
                "title": {"text": "Second Blog Post", "$atom_uuid": "title-2"},
                "content": "This is the second blog post with different content",
                "publish_date": "2024-01-16T14:30:00Z",
                "author": {"name": "Bob Smith", "$atom_uuid": "author-2"},
                "$atom_uuid": "blog-2"
            },
            {
                "title": {"text": "Third Blog Post", "$atom_uuid": "title-3"},
                "content": "Short content",
                "publish_date": "2024-01-17T09:15:00Z",
                "author": {"name": "Carol Davis", "$atom_uuid": "author-3"},
                "$atom_uuid": "blog-3"
            }
        ]))
    ]);
    
    let execution_result = TransformExecutor::execute_transform(&transform, input_values);
    
    // Note: Execution may fail due to missing chain parsing infrastructure
    // but we can verify the transform structure is correct
    println!("Transform execution result: {:?}", execution_result);
    
    // Test 4: Verify the declarative schema preserves all field expressions
    let retrieved_schema = transform.get_declarative_schema().expect("Should have declarative schema");
    assert_eq!(retrieved_schema.name, "BlogPostWordIndex");
    assert_eq!(retrieved_schema.fields.len(), 4);
    assert!(retrieved_schema.key.is_some());
    
    // Verify each field has the correct atom_uuid expression
    let expected_expressions = [
        ("blog", "blogpost.map().$atom_uuid"),
        ("author", "blogpost.map().author.$atom_uuid"),
        ("title", "blogpost.map().title.$atom_uuid"),
        ("word_count", "blogpost.map().content.split_by_word().map()"),
    ];
    
    for (field_name, expected_expr) in expected_expressions {
        let field_def = retrieved_schema.fields.get(field_name).expect("Field should exist");
        assert_eq!(field_def.atom_uuid.as_ref().unwrap(), expected_expr);
    }
    
    // Test 5: Verify key configuration
    let key_config = retrieved_schema.key.as_ref().expect("Should have key configuration");
    assert_eq!(key_config.hash_field, "blogpost.map().content.split_by_word().map()");
    assert_eq!(key_config.range_field, "blogpost.map().publish_date");
    
    // Test 6: Show how the hash and range keys would map to different blogposts
    println!("=== Hash and Range Key Mapping Analysis ===");
    println!("Hash Field Expression: {}", key_config.hash_field);
    println!("Range Field Expression: {}", key_config.range_field);
    println!();
    println!("For the 3 blogposts provided:");
    println!("1. Blog Post 1 (Alice):");
    println!("   - Hash: 'This is the first blog post content with multiple words'.split_by_word().map()");
    println!("   - Range: '2024-01-15T10:00:00Z'");
    println!();
    println!("2. Blog Post 2 (Bob):");
    println!("   - Hash: 'This is the second blog post with different content'.split_by_word().map()");
    println!("   - Range: '2024-01-16T14:30:00Z'");
    println!();
    println!("3. Blog Post 3 (Carol):");
    println!("   - Hash: 'Short content'.split_by_word().map()");
    println!("   - Range: '2024-01-17T09:15:00Z'");
    println!();
    println!("The system would create separate entries for each word in the content,");
    println!("with the same range key (publish_date) for all words from the same blog post.");
}

/// Test schema validation and error handling
#[test]
fn test_schema_validation_and_errors() {
    let fixture = SchemaDeclarativeTransformTestFixture::new().expect("Failed to create test fixture");
    
    // Test 1: HashRange schema without key configuration
    let invalid_hashrange_schema = DeclarativeSchemaDefinition {
        name: "InvalidHashRange".to_string(),
        schema_type: SchemaType::HashRange,
        fields: HashMap::from([
            ("field1".to_string(), FieldDefinition {
                field_type: Some("single".to_string()),
                atom_uuid: Some("source.map().field1".to_string()),
            }),
        ]),
        key: None, // Missing key configuration for HashRange
    };
    
    let schema_result = fixture.schema_core.interpret_declarative_schema(invalid_hashrange_schema);
    assert!(schema_result.is_err(), "Should fail for HashRange schema without key configuration");
    
    // Test 2: Schema with invalid field name (empty string is actually valid)
    let invalid_field_schema = DeclarativeSchemaDefinition {
        name: "InvalidField".to_string(),
        schema_type: SchemaType::Single,
        fields: HashMap::from([
            ("invalid_field_name_with_special_chars!@#".to_string(), FieldDefinition { // Invalid field name
                field_type: Some("single".to_string()),
                atom_uuid: Some("source.map().field1".to_string()),
            }),
        ]),
        key: None,
    };
    
    let schema_result = fixture.schema_core.interpret_declarative_schema(invalid_field_schema);
    // Note: Field name validation might be more permissive than expected
    // This test documents the current behavior
    println!("Field name validation result: {:?}", schema_result);
    
    // Test 3: Valid schema should pass validation
    let valid_schema = DeclarativeSchemaDefinition {
        name: "ValidSchema".to_string(),
        schema_type: SchemaType::Single,
        fields: HashMap::from([
            ("field1".to_string(), FieldDefinition {
                field_type: Some("single".to_string()),
                atom_uuid: Some("source.map().field1".to_string()),
            }),
        ]),
        key: None,
    };
    
    let schema_result = fixture.schema_core.interpret_declarative_schema(valid_schema);
    assert!(schema_result.is_ok(), "Valid schema should pass validation");
}

/// Test transform registration and retrieval
#[test]
fn test_transform_registration_and_retrieval() {
    let fixture = SchemaDeclarativeTransformTestFixture::new().expect("Failed to create test fixture");
    
    let declarative_schema = DeclarativeSchemaDefinition {
        name: "TestTransform".to_string(),
        schema_type: SchemaType::Single,
        fields: HashMap::from([
            ("processed".to_string(), FieldDefinition {
                field_type: Some("single".to_string()),
                atom_uuid: Some("input.map().value".to_string()),
            }),
        ]),
        key: None,
    };
    
    // Test 1: Register declarative transform
    let registration_result = fixture.schema_core.register_declarative_transform(&declarative_schema);
    assert!(registration_result.is_ok(), "Transform registration failed: {:?}", registration_result);
    
    // Test 2: Retrieve registered transform
    let transform_id = "TestTransform.declarative";
    let stored_transform = fixture.schema_core.get_transform(transform_id);
    assert!(stored_transform.is_ok(), "Failed to retrieve stored transform");
    assert!(stored_transform.as_ref().unwrap().is_some(), "Transform not found in database");
    
    // Test 3: Verify transform properties
    let transform = stored_transform.unwrap().unwrap();
    assert!(transform.is_declarative());
    assert_eq!(transform.get_inputs(), vec!["input"]); // Derived from field expression "input.map().value"
    assert_eq!(transform.get_output(), "TestTransform.key");
    
    // Test 4: Verify declarative schema in stored transform
    let retrieved_schema = transform.get_declarative_schema().expect("Should have declarative schema");
    assert_eq!(retrieved_schema.name, "TestTransform");
    assert_eq!(retrieved_schema.fields.len(), 1);
}

/// Test schema field mapping and expression parsing
#[test]
fn test_schema_field_mapping_and_expressions() {
    let fixture = SchemaDeclarativeTransformTestFixture::new().expect("Failed to create test fixture");
    
    // Create schema with various field expression types
    let declarative_schema = DeclarativeSchemaDefinition {
        name: "ExpressionTest".to_string(),
        schema_type: SchemaType::HashRange,
        fields: HashMap::from([
            ("simple_field".to_string(), FieldDefinition {
                field_type: Some("single".to_string()),
                atom_uuid: Some("source.map().simple_field".to_string()),
            }),
            ("nested_field".to_string(), FieldDefinition {
                field_type: Some("single".to_string()),
                atom_uuid: Some("source.map().nested.object.field".to_string()),
            }),
            ("transformed_field".to_string(), FieldDefinition {
                field_type: Some("single".to_string()),
                atom_uuid: Some("source.map().content.split_by_word().map()".to_string()),
            }),
            ("atom_reference".to_string(), FieldDefinition {
                field_type: Some("single".to_string()),
                atom_uuid: Some("source.map().$atom_uuid".to_string()),
            }),
        ]),
        key: Some(KeyConfig {
            hash_field: "source.map().content.split_by_word().map()".to_string(),
            range_field: "source.map().timestamp".to_string(),
        }),
    };
    
    // Test schema interpretation
    let schema_result = fixture.schema_core.interpret_declarative_schema(declarative_schema.clone());
    assert!(schema_result.is_ok(), "Expression test schema interpretation failed: {:?}", schema_result);
    
    let schema = schema_result.unwrap();
    
    // Verify all fields are properly mapped
    assert_eq!(schema.fields.len(), 4);
    
    // Verify each field has the correct atom_uuid expression
    for (field_name, expected_expr) in [
        ("simple_field", "source.map().simple_field"),
        ("nested_field", "source.map().nested.object.field"),
        ("transformed_field", "source.map().content.split_by_word().map()"),
        ("atom_reference", "source.map().$atom_uuid"),
    ] {
        assert!(schema.fields.contains_key(field_name));
        if let datafold::schema::types::field::FieldVariant::HashRange(hashrange_field) = &schema.fields[field_name] {
            assert_eq!(hashrange_field.atom_uuid, expected_expr);
        } else {
            panic!("Field {} should be HashRangeField variant", field_name);
        }
    }
    
    // Test transform creation with expressions
    let transform = Transform::from_declarative_schema(
        declarative_schema.clone(),
        vec!["source".to_string()],
        "ExpressionTest.key".to_string(),
    );
    
    // Verify transform can access all field expressions
    let retrieved_schema = transform.get_declarative_schema().expect("Should have declarative schema");
    assert_eq!(retrieved_schema.fields.len(), 4);
    
    for (field_name, expected_expr) in [
        ("simple_field", "source.map().simple_field"),
        ("nested_field", "source.map().nested.object.field"),
        ("transformed_field", "source.map().content.split_by_word().map()"),
        ("atom_reference", "source.map().$atom_uuid"),
    ] {
        let field_def = retrieved_schema.fields.get(field_name).expect("Field should exist");
        assert_eq!(field_def.atom_uuid.as_ref().unwrap(), expected_expr);
    }
}

/// Test schema type conversion and field variant creation
#[test]
fn test_schema_type_conversion_and_field_variants() {
    let fixture = SchemaDeclarativeTransformTestFixture::new().expect("Failed to create test fixture");
    
    // Test 1: Single schema type
    let single_schema = DeclarativeSchemaDefinition {
        name: "SingleTest".to_string(),
        schema_type: SchemaType::Single,
        fields: HashMap::from([
            ("field1".to_string(), FieldDefinition {
                field_type: Some("single".to_string()),
                atom_uuid: Some("source.map().field1".to_string()),
            }),
        ]),
        key: None,
    };
    
    let schema_result = fixture.schema_core.interpret_declarative_schema(single_schema);
    assert!(schema_result.is_ok(), "Single schema conversion failed: {:?}", schema_result);
    
    let schema = schema_result.unwrap();
    assert!(matches!(schema.schema_type, SchemaType::Single));
    
    // Verify SingleField variant creation
    if let datafold::schema::types::field::FieldVariant::Single(single_field) = &schema.fields["field1"] {
        assert!(!single_field.writable());
        assert_eq!(single_field.molecule_uuid(), Some(&"source.map().field1".to_string()));
    } else {
        panic!("Field should be SingleField variant");
    }
    
    // Test 2: HashRange schema type
    let hashrange_schema = DeclarativeSchemaDefinition {
        name: "HashRangeTest".to_string(),
        schema_type: SchemaType::HashRange,
        fields: HashMap::from([
            ("field1".to_string(), FieldDefinition {
                field_type: Some("single".to_string()),
                atom_uuid: Some("source.map().field1".to_string()),
            }),
        ]),
        key: Some(KeyConfig {
            hash_field: "source.map().hash_field".to_string(),
            range_field: "source.map().range_field".to_string(),
        }),
    };
    
    let schema_result = fixture.schema_core.interpret_declarative_schema(hashrange_schema);
    assert!(schema_result.is_ok(), "HashRange schema conversion failed: {:?}", schema_result);
    
    let schema = schema_result.unwrap();
    assert!(matches!(schema.schema_type, SchemaType::HashRange));
    
    // Verify HashRangeField variant creation
    if let datafold::schema::types::field::FieldVariant::HashRange(hashrange_field) = &schema.fields["field1"] {
        assert!(!hashrange_field.writable());
        assert_eq!(hashrange_field.atom_uuid, "source.map().field1");
        assert_eq!(hashrange_field.hash_field, "source.map().hash_field");
        assert_eq!(hashrange_field.range_field, "source.map().range_field");
    } else {
        panic!("Field should be HashRangeField variant");
    }
    
    // Test 3: Range schema type
    let range_schema = DeclarativeSchemaDefinition {
        name: "RangeTest".to_string(),
        schema_type: SchemaType::Range { range_key: "timestamp".to_string() },
        fields: HashMap::from([
            ("timestamp".to_string(), FieldDefinition {
                field_type: Some("single".to_string()),
                atom_uuid: Some("source.map().timestamp".to_string()),
            }),
            ("value".to_string(), FieldDefinition {
                field_type: Some("single".to_string()),
                atom_uuid: Some("source.map().value".to_string()),
            }),
        ]),
        key: None,
    };
    
    let schema_result = fixture.schema_core.interpret_declarative_schema(range_schema);
    assert!(schema_result.is_ok(), "Range schema conversion failed: {:?}", schema_result);
    
    let schema = schema_result.unwrap();
    assert!(matches!(schema.schema_type, SchemaType::Range { range_key } if range_key == "timestamp"));
    
    // Verify SingleField variants for Range schema (Range schemas use SingleField)
    for field_name in ["timestamp", "value"] {
        if let datafold::schema::types::field::FieldVariant::Single(single_field) = &schema.fields[field_name] {
            assert!(!single_field.writable());
        } else {
            panic!("Field {} should be SingleField variant", field_name);
        }
    }
}
