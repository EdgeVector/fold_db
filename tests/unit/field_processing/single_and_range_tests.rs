//! Unit tests for Single and Range molecule creation with universal key snapshot
//!
//! Tests the refactored Single and Range flows that use the universal key snapshot
//! instead of heuristic JSON extraction.

use crate::test_utils::{normalized_fields, TestFixture};
use datafold::fees::types::config::FieldPaymentConfig;
use datafold::fees::SchemaPaymentConfig;
use datafold::fold_db_core::infrastructure::message_bus::{
    request_events::{FieldValueSetRequest, FieldValueSetResponse},
    MessageBus,
};
use datafold::fold_db_core::managers::atom::field_processing::resolve_universal_keys;
use datafold::fold_db_core::managers::atom::AtomManager;
use datafold::permissions::types::policy::PermissionsPolicy;
use datafold::schema::json_schema::KeyConfig;
use datafold::schema::types::field::{RangeField, SingleField};
use datafold::schema::types::{FieldVariant, Schema, SchemaType};
use serde_json::json;
use std::collections::HashMap;
use std::sync::Arc;
use std::thread;
use std::time::Duration;

/// Test Single molecule creation with universal key configuration
#[test]
fn test_single_molecule_creation_with_universal_keys() {
    let fixture = TestFixture::new().unwrap();

    // Create a Single schema with universal key configuration
    let schema = Schema {
        name: "TestSingleWithKeys".to_string(),
        schema_type: SchemaType::Single,
        key: Some(KeyConfig {
            hash_field: "user_id".to_string(),
            range_field: "timestamp".to_string(),
        }),
        fields: {
            let mut fields = HashMap::new();
            fields.insert(
                "content".to_string(),
                FieldVariant::Single(SingleField::new(
                    PermissionsPolicy::default(),
                    FieldPaymentConfig::default(),
                    HashMap::new(),
                )),
            );
            fields
        },
        payment_config: SchemaPaymentConfig::default(),
        hash: None,
    };

    fixture.db_ops.store_schema(&schema.name, &schema).unwrap();

    // Create a FieldValueSetRequest
    let request = FieldValueSetRequest::new(
        "test_single_correlation".to_string(),
        "TestSingleWithKeys".to_string(),
        "content".to_string(),
        json!({
            "content": "test content",
            "user_id": "user123",
            "timestamp": "2023-01-01T00:00:00Z"
        }),
        "test_pubkey".to_string(),
    );

    // Test universal key resolution
    let resolved_keys =
        resolve_universal_keys(&fixture.atom_manager, "TestSingleWithKeys", &request.value)
            .unwrap();

    assert_eq!(resolved_keys.hash, Some("user123".to_string()));
    assert_eq!(
        resolved_keys.range,
        Some("2023-01-01T00:00:00Z".to_string())
    );
    let resolved_fields = normalized_fields(&resolved_keys.fields);
    assert_eq!(resolved_fields.get("content"), Some(&json!("test content")));

    // Test end-to-end processing
    let message_bus = Arc::new(MessageBus::new());
    let atom_manager = AtomManager::new((*fixture.db_ops).clone(), Arc::clone(&message_bus));

    let mut response_consumer = message_bus.subscribe::<FieldValueSetResponse>();

    message_bus.publish(request).unwrap();
    thread::sleep(Duration::from_millis(100));

    let response = response_consumer
        .recv_timeout(Duration::from_millis(500))
        .unwrap();

    assert!(response.success);
    assert!(response.molecule_uuid.is_some());
    assert!(response.key_snapshot.is_some());

    let key_snapshot = response.key_snapshot.unwrap();
    assert_eq!(key_snapshot.hash, Some("user123".to_string()));
    assert_eq!(key_snapshot.range, Some("2023-01-01T00:00:00Z".to_string()));
    let snapshot_fields = normalized_fields(&key_snapshot.fields);
    assert_eq!(snapshot_fields.get("content"), Some(&json!("test content")));
}

