use crate::schema::types::errors::SchemaError;
use crate::transform::native_schema_registry::{NativeSchemaRegistry, NativeSchemaRegistryError};
use crate::transform::native::types::{FieldValue, FieldType};
use std::sync::Arc;
use std::collections::HashMap;

/// Integration layer between the existing JSON-based schema system and native types
/// This enables the existing schema system to work with native FieldValue types
#[derive(Debug, Clone)]
pub struct SchemaNativeIntegration {
    native_registry: Arc<NativeSchemaRegistry>,
}

impl SchemaNativeIntegration {
    /// Create a new schema native integration
    pub fn new(native_registry: Arc<NativeSchemaRegistry>) -> Self {
        Self { native_registry }
    }

    /// Convert a JSON schema to native types and register it
    pub async fn convert_and_register_json_schema(
        &self,
        json_content: &str,
    ) -> Result<String, SchemaError> {
        // Load into native registry
        let schema_name = self.native_registry
            .load_native_schema_from_json(json_content)
            .await
            .map_err(|e| SchemaError::InvalidData(format!("Failed to load native schema: {}", e)))?;

        Ok(schema_name)
    }

    /// Validate data using native types
    pub async fn validate_data_with_native_types(
        &self,
        schema_name: &str,
        data: FieldValue,
    ) -> Result<bool, SchemaError> {
        self.native_registry
            .validate_data(schema_name, &data)
            .await
            .map_err(|e| SchemaError::InvalidData(format!("Native validation failed: {}", e)))
    }

    /// Execute transforms using native types
    pub async fn execute_transform_with_native_types(
        &self,
        schema_name: &str,
        data: FieldValue,
    ) -> Result<FieldValue, SchemaError> {
        self.native_registry
            .execute_transform(schema_name, data)
            .await
            .map_err(|e| SchemaError::InvalidData(format!("Native transform failed: {}", e)))
    }

    /// Get native field type for a schema field
    pub async fn get_native_field_type(
        &self,
        schema_name: &str,
        field_name: &str,
    ) -> Result<Option<FieldType>, SchemaError> {
        let schema = self.native_registry
            .get_schema(schema_name)
            .map_err(|e| SchemaError::InvalidData(format!("Failed to get schema: {}", e)))?;

        Ok(schema.get_field_type(field_name).cloned())
    }

    /// Check if a schema exists in the native registry
    pub fn native_schema_exists(&self, schema_name: &str) -> bool {
        self.native_registry.schema_exists(schema_name)
    }

    /// Sync native schemas with existing schema system
    pub async fn sync_native_schemas(&self) -> Result<(), SchemaError> {
        self.native_registry
            .sync_with_existing_schemas()
            .await
            .map_err(|e| SchemaError::InvalidData(format!("Sync failed: {}", e)))
    }

    /// Get the underlying native registry
    pub fn native_registry(&self) -> &NativeSchemaRegistry {
        &self.native_registry
    }

    /// Bridge method to convert existing JSON schema operations to native operations
    /// This is a compatibility layer for gradual migration
    pub async fn bridge_json_to_native_validation(
        &self,
        schema_name: &str,
        json_data: serde_json::Value,
    ) -> Result<bool, SchemaError> {
        // Convert JSON data to native FieldValue
        let native_data = FieldValue::from_json_value(json_data);

        // Validate using native types
        self.validate_data_with_native_types(schema_name, native_data).await
    }

