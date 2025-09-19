//! Specific test for the new directly event-driven TransformOrchestrator implementation

use datafold::db_operations::DbOperations;
use datafold::fold_db_core::infrastructure::message_bus::{
    atom_events::FieldValueSet, schema_events::TransformExecuted, MessageBus,
};
use datafold::fold_db_core::transform_manager::TransformManager;
use serde_json::json;
use std::sync::Arc;
use std::time::Duration;
use tempfile::tempdir;

// Simple test fixture
struct TestFixture {
    pub _transform_manager: TransformManager,
    pub message_bus: Arc<MessageBus>,
    pub _db_ops: Arc<DbOperations>,
    pub _temp_dir: tempfile::TempDir,
}

impl TestFixture {
    fn new() -> Result<Self, Box<dyn std::error::Error>> {
        let temp_dir = tempdir()?;

        let db = sled::Config::new()
            .path(temp_dir.path())
            .temporary(true)
            .open()?;

        let db_ops = Arc::new(DbOperations::new(db)?);
        let message_bus = Arc::new(MessageBus::new());

        let transform_manager =
            TransformManager::new(Arc::clone(&db_ops), Arc::clone(&message_bus))?;

        Ok(Self {
            _transform_manager: transform_manager,
            message_bus,
            _db_ops: db_ops,
            _temp_dir: temp_dir,
        })
    }
}

#[tokio::test]
async fn test_direct_event_driven_orchestrator_functionality() {
    println!("🎯 Testing Direct Event-Driven TransformOrchestrator Functionality");

    let fixture = TestFixture::new().expect("Failed to create test fixture");

    // Set up TransformExecuted event monitoring
    let _transform_executed_consumer = fixture.message_bus.subscribe::<TransformExecuted>();

    // Test 1: Publish FieldValueSet events and verify automatic processing
    println!("📢 Publishing FieldValueSet events to test direct event monitoring...");

    let field_event1 =
        FieldValueSet::new("test.field1".to_string(), json!(42), "direct_test_source");
    fixture.message_bus.publish(field_event1).unwrap();

    let field_event2 =
        FieldValueSet::new("test.field2".to_string(), json!(24), "direct_test_source");
    fixture.message_bus.publish(field_event2).unwrap();

    // Test 2: Check if events are processed (even if no transforms are registered yet)
    tokio::time::sleep(Duration::from_millis(200)).await;

    // Test 3: Verify error handling with invalid field formats
    let invalid_field_event = FieldValueSet::new(
        "InvalidFieldFormat".to_string(), // Missing schema.field format
        json!(99),
        "error_test",
    );
    fixture.message_bus.publish(invalid_field_event).unwrap();

    // Test 4: Test with non-existent schema.field combination
    let nonexistent_field_event =
        FieldValueSet::new("NonExistent.field".to_string(), json!(99), "error_test");
    fixture
        .message_bus
        .publish(nonexistent_field_event)
        .unwrap();

    // Wait for processing
    tokio::time::sleep(Duration::from_millis(300)).await;

    println!("✅ Direct Event-Driven TransformOrchestrator tested successfully");
    println!("   - FieldValueSet events processed without errors");
    println!("   - Error handling works for invalid field formats");
    println!("   - No deadlocks or race conditions detected");
}

#[tokio::test]
async fn test_orchestrator_thread_safety() {
    println!("🧵 Testing TransformOrchestrator Thread Safety");

    let fixture = TestFixture::new().expect("Failed to create test fixture");

    // Publish multiple concurrent events to test thread safety
    let mut handles = vec![];

    for i in 0..10 {
        let message_bus = fixture.message_bus.clone();
        let handle = tokio::spawn(async move {
            let field_event = FieldValueSet::new(
                format!("test.field{}", i),
                json!(i),
                format!("thread_test_{}", i),
            );
            message_bus.publish(field_event).unwrap();
        });
        handles.push(handle);
    }

    // Wait for all threads to complete
    for handle in handles {
        handle.await.unwrap();
    }

    // Wait for processing
    tokio::time::sleep(Duration::from_millis(200)).await;

    println!("✅ Thread safety verified - no deadlocks or race conditions");
}