/// Test Range molecule creation with universal key configuration
#[test]
fn test_range_molecule_creation_with_universal_keys() {
    let fixture = TestFixture::new().unwrap();

    // Create a Range schema with universal key configuration
    let schema = Schema {
        name: "TestRangeWithKeys".to_string(),
        schema_type: SchemaType::Range {
            range_key: "created_at".to_string(),
        },
        key: Some(KeyConfig {
            hash_field: "user_id".to_string(),
            range_field: "created_at".to_string(),
        }),
        fields: {
            let mut fields = HashMap::new();
            fields.insert(
                "score".to_string(),
                FieldVariant::Range(RangeField::new(
                    PermissionsPolicy::default(),
                    FieldPaymentConfig::default(),
                    HashMap::new(),
                )),
            );
            fields
        },
        payment_config: SchemaPaymentConfig::default(),
        hash: None,
    };

    fixture.db_ops.store_schema(&schema.name, &schema).unwrap();

    // Create a FieldValueSetRequest
    let request = FieldValueSetRequest::new(
        "test_range_correlation".to_string(),
        "TestRangeWithKeys".to_string(),
        "score".to_string(),
        json!({
            "score": 95,
            "user_id": "user456",
            "created_at": "2023-02-01T10:00:00Z"
        }),
        "test_pubkey".to_string(),
    );

    // Test universal key resolution
    let resolved_keys =
        resolve_universal_keys(&fixture.atom_manager, "TestRangeWithKeys", &request.value).unwrap();

    assert_eq!(resolved_keys.hash, Some("user456".to_string()));
    assert_eq!(
        resolved_keys.range,
        Some("2023-02-01T10:00:00Z".to_string())
    );
    let resolved_fields = normalized_fields(&resolved_keys.fields);
    assert_eq!(resolved_fields.get("score"), Some(&json!(95)));

    // Test end-to-end processing
    let message_bus = Arc::new(MessageBus::new());
    let atom_manager = AtomManager::new((*fixture.db_ops).clone(), Arc::clone(&message_bus));

    let mut response_consumer = message_bus.subscribe::<FieldValueSetResponse>();

    message_bus.publish(request).unwrap();
    thread::sleep(Duration::from_millis(100));

    let response = response_consumer
        .recv_timeout(Duration::from_millis(500))
        .unwrap();

    assert!(response.success);
    assert!(response.molecule_uuid.is_some());
    assert!(response.key_snapshot.is_some());

    let key_snapshot = response.key_snapshot.unwrap();
    assert_eq!(key_snapshot.hash, Some("user456".to_string()));
    assert_eq!(key_snapshot.range, Some("2023-02-01T10:00:00Z".to_string()));
    let snapshot_fields = normalized_fields(&key_snapshot.fields);
    assert_eq!(snapshot_fields.get("score"), Some(&json!(95)));

    // Verify molecule UUID contains range information
    let molecule_uuid = response.molecule_uuid.unwrap();
    assert!(molecule_uuid.contains("range"));
    assert!(molecule_uuid.contains("2023-02-01T10:00:00Z"));
}

/// Test error response when schema is not found
#[test]
fn test_error_when_schema_not_found() {
    let fixture = TestFixture::new().unwrap();

    // Create a FieldValueSetRequest for a non-existent schema
    let request = FieldValueSetRequest::new(
        "test_legacy_correlation".to_string(),
        "NonExistentSchema".to_string(),
        "field".to_string(),
        json!({
            "field": "value",
            "range_key": "2023-01-01T00:00:00Z"
        }),
        "test_pubkey".to_string(),
    );

    // Test end-to-end processing (should return an error response)
    let message_bus = Arc::new(MessageBus::new());
    let atom_manager = AtomManager::new((*fixture.db_ops).clone(), Arc::clone(&message_bus));

    let mut response_consumer = message_bus.subscribe::<FieldValueSetResponse>();

    message_bus.publish(request).unwrap();
    thread::sleep(Duration::from_millis(100));

    let response = response_consumer
        .recv_timeout(Duration::from_millis(500))
        .unwrap();

    assert!(!response.success);
    assert!(response.molecule_uuid.is_none());
    assert!(response.key_snapshot.is_none());

    let error_msg = response.error.unwrap_or_default();
    assert!(error_msg.contains("Failed to resolve keys for NonExistentSchema.field"));
    assert!(error_msg.contains("Schema 'NonExistentSchema' not found"));
}

/// Test Single molecule creation without key configuration
#[test]
fn test_single_molecule_creation_without_keys() {
    let fixture = TestFixture::new().unwrap();

    // Create a Single schema without key configuration
    let schema = Schema {
        name: "TestSingleNoKeys".to_string(),
        schema_type: SchemaType::Single,
        key: None,
        fields: {
            let mut fields = HashMap::new();
            fields.insert(
                "content".to_string(),
                FieldVariant::Single(SingleField::new(
                    PermissionsPolicy::default(),
                    FieldPaymentConfig::default(),
                    HashMap::new(),
                )),
            );
            fields
        },
        payment_config: SchemaPaymentConfig::default(),
        hash: None,
    };

    fixture.db_ops.store_schema(&schema.name, &schema).unwrap();

    // Create a FieldValueSetRequest
    let request = FieldValueSetRequest::new(
        "test_single_no_keys_correlation".to_string(),
        "TestSingleNoKeys".to_string(),
        "content".to_string(),
        json!({
            "content": "test content"
        }),
        "test_pubkey".to_string(),
    );

    // Test universal key resolution
    let resolved_keys =
        resolve_universal_keys(&fixture.atom_manager, "TestSingleNoKeys", &request.value).unwrap();

    assert_eq!(resolved_keys.hash, None);
    assert_eq!(resolved_keys.range, None);
    let resolved_fields = normalized_fields(&resolved_keys.fields);
    assert_eq!(resolved_fields.get("content"), Some(&json!("test content")));

    // Test end-to-end processing
    let message_bus = Arc::new(MessageBus::new());
    let atom_manager = AtomManager::new((*fixture.db_ops).clone(), Arc::clone(&message_bus));

    let mut response_consumer = message_bus.subscribe::<FieldValueSetResponse>();

    message_bus.publish(request).unwrap();
    thread::sleep(Duration::from_millis(100));

    let response = response_consumer
        .recv_timeout(Duration::from_millis(500))
        .unwrap();

    assert!(response.success);
    assert!(response.molecule_uuid.is_some());
    assert!(response.key_snapshot.is_some());

    let key_snapshot = response.key_snapshot.unwrap();
    assert_eq!(key_snapshot.hash, None);
    assert_eq!(key_snapshot.range, None);
    let snapshot_fields = normalized_fields(&key_snapshot.fields);
    assert_eq!(snapshot_fields.get("content"), Some(&json!("test content")));
}
