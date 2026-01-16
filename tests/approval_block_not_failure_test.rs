use datafold::fold_db_core::infrastructure::message_bus::query_events::MutationExecuted;
// use datafold::fold_db_core::infrastructure::message_bus::query_events::MutationExecuted; // Duplicated
// use datafold::fold_db_core::infrastructure::message_bus::schema_events::TransformExecuted;
use datafold::fold_db_core::infrastructure::message_bus::Event;
use datafold::fold_db_core::FoldDB;
use datafold::schema::SchemaState;
use serde_json::json;
use std::time::Duration;
use tempfile::TempDir;

/// Test to verify that unapproved transforms don't emit TransformTriggered events
/// With the optimization, unapproved transforms are filtered BEFORE event emission,
/// preventing unnecessary event traffic and execution attempts
#[tokio::test]
async fn test_approval_block_not_counted_as_failure() {
    let temp_dir = TempDir::new().expect("Failed to create temp directory");
    let test_db_path = temp_dir
        .path()
        .to_str()
        .expect("Failed to convert path to string");

    let fold_db = FoldDB::new(test_db_path)
        .await
        .expect("Failed to create FoldDB");

    // Load and approve BlogPost schema
    let blogpost_schema_json = json!({
        "name": "BlogPost",
        "key": {"range_field": "publish_date"},
        "fields": {
            "title": {},
            "content": {},
            "author": {},
            "publish_date": {},
            "tags": {}
        }
    });

    fold_db
        .schema_manager()
        .load_schema_from_json(&serde_json::to_string(&blogpost_schema_json).unwrap())
        .await
        .expect("Failed to load BlogPost schema");

    fold_db
        .schema_manager()
        .set_schema_state("BlogPost", SchemaState::Approved)
        .await
        .expect("Failed to approve BlogPost schema");

    // Load BlogPostWordIndex schema but DON'T approve it
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

    fold_db
        .schema_manager()
        .load_schema_from_json(&serde_json::to_string(&wordindex_schema_json).unwrap())
        .await
        .expect("Failed to load BlogPostWordIndex schema");

    // Wait for registration
    tokio::time::sleep(Duration::from_millis(100)).await;

    // Get initial statistics
    let initial_stats = fold_db.get_event_statistics();

    println!("📊 Initial statistics:");
    println!("  Executions: {}", initial_stats.transform_executions);
    println!("  Successes: {}", initial_stats.transform_successes);
    println!("  Failures: {}", initial_stats.transform_failures);

    // Subscribe to TransformTriggered and TransformExecuted events
    let message_bus = fold_db.message_bus();
    let mut transform_triggered_consumer = message_bus.subscribe("TransformTriggered").await;
    let mut transform_executed_consumer = message_bus.subscribe("TransformExecuted").await;

    // Trigger transform by publishing MutationExecuted
    let mutation_event = MutationExecuted::new(
        "write_mutation",
        "BlogPost",
        44,
        vec!["content".to_string(), "publish_date".to_string()],
    );

    message_bus
        .publish_event(Event::MutationExecuted(mutation_event))
        .await
        .expect("Failed to publish MutationExecuted event");

    // Wait for transform triggering attempt
    tokio::time::sleep(Duration::from_millis(300)).await;

    // Collect TransformTriggered events - should be NONE for unapproved schemas
    let mut triggered_transforms = Vec::new();
    while let Ok(event) = transform_triggered_consumer.try_recv() {
        if let Event::TransformTriggered(e) = event {
            triggered_transforms.push(e.transform_id);
        }
    }

    // Collect TransformExecuted events - should also be NONE
    let mut executed_results = Vec::new();
    while let Ok(event) = transform_executed_consumer.try_recv() {
        if let Event::TransformExecuted(e) = event {
            executed_results.push((e.transform_id, e.result));
        }
    }

    println!("\n📋 Transform events:");
    println!("  Triggered: {:?}", triggered_transforms);
    println!("  Executed: {:?}", executed_results);

    // Verify NO TransformTriggered event was emitted for the unapproved transform
    let wordindex_triggered = triggered_transforms
        .iter()
        .any(|id| id == "BlogPostWordIndex");

    assert!(
        !wordindex_triggered,
        "TransformTriggered should NOT be emitted for unapproved transform"
    );

    // Verify NO TransformExecuted event was emitted
    let wordindex_executed = executed_results
        .iter()
        .any(|(id, _)| id == "BlogPostWordIndex");

    assert!(
        !wordindex_executed,
        "TransformExecuted should NOT be emitted for unapproved transform"
    );

    println!("✅ Unapproved transform was correctly filtered - no events emitted");

    // Get final statistics
    let final_stats = fold_db.get_event_statistics();

    println!("\n📊 Final statistics:");
    println!("  Executions: {}", final_stats.transform_executions);
    println!("  Successes: {}", final_stats.transform_successes);
    println!("  Failures: {}", final_stats.transform_failures);

    // CRITICAL ASSERTION: Unapproved transforms should NOT generate ANY events or statistics
    // They're filtered BEFORE TransformTriggered event emission
    let executions_unchanged =
        final_stats.transform_executions == initial_stats.transform_executions;
    let successes_unchanged = final_stats.transform_successes == initial_stats.transform_successes;
    let failures_unchanged = final_stats.transform_failures == initial_stats.transform_failures;
    let triggers_unchanged = final_stats.transform_triggers == initial_stats.transform_triggers;

    assert!(
        executions_unchanged,
        "Unapproved transforms should NOT be counted as executions! \
         Initial executions: {}, Final executions: {}",
        initial_stats.transform_executions, final_stats.transform_executions
    );

    assert!(
        successes_unchanged,
        "Unapproved transforms should NOT be counted as successes! \
         Initial successes: {}, Final successes: {}",
        initial_stats.transform_successes, final_stats.transform_successes
    );

    assert!(
        failures_unchanged,
        "Unapproved transforms should NOT be counted as failures! \
         Initial failures: {}, Final failures: {}",
        initial_stats.transform_failures, final_stats.transform_failures
    );

    assert!(
        triggers_unchanged,
        "Unapproved transforms should NOT be counted as triggers! \
         Initial triggers: {}, Final triggers: {}",
        initial_stats.transform_triggers, initial_stats.transform_triggers
    );

    println!("✅ Unapproved transform was NOT counted in any statistics");
    println!("✅ TEST PASSED: Unapproved transforms are filtered before event emission");
}
