use datafold::fold_db_core::FoldDB;
use serde_json::json;
use tempfile::TempDir;

/// Test to verify that loading the BlogPostWordIndex schema properly registers declarative transforms
#[tokio::test]
async fn test_blogpost_wordindex_transform_registration() {
    // Create a temporary directory for this test
    let temp_dir = TempDir::new().expect("Failed to create temp directory");
    let test_db_path = temp_dir.path().to_str().expect("Failed to convert path to string");
    
    // Create a new FoldDB instance
    let fold_db = FoldDB::new(test_db_path, None).await.expect("Failed to create FoldDB");
    
    // Load the BlogPost schema first (source schema)
    let blogpost_schema_json = json!({
        "name": "BlogPost",
        "key": {
            "range_field": "publish_date"
        },
        "fields": {
            "title": {},
            "content": {},
            "author": {},
            "publish_date": {},
            "tags": {}
        }
    });
    
    let blogpost_schema_str = serde_json::to_string(&blogpost_schema_json)
        .expect("Failed to serialize BlogPost schema");
    
    // Load BlogPost schema into the database
    fold_db.schema_manager().load_schema_from_json(&blogpost_schema_str)
        .await
        .expect("Failed to load BlogPost schema");
    
    // Load the BlogPostWordIndex schema with transform_fields
    let wordindex_schema_json = json!({
        "name": "BlogPostWordIndex",
        "key": {
            "hash_field": "word",
            "range_field": "publish_date"
        },
        "transform_fields": {
            "word": "BlogPost.map().content.split_by_word().map()",
            "publish_date": "BlogPost.map().publish_date",
            "content": "BlogPost.map().content",
            "author": "BlogPost.map().author",
            "title": "BlogPost.map().title",
            "tags": "BlogPost.map().tags"
        }
    });
    
    let wordindex_schema_str = serde_json::to_string(&wordindex_schema_json)
        .expect("Failed to serialize BlogPostWordIndex schema");
    
    // Load BlogPostWordIndex schema into the database
    // This should trigger the registration of declarative transforms
    fold_db.schema_manager().load_schema_from_json(&wordindex_schema_str)
        .await
        .expect("Failed to load BlogPostWordIndex schema");
    
    // Wait a moment for async event processing
    std::thread::sleep(std::time::Duration::from_millis(100));
    
    // Get the transform manager to check registered transforms
    let transform_manager = fold_db.transform_manager();
    
    // List all registered transforms
    let registered_transforms = transform_manager.list_transforms()
        .expect("Failed to list transforms");
    
    // Verify that ONE transform was registered for the BlogPostWordIndex schema
    let expected_transform_id = "BlogPostWordIndex";
    
    println!("📋 Registered transforms: {:?}", registered_transforms.keys().collect::<Vec<_>>());
    
    // Check that the transform is registered
    assert!(
        registered_transforms.contains_key(expected_transform_id),
        "Transform '{}' should be registered after loading BlogPostWordIndex schema",
        expected_transform_id
    );
    
    // Verify that only ONE transform is registered
    assert_eq!(
        registered_transforms.len(),
        1,
        "Should have exactly 1 registered transform for BlogPostWordIndex schema"
    );
    
    // Verify transform field mappings are correctly established
    // The transforms should be triggered by changes to BlogPost fields
    let expected_trigger_fields = vec![
        "BlogPost.content",
        "BlogPost.publish_date", 
        "BlogPost.author",
        "BlogPost.title",
        "BlogPost.tags"
    ];
    
    for field in &expected_trigger_fields {
        let transforms_for_field = transform_manager.get_transforms_for_field("BlogPost", field.split('.').nth(1).unwrap())
            .expect("Failed to get transforms for field");
        
        println!("🔗 Transforms for field '{}': {:?}", field, transforms_for_field);
        
        // Each BlogPost field should trigger at least one transform
        assert!(
            !transforms_for_field.is_empty(),
            "Field '{}' should trigger at least one transform",
            field
        );
    }
    
    // Verify that BlogPost.content triggers the BlogPostWordIndex transform
    let transforms_for_content = transform_manager.get_transforms_for_field("BlogPost", "content")
        .expect("Failed to get transforms for content field");
    
    assert!(
        transforms_for_content.contains(expected_transform_id),
        "BlogPost.content should trigger the BlogPostWordIndex transform"
    );
    
    // Verify that transform is properly stored in the database
    assert!(
        transform_manager.list_transforms().expect("Failed to list transforms").contains_key(expected_transform_id),
        "Transform '{}' should exist in the database",
        expected_transform_id
    );
    
    // Verify the transform schema structure
    for (transform_id, transform) in &registered_transforms {
        // Transform now stores only schema_name, look up the full schema from database
        assert_eq!(
            transform.get_schema_name(),
            "BlogPostWordIndex",
            "Transform '{}' should have schema name 'BlogPostWordIndex'",
            transform_id
        );
        
        // Verify that the transform has the correct key configuration by looking up the schema
        let schema = transform_manager.db_ops.get_schema(transform.get_schema_name())
            .await
            .expect("Failed to get schema")
            .expect("Schema should exist");
        assert!(schema.key.is_some(), "Transform '{}' should have key configuration", transform_id);
        let key_config = schema.key.as_ref().unwrap();
        assert_eq!(key_config.hash_field, Some("word".to_string()));
        assert_eq!(key_config.range_field, Some("publish_date".to_string()));
    }
    
    println!("✅ All BlogPostWordIndex transform registration tests passed!");
}

