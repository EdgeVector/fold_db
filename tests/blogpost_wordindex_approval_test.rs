use fold_db::fold_db_core::FoldDB;
use fold_db::SchemaState;
use serde_json::json;
use std::thread;
use std::time::Duration;
use tempfile::TempDir;

/// Test to verify that approving the BlogPostWordIndex schema makes transforms visible in the UI
/// This test simulates the complete workflow:
/// 1. Load schemas (BlogPost and BlogPostWordIndex)
/// 2. Approve the BlogPostWordIndex schema
/// 3. Verify transforms are registered and visible
#[tokio::test]
async fn test_blogpost_wordindex_approval_and_transform_visibility() {
    // Create a temporary directory for this test
    let temp_dir = TempDir::new().expect("Failed to create temp directory");
    let test_db_path = temp_dir
        .path()
        .to_str()
        .expect("Failed to convert path to string");

    // Create a new FoldDB instance
    let fold_db = FoldDB::new(test_db_path)
        .await
        .expect("Failed to create FoldDB");

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

    // Load BlogPost schema into the database
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
    fold_db
        .schema_manager()
        .load_schema_from_json(&wordindex_schema_str)
        .await
        .expect("Failed to load BlogPostWordIndex schema");

    // Wait a moment for async event processing
    thread::sleep(Duration::from_millis(100));

    // Verify that BlogPostWordIndex is initially in "Available" state
    let schema_states = fold_db
        .schema_manager()
        .get_schema_states()
        .expect("Failed to get schema states");

    assert_eq!(
        schema_states
            .get("BlogPostWordIndex")
            .copied()
            .unwrap_or_default(),
        SchemaState::Available,
        "BlogPostWordIndex should be in Available state before approval"
    );

    // Get the transform manager to check registered transforms
    let transform_manager = fold_db.transform_manager();

    // List all registered transforms
    let registered_transforms = transform_manager
        .list_transforms()
        .expect("Failed to list transforms");

    // Verify that ONE transform was registered for the BlogPostWordIndex schema
    let expected_transform_id = "BlogPostWordIndex";

    println!(
        "📋 Registered transforms before approval: {:?}",
        registered_transforms.keys().collect::<Vec<_>>()
    );

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

    // Now approve the BlogPostWordIndex schema
    fold_db
        .schema_manager()
        .set_schema_state("BlogPostWordIndex", SchemaState::Approved)
        .await
        .expect("Failed to approve BlogPostWordIndex schema");

    // Wait a moment for state change to be processed
    thread::sleep(Duration::from_millis(100));

    // Verify that BlogPostWordIndex is now in "Approved" state
    let updated_schema_states = fold_db
        .schema_manager()
        .get_schema_states()
        .expect("Failed to get updated schema states");

    assert_eq!(
        updated_schema_states
            .get("BlogPostWordIndex")
            .copied()
            .unwrap_or_default(),
        SchemaState::Approved,
        "BlogPostWordIndex should be in Approved state after approval"
    );

    // Verify transform is still registered after approval
    let updated_registered_transforms = transform_manager
        .list_transforms()
        .expect("Failed to list transforms after approval");

    println!(
        "📋 Registered transforms after approval: {:?}",
        updated_registered_transforms.keys().collect::<Vec<_>>()
    );

    // Check that the transform is still registered after approval
    assert!(
        updated_registered_transforms.contains_key(expected_transform_id),
        "Transform '{}' should still be registered after approving BlogPostWordIndex schema",
        expected_transform_id
    );

    // Verify that only ONE transform is still registered
    assert_eq!(
        updated_registered_transforms.len(),
        1,
        "Should still have exactly 1 registered transform after approval"
    );

    // Verify that the schema with approved state is accessible
    let schemas_with_states = fold_db
        .schema_manager()
        .get_schemas_with_states()
        .expect("Failed to get schemas with states");

    let blogpost_wordindex_schema = schemas_with_states
        .iter()
        .find(|schema_with_state| schema_with_state.name() == "BlogPostWordIndex")
        .expect("BlogPostWordIndex schema should be found in schemas with states");

    assert_eq!(
        blogpost_wordindex_schema.state,
        SchemaState::Approved,
        "BlogPostWordIndex schema should be in Approved state in schemas with states"
    );

    println!("✅ BlogPostWordIndex schema approved successfully");
    println!(
        "✅ Transform '{}' is registered and visible",
        expected_transform_id
    );
    println!("✅ Transform registration persisted through schema approval");
}

