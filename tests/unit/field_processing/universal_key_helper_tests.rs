//! Unit tests for universal key helper functions in field processing
//!
//! Tests the resolve_universal_keys helper function and ResolvedAtomKeys struct
//! for various schema types including Single, Range (legacy + universal), HashRange,
//! and dotted-path configurations.

use crate::test_utils::{normalized_fields, single_schema_with_key, TestFixture};
use datafold::fees::types::config::FieldPaymentConfig;
use datafold::fees::SchemaPaymentConfig;
use datafold::fold_db_core::managers::atom::field_processing::{
    resolve_universal_keys, ResolvedAtomKeys,
};
use datafold::permissions::types::policy::PermissionsPolicy;
use datafold::schema::json_schema::KeyConfig;
use datafold::schema::types::field::{HashRangeField, RangeField, SingleField};
use datafold::schema::types::{FieldVariant, Schema, SchemaType};
use serde_json::json;
use std::collections::HashMap;

/// Test ResolvedAtomKeys struct creation and methods
#[test]
fn test_resolved_atom_keys_creation() {
    let mut fields = serde_json::Map::new();
    fields.insert("content".to_string(), json!("test content"));
    fields.insert("author".to_string(), json!("test author"));

    let resolved_keys = ResolvedAtomKeys::new(
        Some("hash123".to_string()),
        Some("range456".to_string()),
        fields.clone(),
    );

    assert_eq!(resolved_keys.hash, Some("hash123".to_string()));
    assert_eq!(resolved_keys.range, Some("range456".to_string()));
    assert_eq!(resolved_keys.fields, fields);
    assert_eq!(resolved_keys.hash_str(), "hash123");
    assert_eq!(resolved_keys.range_str(), "range456");
}

/// Test ResolvedAtomKeys with None values
#[test]
fn test_resolved_atom_keys_none_values() {
    let fields = serde_json::Map::new();

    let resolved_keys = ResolvedAtomKeys::new(None, None, fields);

    assert_eq!(resolved_keys.hash, None);
    assert_eq!(resolved_keys.range, None);
    assert_eq!(resolved_keys.hash_str(), "");
    assert_eq!(resolved_keys.range_str(), "");
}

