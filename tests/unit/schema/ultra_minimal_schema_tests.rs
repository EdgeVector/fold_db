use std::collections::HashMap;

use datafold::schema::types::json_schema::{JsonSchemaDefinition, JsonSchemaField};
use datafold::schema::types::schema::SchemaType;
use datafold::schema::types::field::FieldType;
use datafold::permissions::types::policy::TrustDistance;
use datafold::fees::types::config::TrustDistanceScaling;
use datafold::fees::payment_config::SchemaPaymentConfig;

/// Tests for ultra-minimal schema support with empty field objects
/// This validates that JsonSchemaField default values work correctly

#[test]
fn test_ultra_minimal_schema_with_empty_fields() {
    let json = r#"
    {
      "name": "BlogPost",
      "schema_type": {
        "Range": {
          "range_key": "publish_date"
        }
      },
      "fields": {
        "title": {},
        "content": {},
        "author": {}
      },
      "payment_config": {
        "base_multiplier": 1.0,
        "min_payment_threshold": 0
      }
    }
    "#;
    
    let schema: JsonSchemaDefinition = serde_json::from_str(json).unwrap();
    
    // Verify schema structure
    assert_eq!(schema.name, "BlogPost");
    assert_eq!(schema.fields.len(), 3);
    
    // Verify default values are applied to all fields
    for field_name in ["title", "content", "author"] {
        let field = schema.fields.get(field_name).unwrap();
        
        // Check default field type
        match field.field_type {
            FieldType::Single => {},
            _ => panic!("Expected FieldType::Single, got {:?}", field.field_type),
        }
        
        // Check default payment config
        assert_eq!(field.payment_config.base_multiplier, 1.0);
        match field.payment_config.trust_distance_scaling {
            TrustDistanceScaling::None => {},
            _ => panic!("Expected TrustDistanceScaling::None, got {:?}", field.payment_config.trust_distance_scaling),
        }
        assert_eq!(field.payment_config.min_payment, None);
        
        // Check default permission policy
        match field.permission_policy.read {
            TrustDistance::Distance(0) => {},
            _ => panic!("Expected TrustDistance::Distance(0), got {:?}", field.permission_policy.read),
        }
        match field.permission_policy.write {
            TrustDistance::Distance(0) => {},
            _ => panic!("Expected TrustDistance::Distance(0), got {:?}", field.permission_policy.write),
        }
        assert!(field.permission_policy.explicit_read.is_none());
        assert!(field.permission_policy.explicit_write.is_none());
        
        // Check other defaults
        assert_eq!(field.molecule_uuid, None);
        assert!(field.field_mappers.is_empty());
        assert!(field.transform.is_none());
    }
}

#[test]
fn test_mixed_format_schema() {
    let json = r#"
    {
      "name": "MixedSchema",
      "schema_type": "Single",
      "fields": {
        "title": {},
        "content": {
          "permission_policy": {
            "read_policy": {"Distance": 1},
            "write_policy": {"Distance": 2}
          },
          "payment_config": {
            "base_multiplier": 2.0,
            "trust_distance_scaling": "None",
            "min_payment": 10
          },
          "field_type": "Single"
        },
        "author": {}
      },
      "payment_config": {
        "base_multiplier": 1.0,
        "min_payment_threshold": 0
      }
    }
    "#;
    
    let schema: JsonSchemaDefinition = serde_json::from_str(json).unwrap();
    
    // Verify schema structure
    assert_eq!(schema.name, "MixedSchema");
    assert_eq!(schema.fields.len(), 3);
    
    // Check title field (empty object - should get defaults)
    let title_field = schema.fields.get("title").unwrap();
    match title_field.field_type {
        FieldType::Single => {},
        _ => panic!("Expected FieldType::Single, got {:?}", title_field.field_type),
    }
    assert_eq!(title_field.payment_config.base_multiplier, 1.0);
    match title_field.permission_policy.read {
        TrustDistance::Distance(0) => {},
        _ => panic!("Expected TrustDistance::Distance(0), got {:?}", title_field.permission_policy.read),
    }
    
    // Check content field (explicit values - should use provided values)
    let content_field = schema.fields.get("content").unwrap();
    match content_field.field_type {
        FieldType::Single => {},
        _ => panic!("Expected FieldType::Single, got {:?}", content_field.field_type),
    }
    assert_eq!(content_field.payment_config.base_multiplier, 2.0);
    assert_eq!(content_field.payment_config.min_payment, Some(10));
    match content_field.permission_policy.read {
        TrustDistance::Distance(1) => {},
        _ => panic!("Expected TrustDistance::Distance(1), got {:?}", content_field.permission_policy.read),
    }
    match content_field.permission_policy.write {
        TrustDistance::Distance(2) => {},
        _ => panic!("Expected TrustDistance::Distance(2), got {:?}", content_field.permission_policy.write),
    }
    
    // Check author field (empty object - should get defaults)
    let author_field = schema.fields.get("author").unwrap();
    match author_field.field_type {
        FieldType::Single => {},
        _ => panic!("Expected FieldType::Single, got {:?}", author_field.field_type),
    }
    assert_eq!(author_field.payment_config.base_multiplier, 1.0);
    match author_field.permission_policy.read {
        TrustDistance::Distance(0) => {},
        _ => panic!("Expected TrustDistance::Distance(0), got {:?}", author_field.permission_policy.read),
    }
}

