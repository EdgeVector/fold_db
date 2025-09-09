//! Executor for declarative transforms.
//!
//! This module provides the high-level interface for applying declarative transforms to field values.
//! It handles the integration with the schema system and manages the execution context.

use crate::schema::types::{SchemaError, Transform};
use log::info;
use serde_json::Value as JsonValue;
use std::collections::HashMap;

// Import the new modules
use crate::transform::validation;
use crate::transform::hash_range_executor;
use crate::transform::range_executor;
use crate::transform::single_executor;

/// Executor for transforms.
pub struct TransformExecutor;

impl TransformExecutor {
    /// Executes a declarative transform with the given input values.
    ///
    /// # Arguments
    ///
    /// * `transform` - The declarative transform to execute
    /// * `input_values` - The input values for the transform
    ///
    /// # Returns
    ///
    /// The result of the declarative transform execution
    pub fn execute_transform(
        transform: &Transform,
        input_values: HashMap<String, JsonValue>,
    ) -> Result<JsonValue, SchemaError> {
        info!("🧮 TransformExecutor: Starting declarative transform computation");
        
        // Log individual input values
        info!("📊 Input values for computation:");
        for (key, value) in &input_values {
            info!("  - {}: {}", key, value);
        }
        
        info!("🔄 Executing declarative transform");
        Self::execute_declarative_transform(transform, input_values)
    }


    /// Executes a declarative transform with actual execution logic.
    ///
    /// # Arguments
    ///
    /// * `transform` - The declarative transform to execute
    /// * `input_values` - The input values for the transform
    ///
    /// # Returns
    ///
    /// The result of the declarative transform execution
    fn execute_declarative_transform(
        transform: &Transform,
        input_values: HashMap<String, JsonValue>,
    ) -> Result<JsonValue, SchemaError> {
        info!("🏗️ Executing declarative transform");
        
        let schema = transform.get_declarative_schema()
            .ok_or_else(|| SchemaError::InvalidTransform("Declarative transform must have schema".to_string()))?;
        
        info!("📋 Declarative schema: {}", schema.name);
        info!("🔧 Schema type: {:?}", schema.schema_type);
        info!("📊 Schema fields: {:?}", schema.fields.keys().collect::<Vec<_>>());
        
        // Route to appropriate execution based on schema type
        match &schema.schema_type {
            crate::schema::types::schema::SchemaType::Single => {
                info!("🎯 Executing Single schema type");
                single_executor::execute_single_schema(schema, input_values)
            }
            crate::schema::types::schema::SchemaType::Range { range_key } => {
                info!("🎯 Executing Range schema type with range_key: {}", range_key);
                range_executor::execute_range_schema(schema, input_values, range_key)
            }
            crate::schema::types::schema::SchemaType::HashRange => {
                info!("🎯 Executing HashRange schema type");
                hash_range_executor::execute_hashrange_schema(schema, input_values)
            }
        }
    }


    /// Validates a declarative transform for correctness.
    ///
    /// # Arguments
    ///
    /// * `transform` - The declarative transform to validate
    ///
    /// # Returns
    ///
    /// Validation result or error
    pub fn validate_transform(transform: &Transform) -> Result<(), SchemaError> {
        // Validate declarative transform
        let schema = transform.get_declarative_schema()
            .ok_or_else(|| SchemaError::InvalidTransform("Declarative transform must have schema".to_string()))?;
        
        // Validate schema structure
        schema.validate()?;
        
        // Validate field alignment
        validation::validate_field_alignment(schema)?;
        
        Ok(())
    }

    /// Executes a transform with the given input values (alias for execute_transform).
    ///
    /// # Arguments
    ///
    /// * `transform` - The transform to execute
    /// * `input_values` - The input values for the transform
    ///
    /// # Returns
    ///
    /// The result of the transform execution
    pub fn execute_transform_with_expr(
        transform: &Transform,
        input_values: HashMap<String, JsonValue>,
    ) -> Result<JsonValue, SchemaError> {
        Self::execute_transform(transform, input_values)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::schema::types::json_schema::DeclarativeSchemaDefinition;
    use crate::schema::types::schema::SchemaType;
    use serde_json::json;

    #[test]
    fn test_execute_declarative_single_schema() {
        // Create a simple Single schema for testing
        let mut fields = std::collections::HashMap::new();
        fields.insert("title".to_string(), crate::schema::types::json_schema::FieldDefinition {
            field_type: Some("string".to_string()),
            atom_uuid: Some("input.title".to_string()),
        });
        
        let schema = DeclarativeSchemaDefinition {
            name: "test_schema".to_string(),
            schema_type: SchemaType::Single,
            fields,
            key: None,
        };
        
        let transform = Transform::from_declarative_schema(
            schema,
            vec!["title".to_string()],
            "result".to_string(),
        );
        
        let input_values = HashMap::from([
            ("title".to_string(), json!("Hello World")),
        ]);
        
        let result = TransformExecutor::execute_transform(&transform, input_values);
        
        match result {
            Ok(json_result) => {
                // For Single schemas, the result should be an object with the field
                assert!(json_result.is_object());
                let obj = json_result.as_object().unwrap();
                assert_eq!(obj.get("title").unwrap(), "Hello World");
            }
            Err(err) => {
                panic!("Declarative transform execution failed: {}", err);
            }
        }
    }

    #[test]
    fn test_validate_declarative_transform() {
        // Create a simple Single schema for testing
        let mut fields = std::collections::HashMap::new();
        fields.insert("name".to_string(), crate::schema::types::json_schema::FieldDefinition {
            field_type: Some("string".to_string()),
            atom_uuid: Some("input.name".to_string()),
        });
        
        let schema = DeclarativeSchemaDefinition {
            name: "test_schema".to_string(),
            schema_type: SchemaType::Single,
            fields,
            key: None,
        };
        
        let transform = Transform::from_declarative_schema(
            schema,
            vec!["name".to_string()],
            "result".to_string(),
        );
        
        let result = TransformExecutor::validate_transform(&transform);
        assert!(result.is_ok(), "Declarative transform validation should succeed");
    }
}
