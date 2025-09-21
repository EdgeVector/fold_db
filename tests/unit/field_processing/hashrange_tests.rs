//! HashRange field processing tests verifying universal key snapshot adoption

use crate::test_utils::TestFixture;
use datafold::fees::types::config::FieldPaymentConfig;
use datafold::fees::SchemaPaymentConfig;
use datafold::fold_db_core::infrastructure::message_bus::{
    atom_events::FieldValueSet,
    request_events::{FieldValueSetRequest, FieldValueSetResponse},
    MessageBus,
};
use datafold::fold_db_core::managers::atom::AtomManager;
use datafold::permissions::types::policy::PermissionsPolicy;
use datafold::schema::json_schema::KeyConfig;
use datafold::schema::types::{field::HashRangeField, FieldVariant, Schema, SchemaType};
use serde_json::{json, Value};
use std::collections::HashMap;
use std::sync::Arc;
use std::thread;
use std::time::Duration;

/// Helper to create a HashRange schema for tests
fn create_hashrange_schema(name: &str) -> Schema {
    let mut fields = HashMap::new();
    fields.insert(
        "content".to_string(),
        FieldVariant::HashRange(Box::new(HashRangeField::new(
            PermissionsPolicy::default(),
            FieldPaymentConfig::default(),
            HashMap::new(),
            "user_id".to_string(),
            "timestamp".to_string(),
            "atom_uuid".to_string(),
        ))),
    );

    Schema {
        name: name.to_string(),
        schema_type: SchemaType::HashRange,
        key: Some(KeyConfig {
            hash_field: "user_id".to_string(),
            range_field: "timestamp".to_string(),
        }),
        fields,
        payment_config: SchemaPaymentConfig::default(),
        hash: None,
    }
}

#[test]
fn test_hashrange_event_includes_normalized_metadata() {
    let fixture = TestFixture::new().unwrap();
    let schema = create_hashrange_schema("TestHashRangeEvent");
    fixture
        .db_ops
        .store_schema(&schema.name, &schema)
        .expect("schema stored");

    let message_bus = Arc::new(MessageBus::new());
    let atom_manager = AtomManager::new((*fixture.db_ops).clone(), Arc::clone(&message_bus));

    let mut response_consumer = message_bus.subscribe::<FieldValueSetResponse>();
    let mut event_consumer = message_bus.subscribe::<FieldValueSet>();

    let request_payload = json!({
        "value": "Normalized content",
        "user_id": "user123",
        "timestamp": "2023-01-01T00:00:00Z"
    });

    let request = FieldValueSetRequest::new(
        "hashrange_event_correlation".to_string(),
        schema.name.clone(),
        "content".to_string(),
        request_payload.clone(),
        "test_pubkey".to_string(),
    );

    message_bus.publish(request).unwrap();
    thread::sleep(Duration::from_millis(100));

    let response = response_consumer
        .recv_timeout(Duration::from_millis(500))
        .expect("response received");
    assert!(response.success);
    let snapshot = response
        .key_snapshot
        .expect("response should include key snapshot");
    assert_eq!(snapshot.hash, Some("user123".to_string()));
    assert_eq!(snapshot.range, Some("2023-01-01T00:00:00Z".to_string()));
    assert_eq!(
        snapshot.fields.get("value"),
        Some(&json!("Normalized content"))
    );

    let event = event_consumer
        .recv_timeout(Duration::from_millis(500))
        .expect("event received");
    assert_eq!(event.field, "TestHashRangeEvent.content");
    let event_snapshot = event
        .key_snapshot
        .expect("event should include key snapshot");
    assert_eq!(event_snapshot.hash, Some("user123".to_string()));
    assert_eq!(
        event_snapshot.range,
        Some("2023-01-01T00:00:00Z".to_string())
    );
    assert_eq!(
        event_snapshot.fields.get("value"),
        Some(&json!("Normalized content"))
    );
    let context = event
        .mutation_context
        .expect("event should include mutation context");
    assert_eq!(context.hash_key, Some("user123".to_string()));
    assert_eq!(context.range_key, Some("2023-01-01T00:00:00Z".to_string()));
    assert!(!context.incremental);

    // Ensure data persisted using normalized metadata
    let storage_key = format!("{}_{}_{}", schema.name, "content", "user123");
    let stored_map = fixture
        .db_ops
        .get_item::<serde_json::Map<String, Value>>(&storage_key)
        .expect("storage lookup succeeded")
        .expect("hashrange entry stored");
    let stored_entry = stored_map
        .get("2023-01-01T00:00:00Z")
        .expect("range entry stored");
    assert_eq!(
        stored_entry,
        &json!({
            "hash": "user123",
            "range": "2023-01-01T00:00:00Z",
            "fields": {"value": "Normalized content"}
        })
    );

    // Silence unused variable warning for atom_manager until additional assertions are added
    std::mem::drop(atom_manager);
}