#[test]
fn test_backward_compatibility_with_existing_schemas() {
    // Test that existing verbose schemas still work
    let json = r#"
    {
      "name": "VerboseSchema",
      "schema_type": "Single",
      "fields": {
        "title": {
          "permission_policy": {
            "read_policy": {"Distance": 0},
            "write_policy": {"Distance": 1}
          },
          "payment_config": {
            "base_multiplier": 1.0,
            "trust_distance_scaling": "None",
            "min_payment": null
          },
          "field_type": "Single",
          "field_mappers": {},
          "transform": null
        }
      },
      "payment_config": {
        "base_multiplier": 1.0,
        "min_payment_threshold": 0
      }
    }
    "#;
    
    let schema: JsonSchemaDefinition = serde_json::from_str(json).unwrap();
    
    // Verify schema structure
    assert_eq!(schema.name, "VerboseSchema");
    assert_eq!(schema.fields.len(), 1);
    
    // Check that explicit values are preserved
    let title_field = schema.fields.get("title").unwrap();
    match title_field.field_type {
        FieldType::Single => {},
        _ => panic!("Expected FieldType::Single, got {:?}", title_field.field_type),
    }
    assert_eq!(title_field.payment_config.base_multiplier, 1.0);
    match title_field.permission_policy.read {
        TrustDistance::Distance(0) => {},
        _ => panic!("Expected TrustDistance::Distance(0), got {:?}", title_field.permission_policy.read),
    }
    match title_field.permission_policy.write {
        TrustDistance::Distance(1) => {},
        _ => panic!("Expected TrustDistance::Distance(1), got {:?}", title_field.permission_policy.write),
    }
}

#[test]
fn test_blogpost_simplified_schema_file() {
    // Test with the actual BlogPost-simplified.json file
    let json = r#"
    {
      "name": "BlogPost",
      "schema_type": {
        "Range": {
          "range_key": "publish_date"
        }
      },
      "fields": {
        "title": {},
        "content": {},
        "author": {},
        "publish_date": {},
        "tags": {}
      },
      "payment_config": {
        "base_multiplier": 1.0,
        "min_payment_threshold": 0
      }
    }
    "#;
    
    let schema: JsonSchemaDefinition = serde_json::from_str(json).unwrap();
    
    // Verify schema structure
    assert_eq!(schema.name, "BlogPost");
    assert_eq!(schema.fields.len(), 5);
    
    // Verify all fields get default values
    for field_name in ["title", "content", "author", "publish_date", "tags"] {
        let field = schema.fields.get(field_name).unwrap();
        
        // Check default field type
        match field.field_type {
            FieldType::Single => {},
            _ => panic!("Expected FieldType::Single, got {:?}", field.field_type),
        }
        
        // Check default payment config
        assert_eq!(field.payment_config.base_multiplier, 1.0);
        match field.payment_config.trust_distance_scaling {
            TrustDistanceScaling::None => {},
            _ => panic!("Expected TrustDistanceScaling::None, got {:?}", field.payment_config.trust_distance_scaling),
        }
        assert_eq!(field.payment_config.min_payment, None);
        
        // Check default permission policy
        match field.permission_policy.read {
            TrustDistance::Distance(0) => {},
            _ => panic!("Expected TrustDistance::Distance(0), got {:?}", field.permission_policy.read),
        }
        match field.permission_policy.write {
            TrustDistance::Distance(0) => {},
            _ => panic!("Expected TrustDistance::Distance(0), got {:?}", field.permission_policy.write),
        }
        assert!(field.permission_policy.explicit_read.is_none());
        assert!(field.permission_policy.explicit_write.is_none());
        
        // Check other defaults
        assert_eq!(field.molecule_uuid, None);
        assert!(field.field_mappers.is_empty());
        assert!(field.transform.is_none());
    }
}

#[test]
fn test_schema_serialization_round_trip() {
    // Create a schema with empty field objects
    let mut fields = HashMap::new();
    fields.insert("title".to_string(), JsonSchemaField {
        permission_policy: datafold::schema::types::json_schema::JsonPermissionPolicy {
            read: TrustDistance::Distance(0),
            write: TrustDistance::Distance(0),
            explicit_read: None,
            explicit_write: None,
        },
        molecule_uuid: None,
        payment_config: datafold::schema::types::json_schema::JsonFieldPaymentConfig {
            base_multiplier: 1.0,
            trust_distance_scaling: TrustDistanceScaling::None,
            min_payment: None,
        },
        field_mappers: HashMap::new(),
        field_type: FieldType::Single,
        transform: None,
    });
    
    let schema = JsonSchemaDefinition {
        name: "TestSchema".to_string(),
        schema_type: SchemaType::Single,
        fields,
        payment_config: SchemaPaymentConfig::default(),
        hash: None,
    };
    
    // Serialize
    let serialized = serde_json::to_string(&schema).unwrap();
    
    // Deserialize
    let deserialized: JsonSchemaDefinition = serde_json::from_str(&serialized).unwrap();
    
    // Verify round trip
    assert_eq!(deserialized.name, "TestSchema");
    assert_eq!(deserialized.fields.len(), 1);
    
    let field = deserialized.fields.get("title").unwrap();
    match field.field_type {
        FieldType::Single => {},
        _ => panic!("Expected FieldType::Single, got {:?}", field.field_type),
    }
    assert_eq!(field.payment_config.base_multiplier, 1.0);
    match field.permission_policy.read {
        TrustDistance::Distance(0) => {},
        _ => panic!("Expected TrustDistance::Distance(0), got {:?}", field.permission_policy.read),
    }
}