/// Test to verify that the declarative transform registration works with the actual schema file
#[tokio::test]
async fn test_blogpost_wordindex_from_file() {
    // Create a temporary directory for this test
    let temp_dir = TempDir::new().expect("Failed to create temp directory");
    let test_db_path = temp_dir.path().to_str().expect("Failed to convert path to string");
    
    // Create a new FoldDB instance
    let mut fold_db = FoldDB::new(test_db_path, None).await.expect("Failed to create FoldDB");
    
    // Load the BlogPost schema first
    let blogpost_schema_json = json!({
        "name": "BlogPost",
        "key": {
            "range_field": "publish_date"
        },
        "fields": {
            "title": {},
            "content": {},
            "author": {},
            "publish_date": {},
            "tags": {}
        }
    });
    
    let blogpost_schema_str = serde_json::to_string(&blogpost_schema_json)
        .expect("Failed to serialize BlogPost schema");
    
    fold_db.schema_manager().load_schema_from_json(&blogpost_schema_str)
        .await
        .expect("Failed to load BlogPost schema");
    
    // Load the BlogPostWordIndex schema from the actual file
    let wordindex_schema_path = std::env::current_dir()
        .expect("Failed to get current directory")
        .join("tests/schemas_for_testing")
        .join("BlogPostWordIndex.json");
    
    assert!(wordindex_schema_path.exists(), "BlogPostWordIndex.json should exist in tests/schemas_for_testing");
    
    // Load the schema from file - this should trigger transform registration
    fold_db.load_schema_from_file(&wordindex_schema_path)
        .await
        .expect("Failed to load BlogPostWordIndex schema from file");
    
    // Wait for async event processing
    std::thread::sleep(std::time::Duration::from_millis(100));
    
    // Verify transforms were registered
    let transform_manager = fold_db.transform_manager();
    let registered_transforms = transform_manager.list_transforms()
        .expect("Failed to list transforms");
    
    println!("📋 Transforms registered from file: {:?}", registered_transforms.keys().collect::<Vec<_>>());
    
    // Should have 1 transform for the BlogPostWordIndex schema
    let expected_transform_id = "BlogPostWordIndex";
    
    assert_eq!(
        registered_transforms.len(),
        1,
        "Should have 1 registered transform for BlogPostWordIndex schema loaded from file"
    );
    
    // Verify the transform exists
    assert!(
        registered_transforms.contains_key(expected_transform_id),
        "Transform '{}' should be registered when loading from file",
        expected_transform_id
    );
    
    println!("✅ BlogPostWordIndex transform registration from file test passed!");
}
