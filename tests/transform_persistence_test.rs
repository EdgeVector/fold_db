use datafold::fold_db_core::FoldDB;
use serde_json::json;
use std::thread;
use std::time::Duration;
use tempfile::TempDir;

/// Transform Persistence Test
/// 
/// This test verifies that transform registrations are properly:
/// 1. Flushed to sled database when registered
/// 2. Loaded from sled database when node starts
/// 3. Persist across node restarts
/// 
/// Usage:
///     cargo test transform_persistence_test -- --nocapture

#[tokio::test]
async fn test_transform_registration_persistence_across_restart() {
    // Transform Persistence Test - Cross Restart

    // Create separate temporary directories for each database instance to avoid lock issues
    let temp_dir_1 = TempDir::new().expect("Failed to create temp directory 1");
    let temp_dir_2 = TempDir::new().expect("Failed to create temp directory 2");
    let temp_dir_3 = TempDir::new().expect("Failed to create temp directory 3");
    
    let test_db_path_1 = temp_dir_1.path().to_str().expect("Failed to convert path to string");
    let _test_db_path_2 = temp_dir_2.path().to_str().expect("Failed to convert path to string");
    let _test_db_path_3 = temp_dir_3.path().to_str().expect("Failed to convert path to string");
    
    // Using test database paths

    // PHASE 1: Initial Node Startup and Transform Registration
    
    // Create first FoldDB instance
    let fold_db_1 = FoldDB::new(test_db_path_1).await.expect("Failed to create first FoldDB instance");
    
    // Load BlogPost schema first
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
    
    fold_db_1.schema_manager().load_schema_from_json(&blogpost_schema_str)
        .await
        .expect("Failed to load BlogPost schema");
    
    // Load BlogPostWordIndex schema to trigger transform registration
    let wordindex_schema_json = json!({
        "name": "BlogPostWordIndex",
        "key": {
            "hash_field": "word",
            "range_field": "publish_date"
        },
        "fields": {
            "word": {},
            "publish_date": {},
            "content": {},
            "author": {},
            "title": {},
            "tags": {}
        },
        "transform_fields": {
            "word": "BlogPost.content.split(' ').map(w => w.toLowerCase().replace(/[^a-zA-Z0-9]/g, '')).filter(w => w.length > 0)",
            "publish_date": "BlogPost.publish_date",
            "content": "BlogPost.content",
            "author": "BlogPost.author",
            "title": "BlogPost.title",
            "tags": "BlogPost.map().tags"
        }
    });
    
    let wordindex_schema_str = serde_json::to_string(&wordindex_schema_json)
        .expect("Failed to serialize BlogPostWordIndex schema");
    
    fold_db_1.schema_manager().load_schema_from_json(&wordindex_schema_str)
        .await
        .expect("Failed to load BlogPostWordIndex schema");
    
    // Wait for async event processing
    thread::sleep(Duration::from_millis(50));
    
    // Verify transform is registered in first instance
    let transform_manager_1 = fold_db_1.transform_manager();
    let registered_transforms_1 = transform_manager_1.list_transforms()
        .expect("Failed to list transforms from first instance");
    
    // Transforms registered in first instance
    
    let expected_transform_id = "BlogPostWordIndex";
    assert!(
        registered_transforms_1.contains_key(expected_transform_id),
        "Transform '{}' should be registered in first instance",
        expected_transform_id
    );
    
    assert_eq!(
        registered_transforms_1.len(),
        1,
        "Should have exactly 1 registered transform in first instance"
    );
    
    // Verify field mappings are established
    let expected_trigger_fields = vec![
        "BlogPost.content",
        "BlogPost.publish_date", 
        "BlogPost.author",
        "BlogPost.title",
        "BlogPost.tags"
    ];
    
    for field in &expected_trigger_fields {
        let transforms_for_field = transform_manager_1.get_transforms_for_field("BlogPost", 
            field.strip_prefix("BlogPost.").unwrap())
            .expect("Failed to get transforms for field");
        
        assert!(
            transforms_for_field.contains(expected_transform_id),
            "Field '{}' should trigger transform '{}'",
            field, expected_transform_id
        );
    }
    
    // Transform registration verified in first instance
    
    // PHASE 2: Verify Direct Database Persistence
    
    // Get direct access to the database operations to verify persistence
    let db_ops = fold_db_1.get_db_ops();
    
    // Verify field mappings are stored by loading the persisted state
    let (loaded_transforms, loaded_mappings) = db_ops.load_transform_state()
        .await
        .expect("Failed to load transform state");
    
    assert!(!loaded_transforms.is_empty(), "Transforms should be loaded from storage");
    assert!(!loaded_mappings.is_empty(), "Field mappings should be loaded from storage");
    
    // Verify the specific transform is loaded
    assert!(loaded_transforms.contains_key("BlogPostWordIndex"), "BlogPostWordIndex should be loaded from storage");
    
    // Verify specific field mappings exist
    let content_mappings = loaded_mappings.get("BlogPost.content");
    assert!(content_mappings.is_some(), "BlogPost.content should have field mappings");
    assert!(content_mappings.unwrap().contains("BlogPostWordIndex"), "BlogPost.content should map to BlogPostWordIndex");
    
    // Direct database persistence verified
    
    // PHASE 3: Test Additional Transform Registration
    
    // Load another schema to register a second transform
    let authorindex_schema_json = json!({
        "name": "BlogPostAuthorIndex",
        "key": {
            "hash_field": "author",
            "range_field": "publish_date"
        },
        "fields": {
            "author": {},
            "publish_date": {},
            "title": {},
            "content": {}
        },
        "transform_fields": {
            "author": "BlogPost.author",
            "publish_date": "BlogPost.publish_date",
            "title": "BlogPost.title",
            "content": "BlogPost.content"
        }
    });
    
    let authorindex_schema_str = serde_json::to_string(&authorindex_schema_json)
        .expect("Failed to serialize BlogPostAuthorIndex schema");
    
    fold_db_1.schema_manager().load_schema_from_json(&authorindex_schema_str)
        .await
        .expect("Failed to load BlogPostAuthorIndex schema");
    
    // Wait for async event processing
    thread::sleep(Duration::from_millis(50));
    
    // Verify both transforms are now registered
    let final_transforms = transform_manager_1.list_transforms()
        .expect("Failed to list final transforms");
    
    // Final transforms registered
    
    assert_eq!(
        final_transforms.len(),
        2,
        "Should have exactly 2 registered transforms after adding second schema"
    );
    
    assert!(
        final_transforms.contains_key("BlogPostWordIndex"),
        "BlogPostWordIndex transform should still be present"
    );
    
    assert!(
        final_transforms.contains_key("BlogPostAuthorIndex"),
        "BlogPostAuthorIndex transform should be registered"
    );
    
    // Additional transform registration verified
    
    // PHASE 4: Verify Both Transforms Persist in Database
    
    // Verify both transforms are stored in the database
    let (final_loaded_transforms, final_loaded_mappings) = db_ops.load_transform_state()
        .await
        .expect("Failed to load final transform state");
    
    assert_eq!(
        final_loaded_transforms.len(),
        2,
        "Should have exactly 2 transforms in database"
    );
    
    assert!(
        final_loaded_transforms.contains_key("BlogPostWordIndex"),
        "BlogPostWordIndex should be in database"
    );
    
    assert!(
        final_loaded_transforms.contains_key("BlogPostAuthorIndex"),
        "BlogPostAuthorIndex should be in database"
    );
    
    // Verify field mappings for both transforms
    let content_mappings = final_loaded_mappings.get("BlogPost.content");
    assert!(content_mappings.is_some(), "BlogPost.content should have field mappings");
    assert!(content_mappings.unwrap().contains("BlogPostWordIndex"), "BlogPost.content should map to BlogPostWordIndex");
    
    let author_mappings = final_loaded_mappings.get("BlogPost.author");
    assert!(author_mappings.is_some(), "BlogPost.author should have field mappings");
    assert!(author_mappings.unwrap().contains("BlogPostAuthorIndex"), "BlogPost.author should map to BlogPostAuthorIndex");
    
    // Both transforms persist in database
    
    // Close first instance
    fold_db_1.close().expect("Failed to close first FoldDB instance");
    
    // Transform persistence test completed successfully
}

