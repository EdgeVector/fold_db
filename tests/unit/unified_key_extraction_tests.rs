use std::collections::HashMap;

use datafold::schema::types::json_schema::KeyConfig;
use datafold::schema::types::schema::{Schema, SchemaType};
use datafold::schema::schema_operations::{extract_unified_keys, shape_unified_result};

fn create_single_schema_with_key() -> Schema {
    Schema {
        name: "TestSingle".to_string(),
        schema_type: SchemaType::Single,
        key: Some(KeyConfig { hash_field: "user_id".to_string(), range_field: "timestamp".to_string() }),
        fields: HashMap::new(),
        payment_config: Default::default(),
        hash: None,
    }
}

fn create_single_schema_without_key() -> Schema {
    Schema {
        name: "TestSingleNoKey".to_string(),
        schema_type: SchemaType::Single,
        key: None,
        fields: HashMap::new(),
        payment_config: Default::default(),
        hash: None,
    }
}

fn create_range_schema_legacy() -> Schema {
    Schema {
        name: "TestRangeLegacy".to_string(),
        schema_type: SchemaType::Range { range_key: "created_at".to_string() },
        key: None,
        fields: HashMap::new(),
        payment_config: Default::default(),
        hash: None,
    }
}

fn create_range_schema_universal() -> Schema {
    Schema {
        name: "TestRangeUniversal".to_string(),
        schema_type: SchemaType::Range { range_key: "created_at".to_string() },
        key: Some(KeyConfig { hash_field: "user_id".to_string(), range_field: "timestamp".to_string() }),
        fields: HashMap::new(),
        payment_config: Default::default(),
        hash: None,
    }
}

fn create_hashrange_schema() -> Schema {
    Schema {
        name: "TestHashRange".to_string(),
        schema_type: SchemaType::HashRange,
        key: Some(KeyConfig { hash_field: "category".to_string(), range_field: "timestamp".to_string() }),
        fields: HashMap::new(),
        payment_config: Default::default(),
        hash: None,
    }
}

#[test]
fn test_extract_unified_keys_single_with_key() {
    let schema = create_single_schema_with_key();
    let data = serde_json::json!({
        "user_id": "user123",
        "timestamp": "2023-01-01T00:00:00Z",
        "content": "test content"
    });

    let result = extract_unified_keys(&schema, &data).unwrap();
    assert_eq!(result.0, Some("user123".to_string()));
    assert_eq!(result.1, Some("2023-01-01T00:00:00Z".to_string()));
}

#[test]
fn test_extract_unified_keys_single_without_key() {
    let schema = create_single_schema_without_key();
    let data = serde_json::json!({
        "content": "test content"
    });

    let result = extract_unified_keys(&schema, &data).unwrap();
    assert_eq!(result.0, None);
    assert_eq!(result.1, None);
}

#[test]
fn test_extract_unified_keys_range_legacy() {
    let schema = create_range_schema_legacy();
    let data = serde_json::json!({
        "created_at": "2023-01-01T00:00:00Z",
        "content": "test content"
    });

    let result = extract_unified_keys(&schema, &data).unwrap();
    assert_eq!(result.0, None);
    assert_eq!(result.1, Some("2023-01-01T00:00:00Z".to_string()));
}

#[test]
fn test_extract_unified_keys_range_universal() {
    let schema = create_range_schema_universal();
    let data = serde_json::json!({
        "user_id": "user123",
        "timestamp": "2023-01-01T00:00:00Z",
        "content": "test content"
    });

    let result = extract_unified_keys(&schema, &data).unwrap();
    assert_eq!(result.0, Some("user123".to_string()));
    assert_eq!(result.1, Some("2023-01-01T00:00:00Z".to_string()));
}

#[test]
fn test_extract_unified_keys_hashrange() {
    let schema = create_hashrange_schema();
    let data = serde_json::json!({
        "category": "tech",
        "timestamp": "2023-01-01T00:00:00Z",
        "content": "test content"
    });

    let result = extract_unified_keys(&schema, &data).unwrap();
    assert_eq!(result.0, Some("tech".to_string()));
    assert_eq!(result.1, Some("2023-01-01T00:00:00Z".to_string()));
}

