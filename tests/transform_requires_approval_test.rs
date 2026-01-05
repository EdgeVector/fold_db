use datafold::fold_db_core::infrastructure::message_bus::query_events::MutationExecuted;

use datafold::fold_db_core::infrastructure::message_bus::Event;
use datafold::fold_db_core::FoldDB;
use datafold::schema::SchemaState;
use serde_json::json;
use std::time::Duration;
use tempfile::TempDir;

/// Test to verify that TransformTriggered events are NOT emitted for unapproved transforms
/// This is an optimization that prevents unnecessary event traffic and execution attempts
#[tokio::test]
async fn test_transform_requires_approval_to_execute() {
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

    let blogpost_schema_str =
        serde_json::to_string(&blogpost_schema_json).expect("Failed to serialize BlogPost schema");

    fold_db
        .schema_manager()
        .load_schema_from_json(&blogpost_schema_str)
        .await
        .expect("Failed to load BlogPost schema");

    // Approve the BlogPost schema
    fold_db
        .schema_manager()
        .set_schema_state("BlogPost", SchemaState::Approved)
        .await
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

    fold_db
        .schema_manager()
        .load_schema_from_json(&wordindex_schema_str)
        .await
        .expect("Failed to load BlogPostWordIndex schema");

    // Wait for schema registration and transform registration to complete
    tokio::time::sleep(Duration::from_millis(100)).await;

    // Verify BlogPostWordIndex is in Available state (NOT approved)
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
        "BlogPostWordIndex should be in Available state"
    );

    println!("✅ BlogPostWordIndex is in Available state (not approved)");

    // Verify transform is registered (registration happens regardless of approval)
    let transform_manager = fold_db.transform_manager();
    let registered_transforms = transform_manager
        .list_transforms()
        .expect("Failed to list transforms");

    assert!(
        registered_transforms.contains_key("BlogPostWordIndex"),
        "Transform should be registered even if schema is not approved"
    );

    println!("✅ Transform 'BlogPostWordIndex' is registered");

    // Get message bus for publishing and subscribing to events
    let message_bus = fold_db.message_bus();

    // Subscribe using string topics
    let mut transform_triggered_consumer = message_bus.subscribe("TransformTriggered").await;
    let mut transform_executed_consumer = message_bus.subscribe("TransformExecuted").await;

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

    message_bus
        .publish_event(Event::MutationExecuted(mutation_event))
        .await
        .expect("Failed to publish MutationExecuted event");

    // Wait for events
    tokio::time::sleep(Duration::from_millis(200)).await;

    // Collect TransformTriggered events
    let mut triggered_transform_ids = Vec::new();
    while let Ok(event) = transform_triggered_consumer.try_recv() {
        if let Event::TransformTriggered(e) = event {
            triggered_transform_ids.push(e.transform_id);
        }
    }

    // Collect TransformExecuted events
    let mut executed_results = Vec::new();
    while let Ok(event) = transform_executed_consumer.try_recv() {
        if let Event::TransformExecuted(e) = event {
            executed_results.push((e.transform_id, e.result));
        }
    }

    println!("📋 Triggered transforms: {:?}", triggered_transform_ids);
    println!("📋 Executed transforms: {:?}", executed_results);

    // The transform should NOT be triggered - filtered out before event emission
    assert!(
        !triggered_transform_ids.contains(&"BlogPostWordIndex".to_string()),
        "Transform should NOT be triggered for unapproved schema (filtered before event emission)"
    );

    // And NO execution should occur
    let blogpost_word_index_execution = executed_results
        .iter()
        .find(|(id, _)| id == "BlogPostWordIndex");

    assert!(
        blogpost_word_index_execution.is_none(),
        "Transform should NOT be executed for unapproved schema"
    );

    println!("✅ Transform correctly filtered - no TransformTriggered or TransformExecuted events emitted");
}

/// Test to verify that transforms DO execute when target schema IS approved
#[tokio::test]
async fn test_transform_executes_when_approved() {
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

    let blogpost_schema_str =
        serde_json::to_string(&blogpost_schema_json).expect("Failed to serialize BlogPost schema");

    fold_db
        .schema_manager()
        .load_schema_from_json(&blogpost_schema_str)
        .await
        .expect("Failed to load BlogPost schema");

    // Approve the BlogPost schema
    fold_db
        .schema_manager()
        .set_schema_state("BlogPost", SchemaState::Approved)
        .await
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

    fold_db
        .schema_manager()
        .load_schema_from_json(&wordindex_schema_str)
        .await
        .expect("Failed to load BlogPostWordIndex schema");

    // Approve the BlogPostWordIndex schema
    fold_db
        .schema_manager()
        .set_schema_state("BlogPostWordIndex", SchemaState::Approved)
        .await
        .expect("Failed to approve BlogPostWordIndex schema");

    // Wait for schema registration and transform registration to complete
    tokio::time::sleep(Duration::from_millis(100)).await;

    // Verify BlogPostWordIndex is in Approved state
    let schema_states = fold_db
        .schema_manager()
        .get_schema_states()
        .expect("Failed to get schema states");

    assert_eq!(
        schema_states
            .get("BlogPostWordIndex")
            .copied()
            .unwrap_or_default(),
        SchemaState::Approved,
        "BlogPostWordIndex should be in Approved state"
    );

    println!("✅ BlogPostWordIndex is in Approved state");

    // Get message bus for publishing and subscribing to events
    let message_bus = fold_db.message_bus();

    // Subscribe using string topics
    let mut transform_executed_consumer = message_bus.subscribe("TransformExecuted").await;

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

    message_bus
        .publish_event(Event::MutationExecuted(mutation_event))
        .await
        .expect("Failed to publish MutationExecuted event");

    // Wait for events (increased timeout)
    tokio::time::sleep(Duration::from_millis(500)).await;

    // Collect TransformExecuted events
    let mut executed_results = Vec::new();
    while let Ok(event) = transform_executed_consumer.try_recv() {
        if let Event::TransformExecuted(e) = event {
            executed_results.push((e.transform_id, e.result));
        }
    }

    println!("📋 Executed transforms: {:?}", executed_results);

    // The transform should execute successfully
    let blogpost_word_index_execution = executed_results
        .iter()
        .find(|(id, _)| id == "BlogPostWordIndex");

    if let Some((_, result)) = blogpost_word_index_execution {
        // The result should NOT contain an error message
        assert!(
            !result.contains("not approved"),
            "Transform execution should succeed for approved schema, got: {}",
            result
        );
        println!("✅ Transform execution succeeded: {}", result);
    } else {
        panic!("Transform should have been executed for approved schema");
    }
}
