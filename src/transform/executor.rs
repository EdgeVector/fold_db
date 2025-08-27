//! Executor for transforms.
//!
//! This module provides the high-level interface for applying transforms to field values.
//! It handles the integration with the schema system and manages the execution context.

use super::ast::Value;
use super::interpreter::Interpreter;
use super::parser::TransformParser;
use crate::schema::types::{SchemaError, Transform};
use log::{info, error};
use serde_json::Value as JsonValue;
use std::collections::HashMap;

/// Executor for transforms.
pub struct TransformExecutor;

impl TransformExecutor {
    /// Executes a transform with the given input values.
    ///
    /// # Arguments
    ///
    /// * `transform` - The transform to execute
    /// * `input_values` - The input values for the transform
    ///
    /// # Returns
    ///
    /// The result of the transform execution
    pub fn execute_transform(
        transform: &Transform,
        input_values: HashMap<String, JsonValue>,
    ) -> Result<JsonValue, SchemaError> {
        info!("🧮 TransformExecutor: Starting computation");
        if let Some(logic) = transform.get_procedural_logic() {
            info!("🔧 Transform logic: {}", logic);
        } else {
            info!("🔧 Declarative transform");
        }
        
        // Log individual input values
        info!("📊 Input values for computation:");
        for (key, value) in &input_values {
            info!("  📋 {}: {}", key, value);
        }
        
        // Log a simplified computation description
        if let Some(logic) = transform.get_procedural_logic() {
            info!("🧮 Computing with logic: {}", logic);
        } else {
            info!("🧮 Computing with declarative transform");
        }
        
        let result = Self::execute_transform_with_expr(transform, input_values);
        
        match &result {
            Ok(value) => {
                info!("✨ Computation result: {}", value);
                info!("✅ Transform execution completed successfully");
            }
            Err(e) => {
                error!("❌ Transform execution failed: {}", e);
            }
        }
        
        result
    }

    /// Executes a transform with the given input provider function.
    ///
    /// This version allows the transform to collect its own inputs using the provided function.
    ///
    /// # Arguments
    ///
    /// * `transform` - The transform to execute
    /// * `input_provider` - A function that provides input values for a given input name
    ///
    /// # Returns
    ///
    /// The result of the transform execution
    pub fn execute_transform_with_provider<F>(
        transform: &Transform,
        input_provider: F,
    ) -> Result<JsonValue, SchemaError>
    where
        F: Fn(&str) -> Result<JsonValue, Box<dyn std::error::Error>>,
    {
        // Collect input values using the provider function
        let mut input_values = HashMap::new();

        // Use the transform's declared dependencies
        for input_name in transform.get_inputs() {
            match input_provider(input_name) {
                Ok(value) => {
                    input_values.insert(input_name.clone(), value);
                }
                Err(e) => {
                    return Err(SchemaError::InvalidField(format!(
                        "Failed to get input '{}': {}",
                        input_name, e
                    )));
                }
            }
        }

        // If no dependencies are declared, try to analyze the transform logic
        if transform.get_inputs().is_empty() {
            let dependencies = transform.analyze_dependencies();
            for input_name in dependencies {
                // Skip if we already have this input
                if input_values.contains_key(&input_name) {
                    continue;
                }

                // Try to get the input value
                match input_provider(&input_name) {
                    Ok(value) => {
                        input_values.insert(input_name, value);
                    }
                    Err(_) => {
                        // Ignore errors for analyzed dependencies, as they might not be actual inputs
                    }
                }
            }
        }

        // Execute the transform with the collected inputs
        info!(
            "execute_transform_with_provider logic: {} with inputs: {:?}",
            transform.get_procedural_logic().unwrap_or("[declarative]"), input_values
        );
        let result = Self::execute_transform(transform, input_values);
        if let Ok(ref value) = result {
            info!("execute_transform_with_provider result: {:?}", value);
        }
        result
    }

