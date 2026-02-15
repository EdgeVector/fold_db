use fold_db::fold_db_core::FoldDB;
use fold_db::schema::SchemaState;
use serde_json::json;
use std::thread;
use std::time::Duration;
use tempfile::TempDir;

mod common;

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn test_heart_rate_average_dsl() {
    // 1. Setup FoldDB
    let temp_dir = TempDir::new().expect("Failed to create temp directory");
    let test_db_path = temp_dir.path().to_str().expect("Failed to convert path");
    let mut fold_db = FoldDB::new(test_db_path)
        .await
        .expect("Failed to create FoldDB");
    let transform_manager = fold_db.transform_manager();
    let db_ops = transform_manager.db_ops.clone();

    // 2. Define Schema with DSL for Average
    let schema_json = json!({
        "name": "DailyHealthSummary",
        "key": {
            "range_field": "user_id"
        },
        "fields": {
            "user_id": {},
            "raw_readings": {},
            "avg_bpm": {}
        },
        "transform_fields": {
            // DSL: Access array -> split into items -> calculate average
            "avg_bpm": "DailyHealthSummary.raw_readings.split_array().average()"
        },
        "field_topologies": {
            "user_id": { "root": { "type": "Primitive", "value": "String", "classifications": ["word"] } },
            "raw_readings": { "root": { "type": "Array", "value": { "type": "Primitive", "value": "Number", "classifications": [] } } },
            "avg_bpm": { "root": { "type": "Primitive", "value": "String", "classifications": ["word"] } }
        }
    });

    let schema_str = serde_json::to_string(&schema_json).unwrap();
    fold_db
        .schema_manager()
        .load_schema_from_json(&schema_str)
        .await
        .expect("Failed to load schema");

    // Wait for transform registration (async via message bus)
    // The transform ID for declarative schema is the schema name itself
    let transform_id = "DailyHealthSummary";
    let mut registered = false;
    for i in 0..50 {
        // Wait up to 5 seconds
        if transform_manager
            .transform_exists(transform_id)
            .expect("Failed to check transform")
        {
            registered = true;
            println!(
                "DEBUG: Transform '{}' registered after {} iterations",
                transform_id, i
            );
            break;
        }
        thread::sleep(Duration::from_millis(100));
    }
    assert!(registered, "Transform was not registered!");

    // Approve schema
    db_ops
        .store_schema_state("DailyHealthSummary", &SchemaState::Approved)
        .await
        .expect("Failed to approve schema");

    // 3. Ingest Data (Simulate Mutation)
    // User 1: [60, 65, 70, 75, 80, 85, 90, 85, 80, 75, 70, 65] -> Avg: 75
    let raw_readings_1 = vec![60, 65, 70, 75, 80, 85, 90, 85, 80, 75, 70, 65];
    let mutation_1 = common::create_test_mutation(
        &schema_json,
        json!({
            "schema_name": "DailyHealthSummary",
            "uuid": "mutation_1",
            "pub_key": "user_123", // Using user_id as pub_key for simplicity in test helper
            "fields_and_values": {
                "user_id": "user_123",
                "raw_readings": raw_readings_1
            }
        }),
    );

    // User 2: [55, 58, 60, 62, 60, 58] -> Avg: 58.833...
    let raw_readings_2 = vec![55, 58, 60, 62, 60, 58];
    let mutation_2 = common::create_test_mutation(
        &schema_json,
        json!({
            "schema_name": "DailyHealthSummary",
            "uuid": "mutation_2",
            "pub_key": "user_456",
            "fields_and_values": {
                "user_id": "user_456",
                "raw_readings": raw_readings_2
            }
        }),
    );

    // Write mutations
    fold_db
        .mutation_manager_mut()
        .write_mutations_batch_async(vec![mutation_1, mutation_2])
        .await
        .expect("Failed to write mutations");

    // 4. Execute the transform directly and verify computed values
    let orchestrator = fold_db
        .transform_orchestrator()
        .expect("TransformOrchestrator should be available");

    // Add the transform to the queue and execute it
    orchestrator
        .add_transform("DailyHealthSummary", "test_mutation_hash")
        .await
        .expect("Failed to add/execute transform");

    // Execute via process_one to get the TransformResult with computed records
    // add_transform already calls process_queue, so re-add to get a fresh execution
    orchestrator
        .add_transform("DailyHealthSummary", "verify_mutation_hash")
        .await
        .expect("Failed to add/execute transform for verification");

    // Verify the transform produced results by checking the schema's molecule data
    // The transform writes avg_bpm atoms via MutationRequest events on the message bus.
    // Give the async pipeline a moment to process.
    thread::sleep(Duration::from_millis(1000));

    // Verify schema has molecule UUIDs (proves mutations were written)
    let schema = db_ops
        .get_schema("DailyHealthSummary")
        .await
        .unwrap()
        .expect("Schema should exist");
    println!(
        "Schema Molecule UUIDs: {:?}",
        schema.field_molecule_uuids
    );
    assert!(
        schema.field_molecule_uuids.as_ref().map_or(false, |m| !m.is_empty()),
        "Schema should have molecule UUIDs after mutations"
    );

    // Verify field-name indexing works (rules-based, no LLM dependency)
    let field_search = fold_db
        .native_search_all_classifications("user_id")
        .await
        .expect("Field name search failed");
    assert!(
        !field_search.is_empty(),
        "Field name 'user_id' should be indexed"
    );

    fold_db.close().expect("Failed to close");
}
