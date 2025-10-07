use datafold::fold_db_core::FoldDB;
use datafold::schema::SchemaState;
use serde_json::json;
use tempfile::TempDir;
use std::time::Duration;

/// Test to verify that backfill is triggered when a schema is approved
#[test]
fn test_backfill_triggered_on_schema_approval() {
    // Create a temporary directory for this test
    let temp_dir = TempDir::new().expect("Failed to create temp directory");
    let test_db_path = temp_dir.path().to_str().expect("Failed to convert path to string");
    
    // Create a new FoldDB instance
    let fold_db = FoldDB::new(test_db_path).expect("Failed to create FoldDB");
    
    // Load and approve the BlogPost schema (source schema)
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
    
    fold_db.schema_manager().set_schema_state("BlogPost", SchemaState::Approved)
        .expect("Failed to approve BlogPost schema");
    
    // Load the BlogPostWordIndex schema with transform_fields (but don't approve it yet)
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
    
    fold_db.schema_manager().load_schema_from_json(&wordindex_schema_str)
        .expect("Failed to load BlogPostWordIndex schema");
    
    // Wait for transform registration to complete
    std::thread::sleep(Duration::from_millis(200));
    
    // Verify BlogPostWordIndex is in Available state (NOT approved)
    let schema_states = fold_db.schema_manager().get_schema_states()
        .expect("Failed to get schema states");
    
    assert_eq!(
        schema_states.get("BlogPostWordIndex").copied().unwrap_or_default(),
        SchemaState::Available,
        "BlogPostWordIndex should be in Available state before approval"
    );
    
    println!("✅ BlogPostWordIndex is in Available state (not approved yet)");
    
    // Get backfills before approval
    let backfills_before = fold_db.get_all_backfills();
    
    let backfill_before_count = backfills_before.len();
    println!("📋 Backfills before approval: {}", backfill_before_count);
    
    // Now approve the BlogPostWordIndex schema - this should trigger a backfill
    println!("🔄 Approving BlogPostWordIndex schema...");
    fold_db.schema_manager().set_schema_state("BlogPostWordIndex", SchemaState::Approved)
        .expect("Failed to approve BlogPostWordIndex schema");
    
    // Wait for the SchemaApproved event to be processed and backfill to run
    std::thread::sleep(Duration::from_millis(500));
    
    // Verify BlogPostWordIndex is now in Approved state
    let schema_states = fold_db.schema_manager().get_schema_states()
        .expect("Failed to get schema states");
    
    assert_eq!(
        schema_states.get("BlogPostWordIndex").copied().unwrap_or_default(),
        SchemaState::Approved,
        "BlogPostWordIndex should be in Approved state after approval"
    );
    
    println!("✅ BlogPostWordIndex is now in Approved state");
    
    // Check if backfill was triggered
    let backfills_after = fold_db.get_all_backfills();
    
    println!("📋 Backfills after approval: {}", backfills_after.len());
    
    // Find the backfill for BlogPostWordIndex
    let blogpost_backfill = backfills_after.iter()
        .find(|b| b.transform_id == "BlogPostWordIndex");
    
    if let Some(backfill) = blogpost_backfill {
        println!("✅ Backfill found for BlogPostWordIndex:");
        println!("   - Status: {:?}", backfill.status);
        println!("   - Records produced: {}", backfill.records_produced);
        println!("   - Source schema: {}", backfill.source_schema);
        
        // Verify backfill completed successfully
        assert_eq!(
            backfill.status,
            datafold::fold_db_core::infrastructure::backfill_tracker::BackfillStatus::Completed,
            "Backfill should be completed"
        );
        
        // Note: records_produced can be 0 if there's no source data, which is fine
        // The important thing is that the backfill ran and completed
        println!("✅ Backfill completed with {} records produced (0 is expected when no source data exists)", backfill.records_produced);
    } else {
        panic!("No backfill found for BlogPostWordIndex after approval");
    }
    
    println!("✅ Backfill was successfully triggered on schema approval");
}

/// Test to verify that backfill is NOT triggered for non-transform schemas
#[test]
fn test_no_backfill_for_regular_schema_approval() {
    // Create a temporary directory for this test
    let temp_dir = TempDir::new().expect("Failed to create temp directory");
    let test_db_path = temp_dir.path().to_str().expect("Failed to convert path to string");
    
    // Create a new FoldDB instance
    let fold_db = FoldDB::new(test_db_path).expect("Failed to create FoldDB");
    
    // Load a regular schema (no transform_fields)
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
    
    // Wait for schema loading to complete
    std::thread::sleep(Duration::from_millis(100));
    
    // Get backfills before approval
    let backfills_before = fold_db.get_all_backfills();
    
    let blogpost_backfill_before = backfills_before.iter()
        .find(|b| b.transform_id == "BlogPost");
    
    assert!(
        blogpost_backfill_before.is_none(),
        "No backfill should exist for regular schema before approval"
    );
    
    println!("✅ No backfill exists for BlogPost before approval (as expected)");
    
    // Approve the BlogPost schema
    fold_db.schema_manager().set_schema_state("BlogPost", SchemaState::Approved)
        .expect("Failed to approve BlogPost schema");
    
    // Wait for the SchemaApproved event to be processed
    std::thread::sleep(Duration::from_millis(200));
    
    // Verify no backfill was created for regular schema
    let backfills_after = fold_db.get_all_backfills();
    
    let blogpost_backfill_after = backfills_after.iter()
        .find(|b| b.transform_id == "BlogPost");
    
    assert!(
        blogpost_backfill_after.is_none(),
        "No backfill should be created for regular schema even after approval"
    );
    
    println!("✅ No backfill was triggered for regular schema (as expected)");
}

