use datafold::fold_db_core::FoldDB;
use serde_json::json;
use tempfile::TempDir;

/// Test to verify that loading the BlogPostWordIndex schema properly registers declarative transforms
#[test]
fn test_blogpost_wordindex_transform_registration() {
    // Create a temporary directory for this test
    let temp_dir = TempDir::new().expect("Failed to create temp directory");
    let test_db_path = temp_dir.path().to_str().expect("Failed to convert path to string");
    
    // Create a new FoldDB instance
    let fold_db = FoldDB::new(test_db_path).expect("Failed to create FoldDB");
    
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
        .expect("Failed to load BlogPostWordIndex schema");
    
    // Wait a moment for async event processing
    std::thread::sleep(std::time::Duration::from_millis(100));
    
    // Get the transform manager to check registered transforms
    let transform_manager = fold_db.transform_manager();
    
    // List all registered transforms
    let registered_transforms = transform_manager.list_transforms()
        .expect("Failed to list transforms");
    
    // Verify that transforms were registered for each transform field
    let expected_transform_ids = vec![
        "BlogPostWordIndex_word",
        "BlogPostWordIndex_publish_date", 
        "BlogPostWordIndex_content",
        "BlogPostWordIndex_author",
        "BlogPostWordIndex_title",
        "BlogPostWordIndex_tags"
    ];
    
    println!("📋 Registered transforms: {:?}", registered_transforms.keys().collect::<Vec<_>>());
    
    // Check that all expected transforms are registered
    for expected_id in &expected_transform_ids {
        assert!(
            registered_transforms.contains_key(*expected_id),
            "Transform '{}' should be registered after loading BlogPostWordIndex schema",
            expected_id
        );
    }
    
    // Verify that the number of registered transforms matches expectations
    assert_eq!(
        registered_transforms.len(),
        expected_transform_ids.len(),
        "Should have exactly {} registered transforms for BlogPostWordIndex schema",
        expected_transform_ids.len()
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
        let transforms_for_field = transform_manager.get_transforms_for_field("BlogPost", &field.split('.').nth(1).unwrap())
            .expect("Failed to get transforms for field");
        
        println!("🔗 Transforms for field '{}': {:?}", field, transforms_for_field);
        
        // Each BlogPost field should trigger at least one transform
        assert!(
            !transforms_for_field.is_empty(),
            "Field '{}' should trigger at least one transform",
            field
        );
    }
    
    // Verify that BlogPost.content triggers the word transform specifically
    let transforms_for_content = transform_manager.get_transforms_for_field("BlogPost", "content")
        .expect("Failed to get transforms for content field");
    
    assert!(
        transforms_for_content.contains("BlogPostWordIndex_word"),
        "BlogPost.content should trigger the BlogPostWordIndex_word transform"
    );
    
    // Verify that transforms are properly stored in the database
    for expected_id in &expected_transform_ids {
        assert!(
            transform_manager.list_transforms().expect("Failed to list transforms").contains_key(*expected_id),
            "Transform '{}' should exist in the database",
            expected_id
        );
    }
    
    // Verify the transform schema structure
    for (transform_id, transform) in &registered_transforms {
        assert_eq!(
            transform.schema.name,
            "BlogPostWordIndex",
            "Transform '{}' should have schema name 'BlogPostWordIndex'",
            transform_id
        );
        
        // Verify that the transform has the correct key configuration
        assert!(transform.schema.key.is_some(), "Transform '{}' should have key configuration", transform_id);
        let key_config = transform.schema.key.as_ref().unwrap();
        assert_eq!(key_config.hash_field, Some("word".to_string()));
        assert_eq!(key_config.range_field, Some("publish_date".to_string()));
    }
    
    println!("✅ All BlogPostWordIndex transform registration tests passed!");
}

/// Test to verify that the declarative transform registration works with the actual schema file
#[test]
fn test_blogpost_wordindex_from_file() {
    // Create a temporary directory for this test
    let temp_dir = TempDir::new().expect("Failed to create temp directory");
    let test_db_path = temp_dir.path().to_str().expect("Failed to convert path to string");
    
    // Create a new FoldDB instance
    let mut fold_db = FoldDB::new(test_db_path).expect("Failed to create FoldDB");
    
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
        .expect("Failed to load BlogPost schema");
    
    // Load the BlogPostWordIndex schema from the actual file
    let wordindex_schema_path = std::env::current_dir()
        .expect("Failed to get current directory")
        .join("available_schemas")
        .join("BlogPostWordIndex.json");
    
    assert!(wordindex_schema_path.exists(), "BlogPostWordIndex.json should exist in available_schemas");
    
    // Load the schema from file - this should trigger transform registration
    fold_db.load_schema_from_file(&wordindex_schema_path)
        .expect("Failed to load BlogPostWordIndex schema from file");
    
    // Wait for async event processing
    std::thread::sleep(std::time::Duration::from_millis(100));
    
    // Verify transforms were registered
    let transform_manager = fold_db.transform_manager();
    let registered_transforms = transform_manager.list_transforms()
        .expect("Failed to list transforms");
    
    println!("📋 Transforms registered from file: {:?}", registered_transforms.keys().collect::<Vec<_>>());
    
    // Should have 6 transforms for the 6 transform_fields in the schema
    assert_eq!(
        registered_transforms.len(),
        6,
        "Should have 6 registered transforms for BlogPostWordIndex schema loaded from file"
    );
    
    // Verify specific transforms exist
    let expected_transforms = [
        "BlogPostWordIndex_word",
        "BlogPostWordIndex_publish_date",
        "BlogPostWordIndex_content", 
        "BlogPostWordIndex_author",
        "BlogPostWordIndex_title",
        "BlogPostWordIndex_tags"
    ];
    
    for expected_transform in &expected_transforms {
        assert!(
            registered_transforms.contains_key(*expected_transform),
            "Transform '{}' should be registered when loading from file",
            expected_transform
        );
    }
    
    println!("✅ BlogPostWordIndex transform registration from file test passed!");
}
