use crate::test_utils::TestFixture;
use datafold::fees::types::config::FieldPaymentConfig;
use datafold::fees::SchemaPaymentConfig;
use datafold::fold_db_core::services::mutation::MutationService;
use datafold::permissions::types::policy::PermissionsPolicy;
use datafold::schema::types::field::{FieldVariant, HashRangeField, RangeField, SingleField};
use datafold::schema::types::json_schema::KeyConfig;
use datafold::schema::types::{Schema, SchemaError, SchemaType};
use serde_json::json;
use std::collections::HashMap;
use std::sync::Arc;

fn build_single_schema() -> Schema {
    let mut fields = HashMap::new();
    let status_field = SingleField::new(
        PermissionsPolicy::default(),
        FieldPaymentConfig::default(),
        HashMap::new(),
    );
    fields.insert("status".to_string(), FieldVariant::Single(status_field));

    Schema {
        name: "UserStatus".to_string(),
        schema_type: SchemaType::Single,
        key: None,
        fields,
        payment_config: SchemaPaymentConfig::default(),
        hash: Some("test_hash".to_string()),
    }
}

fn build_range_schema_with_universal_key() -> Schema {
    let mut fields = HashMap::new();
    let status_field = RangeField::new(
        PermissionsPolicy::default(),
        FieldPaymentConfig::default(),
        HashMap::new(),
    );
    fields.insert("status".to_string(), FieldVariant::Range(status_field));

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

fn build_legacy_range_schema() -> Schema {
    let mut fields = HashMap::new();
    let status_field = RangeField::new(
        PermissionsPolicy::default(),
        FieldPaymentConfig::default(),
        HashMap::new(),
    );
    fields.insert("status".to_string(), FieldVariant::Range(status_field));

    Schema {
        name: "LegacyRange".to_string(),
        schema_type: SchemaType::Range {
            range_key: "range_key".to_string(),
        },
        key: None,
        fields,
        payment_config: SchemaPaymentConfig::default(),
        hash: Some("test_hash".to_string()),
    }
}

fn build_hashrange_schema() -> Schema {
    let mut fields = HashMap::new();
    let content_field = HashRangeField::new(
        PermissionsPolicy::default(),
        FieldPaymentConfig::default(),
        HashMap::new(),
        "user_id".to_string(),
        "timestamp".to_string(),
        "content_atom".to_string(),
    );
    fields.insert(
        "content".to_string(),
        FieldVariant::HashRange(Box::new(content_field)),
    );

    Schema {
        name: "ArticleContent".to_string(),
        schema_type: SchemaType::HashRange,
        key: Some(KeyConfig {
            hash_field: "user_id".to_string(),
            range_field: "timestamp".to_string(),
        }),
        fields,
        payment_config: SchemaPaymentConfig::default(),
        hash: Some("test_hash".to_string()),
    }
}

fn build_hashrange_schema_without_key() -> Schema {
    let mut fields = HashMap::new();
    let content_field = HashRangeField::new(
        PermissionsPolicy::default(),
        FieldPaymentConfig::default(),
        HashMap::new(),
        "user_id".to_string(),
        "timestamp".to_string(),
        "content_atom".to_string(),
    );
    fields.insert(
        "content".to_string(),
        FieldVariant::HashRange(Box::new(content_field)),
    );

    Schema {
        name: "ArticleContent".to_string(),
        schema_type: SchemaType::HashRange,
        key: None,
        fields,
        payment_config: SchemaPaymentConfig::default(),
        hash: Some("test_hash".to_string()),
    }
}

#[test]
fn builds_single_schema_payload() -> Result<(), SchemaError> {
    let fixture = TestFixture::new()?;
    let service = MutationService::new(Arc::clone(&fixture.message_bus));
    let schema = build_single_schema();
    let value = json!("active");

    let normalized = service.normalized_field_value_request(
        &schema,
        "status",
        &value,
        None,
        None,
        Some("mutation-single"),
    )?;

    let payload = normalized
        .request
        .value
        .as_object()
        .expect("payload should be object");
    assert_eq!(payload.get("hash").and_then(|v| v.as_str()), Some(""));
    assert_eq!(payload.get("range").and_then(|v| v.as_str()), Some(""));

    let fields = payload
        .get("fields")
        .and_then(|v| v.as_object())
        .expect("fields map should be present");
    assert_eq!(fields.get("status"), Some(&json!("active")));

    assert!(normalized.context.hash.is_none());
    assert!(normalized.context.range.is_none());
    assert_eq!(
        normalized.context.fields.get("status"),
        Some(&json!("active"))
    );

    let context = normalized
        .request
        .mutation_context
        .as_ref()
        .expect("mutation context expected for diagnostics");
    assert!(context.hash_key.is_none());
    assert!(context.range_key.is_none());
    assert_eq!(context.mutation_hash.as_deref(), Some("mutation-single"));
    assert!(!context.incremental);
    Ok(())
}

#[test]
fn builds_range_schema_payload_with_universal_key() -> Result<(), SchemaError> {
    let fixture = TestFixture::new()?;
    let service = MutationService::new(Arc::clone(&fixture.message_bus));
    let schema = build_range_schema_with_universal_key();
    let value = json!(42);
    let range_value = json!("session-42");

    let normalized = service.normalized_field_value_request(
        &schema,
        "status",
        &value,
        None,
        Some(&range_value),
        Some("mutation-range"),
    )?;

    assert_eq!(normalized.context.hash, None);
    assert_eq!(normalized.context.range.as_deref(), Some("session-42"));
    assert_eq!(normalized.context.fields.get("status"), Some(&json!(42)));

    let payload = normalized
        .request
        .value
        .as_object()
        .expect("payload should be object");
    assert_eq!(
        payload.get("range").and_then(|v| v.as_str()),
        Some("session-42")
    );

    let context = normalized
        .request
        .mutation_context
        .as_ref()
        .expect("range mutation should provide context");
    assert_eq!(context.range_key.as_deref(), Some("session-42"));
    assert!(context.hash_key.is_none());
    assert!(context.incremental);
    Ok(())
}

#[test]
fn builds_legacy_range_schema_payload() -> Result<(), SchemaError> {
    let fixture = TestFixture::new()?;
    let service = MutationService::new(Arc::clone(&fixture.message_bus));
    let schema = build_legacy_range_schema();
    let value = json!("complete");
    let range_value = json!("legacy-range-value");

    let normalized = service.normalized_field_value_request(
        &schema,
        "status",
        &value,
        None,
        Some(&range_value),
        Some("mutation-legacy-range"),
    )?;

    assert_eq!(
        normalized.context.range.as_deref(),
        Some("legacy-range-value")
    );
    assert_eq!(
        normalized.context.fields.get("status"),
        Some(&json!("complete"))
    );

    let payload = normalized
        .request
        .value
        .as_object()
        .expect("payload should be object");
    assert_eq!(
        payload.get("range").and_then(|v| v.as_str()),
        Some("legacy-range-value")
    );

    Ok(())
}

#[test]
fn builds_hashrange_schema_payload() -> Result<(), SchemaError> {
    let fixture = TestFixture::new()?;
    let service = MutationService::new(Arc::clone(&fixture.message_bus));
    let schema = build_hashrange_schema();
    let value = json!({ "text": "Hello World" });
    let hash_value = json!("user-123");
    let range_value = json!("2025-01-01T00:00:00Z");

    let normalized = service.normalized_field_value_request(
        &schema,
        "content",
        &value,
        Some(&hash_value),
        Some(&range_value),
        Some("mutation-hashrange"),
    )?;

    assert_eq!(normalized.context.hash.as_deref(), Some("user-123"));
    assert_eq!(
        normalized.context.range.as_deref(),
        Some("2025-01-01T00:00:00Z")
    );
    assert_eq!(
        normalized.context.fields.get("content"),
        Some(&json!({ "text": "Hello World" }))
    );

    let payload = normalized
        .request
        .value
        .as_object()
        .expect("payload should be object");
    let fields = payload
        .get("fields")
        .and_then(|v| v.as_object())
        .expect("fields map expected");
    assert_eq!(fields.len(), 1);
    assert!(fields.contains_key("content"));

    let context = normalized
        .request
        .mutation_context
        .as_ref()
        .expect("hashrange mutations should carry context");
    assert_eq!(context.hash_key.as_deref(), Some("user-123"));
    assert_eq!(context.range_key.as_deref(), Some("2025-01-01T00:00:00Z"));
    assert!(context.incremental);
    Ok(())
}

#[test]
fn errors_when_hashrange_key_configuration_missing() {
    let fixture = TestFixture::new().expect("fixture");
    let service = MutationService::new(Arc::clone(&fixture.message_bus));
    let schema = build_hashrange_schema_without_key();
    let value = json!({ "text": "Hello" });
    let hash_value = json!("user-123");
    let range_value = json!("2025-01-01T00:00:00Z");

    let result = service.normalized_field_value_request(
        &schema,
        "content",
        &value,
        Some(&hash_value),
        Some(&range_value),
        Some("mutation-missing-key"),
    );

    assert!(result.is_err());
    let error = result.unwrap_err();
    match error {
        SchemaError::InvalidData(message) => {
            assert!(message.contains("requires key configuration"));
        }
        other => panic!("expected InvalidData error, got {:?}", other),
    }
}

#[test]
fn errors_when_range_key_missing_for_range_schema() {
    let fixture = TestFixture::new().expect("fixture");
    let service = MutationService::new(Arc::clone(&fixture.message_bus));
    let schema = build_range_schema_with_universal_key();
    let value = json!(13);

    let result = service.normalized_field_value_request(
        &schema,
        "status",
        &value,
        None,
        None,
        Some("mutation-missing-range"),
    );

    assert!(result.is_err());
    let error = result.unwrap_err();
    match error {
        SchemaError::InvalidData(message) => {
            assert!(message.contains("requires range key value"));
        }
        other => panic!("expected InvalidData error, got {:?}", other),
    }
}

#[test]
fn errors_when_hashrange_range_value_missing() {
    let fixture = TestFixture::new().expect("fixture");
    let service = MutationService::new(Arc::clone(&fixture.message_bus));
    let schema = build_hashrange_schema();
    let value = json!({ "text": "Hello" });
    let hash_value = json!("user-123");

    let result = service.normalized_field_value_request(
        &schema,
        "content",
        &value,
        Some(&hash_value),
        None,
        Some("mutation-missing-range-key"),
    );

    assert!(result.is_err());
    let error = result.unwrap_err();
    match error {
        SchemaError::InvalidData(message) => {
            assert!(message.contains("requires range key value"));
        }
        other => panic!("expected InvalidData error, got {:?}", other),
    }
}