    /// Bridge method to convert native transform results back to JSON
    /// This enables existing systems to consume native transform results
    pub async fn bridge_native_to_json_result(
        &self,
        schema_name: &str,
        native_data: FieldValue,
    ) -> Result<serde_json::Value, SchemaError> {
        // Execute transform using native types
        let result = self.execute_transform_with_native_types(schema_name, native_data).await?;

        // Convert back to JSON
        Ok(result.to_json_value())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::Arc;
    use tokio;

    fn create_test_native_registry() -> Arc<NativeSchemaRegistry> {
        let db_ops = Arc::new(MockDatabaseOperations);
        Arc::new(NativeSchemaRegistry::new(db_ops))
    }

    // Mock implementation for testing
    #[derive(Debug)]
    struct MockDatabaseOperations;

    #[async_trait::async_trait]
    impl crate::transform::native_schema_registry::DatabaseOperationsTrait for MockDatabaseOperations {
        async fn store_schema(&self, _name: &str, _schema: &str) -> Result<(), crate::schema::types::errors::SchemaError> {
            Ok(())
        }

        async fn get_schema(&self, _name: &str) -> Result<Option<String>, crate::schema::types::errors::SchemaError> {
            Ok(None)
        }

        async fn delete_schema(&self, _name: &str) -> Result<(), crate::schema::types::errors::SchemaError> {
            Ok(())
        }

        async fn list_schemas(&self) -> Result<Vec<String>, crate::schema::types::errors::SchemaError> {
            Ok(vec![])
        }
    }

    #[tokio::test]
    async fn test_create_schema_native_integration() {
        let native_registry = create_test_native_registry();
        let integration = SchemaNativeIntegration::new(native_registry);

        assert!(!integration.native_schema_exists("test_schema"));
    }

    #[tokio::test]
    async fn test_bridge_json_to_native_validation() {
        let native_registry = create_test_native_registry();
        let integration = SchemaNativeIntegration::new(native_registry);

        // First register a schema
        let json_schema = r#"{
            "name": "test_schema",
            "schema_type": "Single",
            "payment_config": {
                "base_multiplier": 1.0,
                "min_payment_threshold": 0
            },
            "fields": {
                "name": {
                    "field_type": "Single",
                    "permission_policy": {
                        "read_policy": {
                            "Distance": 0
                        },
                        "write_policy": {
                            "Distance": 0
                        }
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
                        "read_policy": {
                            "Distance": 0
                        },
                        "write_policy": {
                            "Distance": 0
                        }
                    },
                    "payment_config": {
                        "base_multiplier": 1.0,
                        "trust_distance_scaling": "None",
                        "min_payment": null
                    },
                    "field_mappers": {}
                }
            }
        }"#;

        println!("JSON Schema content: {}", json_schema);
        integration.convert_and_register_json_schema(json_schema).await.unwrap();

        // Test valid data
        let valid_json = serde_json::json!({
            "name": "John",
            "age": 30
        });

        let is_valid = integration.bridge_json_to_native_validation("test_schema", valid_json).await.unwrap();
        assert!(is_valid);

        // Test invalid data
        let invalid_json = serde_json::json!({
            "name": 123,
            "age": "thirty"
        });

        let is_valid = integration.bridge_json_to_native_validation("test_schema", invalid_json).await.unwrap();
        assert!(!is_valid);
    }

    #[tokio::test]
    async fn test_bridge_native_to_json_result() {
        let native_registry = create_test_native_registry();
        let integration = SchemaNativeIntegration::new(native_registry);

        // Register a schema
        let json_schema = r#"{
            "name": "test_schema",
            "schema_type": "Single",
            "payment_config": {
                "base_multiplier": 1.0,
                "min_payment_threshold": 0
            },
            "fields": {
                "name": {
                    "field_type": "Single",
                    "permission_policy": {
                        "read_policy": {
                            "Distance": 0
                        },
                        "write_policy": {
                            "Distance": 0
                        }
                    },
                    "payment_config": {
                        "base_multiplier": 1.0,
                        "trust_distance_scaling": "None",
                        "min_payment": null
                    },
                    "field_mappers": {}
                }
            }
        }"#;

        integration.convert_and_register_json_schema(json_schema).await.unwrap();

        // Test native data conversion
        let native_data = FieldValue::Object(
            vec![("name".to_string(), FieldValue::String("Alice".to_string()))]
            .into_iter()
            .collect()
        );

        let json_result = integration.bridge_native_to_json_result("test_schema", native_data).await.unwrap();

        assert_eq!(json_result["name"], "Alice");
    }
}