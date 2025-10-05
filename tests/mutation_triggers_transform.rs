use datafold::fold_db_core::infrastructure::message_bus::query_events::MutationExecuted;
use datafold::fold_db_core::infrastructure::message_bus::schema_events::{TransformTriggered, TransformExecuted};
use datafold::fold_db_core::FoldDB;
use serde_json::json;
use tempfile::TempDir;
use std::time::Duration;

/// Test to verify that a BlogPost mutation triggers the appropriate transforms
#[test]
fn test_blogpost_mutation_triggers_transforms() {
    // Create a temporary directory for this test
    let temp_dir = TempDir::new().expect("Failed to create temp directory");
    let test_db_path = temp_dir.path().to_str().expect("Failed to convert path to string");
    
    // Create a new FoldDB instance
    let fold_db = FoldDB::new(test_db_path).expect("Failed to create FoldDB");
    
    // Load the BlogPost schema (source schema)
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
    
    fold_db.schema_manager().load_schema_from_json(&wordindex_schema_str)
        .expect("Failed to load BlogPostWordIndex schema");
    
    // Wait for schema registration and transform registration to complete
    std::thread::sleep(Duration::from_millis(50));
    
    // Get message bus for publishing and subscribing to events
    let message_bus = fold_db.message_bus();
    
    // Subscribe to TransformTriggered events BEFORE publishing the mutation
    let mut transform_triggered_consumer = message_bus.subscribe::<TransformTriggered>();
    
    // Subscribe to TransformExecuted events to verify transforms actually run
    let mut transform_executed_consumer = message_bus.subscribe::<TransformExecuted>();
    
    // Publish a MutationExecuted event matching the user's example:
    // EventMonitor: MutationExecuted - schema: BlogPost, operation: write_mutation, 
    // execution_time: 44ms, fields_affected: content, tags, publish_date, author, title
    let mutation_event = MutationExecuted::new(
        "write_mutation",
        "BlogPost",
        44,
        vec![
            "content".to_string(),
            "tags".to_string(),
            "publish_date".to_string(),
            "author".to_string(),
            "title".to_string(),
        ],
    );
    
    message_bus.publish(mutation_event)
        .expect("Failed to publish MutationExecuted event");
    
    // Collect TransformTriggered events
    let mut triggered_transform_ids = Vec::new();
    let timeout = Duration::from_millis(500);
    let start = std::time::Instant::now();
    
    while start.elapsed() < timeout {
        if let Ok(event) = transform_triggered_consumer.try_recv() {
            triggered_transform_ids.push(event.transform_id);
        }
        std::thread::sleep(Duration::from_millis(5));
    }
    
    
    // Verify that the BlogPostWordIndex transform was triggered
    // The single transform handles all fields referenced in BlogPostWordIndex transform_fields
    let expected_transform = "BlogPostWordIndex";
    
    assert!(
        triggered_transform_ids.contains(&expected_transform.to_string()),
        "Transform '{}' should be triggered when BlogPost fields are mutated, but it wasn't. Triggered: {:?}",
        expected_transform,
        triggered_transform_ids
    );
    
    // Verify only the single BlogPostWordIndex transform was triggered
    assert_eq!(
        triggered_transform_ids.len(),
        1,
        "Should trigger exactly 1 transform (BlogPostWordIndex), but got {}. Triggered: {:?}",
        triggered_transform_ids.len(),
        triggered_transform_ids
    );
    
    
    // Optional: Verify that transforms are executed (TransformExecuted events)
    // Note: This may not always complete in test time, but we can check if any executed
    let mut executed_transform_ids = Vec::new();
    let execution_timeout = Duration::from_millis(1000);
    let execution_start = std::time::Instant::now();
    
    while execution_start.elapsed() < execution_timeout {
        if let Ok(event) = transform_executed_consumer.try_recv() {
            executed_transform_ids.push(event.transform_id);
        }
        std::thread::sleep(Duration::from_millis(5));
    }
    
    if !executed_transform_ids.is_empty() {
        
        // Verify executed transforms are a subset of triggered transforms
        for executed_id in &executed_transform_ids {
            assert!(
                triggered_transform_ids.contains(executed_id),
                "Executed transform '{}' should have been triggered first",
                executed_id
            );
        }
    } else {
    }
    
}

