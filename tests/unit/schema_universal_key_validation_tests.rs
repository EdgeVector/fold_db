use std::collections::HashMap;

use datafold::schema::types::json_schema::{
    DeclarativeSchemaDefinition, FieldDefinition, KeyConfig,
};
use datafold::schema::types::schema::SchemaType;

fn minimal_fields() -> HashMap<String, FieldDefinition> {
    let mut fields = HashMap::new();
    fields.insert(
        "key".to_string(),
        FieldDefinition {
            atom_uuid: Some("input.map().$atom_uuid".to_string()),
            field_type: Some("String".to_string()),
        },
    );
    fields
}

#[test]
fn single_schema_allows_optional_key_fields() {
    let schema = DeclarativeSchemaDefinition {
        name: "SingleWithKey".to_string(),
        schema_type: SchemaType::Single,
        key: Some(KeyConfig {
            hash_field: "input.map().user_id".to_string(),
            range_field: "input.map().timestamp".to_string(),
        }),
        fields: minimal_fields(),
    };

    assert!(schema.validate().is_ok());
}

#[test]
fn range_schema_allows_optional_key_and_validates_when_present() {
    // With both fields present
    let schema_with_both = DeclarativeSchemaDefinition {
        name: "RangeWithKeyBoth".to_string(),
        schema_type: SchemaType::Range {
            range_key: "key".to_string(),
        },
        key: Some(KeyConfig {
            hash_field: "input.map().user_id".to_string(),
            range_field: "input.map().timestamp".to_string(),
        }),
        fields: minimal_fields(),
    };
    assert!(schema_with_both.validate().is_ok());

    // With only hash_field present
    let schema_with_hash_only = DeclarativeSchemaDefinition {
        name: "RangeWithHashOnly".to_string(),
        schema_type: SchemaType::Range {
            range_key: "key".to_string(),
        },
        key: Some(KeyConfig {
            hash_field: "input.map().user_id".to_string(),
            range_field: String::new(),
        }),
        fields: minimal_fields(),
    };
    assert!(schema_with_hash_only.validate().is_ok());
}

#[test]
fn hashrange_schema_requires_both_key_fields() {
    // Valid when both present
    let valid = DeclarativeSchemaDefinition {
        name: "HashRangeValid".to_string(),
        schema_type: SchemaType::HashRange,
        key: Some(KeyConfig {
            hash_field: "input.map().user_id".to_string(),
            range_field: "input.map().timestamp".to_string(),
        }),
        fields: minimal_fields(),
    };
    assert!(valid.validate().is_ok());

    // Invalid when missing range_field
    let missing_range = DeclarativeSchemaDefinition {
        name: "HashRangeMissingRange".to_string(),
        schema_type: SchemaType::HashRange,
        key: Some(KeyConfig {
            hash_field: "input.map().user_id".to_string(),
            range_field: String::new(),
        }),
        fields: minimal_fields(),
    };
    assert!(missing_range.validate().is_err());

    // Invalid when missing hash_field
    let missing_hash = DeclarativeSchemaDefinition {
        name: "HashRangeMissingHash".to_string(),
        schema_type: SchemaType::HashRange,
        key: Some(KeyConfig {
            hash_field: String::new(),
            range_field: "input.map().timestamp".to_string(),
        }),
        fields: minimal_fields(),
    };
    assert!(missing_hash.validate().is_err());
}