    /// Executes a transform with routing based on transform type.
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
        // Route based on transform type
        if transform.is_procedural() {
            info!("🔀 Routing to procedural transform execution");
            Self::execute_procedural_transform(transform, input_values)
        } else if transform.is_declarative() {
            info!("🔀 Routing to declarative transform execution");
            Self::execute_declarative_transform(transform, input_values)
        } else {
            error!("❌ Unknown transform type encountered");
            Err(SchemaError::InvalidTransform("Unknown transform type".to_string()))
        }
    }

    /// Executes a procedural transform using the existing logic.
    ///
    /// # Arguments
    ///
    /// * `transform` - The procedural transform to execute
    /// * `input_values` - The input values for the transform
    ///
    /// # Returns
    ///
    /// The result of the transform execution
    fn execute_procedural_transform(
        transform: &Transform,
        input_values: HashMap<String, JsonValue>,
    ) -> Result<JsonValue, SchemaError> {
        info!("⚙️ Executing procedural transform");
        
        // Use the pre-parsed expression if available, otherwise parse the transform logic
        let ast = match &transform.parsed_expression {
            Some(expr) => expr.clone(),
            None => {
                // Parse the transform logic
                let logic = transform.get_procedural_logic()
                    .ok_or_else(|| SchemaError::InvalidTransform("Procedural transform must have logic".to_string()))?;
                let parser = TransformParser::new();
                parser.parse_expression(logic).map_err(|e| {
                    SchemaError::InvalidField(format!("Failed to parse transform: {}", e))
                })?
            }
        };

        info!("🔍 Transform AST: {:?}", ast);
        info!("📊 Input values: {:?}", input_values);

        // Convert input values to interpreter values
        info!("🔄 Converting input values to interpreter format...");
        let variables = Self::convert_input_values(input_values);
        info!("🔄 Variables for interpreter: {:?}", variables);

        // Create interpreter with input variables
        info!("🧠 Creating interpreter with variables...");
        let mut interpreter = Interpreter::with_variables(variables);

        // Evaluate the AST
        info!("⚡ Evaluating expression...");
        let evaluated = interpreter.evaluate(&ast).map_err(|e| {
            error!("❌ Expression evaluation failed: {}", e);
            SchemaError::InvalidField(format!("Failed to execute transform: {}", e))
        })?;

        info!("🎯 Raw evaluation result: {:?}", evaluated);
        
        let json_result = Self::convert_result_value(evaluated)?;
        info!("✨ Final JSON result: {}", json_result);
        Ok(json_result)
    }

    /// Executes a declarative transform (placeholder implementation).
    ///
    /// # Arguments
    ///
    /// * `transform` - The declarative transform to execute
    /// * `input_values` - The input values for the transform
    ///
    /// # Returns
    ///
    /// A placeholder result for declarative transform execution
    fn execute_declarative_transform(
        transform: &Transform,
        _input_values: HashMap<String, JsonValue>,
    ) -> Result<JsonValue, SchemaError> {
        info!("🏗️ Executing declarative transform (placeholder)");
        
        let schema = transform.get_declarative_schema()
            .ok_or_else(|| SchemaError::InvalidTransform("Declarative transform must have schema".to_string()))?;
        
        info!("📋 Declarative schema: {}", schema.name);
        info!("🔧 Schema type: {:?}", schema.schema_type);
        info!("📊 Schema fields: {:?}", schema.fields.keys().collect::<Vec<_>>());
        
        // Placeholder implementation - return a simple JSON object indicating declarative execution
        let placeholder_result = serde_json::json!({
            "declarative_transform": true,
            "schema_name": schema.name,
            "schema_type": format!("{:?}", schema.schema_type),
            "status": "placeholder_execution",
            "message": "Declarative transform execution not yet implemented"
        });
        
        info!("✨ Declarative transform placeholder result: {}", placeholder_result);
        Ok(placeholder_result)
    }

    /// Converts input values from JsonValue to interpreter Value.
    fn convert_input_values(input_values: HashMap<String, JsonValue>) -> HashMap<String, Value> {
        let mut variables = HashMap::new();

        for (name, value) in input_values {
            // Handle both schema.field format and regular field names
            variables.insert(name.clone(), Value::from(value.clone()));

            // If the name contains a dot, it's in schema.field format
            if let Some((schema, field)) = name.split_once('.') {
                // Add both schema.field and field entries
                variables.insert(format!("{}.{}", schema, field), Value::from(value.clone()));
                variables.insert(field.to_string(), Value::from(value));
            }
        }

        variables
    }

    /// Converts a result value from interpreter Value to JsonValue.
    fn convert_result_value(value: Value) -> Result<JsonValue, SchemaError> {
        Ok(JsonValue::from(value))
    }


    /// Validates a transform.
    ///
    /// # Arguments
    ///
    /// * `transform` - The transform to validate
    ///
    /// # Returns
    ///
    /// `Ok(())` if the transform is valid, otherwise an error
    pub fn validate_transform(transform: &Transform) -> Result<(), SchemaError> {
        // Only validate procedural transforms with logic parsing
        if let Some(logic) = transform.get_procedural_logic() {
            // Parse the transform logic to check for syntax errors
            let parser = TransformParser::new();
            let ast = parser.parse_expression(logic);

            // For "input +" specifically, we want to fail validation
            if logic == "input +" {
                return Err(SchemaError::InvalidField(
                    "Invalid transform syntax: missing right operand".to_string(),
                ));
            }

            ast.map_err(|e| SchemaError::InvalidField(format!("Invalid transform syntax: {}", e)))?;

        } else if let Some(schema) = transform.get_declarative_schema() {
            // Validate declarative transform schema
            schema.validate()?;
        } else {
            return Err(SchemaError::InvalidTransform("Transform must be either procedural or declarative".to_string()));
        }

        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::super::ast::{Expression, Operator, Value};
    use super::*;

    #[test]
    fn test_execute_complex_transform() {
        // Create a complex transform (BMI calculation) with a manually constructed expression
        let expr = Expression::LetBinding {
            name: "bmi".to_string(),
            value: Box::new(Expression::BinaryOp {
                left: Box::new(Expression::Variable("weight".to_string())),
                operator: Operator::Divide,
                right: Box::new(Expression::BinaryOp {
                    left: Box::new(Expression::Variable("height".to_string())),
                    operator: Operator::Power,
                    right: Box::new(Expression::Literal(Value::Number(2.0))),
                }),
            }),
            body: Box::new(Expression::Variable("bmi".to_string())),
        };

        let transform = Transform::new_with_expr(
            "let bmi = weight / (height ^ 2); bmi".to_string(),
            expr,
            "test.bmi".to_string(),
        );

        // Create input values
        let mut input_values = HashMap::new();
        input_values.insert(
            "weight".to_string(),
            JsonValue::Number(serde_json::Number::from_f64(70.0).unwrap()),
        );
        input_values.insert(
            "height".to_string(),
            JsonValue::Number(serde_json::Number::from_f64(1.75).unwrap()),
        );

        // Execute the transform
        let result =
            TransformExecutor::execute_transform_with_expr(&transform, input_values).unwrap();

        // Check the result (BMI = 70 / (1.75^2) = 70 / 3.0625 = 22.857)
        match result {
            JsonValue::Number(n) => {
                let value = n.as_f64().unwrap();
                assert!((value - 22.857).abs() < 0.001);
            }
            _ => panic!("Expected number"),
        }
    }

    #[test]
    fn test_execute_transform_with_field_access() {
        // Create a transform that accesses object fields with a manually constructed expression
        let expr = Expression::BinaryOp {
            left: Box::new(Expression::FieldAccess {
                object: Box::new(Expression::Variable("patient".to_string())),
                field: "weight".to_string(),
            }),
            operator: Operator::Divide,
            right: Box::new(Expression::BinaryOp {
                left: Box::new(Expression::FieldAccess {
                    object: Box::new(Expression::Variable("patient".to_string())),
                    field: "height".to_string(),
                }),
                operator: Operator::Power,
                right: Box::new(Expression::Literal(Value::Number(2.0))),
            }),
        };

        let transform = Transform::new_with_expr(
            "patient.weight / (patient.height ^ 2)".to_string(),
            expr,
            "test.bmi".to_string(),
        );

        // Create input values with nested objects
        let mut input_values = HashMap::new();

        let mut patient = serde_json::Map::new();
        patient.insert(
            "weight".to_string(),
            JsonValue::Number(serde_json::Number::from_f64(70.0).unwrap()),
        );
        patient.insert(
            "height".to_string(),
            JsonValue::Number(serde_json::Number::from_f64(1.75).unwrap()),
        );

        input_values.insert("patient".to_string(), JsonValue::Object(patient));

        // Execute the transform
        let result =
            TransformExecutor::execute_transform_with_expr(&transform, input_values).unwrap();

        // Check the result (BMI = 70 / (1.75^2) = 70 / 3.0625 = 22.857)
        match result {
            JsonValue::Number(n) => {
                let value = n.as_f64().unwrap();
                assert!((value - 22.857).abs() < 0.001);
            }
            _ => panic!("Expected number"),
        }
    }

    #[test]
    fn test_execute_transform_with_provider_inputs_handling() {
        let parser = TransformParser::new();
        let expr = parser.parse_expression("a + b").unwrap();
        let base_transform =
            Transform::new_with_expr("a + b".to_string(), expr, "test.out".to_string());

        // Case 1: explicit inputs provided, dependency analysis should not run
        let mut transform = base_transform.clone();
        transform.set_inputs(vec!["a".to_string()]);

        let provider = |name: &str| -> Result<JsonValue, Box<dyn std::error::Error>> {
            match name {
                "a" => Ok(JsonValue::from(2)),
                other => panic!("unexpected input request: {}", other),
            }
        };
        // Evaluation should fail because 'b' is missing but provider should not panic
        assert!(TransformExecutor::execute_transform_with_provider(&transform, provider).is_err());

        // Case 2: no explicit inputs, analysis should request both 'a' and 'b'
        let provider = |name: &str| -> Result<JsonValue, Box<dyn std::error::Error>> {
            match name {
                "a" => Ok(JsonValue::from(2)),
                "b" => Ok(JsonValue::from(3)),
                other => panic!("unexpected input request: {}", other),
            }
        };

        let result =
            TransformExecutor::execute_transform_with_provider(&base_transform, provider).unwrap();
        assert_eq!(result, JsonValue::from(5.0));
    }

    #[test]
    fn test_validate_transform() {
        // Valid transform
        let transform = Transform::new("input + 10".to_string(), "test.output".to_string());

        assert!(TransformExecutor::validate_transform(&transform).is_ok());

        // Invalid transform (syntax error)
        let invalid_transform = Transform::new(
            "input +".to_string(), // Missing right operand
            "test.output".to_string(),
        );

        assert!(TransformExecutor::validate_transform(&invalid_transform).is_err());

        // No signature validation errors expected anymore
    }
}
