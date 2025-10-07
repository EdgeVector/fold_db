use datafold::fold_db_core::infrastructure::message_bus::query_events::MutationExecuted;
use datafold::fold_db_core::infrastructure::message_bus::schema_events::{TransformTriggered, TransformExecuted};
use datafold::fold_db_core::FoldDB;
use datafold::schema::SchemaState;
use serde_json::json;
use tempfile::TempDir;
use std::time::Duration;

/// Test to verify that transforms DO NOT execute when target schema is not approved
#[test]
fn test_transform_requires_approval_to_execute() {
    // Create a temporary directory for this test
    let temp_dir = TempDir::new().expect("Failed to create temp directory");
    let test_db_path = temp_dir.path().to_str().expect("Failed to convert path to string");
    
    // Create a new FoldDB instance
    let fold_db = FoldDB::new(test_db_path).expect("Failed to create FoldDB");
    
    // Load the BlogPost schema (source schema) and approve it
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
    
    // Approve the BlogPost schema
    fold_db.schema_manager().set_schema_state("BlogPost", SchemaState::Approved)
        .expect("Failed to approve BlogPost schema");
    
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
    std::thread::sleep(Duration::from_millis(100));
    
    // Verify BlogPostWordIndex is in Available state (NOT approved)
    let schema_states = fold_db.schema_manager().get_schema_states()
        .expect("Failed to get schema states");
    
    assert_eq!(
        schema_states.get("BlogPostWordIndex").copied().unwrap_or_default(),
        SchemaState::Available,
        "BlogPostWordIndex should be in Available state"
    );
    
    println!("✅ BlogPostWordIndex is in Available state (not approved)");
    
    // Verify transform is registered (registration happens regardless of approval)
    let transform_manager = fold_db.transform_manager();
    let registered_transforms = transform_manager.list_transforms()
        .expect("Failed to list transforms");
    
    assert!(
        registered_transforms.contains_key("BlogPostWordIndex"),
        "Transform should be registered even if schema is not approved"
    );
    
    println!("✅ Transform 'BlogPostWordIndex' is registered");
    
    // Get message bus for publishing and subscribing to events
    let message_bus = fold_db.message_bus();
    
    // Subscribe to TransformTriggered events
    let mut transform_triggered_consumer = message_bus.subscribe::<TransformTriggered>();
    
    // Subscribe to TransformExecuted events
    let mut transform_executed_consumer = message_bus.subscribe::<TransformExecuted>();
    
    // Create a BlogPost mutation to trigger the transform
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
    
    // Wait for events
    std::thread::sleep(Duration::from_millis(200));
    
    // Collect TransformTriggered events
    let mut triggered_transform_ids = Vec::new();
    while let Ok(event) = transform_triggered_consumer.try_recv() {
        triggered_transform_ids.push(event.transform_id);
    }
    
    // Collect TransformExecuted events
    let mut executed_results = Vec::new();
    while let Ok(event) = transform_executed_consumer.try_recv() {
        executed_results.push((event.transform_id, event.result));
    }
    
    println!("📋 Triggered transforms: {:?}", triggered_transform_ids);
    println!("📋 Executed transforms: {:?}", executed_results);
    
    // The transform should be triggered (added to queue)
    assert!(
        triggered_transform_ids.contains(&"BlogPostWordIndex".to_string()),
        "Transform should be triggered for unapproved schema"
    );
    
    // But execution should FAIL because schema is not approved
    let blogpost_word_index_execution = executed_results.iter()
        .find(|(id, _)| id == "BlogPostWordIndex");
    
    if let Some((_, result)) = blogpost_word_index_execution {
        // The result should contain an error message about the schema not being approved
        assert!(
            result.contains("not approved") || result.contains("error"),
            "Transform execution should fail with approval error, got: {}", result
        );
        println!("✅ Transform execution correctly failed: {}", result);
    } else {
        panic!("Transform should have been executed (even if it failed)");
    }
}

/// Test to verify that transforms DO execute when target schema IS approved
#[test]
fn test_transform_executes_when_approved() {
    // Create a temporary directory for this test
    let temp_dir = TempDir::new().expect("Failed to create temp directory");
    let test_db_path = temp_dir.path().to_str().expect("Failed to convert path to string");
    
    // Create a new FoldDB instance
    let fold_db = FoldDB::new(test_db_path).expect("Failed to create FoldDB");
    
    // Load the BlogPost schema (source schema) and approve it
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
    
    // Approve the BlogPost schema
    fold_db.schema_manager().set_schema_state("BlogPost", SchemaState::Approved)
        .expect("Failed to approve BlogPost schema");
    
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
    
    // Approve the BlogPostWordIndex schema
    fold_db.schema_manager().set_schema_state("BlogPostWordIndex", SchemaState::Approved)
        .expect("Failed to approve BlogPostWordIndex schema");
    
    // Wait for schema registration and transform registration to complete
    std::thread::sleep(Duration::from_millis(100));
    
    // Verify BlogPostWordIndex is in Approved state
    let schema_states = fold_db.schema_manager().get_schema_states()
        .expect("Failed to get schema states");
    
    assert_eq!(
        schema_states.get("BlogPostWordIndex").copied().unwrap_or_default(),
        SchemaState::Approved,
        "BlogPostWordIndex should be in Approved state"
    );
    
    println!("✅ BlogPostWordIndex is in Approved state");
    
    // Get message bus for publishing and subscribing to events
    let message_bus = fold_db.message_bus();
    
    // Subscribe to TransformExecuted events
    let mut transform_executed_consumer = message_bus.subscribe::<TransformExecuted>();
    
    // Create a BlogPost mutation to trigger the transform
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
    
    // Wait for events
    std::thread::sleep(Duration::from_millis(200));
    
    // Collect TransformExecuted events
    let mut executed_results = Vec::new();
    while let Ok(event) = transform_executed_consumer.try_recv() {
        executed_results.push((event.transform_id, event.result));
    }
    
    println!("📋 Executed transforms: {:?}", executed_results);
    
    // The transform should execute successfully
    let blogpost_word_index_execution = executed_results.iter()
        .find(|(id, _)| id == "BlogPostWordIndex");
    
    if let Some((_, result)) = blogpost_word_index_execution {
        // The result should NOT contain an error message
        assert!(
            !result.contains("not approved"),
            "Transform execution should succeed for approved schema, got: {}", result
        );
        println!("✅ Transform execution succeeded: {}", result);
    } else {
        panic!("Transform should have been executed for approved schema");
    }
}

