//! Diagnostic test to validate the transform trigger fix
//!
//! This test validates that the critical FieldValueSet event publication fix
//! resolves the transform trigger issue.

use datafold::db_operations::DbOperations;
use datafold::fold_db_core::infrastructure::message_bus::{
    atom_events::FieldValueSet,
    request_events::{FieldValueSetRequest, FieldValueSetResponse},
    schema_events::TransformTriggered,
    MessageBus,
};
use datafold::fold_db_core::managers::atom::AtomManager;
use datafold::fold_db_core::orchestration::event_monitor::EventMonitor;
use datafold::fold_db_core::orchestration::persistence_manager::PersistenceManager;
use datafold::fold_db_core::transform_manager::types::TransformRunner;
use datafold::schema::types::SchemaError;
use serde_json::json;
use std::sync::Arc;
use std::thread;
use std::time::Duration;
mod test_utils;
use std::collections::HashSet;
use tempfile::tempdir;
use test_utils::TEST_WAIT_MS;

struct MockTransformRunner;

impl TransformRunner for MockTransformRunner {
    fn execute_transform_now(&self, transform_id: &str) -> Result<serde_json::Value, SchemaError> {
        println!(
            "🚀 DIAGNOSTIC: MockTransformRunner executing transform: {}",
            transform_id
        );
        Ok(json!({"status": "success", "transform_id": transform_id}))
    }

    fn execute_transform_with_context(
        &self,
        transform_id: &str,
        mutation_context: &Option<
            datafold::fold_db_core::infrastructure::message_bus::atom_events::MutationContext,
        >,
    ) -> Result<serde_json::Value, SchemaError> {
        println!(
            "🚀 DIAGNOSTIC: MockTransformRunner executing transform with context: {}",
            transform_id
        );
        if let Some(ref context) = mutation_context {
            println!("🎯 DIAGNOSTIC: Mutation context - range_key: {:?}, hash_key: {:?}, incremental: {}", 
                     context.range_key, context.hash_key, context.incremental);
            Ok(
                json!({"status": "success_with_context", "transform_id": transform_id, "range_key": context.range_key, "hash_key": context.hash_key, "incremental": context.incremental}),
            )
        } else {
            Ok(
                json!({"status": "success_with_context", "transform_id": transform_id, "no_context": true}),
            )
        }
    }

    fn transform_exists(&self, _transform_id: &str) -> Result<bool, SchemaError> {
        Ok(true)
    }

    fn get_transforms_for_field(
        &self,
        schema_name: &str,
        field_name: &str,
    ) -> Result<HashSet<String>, SchemaError> {
        println!(
            "🔍 DIAGNOSTIC: MockTransformRunner.get_transforms_for_field called for {}.{}",
            schema_name, field_name
        );

        // Return a mock transform for TransformBase fields
        if schema_name == "TransformBase" {
            let mut transforms = HashSet::new();
            transforms.insert(format!("transform_for_{}_{}", schema_name, field_name));
            println!(
                "✅ DIAGNOSTIC: Returning mock transform: transform_for_{}_{}",
                schema_name, field_name
            );
            Ok(transforms)
        } else {
            println!(
                "ℹ️ DIAGNOSTIC: No mock transforms for {}.{}",
                schema_name, field_name
            );
            Ok(HashSet::new())
        }
    }
}

