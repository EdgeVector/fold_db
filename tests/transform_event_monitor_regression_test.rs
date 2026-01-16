use datafold::fold_db_core::FoldDB;
use std::thread;
use std::time::Duration;
use tempfile::TempDir;

/// Test to ensure that duplicate transform registration is prevented
/// This test verifies the regression fix for preventing re-registration of already existing transforms
/// by loading the same schema twice and ensuring no duplicate transforms are created
#[tokio::test]
async fn test_duplicate_transform_registration_prevention() {
    use serde_json::json;

    // Create temporary directory for test database
    let temp_dir = TempDir::new().expect("Failed to create temp directory");
    let test_db_path = temp_dir
        .path()
        .to_str()
        .expect("Failed to convert path to string");

    // Create FoldDB instance
    let fold_db = FoldDB::new(test_db_path)
        .await
        .expect("Failed to create FoldDB instance");
    let transform_manager = fold_db.transform_manager();

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

    let blogpost_schema_str =
        serde_json::to_string(&blogpost_schema_json).expect("Failed to serialize BlogPost schema");

    fold_db
        .schema_manager()
        .load_schema_from_json(&blogpost_schema_str)
        .await
        .expect("Failed to load BlogPost schema");

    // Load the BlogPostWordIndex schema with transform_fields (first time)
    let wordindex_schema_json = json!({
        "name": "BlogPostWordIndex",
        "key": {
            "hash_field": "word",
            "range_field": "publish_date"
        },
        "transform_fields": {
            "word": "BlogPost.content.split_by_word()",
            "publish_date": "BlogPost.publish_date",
            "content": "BlogPost.content",
            "author": "BlogPost.author",
            "title": "BlogPost.title",
            "tags": "BlogPost.tags"
        }
    });

    let wordindex_schema_str = serde_json::to_string(&wordindex_schema_json)
        .expect("Failed to serialize BlogPostWordIndex schema");

    // First load of BlogPostWordIndex schema
    fold_db
        .schema_manager()
        .load_schema_from_json(&wordindex_schema_str)
        .await
        .expect("Failed to load BlogPostWordIndex schema");

    // Wait for async event processing
    thread::sleep(Duration::from_millis(100));

    // Verify first registration was successful
    let registered_transforms = transform_manager
        .list_transforms()
        .expect("Failed to list transforms");
    assert!(
        registered_transforms.contains_key("BlogPostWordIndex"),
        "Transform should be registered after first load"
    );
    assert_eq!(
        registered_transforms.len(),
        1,
        "Should have exactly one transform registered after first load"
    );

    // Now load the same schema again (should be prevented from duplicate registration)
    fold_db
        .schema_manager()
        .load_schema_from_json(&wordindex_schema_str)
        .await
        .expect("Failed to load BlogPostWordIndex schema again");

    // Wait for async event processing
    thread::sleep(Duration::from_millis(100));

    // Verify no duplicate transform was created
    let final_transforms = transform_manager
        .list_transforms()
        .expect("Failed to list final transforms");
    assert_eq!(
        final_transforms.len(),
        1,
        "Should still have exactly one transform registered (no duplicates)"
    );
    assert!(
        final_transforms.contains_key("BlogPostWordIndex"),
        "Transform should still be registered"
    );

    // Verify field-to-transform mappings are still correct
    let transforms_for_content = transform_manager
        .get_transforms_for_field("BlogPost", "content")
        .expect("Failed to get transforms for content field");

    assert!(
        transforms_for_content.contains("BlogPostWordIndex"),
        "BlogPost.content should still map to BlogPostWordIndex transform"
    );

    // Close the database
    fold_db.close().expect("Failed to close FoldDB");
}

/// Test to verify that transform registration works correctly through schema loading
/// This test ensures the TransformEventMonitor can handle valid transform registrations
#[tokio::test]
async fn test_transform_registration_through_schema_loading() {
    use serde_json::json;

    // Create temporary directory for test database
    let temp_dir = TempDir::new().expect("Failed to create temp directory");
    let test_db_path = temp_dir
        .path()
        .to_str()
        .expect("Failed to convert path to string");

    // Create FoldDB instance
    let fold_db = FoldDB::new(test_db_path)
        .await
        .expect("Failed to create FoldDB instance");
    let transform_manager = fold_db.transform_manager();

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

    let blogpost_schema_str =
        serde_json::to_string(&blogpost_schema_json).expect("Failed to serialize BlogPost schema");

    fold_db
        .schema_manager()
        .load_schema_from_json(&blogpost_schema_str)
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
            "word": "BlogPost.content.split_by_word()",
            "publish_date": "BlogPost.publish_date",
            "content": "BlogPost.content",
            "author": "BlogPost.author",
            "title": "BlogPost.title",
            "tags": "BlogPost.tags"
        }
    });

    let wordindex_schema_str = serde_json::to_string(&wordindex_schema_json)
        .expect("Failed to serialize BlogPostWordIndex schema");

    // Load BlogPostWordIndex schema - this should trigger transform registration
    fold_db
        .schema_manager()
        .load_schema_from_json(&wordindex_schema_str)
        .await
        .expect("Failed to load BlogPostWordIndex schema");

    // Wait for async event processing
    thread::sleep(Duration::from_millis(100));

    // Verify transform was registered
    let registered_transforms = transform_manager
        .list_transforms()
        .expect("Failed to list transforms");
    assert!(
        registered_transforms.contains_key("BlogPostWordIndex"),
        "Transform should be registered after schema loading"
    );
    assert_eq!(
        registered_transforms.len(),
        1,
        "Should have exactly one transform registered"
    );

    // Verify field-to-transform mappings were created correctly
    let expected_trigger_fields = vec!["content", "publish_date", "author", "title", "tags"];

    for field in &expected_trigger_fields {
        let transforms_for_field = transform_manager
            .get_transforms_for_field("BlogPost", field)
            .expect("Failed to get transforms for field");

        assert!(
            transforms_for_field.contains("BlogPostWordIndex"),
            "Field '{}' should trigger BlogPostWordIndex transform",
            field
        );
    }

    // Close the database
    fold_db.close().expect("Failed to close FoldDB");
}