/// Test resolve_universal_keys with Single schema without key configuration
#[test]
fn test_resolve_universal_keys_single_no_key() {
    let fixture = TestFixture::new().unwrap();

    // Create a Single schema without key configuration
    let schema = Schema {
        name: "TestSingle".to_string(),
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

    let request_payload = json!({
        "content": "test content",
        "author": "test author"
    });

    let result =
        resolve_universal_keys(&fixture.atom_manager, "TestSingle", &request_payload).unwrap();

    assert_eq!(result.hash, None);
    assert_eq!(result.range, None);
    let normalized = normalized_fields(&result.fields);
    assert_eq!(normalized.len(), 2);
    assert_eq!(normalized.get("content"), Some(&json!("test content")));
    assert_eq!(normalized.get("author"), Some(&json!("test author")));
}

/// Test resolve_universal_keys with Single schema with key configuration
#[test]
fn test_resolve_universal_keys_single_with_key() {
    let fixture = TestFixture::new().unwrap();

    // Create a Single schema with key configuration
    let schema = Schema {
        name: "TestSingleKey".to_string(),
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

    let request_payload = json!({
        "content": "test content",
        "user_id": "user123",
        "timestamp": "2023-01-01T00:00:00Z"
    });

    let result =
        resolve_universal_keys(&fixture.atom_manager, "TestSingleKey", &request_payload).unwrap();

    assert_eq!(result.hash, Some("user123".to_string()));
    assert_eq!(result.range, Some("2023-01-01T00:00:00Z".to_string()));
    let normalized = normalized_fields(&result.fields);
    assert_eq!(normalized.get("content"), Some(&json!("test content")));
}

/// Test resolve_universal_keys with Range schema (legacy)
#[test]
fn test_resolve_universal_keys_range_legacy() {
    let fixture = TestFixture::new().unwrap();

    // Create a Range schema with legacy range_key
    let schema = Schema {
        name: "TestRangeLegacy".to_string(),
        schema_type: SchemaType::Range {
            range_key: "created_at".to_string(),
        },
        key: None,
        fields: {
            let mut fields = HashMap::new();
            fields.insert(
                "content".to_string(),
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

    let request_payload = json!({
        "content": "test content",
        "created_at": "2023-01-01T00:00:00Z"
    });

    let result =
        resolve_universal_keys(&fixture.atom_manager, "TestRangeLegacy", &request_payload).unwrap();

    assert_eq!(result.hash, None);
    assert_eq!(result.range, Some("2023-01-01T00:00:00Z".to_string()));
    let normalized = normalized_fields(&result.fields);
    assert_eq!(normalized.get("content"), Some(&json!("test content")));
}

/// Test resolve_universal_keys with Range schema (universal key configuration)
#[test]
fn test_resolve_universal_keys_range_universal() {
    let fixture = TestFixture::new().unwrap();

    // Create a Range schema with universal key configuration
    let schema = Schema {
        name: "TestRangeUniversal".to_string(),
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
                "content".to_string(),
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

    let request_payload = json!({
        "content": "test content",
        "created_at": "2023-01-01T00:00:00Z",
        "user_id": "user123"
    });

    let result = resolve_universal_keys(
        &fixture.atom_manager,
        "TestRangeUniversal",
        &request_payload,
    )
    .unwrap();

    assert_eq!(result.hash, Some("user123".to_string()));
    assert_eq!(result.range, Some("2023-01-01T00:00:00Z".to_string()));
    let normalized = normalized_fields(&result.fields);
    assert_eq!(normalized.get("content"), Some(&json!("test content")));
}

/// Test resolve_universal_keys errors when Range schema payload is missing configured range field
#[test]
fn test_resolve_universal_keys_range_missing_configured_field() {
    let fixture = TestFixture::new().unwrap();

    let schema = Schema {
        name: "TestRangeMissingConfigured".to_string(),
        schema_type: SchemaType::Range {
            range_key: "created_at".to_string(),
        },
        key: Some(KeyConfig {
            hash_field: String::new(),
            range_field: "created_at".to_string(),
        }),
        fields: {
            let mut fields = HashMap::new();
            fields.insert(
                "content".to_string(),
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

    let request_payload = json!({
        "content": "test content"
    });

    let result = resolve_universal_keys(
        &fixture.atom_manager,
        "TestRangeMissingConfigured",
        &request_payload,
    );

    assert!(result.is_err());
    let error_msg = result.unwrap_err().to_string();
    assert!(error_msg.contains("Failed to extract keys"));
    assert!(error_msg.contains("requires key.range_field 'created_at'"));
}

/// Test resolve_universal_keys errors when Range schema payload is missing legacy range field
#[test]
fn test_resolve_universal_keys_range_missing_legacy_field() {
    let fixture = TestFixture::new().unwrap();

    let schema = Schema {
        name: "TestRangeMissingLegacy".to_string(),
        schema_type: SchemaType::Range {
            range_key: "legacy_created_at".to_string(),
        },
        key: None,
        fields: {
            let mut fields = HashMap::new();
            fields.insert(
                "content".to_string(),
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

    let request_payload = json!({
        "content": "test content"
    });

    let result = resolve_universal_keys(
        &fixture.atom_manager,
        "TestRangeMissingLegacy",
        &request_payload,
    );

    assert!(result.is_err());
    let error_msg = result.unwrap_err().to_string();
    assert!(error_msg.contains("Failed to extract keys"));
    assert!(error_msg.contains("requires range key field 'legacy_created_at'"));
    assert!(error_msg.contains("normalized range value"));
}

/// Test resolve_universal_keys with HashRange schema
#[test]
fn test_resolve_universal_keys_hashrange() {
    let fixture = TestFixture::new().unwrap();

    // Create a HashRange schema
    let schema = Schema {
        name: "TestHashRange".to_string(),
        schema_type: SchemaType::HashRange,
        key: Some(KeyConfig {
            hash_field: "user_id".to_string(),
            range_field: "timestamp".to_string(),
        }),
        fields: {
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
            fields
        },
        payment_config: SchemaPaymentConfig::default(),
        hash: None,
    };

    fixture.db_ops.store_schema(&schema.name, &schema).unwrap();

    let request_payload = json!({
        "content": "test content",
        "user_id": "user123",
        "timestamp": "2023-01-01T00:00:00Z"
    });

    let result =
        resolve_universal_keys(&fixture.atom_manager, "TestHashRange", &request_payload).unwrap();

    assert_eq!(result.hash, Some("user123".to_string()));
    assert_eq!(result.range, Some("2023-01-01T00:00:00Z".to_string()));
    let normalized = normalized_fields(&result.fields);
    assert_eq!(normalized.get("content"), Some(&json!("test content")));
}

/// Test resolve_universal_keys with dotted path key configuration
#[test]
fn test_resolve_universal_keys_dotted_path() {
    let fixture = TestFixture::new().unwrap();

    // Create a schema with dotted path key configuration
    let schema = single_schema_with_key(
        "TestDottedPath",
        "content",
        Some("data.user.id"),
        Some("metadata.timestamp"),
    );

    fixture.db_ops.store_schema(&schema.name, &schema).unwrap();

    let request_payload = json!({
        "content": "test content",
        "data": {
            "user": {
                "id": "user123"
            }
        },
        "metadata": {
            "timestamp": "2023-01-01T00:00:00Z"
        }
    });

    let result =
        resolve_universal_keys(&fixture.atom_manager, "TestDottedPath", &request_payload).unwrap();

    assert_eq!(result.hash, Some("user123".to_string()));
    assert_eq!(result.range, Some("2023-01-01T00:00:00Z".to_string()));
    let normalized = normalized_fields(&result.fields);
    assert_eq!(normalized.get("content"), Some(&json!("test content")));
}

/// Test that ResolvedAtomKeys::to_snapshot clones metadata without sharing references
#[test]
fn test_resolved_atom_keys_to_snapshot_clones_fields() {
    let mut fields = serde_json::Map::new();
    fields.insert("content".to_string(), json!("snapshot content"));
    fields.insert("author".to_string(), json!("snapshot author"));

    let mut resolved_keys = ResolvedAtomKeys::new(
        Some("hash-123".to_string()),
        Some("range-456".to_string()),
        fields,
    );

    let snapshot = resolved_keys.to_snapshot();
    assert_eq!(snapshot.hash, Some("hash-123".to_string()));
    assert_eq!(snapshot.range, Some("range-456".to_string()));
    assert_eq!(
        snapshot.fields.get("content"),
        Some(&json!("snapshot content"))
    );

    // Mutate the original fields map and ensure the snapshot remains unchanged
    resolved_keys
        .fields
        .insert("extra".to_string(), json!("mutated value"));
    assert_eq!(snapshot.fields.len(), 2);
    assert!(!snapshot.fields.contains_key("extra"));
}

/// HashRange dotted-path keys must surface descriptive errors when data is incomplete
#[test]
fn test_resolve_universal_keys_hashrange_dotted_path_missing_segments() {
    let fixture = TestFixture::new().unwrap();

    let mut fields = HashMap::new();
    fields.insert(
        "content".to_string(),
        FieldVariant::HashRange(Box::new(HashRangeField::new(
            PermissionsPolicy::default(),
            FieldPaymentConfig::default(),
            HashMap::new(),
            "payload.user.id".to_string(),
            "payload.event.timestamp".to_string(),
            "atom_uuid".to_string(),
        ))),
    );

    let schema = Schema {
        name: "HashRangeDottedPath".to_string(),
        schema_type: SchemaType::HashRange,
        key: Some(KeyConfig {
            hash_field: "payload.user.id".to_string(),
            range_field: "payload.event.timestamp".to_string(),
        }),
        fields,
        payment_config: SchemaPaymentConfig::default(),
        hash: None,
    };

    fixture
        .db_ops
        .store_schema(&schema.name, &schema)
        .expect("schema stored");

    // Payload intentionally omits the nested event timestamp to trigger failure
    let request_payload = json!({
        "content": {"body": "test"},
        "payload": {"user": {"id": "user-42"}}
    });

    let error = resolve_universal_keys(
        &fixture.atom_manager,
        "HashRangeDottedPath",
        &request_payload,
    )
    .expect_err("missing dotted path segments should error");

    let message = error.to_string();
    assert!(message.contains("Failed to extract keys for schema 'HashRangeDottedPath'"));
    assert!(message.contains("HashRange range_field 'payload.event.timestamp' not found in data"));
}

/// Test resolve_universal_keys with missing schema
#[test]
fn test_resolve_universal_keys_missing_schema() {
    let fixture = TestFixture::new().unwrap();

    let request_payload = json!({
        "content": "test content"
    });

    let result =
        resolve_universal_keys(&fixture.atom_manager, "NonExistentSchema", &request_payload);

    assert!(result.is_err());
    let error_msg = result.unwrap_err().to_string();
    assert!(error_msg.contains("Schema 'NonExistentSchema' not found"));
}

/// Test resolve_universal_keys with missing key configuration data
#[test]
fn test_resolve_universal_keys_missing_key_data() {
    let fixture = TestFixture::new().unwrap();

    // Create a HashRange schema
    let schema = Schema {
        name: "TestMissingKeyData".to_string(),
        schema_type: SchemaType::HashRange,
        key: Some(KeyConfig {
            hash_field: "user_id".to_string(),
            range_field: "timestamp".to_string(),
        }),
        fields: {
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
            fields
        },
        payment_config: SchemaPaymentConfig::default(),
        hash: None,
    };

    fixture.db_ops.store_schema(&schema.name, &schema).unwrap();

    // Request payload missing required key fields
    let request_payload = json!({
        "content": "test content"
        // Missing user_id and timestamp
    });

    let result = resolve_universal_keys(
        &fixture.atom_manager,
        "TestMissingKeyData",
        &request_payload,
    );

    assert!(result.is_err());
    let error_msg = result.unwrap_err().to_string();
    assert!(error_msg.contains("Failed to extract keys"));
}

/// Test resolve_universal_keys with empty key configuration
#[test]
fn test_resolve_universal_keys_empty_key_config() {
    let fixture = TestFixture::new().unwrap();

    // Create a HashRange schema with empty key configuration
    let schema = Schema {
        name: "TestEmptyKeyConfig".to_string(),
        schema_type: SchemaType::HashRange,
        key: Some(KeyConfig {
            hash_field: "".to_string(),
            range_field: "".to_string(),
        }),
        fields: {
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
            fields
        },
        payment_config: SchemaPaymentConfig::default(),
        hash: None,
    };

    fixture.db_ops.store_schema(&schema.name, &schema).unwrap();

    let request_payload = json!({
        "content": "test content"
    });

    let result = resolve_universal_keys(
        &fixture.atom_manager,
        "TestEmptyKeyConfig",
        &request_payload,
    );

    assert!(result.is_err());
    let error_msg = result.unwrap_err().to_string();
    assert!(error_msg.contains("Failed to extract keys"));
}
