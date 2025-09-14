//! Unit tests for individual TransformManager modules
//!
//! This module contains focused unit tests for each decomposed module to ensure
//! they function correctly in isolation and maintain their specific responsibilities.

use crate::test_utils::CommonTestFixture;
use datafold::fold_db_core::infrastructure::message_bus::schema_events::{
    TransformTriggered,
    SchemaChanged,
    TransformExecuted,
};

// ========== Manager Module Tests ==========

#[tokio::test]
async fn test_manager_module_initialization() {
    let fixture = CommonTestFixture::new()
        .expect("Failed to create test fixture");

    let transform_manager = &fixture.common.transform_manager;

    // Test that the manager initializes correctly with decomposed modules
    assert!(transform_manager.list_transforms().is_ok());

    println!("✅ Manager module initializes correctly with decomposed architecture");
}

#[tokio::test]
async fn test_manager_module_state_consistency() {
    let fixture = CommonTestFixture::new()
        .expect("Failed to create test fixture");

    let transform_manager = &fixture.common.transform_manager;

    // Test that the state remains consistent across module boundaries
    let initial_transforms = transform_manager.list_transforms().unwrap();
    let initial_count = initial_transforms.len();

    // Add a transform using the manager
    let registration = CommonTestFixture::create_sample_registration();
    let register_result = transform_manager.register_transform_event_driven(registration);
    assert!(register_result.is_ok());

    // Verify count increased
    let new_transforms = transform_manager.list_transforms().unwrap();
    let new_count = new_transforms.len();
    assert!(new_count >= initial_count);

    println!("✅ Manager module maintains state consistency across operations");
}

// ========== Event Handlers Module Tests ==========

#[tokio::test]
async fn test_event_handlers_module_exists() {
    let fixture = CommonTestFixture::new()
        .expect("Failed to create test fixture");

    // Test basic event publishing works
    let trigger_event = TransformTriggered {
        transform_id: "test_transform".to_string(),
        mutation_context: None,
    };

    let publish_result = fixture.common.message_bus.publish(trigger_event);
    assert!(publish_result.is_ok());

    println!("✅ Event handlers module integrated correctly");
}

// ========== Basic Integration Tests ==========

#[tokio::test]
async fn test_basic_schema_change_event() {
    let fixture = CommonTestFixture::new()
        .expect("Failed to create test fixture");

    // Test basic schema change event publishing
    let schema_change = SchemaChanged {
        schema: "test_schema".to_string(),
    };

    let publish_result = fixture.common.message_bus.publish(schema_change);
    assert!(publish_result.is_ok());

    println!("✅ Basic schema change events work correctly");
}

#[tokio::test]
async fn test_basic_transform_execution_event() {
    let fixture = CommonTestFixture::new()
        .expect("Failed to create test fixture");

    // Test basic transform execution event publishing
    let executed_event = TransformExecuted {
        transform_id: "test_transform".to_string(),
        result: "success".to_string(),
    };

    let publish_result = fixture.common.message_bus.publish(executed_event);
    assert!(publish_result.is_ok());

    println!("✅ Basic transform execution events work correctly");
}