#[test]
fn test_extract_unified_keys_hashrange_missing_hash() {
    let schema = create_hashrange_schema();
    let data = serde_json::json!({
        "timestamp": "2023-01-01T00:00:00Z",
        "content": "test content"
    });

    let result = extract_unified_keys(&schema, &data);
    assert!(result.is_err());
    assert!(result.unwrap_err().to_string().contains("category"));
}

#[test]
fn test_extract_unified_keys_hashrange_missing_range() {
    let schema = create_hashrange_schema();
    let data = serde_json::json!({
        "category": "tech",
        "content": "test content"
    });

    let result = extract_unified_keys(&schema, &data);
    assert!(result.is_err());
    assert!(result.unwrap_err().to_string().contains("timestamp"));
}

#[test]
fn test_extract_unified_keys_dotted_path() {
    let schema = Schema {
        name: "TestDotted".to_string(),
        schema_type: SchemaType::Single,
        key: Some(KeyConfig { hash_field: "data.user.id".to_string(), range_field: "metadata.timestamp".to_string() }),
        fields: HashMap::new(),
        payment_config: Default::default(),
        hash: None,
    };

    let data = serde_json::json!({
        "data": {
            "user": {
                "id": "user123"
            }
        },
        "metadata": {
            "timestamp": "2023-01-01T00:00:00Z"
        },
        "content": "test content"
    });

    let result = extract_unified_keys(&schema, &data).unwrap();
    assert_eq!(result.0, Some("user123".to_string()));
    assert_eq!(result.1, Some("2023-01-01T00:00:00Z".to_string()));
}

#[test]
fn test_shape_unified_result_single() {
    let schema = create_single_schema_without_key();
    let data = serde_json::json!({
        "content": "test content",
        "author": "test author"
    });

    let result = shape_unified_result(&schema, &data, None, None).unwrap();
    
    let expected = serde_json::json!({
        "hash": "",
        "range": "",
        "fields": {
            "content": "test content",
            "author": "test author"
        }
    });
    
    assert_eq!(result, expected);
}

#[test]
fn test_shape_unified_result_range() {
    let schema = create_range_schema_legacy();
    let data = serde_json::json!({
        "created_at": "2023-01-01T00:00:00Z",
        "content": "test content"
    });

    let result = shape_unified_result(&schema, &data, None, Some("2023-01-01T00:00:00Z".to_string())).unwrap();
    
    let expected = serde_json::json!({
        "hash": "",
        "range": "2023-01-01T00:00:00Z",
        "fields": {
            "content": "test content"
        }
    });
    
    assert_eq!(result, expected);
}

#[test]
fn test_shape_unified_result_hashrange() {
    let schema = create_hashrange_schema();
    let data = serde_json::json!({
        "category": "tech",
        "timestamp": "2023-01-01T00:00:00Z",
        "content": "test content"
    });

    let result = shape_unified_result(
        &schema, 
        &data, 
        Some("tech".to_string()), 
        Some("2023-01-01T00:00:00Z".to_string())
    ).unwrap();
    
    let expected = serde_json::json!({
        "hash": "tech",
        "range": "2023-01-01T00:00:00Z",
        "fields": {
            "content": "test content"
        }
    });
    
    assert_eq!(result, expected);
}

#[test]
fn test_shape_unified_result_excludes_key_fields() {
    let schema = create_single_schema_with_key();
    let data = serde_json::json!({
        "user_id": "user123",
        "timestamp": "2023-01-01T00:00:00Z",
        "content": "test content",
        "author": "test author"
    });

    let result = shape_unified_result(
        &schema, 
        &data, 
        Some("user123".to_string()), 
        Some("2023-01-01T00:00:00Z".to_string())
    ).unwrap();
    
    // Key fields should be excluded from the fields section
    let expected = serde_json::json!({
        "hash": "user123",
        "range": "2023-01-01T00:00:00Z",
        "fields": {
            "content": "test content",
            "author": "test author"
        }
    });
    
    assert_eq!(result, expected);
}