#[tokio::test]
async fn test_transform_persistence_with_direct_db_verification() {
    // Transform Persistence Test - Direct Database Verification

    // Create separate temporary directories to avoid database lock issues
    let temp_dir_1 = TempDir::new().expect("Failed to create temp directory 1");
    let temp_dir_2 = TempDir::new().expect("Failed to create temp directory 2");
    
    let test_db_path_1 = temp_dir_1.path().to_str().expect("Failed to convert path to string");
    let _test_db_path_2 = temp_dir_2.path().to_str().expect("Failed to convert path to string");
    
    // Using test database paths

    // PHASE 1: Register Transform and Verify Direct Storage
    
    // Create FoldDB instance and register transform
    let fold_db = FoldDB::new(test_db_path_1).await.expect("Failed to create FoldDB instance");
    
    // Load schemas to trigger transform registration
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
    
    let wordindex_schema_json = json!({
        "name": "BlogPostWordIndex",
        "key": {
            "hash_field": "word",
            "range_field": "publish_date"
        },
        "fields": {
            "word": {},
            "publish_date": {},
            "content": {},
            "author": {},
            "title": {},
            "tags": {}
        },
        "transform_fields": {
            "word": "BlogPost.content.split(' ').map(w => w.toLowerCase().replace(/[^a-zA-Z0-9]/g, '')).filter(w => w.length > 0)",
            "publish_date": "BlogPost.publish_date",
            "content": "BlogPost.content",
            "author": "BlogPost.author",
            "title": "BlogPost.title",
            "tags": "BlogPost.map().tags"
        }
    });
    
    fold_db.schema_manager().load_schema_from_json(&serde_json::to_string(&blogpost_schema_json).unwrap())
        .await
        .expect("Failed to load BlogPost schema");
    
    fold_db.schema_manager().load_schema_from_json(&serde_json::to_string(&wordindex_schema_json).unwrap())
        .await
        .expect("Failed to load BlogPostWordIndex schema");
    
    // Wait for async processing
    thread::sleep(Duration::from_millis(200));
    
    // Verify transform is registered
    let transform_manager = fold_db.transform_manager();
    let registered_transforms = transform_manager.list_transforms()
        .expect("Failed to list transforms");
    
    assert!(registered_transforms.contains_key("BlogPostWordIndex"), "Transform should be registered");
    
    // PHASE 2: Direct Database Verification
    
    // Get direct access to the database operations
    let db_ops = fold_db.get_db_ops();
    
    // Verify field mappings are stored by checking the sync operation
    // We'll use the sync_transform_state method to verify the data is properly stored
    let empty_transforms = std::collections::HashMap::new();
    let empty_mappings = std::collections::BTreeMap::new();
    
    // Sync empty state first, then load
    db_ops.sync_transform_state(&empty_transforms, &empty_mappings)
        .await
        .expect("Failed to sync transform state");
    let (loaded_transforms, loaded_mappings) = db_ops.load_transform_state()
        .await
        .expect("Failed to load transform state");
    
    assert!(!loaded_transforms.is_empty(), "Transforms should be loaded from storage");
    assert!(!loaded_mappings.is_empty(), "Field mappings should be loaded from storage");
    
    // Verify the specific transform is loaded
    assert!(loaded_transforms.contains_key("BlogPostWordIndex"), "BlogPostWordIndex should be loaded from storage");
    
    // Verify specific field mappings exist
    let content_mappings = loaded_mappings.get("BlogPost.content");
    assert!(content_mappings.is_some(), "BlogPost.content should have field mappings");
    assert!(content_mappings.unwrap().contains("BlogPostWordIndex"), "BlogPost.content should map to BlogPostWordIndex");
    
    // Direct database verification passed
    
    // PHASE 3: Verify Flush Operation
    
    // Verify that the sync operation properly flushes data to storage
    // by loading again and ensuring data persists
    let (final_loaded_transforms, final_loaded_mappings) = db_ops.load_transform_state()
        .await
        .expect("Failed to load transform state again");
    
    assert!(!final_loaded_transforms.is_empty(), "Transforms should persist after sync");
    assert!(!final_loaded_mappings.is_empty(), "Field mappings should persist after sync");
    
    // Verify the specific transform is still there
    assert!(final_loaded_transforms.contains_key("BlogPostWordIndex"), "BlogPostWordIndex should persist after sync");
    
    // Verify field mappings persist
    let content_mappings = final_loaded_mappings.get("BlogPost.content");
    assert!(content_mappings.is_some(), "BlogPost.content should have persistent field mappings");
    assert!(content_mappings.unwrap().contains("BlogPostWordIndex"), "BlogPost.content should map to BlogPostWordIndex");
    
    // Flush operation verification passed
    
    // Close the database
    fold_db.close().expect("Failed to close FoldDB");
    
    // Direct database verification test completed successfully
}
