//! Integration tests verifying MutationService range workflows emit normalized payloads.

use crate::test_utils::{normalized_fields, TestFixture, TEST_WAIT_MS};
use datafold::fees::types::config::FieldPaymentConfig;
use datafold::fees::SchemaPaymentConfig;
use datafold::fold_db_core::infrastructure::message_bus::request_events::{
    FieldValueSetRequest, FieldValueSetResponse,
};
use datafold::fold_db_core::services::mutation::MutationService;
use datafold::permissions::types::policy::PermissionsPolicy;
use datafold::schema::types::field::{FieldVariant, RangeField};
use datafold::schema::types::json_schema::KeyConfig;
use datafold::schema::types::{Schema, SchemaType};
use serde_json::json;
use std::collections::HashMap;
use std::sync::Arc;
use std::time::Duration;

fn build_range_schema() -> Schema {
    let mut fields = HashMap::new();
    let range_field = RangeField::new(
        PermissionsPolicy::default(),
        FieldPaymentConfig::default(),
        HashMap::new(),
    );
    fields.insert("status".to_string(), FieldVariant::Range(range_field));

    Schema {
        name: "SessionState".to_string(),
        schema_type: SchemaType::Range {
            range_key: "legacy_range".to_string(),
        },
        key: Some(KeyConfig {
            hash_field: String::new(),
            range_field: "session_id".to_string(),
        }),
        fields,
        payment_config: SchemaPaymentConfig::default(),
        hash: Some("test_hash".to_string()),
    }
}

#[test]
fn range_mutation_uses_normalized_payload() -> Result<(), Box<dyn std::error::Error>> {
    let fixture = TestFixture::new()?;
    let mutation_service = MutationService::new(Arc::clone(&fixture.message_bus));

    let schema = build_range_schema();
    fixture
        .db_ops
        .store_schema(&schema.name, &schema)
        .map_err(|e| Box::new(e) as Box<dyn std::error::Error>)?;

    let mut fields_and_values = HashMap::new();
    let expected_value = json!({ "state": "online" });
    fields_and_values.insert("status".to_string(), expected_value.clone());

    let mut request_consumer = fixture.message_bus.subscribe::<FieldValueSetRequest>();
    let mut response_consumer = fixture.message_bus.subscribe::<FieldValueSetResponse>();

    let mutation_hash = "range-mutation-1";
    mutation_service
        .update_range_schema_fields(&schema, &fields_and_values, "session-42", mutation_hash)
        .map_err(|e| Box::new(e) as Box<dyn std::error::Error>)?;

    let request = request_consumer
        .recv_timeout(Duration::from_millis(1000))
        .map_err(|_| {
            Box::new(std::io::Error::new(
                std::io::ErrorKind::TimedOut,
                "Timeout waiting for FieldValueSetRequest",
            )) as Box<dyn std::error::Error>
        })?;

    assert_eq!(request.schema_name, schema.name);
    assert_eq!(request.field_name, "status");

    let payload = request.value.as_object().ok_or_else(|| {
        Box::new(std::io::Error::new(
            std::io::ErrorKind::InvalidData,
            "Normalized payload must be an object",
        )) as Box<dyn std::error::Error>
    })?;

    let payload_hash = payload
        .get("hash")
        .and_then(|v| v.as_str())
        .unwrap_or_default()
        .to_string();
    let payload_range = payload
        .get("range")
        .and_then(|v| v.as_str())
        .unwrap_or_default()
        .to_string();
    assert_eq!(payload_hash, "");
    assert_eq!(payload_range, "session-42");

    let fields = payload
        .get("fields")
        .and_then(|v| v.as_object())
        .ok_or_else(|| {
            Box::new(std::io::Error::new(
                std::io::ErrorKind::InvalidData,
                "Normalized payload missing fields map",
            )) as Box<dyn std::error::Error>
        })?;
    assert_eq!(fields.get("status"), Some(&expected_value));

    let context = request
        .mutation_context
        .as_ref()
        .expect("Range mutations should include context");
    let context_hash = context.hash_key.as_deref().unwrap_or_default();
    let context_range = context.range_key.as_deref().unwrap_or_default();
    assert_eq!(context_hash, payload_hash);
    assert_eq!(context_range, payload_range);
    assert_eq!(context.mutation_hash.as_deref(), Some(mutation_hash));
    assert!(context.incremental);

    std::thread::sleep(Duration::from_millis(TEST_WAIT_MS));

    let response = response_consumer
        .recv_timeout(Duration::from_millis(1000))
        .map_err(|_| {
            Box::new(std::io::Error::new(
                std::io::ErrorKind::TimedOut,
                "Timeout waiting for FieldValueSetResponse",
            )) as Box<dyn std::error::Error>
        })?;

    assert!(response.success, "Range mutation should succeed");

    let snapshot = response
        .key_snapshot
        .as_ref()
        .expect("Range responses should include key snapshot");
    assert!(snapshot.hash.as_ref().map(|v| v.is_empty()).unwrap_or(true));

    let snapshot_hash = snapshot
        .fields
        .get("hash")
        .and_then(|value| value.as_str())
        .unwrap_or_default();
    let snapshot_range = snapshot
        .fields
        .get("range")
        .and_then(|value| value.as_str())
        .unwrap_or_default();
    assert_eq!(snapshot_hash, payload_hash);
    assert_eq!(snapshot_range, payload_range);

    if let Some(range_key) = snapshot.range.as_deref() {
        assert_eq!(range_key, payload_range);
    }

    let snapshot_fields = normalized_fields(&snapshot.fields);
    assert_eq!(snapshot_fields.get("status"), Some(&expected_value));

    let molecule_uuid = response
        .molecule_uuid
        .as_ref()
        .expect("Range mutation should return molecule id");
    println!(
        "✅ Range mutation published normalized payload and produced molecule {}",
        molecule_uuid
    );

    Ok(())
}
