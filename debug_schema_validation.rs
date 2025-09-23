use datafold::transform::native_schema_registry::{NativeSchemaRegistry, DatabaseOperationsTrait};
use datafold::transform::native::types::{FieldValue, FieldType};
use datafold::schema::types::errors::SchemaError;
use std::sync::Arc;
use std::collections::HashMap;
use async_trait::async_trait;

#[derive(Debug)]
struct MockDatabaseOperations;

#[async_trait]
impl DatabaseOperationsTrait for MockDatabaseOperations {
    async fn store_schema(&self, _name: &str, _schema: &str) -> Result<(), SchemaError> {
        Ok(())
    }

    async fn get_schema(&self, name: &str) -> Result<Option<String>, SchemaError> {
        if name == "test_schema" {
            Ok(Some(create_test_schema().to_string()))
        } else {
            Ok(None)
        }
    }

    async fn delete_schema(&self, _name: &str) -> Result<(), SchemaError> {
        Ok(())
    }

    async fn list_schemas(&self) -> Result<Vec<String>, SchemaError> {
        Ok(vec!["test_schema".to_string()])
    }
}

fn create_test_schema() -> &'static str {
    r#"{
        "name": "test_schema",
        "schema_type": "Single",
        "payment_config": {
            "base_multiplier": 1.0,
            "min_payment_threshold": 0
        },
        "fields": {
            "id": {
                "field_type": "Single",
                "permission_policy": {
                    "read_policy": { "Distance": 0 },
                    "write_policy": { "Distance": 0 }
                },
                "payment_config": {
                    "base_multiplier": 1.0,
                    "trust_distance_scaling": "None",
                    "min_payment": null
                },
                "field_mappers": {}
            },
            "name": {
                "field_type": "Single",
                "permission_policy": {
                    "read_policy": { "Distance": 0 },
                    "write_policy": { "Distance": 0 }
                },
                "payment_config": {
                    "base_multiplier": 1.0,
                    "trust_distance_scaling": "None",
                    "min_payment": null
                },
                "field_mappers": {}
            },
            "age": {
                "field_type": "Single",
                "permission_policy": {
                    "read_policy": { "Distance": 0 },
                    "write_policy": { "Distance": 0 }
                },
                "payment_config": {
                    "base_multiplier": 1.0,
                    "trust_distance_scaling": "None",
                    "min_payment": null
                },
                "field_mappers": {}
            },
            "active": {
                "field_type": "Single",
                "permission_policy": {
                    "read_policy": { "Distance": 0 },
                    "write_policy": { "Distance": 0 }
                },
                "payment_config": {
                    "base_multiplier": 1.0,
                    "trust_distance_scaling": "None",
                    "min_payment": null
                },
                "field_mappers": {}
            },
            "score": {
                "field_type": "Single",
                "permission_policy": {
                    "read_policy": { "Distance": 0 },
                    "write_policy": { "Distance": 0 }
                },
                "payment_config": {
                    "base_multiplier": 1.0,
                    "trust_distance_scaling": "None",
                    "min_payment": null
                },
                "field_mappers": {}
            },
            "scores": {
                "field_type": "Single",
                "permission_policy": {
                    "read_policy": { "Distance": 0 },
                    "write_policy": { "Distance": 0 }
                },
                "payment_config": {
                    "base_multiplier": 1.0,
                    "trust_distance_scaling": "None",
                    "min_payment": null
                },
                "field_mappers": {}
            }
        }
    }"#
}

#[tokio::main]
async fn main() {
    println!("🔍 Debug Schema Validation Issue");

    // Create registry
    let registry = NativeSchemaRegistry::new(Arc::new(MockDatabaseOperations));

    // Load schema
    let schema_json = create_test_schema();
    let schema_name = registry.load_native_schema_from_json(schema_json).await.unwrap();
    println!("✅ Loaded schema: {}", schema_name);

    // Get schema and examine field types
    let schema = registry.get_schema("test_schema").unwrap();
    println!("\n📋 Schema field types:");
    for (field_name, field_type) in &schema.fields {
        println!("  {}: {:?}", field_name, field_type);
    }

    // Create invalid data (same as in the failing test)
    let mut invalid_data = HashMap::new();
    invalid_data.insert("id".to_string(), FieldValue::String("not_a_number".to_string())); // String instead of Integer
    invalid_data.insert("name".to_string(), FieldValue::Integer(123)); // Integer instead of String
    invalid_data.insert("age".to_string(), FieldValue::Boolean(false)); // Boolean instead of Integer

    let field_value = FieldValue::Object(invalid_data.clone());
    println!("\n❌ Invalid test data:");
    for (field_name, field_value) in &invalid_data {
        println!("  {}: {:?} ({})", field_name, field_value, field_value.field_type());
    }

    // Test validation
    let is_valid = registry.validate_data("test_schema", &field_value).await.unwrap();
    println!("\n🔍 Validation result: {}", is_valid);

    // Test individual field validation
    println!("\n🔍 Individual field validation:");
    if let FieldValue::Object(fields) = &field_value {
        for (field_name, field_value) in fields {
            if let Some(expected_type) = schema.fields.get(field_name) {
                let matches = expected_type.matches(field_value);
                println!("  {}: {:?} matches {:?} = {}", field_name, field_value, expected_type, matches);
            }
        }
    }

    // Create valid data for comparison
    let mut valid_data = HashMap::new();
    valid_data.insert("id".to_string(), FieldValue::Integer(123));
    valid_data.insert("name".to_string(), FieldValue::String("John Doe".to_string()));
    valid_data.insert("age".to_string(), FieldValue::Integer(30));

    let valid_field_value = FieldValue::Object(valid_data);
    let valid_result = registry.validate_data("test_schema", &valid_field_value).await.unwrap();
    println!("\n✅ Valid data validation result: {}", valid_result);
}