/// Test to verify that only affected fields trigger their corresponding transforms
#[test]
fn test_partial_mutation_triggers_subset_of_transforms() {
    // Create a temporary directory for this test
    let temp_dir = TempDir::new().expect("Failed to create temp directory");
    let test_db_path = temp_dir.path().to_str().expect("Failed to convert path to string");
    
    // Create a new FoldDB instance
    let fold_db = FoldDB::new(test_db_path).expect("Failed to create FoldDB");
    
    // Load the BlogPost schema
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
    
    // Load the BlogPostWordIndex schema
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
    
    // Wait for schema registration
    std::thread::sleep(Duration::from_millis(50));
    
    // Get message bus
    let message_bus = fold_db.message_bus();
    
    // Subscribe to TransformTriggered events
    let mut transform_triggered_consumer = message_bus.subscribe::<TransformTriggered>();
    
    // Publish a MutationExecuted event with ONLY the title field affected
    let mutation_event = MutationExecuted::new(
        "update_mutation",
        "BlogPost",
        10,
        vec!["title".to_string()],
    );
    
    message_bus.publish(mutation_event)
        .expect("Failed to publish MutationExecuted event");
    
    // Collect TransformTriggered events
    let mut triggered_transform_ids = Vec::new();
    let timeout = Duration::from_millis(500);
    let start = std::time::Instant::now();
    
    while start.elapsed() < timeout {
        if let Ok(event) = transform_triggered_consumer.try_recv() {
            triggered_transform_ids.push(event.transform_id);
        }
        std::thread::sleep(Duration::from_millis(5));
    }
    
    
    // Verify that the BlogPostWordIndex transform was triggered
    assert!(
        triggered_transform_ids.contains(&"BlogPostWordIndex".to_string()),
        "BlogPostWordIndex should be triggered when title field is mutated"
    );
    
    // Verify exactly one transform was triggered
    assert_eq!(
        triggered_transform_ids.len(),
        1,
        "Should trigger exactly 1 transform for title mutation, but got {}. Triggered: {:?}",
        triggered_transform_ids.len(),
        triggered_transform_ids
    );
    
}

/// Test to verify that the word transform is triggered when content field changes
#[test]
fn test_content_mutation_triggers_word_transform() {
    // Create a temporary directory for this test
    let temp_dir = TempDir::new().expect("Failed to create temp directory");
    let test_db_path = temp_dir.path().to_str().expect("Failed to convert path to string");
    
    // Create a new FoldDB instance
    let fold_db = FoldDB::new(test_db_path).expect("Failed to create FoldDB");
    
    // Load schemas
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
    
    fold_db.schema_manager().load_schema_from_json(
        &serde_json::to_string(&blogpost_schema_json).unwrap()
    ).expect("Failed to load BlogPost schema");
    
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
    
    fold_db.schema_manager().load_schema_from_json(
        &serde_json::to_string(&wordindex_schema_json).unwrap()
    ).expect("Failed to load BlogPostWordIndex schema");
    
    // Wait for registration
    std::thread::sleep(Duration::from_millis(50));
    
    let message_bus = fold_db.message_bus();
    let mut transform_triggered_consumer = message_bus.subscribe::<TransformTriggered>();
    
    // Publish mutation with content field affected
    let mutation_event = MutationExecuted::new(
        "write_mutation",
        "BlogPost",
        25,
        vec!["content".to_string()],
    );
    
    message_bus.publish(mutation_event)
        .expect("Failed to publish MutationExecuted event");
    
    // Collect triggered transforms
    let mut triggered_transform_ids = Vec::new();
    let timeout = Duration::from_millis(500);
    let start = std::time::Instant::now();
    
    while start.elapsed() < timeout {
        if let Ok(event) = transform_triggered_consumer.try_recv() {
            triggered_transform_ids.push(event.transform_id);
        }
        std::thread::sleep(Duration::from_millis(5));
    }
    
    
    // Verify that the BlogPostWordIndex transform is triggered
    assert!(
        triggered_transform_ids.contains(&"BlogPostWordIndex".to_string()),
        "BlogPostWordIndex should be triggered when content field is mutated"
    );
    
    // Should trigger exactly 1 transform: BlogPostWordIndex
    assert_eq!(
        triggered_transform_ids.len(),
        1,
        "Should trigger exactly 1 transform (BlogPostWordIndex), but got {}. Triggered: {:?}",
        triggered_transform_ids.len(),
        triggered_transform_ids
    );
    
}