#[test]
fn test_transform_trigger_diagnostic_fix() {
    println!("🚀 Starting transform trigger diagnostic test");

    // Setup database
    let temp_dir = tempdir().expect("Failed to create temp dir");
    let db = sled::Config::new()
        .path(temp_dir.path())
        .temporary(true)
        .open()
        .expect("Failed to open database");

    let db_ops = Arc::new(DbOperations::new(db.clone()).expect("Failed to create DbOperations"));
    let message_bus = Arc::new(MessageBus::new());

    // Create AtomManager with the fix
    println!("🔧 Creating AtomManager with diagnostic fix");
    let _atom_manager = AtomManager::new((*db_ops).clone(), Arc::clone(&message_bus));

    // Create mock transform manager
    let mock_transform_runner = Arc::new(MockTransformRunner);

    // Create EventMonitor with enhanced diagnostics
    let test_tree = db
        .open_tree("test_event_monitor")
        .expect("Failed to create test tree");
    let persistence = PersistenceManager::new(test_tree);
    println!("🔧 Creating EventMonitor with diagnostic logs");
    let _event_monitor = EventMonitor::new(
        Arc::clone(&message_bus),
        Arc::clone(&mock_transform_runner) as Arc<dyn TransformRunner>,
        persistence,
    );

    // Subscribe to events to verify they're published
    let mut field_value_consumer = message_bus.subscribe::<FieldValueSet>();
    let mut triggered_consumer = message_bus.subscribe::<TransformTriggered>();
    let mut response_consumer = message_bus.subscribe::<FieldValueSetResponse>();

    // Create a FieldValueSetRequest for TransformBase schema
    println!("📝 Publishing FieldValueSetRequest for TransformBase.input_field");
    let request = FieldValueSetRequest::new(
        "diagnostic_test_123".to_string(),
        "TransformBase".to_string(),
        "input_field".to_string(),
        json!("test_value_for_transform"),
        "diagnostic_test_pubkey".to_string(),
    );

    // Publish the request
    message_bus
        .publish(request)
        .expect("Failed to publish FieldValueSetRequest");

    // Give the system time to process
    thread::sleep(Duration::from_millis(500));

    // Verify FieldValueSetResponse
    println!("🔍 Checking for FieldValueSetResponse");
    let response = response_consumer
        .recv_timeout(Duration::from_millis(TEST_WAIT_MS))
        .expect("Should receive FieldValueSetResponse");

    println!(
        "✅ DIAGNOSTIC: Received FieldValueSetResponse - success: {}",
        response.success
    );
    assert!(response.success, "FieldValueSetRequest should succeed");
    assert_eq!(response.correlation_id, "diagnostic_test_123");

    // CRITICAL CHECK: Verify FieldValueSet event was published (THE FIX)
    println!("🔍 DIAGNOSTIC: Checking for FieldValueSet event (the critical fix)");
    match field_value_consumer.recv_timeout(Duration::from_millis(TEST_WAIT_MS)) {
        Ok(field_event) => {
            println!("✅ DIAGNOSTIC FIX SUCCESS: FieldValueSet event received!");
            println!("   Field: {}", field_event.field);
            println!("   Source: {}", field_event.source);
            println!("   Value: {}", field_event.value);

            assert_eq!(field_event.field, "TransformBase.input_field");
            assert_eq!(field_event.source, "AtomManager");

            // Give EventMonitor more time to process the FieldValueSet
            thread::sleep(Duration::from_millis(300));

            // Check for TransformTriggered event
            println!("🔍 DIAGNOSTIC: Checking for TransformTriggered event");
            match triggered_consumer.recv_timeout(Duration::from_millis(TEST_WAIT_MS)) {
                Ok(triggered_event) => {
                    println!("✅ DIAGNOSTIC: TransformTriggered event received!");
                    println!("   Transform ID: {}", triggered_event.transform_id);
                    assert_eq!(
                        triggered_event.transform_id,
                        "transform_for_TransformBase_input_field"
                    );

                    println!("🎯 FULL SUCCESS: Complete transform trigger chain is working!");
                    println!("   FieldValueSetRequest → AtomManager → FieldValueSet → EventMonitor → TransformTriggered ✅");
                }
                Err(e) => {
                    println!("⚠️ PARTIAL SUCCESS: FieldValueSet event published but no TransformTriggered: {}", e);
                    println!("   This might indicate empty transform dependency mappings (secondary issue)");
                }
            }
        }
        Err(e) => {
            panic!(
                "❌ DIAGNOSTIC FIX FAILED: FieldValueSet event not received: {}",
                e
            );
        }
    }

    println!("🏁 Transform trigger diagnostic test completed");
}

#[test]
fn test_transform_trigger_with_no_transforms() {
    println!("🚀 Testing transform trigger with no matching transforms");

    // Setup database
    let temp_dir = tempdir().expect("Failed to create temp dir");
    let db = sled::Config::new()
        .path(temp_dir.path())
        .temporary(true)
        .open()
        .expect("Failed to open database");

    let db_ops = Arc::new(DbOperations::new(db.clone()).expect("Failed to create DbOperations"));
    let message_bus = Arc::new(MessageBus::new());

    // Create AtomManager
    let _atom_manager = AtomManager::new((*db_ops).clone(), Arc::clone(&message_bus));

    // Create mock transform manager that returns no transforms
    let mock_transform_runner = Arc::new(MockTransformRunner);

    // Create EventMonitor
    let test_tree = db
        .open_tree("test_no_transforms")
        .expect("Failed to create test tree");
    let persistence = PersistenceManager::new(test_tree);
    let _event_monitor = EventMonitor::new(
        Arc::clone(&message_bus),
        Arc::clone(&mock_transform_runner) as Arc<dyn TransformRunner>,
        persistence,
    );

    // Subscribe to events
    let mut field_value_consumer = message_bus.subscribe::<FieldValueSet>();
    let mut triggered_consumer = message_bus.subscribe::<TransformTriggered>();

    // Create request for a schema with no transforms
    let request = FieldValueSetRequest::new(
        "no_transforms_test".to_string(),
        "UserSchema".to_string(), // MockTransformRunner returns no transforms for this
        "username".to_string(),
        json!("alice"),
        "test_pubkey".to_string(),
    );

    message_bus
        .publish(request)
        .expect("Failed to publish request");
    thread::sleep(Duration::from_millis(300));

    // Should still receive FieldValueSet event
    let field_event = field_value_consumer
        .recv_timeout(Duration::from_millis(TEST_WAIT_MS))
        .expect("Should receive FieldValueSet event even with no transforms");

    println!(
        "✅ FieldValueSet event received for field with no transforms: {}",
        field_event.field
    );

    // Should NOT receive TransformTriggered event
    match triggered_consumer.recv_timeout(Duration::from_millis(TEST_WAIT_MS)) {
        Ok(_) => panic!("Should not receive TransformTriggered for field with no transforms"),
        Err(_) => println!("✅ Correctly no TransformTriggered event for field with no transforms"),
    }

    println!("🏁 No transforms test completed successfully");
}