/// Test to verify that the approval process works with the actual schema file
#[tokio::test]
async fn test_blogpost_wordindex_approval_from_file() {
    // Create a temporary directory for this test
    let temp_dir = TempDir::new().expect("Failed to create temp directory");
    let test_db_path = temp_dir
        .path()
        .to_str()
        .expect("Failed to convert path to string");

    // Create a new FoldDB instance
    let mut fold_db = FoldDB::new(test_db_path)
        .await
        .expect("Failed to create FoldDB");

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

    let blogpost_schema_str =
        serde_json::to_string(&blogpost_schema_json).expect("Failed to serialize BlogPost schema");

    fold_db
        .schema_manager()
        .load_schema_from_json(&blogpost_schema_str)
        .await
        .expect("Failed to load BlogPost schema");

    // Load the BlogPostWordIndex schema from the actual file
    let wordindex_schema_path = std::env::current_dir()
        .expect("Failed to get current directory")
        .join("tests/schemas_for_testing")
        .join("BlogPostWordIndex.json");

    assert!(
        wordindex_schema_path.exists(),
        "BlogPostWordIndex.json should exist in tests/schemas_for_testing"
    );

    // Load the schema from file - this should trigger transform registration
    fold_db
        .load_schema_from_file(&wordindex_schema_path)
        .await
        .expect("Failed to load BlogPostWordIndex schema from file");

    // Wait for async event processing
    thread::sleep(Duration::from_millis(100));

    // Verify initial state is Available
    let initial_states = fold_db
        .schema_manager()
        .get_schema_states()
        .expect("Failed to get initial schema states");

    assert_eq!(
        initial_states
            .get("BlogPostWordIndex")
            .copied()
            .unwrap_or_default(),
        SchemaState::Available,
        "BlogPostWordIndex should be in Available state when loaded from file"
    );

    // Verify transform is registered
    let transform_manager = fold_db.transform_manager();
    let registered_transforms = transform_manager
        .list_transforms()
        .expect("Failed to list transforms");

    let expected_transform_id = "BlogPostWordIndex";

    assert!(
        registered_transforms.contains_key(expected_transform_id),
        "Transform '{}' should be registered after loading from file",
        expected_transform_id
    );

    assert_eq!(
        registered_transforms.len(),
        1,
        "Should have exactly 1 registered transform for BlogPostWordIndex schema"
    );

    // Now approve the schema
    fold_db
        .schema_manager()
        .set_schema_state("BlogPostWordIndex", SchemaState::Approved)
        .await
        .expect("Failed to approve BlogPostWordIndex schema from file");

    // Wait for state change
    thread::sleep(Duration::from_millis(100));

    // Verify approval worked
    let final_states = fold_db
        .schema_manager()
        .get_schema_states()
        .expect("Failed to get final schema states");

    assert_eq!(
        final_states
            .get("BlogPostWordIndex")
            .copied()
            .unwrap_or_default(),
        SchemaState::Approved,
        "BlogPostWordIndex should be in Approved state after approval from file"
    );

    // Verify transform is still visible after approval
    let final_registered_transforms = transform_manager
        .list_transforms()
        .expect("Failed to list transforms after file-based approval");

    assert!(
        final_registered_transforms.contains_key(expected_transform_id),
        "Transform '{}' should still be registered after file-based approval",
        expected_transform_id
    );

    assert_eq!(
        final_registered_transforms.len(),
        1,
        "Should still have exactly 1 registered transform after approval"
    );

    println!("✅ BlogPostWordIndex schema approved successfully from file");
    println!(
        "✅ Transform '{}' remains visible after approval",
        expected_transform_id
    );
}
