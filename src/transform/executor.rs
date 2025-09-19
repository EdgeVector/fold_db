//! Executor for declarative transforms.
//!
//! This module provides the high-level interface for applying declarative transforms to field values.
//! It handles the integration with the schema system and manages the execution context.
//!
//! **Note**: This executor only supports declarative transforms. Procedural transforms are not supported.

use crate::schema::types::{
    json_schema::DeclarativeSchemaDefinition, schema::SchemaType, SchemaError, Transform,
};
use crate::transform::aggregation::{
    aggregate_results_unified, SchemaType as AggregationSchemaType,
};
use crate::transform::coordination::execute_multi_chain_coordination_with_monitoring;
use crate::transform::iterator_stack::chain_parser::ParsedChain;
use crate::transform::iterator_stack::field_alignment::AlignmentValidationResult;
use crate::transform::shared_utilities::{
    collect_expressions_from_schema, execute_chains_with_engine, log_schema_execution_start,
    modify_expressions_with_input_prefix, parse_expressions_batch, validate_schema_basic,
};
use crate::transform::validation::{
    validate_field_alignment, validate_field_alignment_unified, validate_hashrange_schema,
};
use log::info;
use serde_json::Value as JsonValue;
use std::collections::HashMap;
use std::time::Instant;

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
    /// The result of the transform execution
    ///
    /// # Errors
    ///
    /// Returns an error if the transform is not declarative or if execution fails
    pub fn execute_transform(
        transform: &Transform,
        input_values: HashMap<String, JsonValue>,
    ) -> Result<JsonValue, SchemaError> {
        info!("🧮 TransformExecutor: Starting declarative transform computation");

        info!("📊 Input values for computation:");
        for (key, value) in &input_values {
            info!("  - {}: {}", key, value);
        }

        // Only support declarative transforms
        if !transform.is_declarative() {
            return Err(SchemaError::InvalidTransform(
                "Only declarative transforms are supported by this executor".to_string(),
            ));
        }

        Self::execute_declarative_transform(transform, input_values)
    }

    /// Executes a declarative transform.
    ///
    /// # Arguments
    ///
    /// * `transform` - The declarative transform to execute
    /// * `input_values` - The input values for the transform
    ///
    /// # Returns
    ///
    /// The result of the transform execution
    fn execute_declarative_transform(
        transform: &Transform,
        input_values: HashMap<String, JsonValue>,
    ) -> Result<JsonValue, SchemaError> {
        info!("🏗️ Executing declarative transform");

        let schema = transform.get_declarative_schema().ok_or_else(|| {
            SchemaError::InvalidTransform("Transform is not declarative".to_string())
        })?;

        Self::execute_declarative_transform_unified(schema, input_values)
    }

    /// Unified execution method that handles all schema types.
    ///
    /// # Arguments
    ///
    /// * `schema` - The declarative schema definition
    /// * `input_values` - The input values for the transform
    ///
    /// # Returns
    ///
    /// The result of the transform execution
    fn execute_declarative_transform_unified(
        schema: &DeclarativeSchemaDefinition,
        input_values: HashMap<String, JsonValue>,
    ) -> Result<JsonValue, SchemaError> {
        match &schema.schema_type {
            SchemaType::Single => Self::execute_single_pattern(schema, &input_values),
            SchemaType::Range { range_key } => {
                Self::execute_range_pattern(schema, &input_values, range_key)
            }
            SchemaType::HashRange => Self::execute_hashrange_pattern(schema, &input_values),
        }
    }

    /// Executes Single schema pattern.
    ///
    /// # Arguments
    ///
    /// * `schema` - The declarative schema definition
    /// * `input_values` - The input values for the transform
    ///
    /// # Returns
    ///
    /// The result of the Single schema execution
    fn execute_single_pattern(
        schema: &DeclarativeSchemaDefinition,
        input_values: &HashMap<String, JsonValue>,
    ) -> Result<JsonValue, SchemaError> {
        Self::execute_with_common_pattern(
            schema,
            input_values,
            "Single",
            |_, collected_expressions| {
                Ok(modify_expressions_with_input_prefix(
                    &collected_expressions,
                    true,
                ))
            },
            |schema, input_values, parsed_chains, expressions, alignment_result| {
                info!(
                    "🚀 Executing Single schema with ExecutionEngine: {}",
                    schema.name
                );

                let mut root_object = serde_json::Map::new();
                root_object.insert(
                    "input".to_string(),
                    JsonValue::Object(
                        input_values
                            .iter()
                            .map(|(k, v)| (k.clone(), v.clone()))
                            .collect(),
                    ),
                );
                let input_data = JsonValue::Object(root_object);

                let execution_result =
                    execute_chains_with_engine(parsed_chains, alignment_result, input_data)?;

                aggregate_results_unified(
                    parsed_chains,
                    &execution_result,
                    input_values,
                    expressions,
                    AggregationSchemaType::Single,
                )
            },
        )
    }

    /// Executes Range schema pattern.
    ///
    /// # Arguments
    ///
    /// * `schema` - The declarative schema definition
    /// * `input_values` - The input values for the transform
    /// * `range_key` - The range key field name from the schema type
    ///
    /// # Returns
    ///
    /// The result of the Range schema execution
    fn execute_range_pattern(
        schema: &DeclarativeSchemaDefinition,
        input_values: &HashMap<String, JsonValue>,
        _range_key: &str, // Keep for backward compatibility but use unified extraction
    ) -> Result<JsonValue, SchemaError> {
        Self::execute_with_common_pattern(
            schema,
            input_values,
            "Range",
            |schema, collected_expressions| {
                let range_expression = match &schema.schema_type {
                    SchemaType::Range { range_key } => {
                        if let Some(key_config) = &schema.key {
                            if !key_config.range_field.trim().is_empty() {
                                key_config.range_field.clone()
                            } else {
                                return Err(SchemaError::InvalidData(
                                    "Range schema with key configuration must have range_field"
                                        .to_string(),
                                ));
                            }
                        } else {
                            range_key.clone()
                        }
                    }
                    _ => {
                        return Err(SchemaError::InvalidData(
                            "Expected Range schema type".to_string(),
                        ));
                    }
                };

                let hash_expression = schema.key.as_ref().and_then(|key_config| {
                    let trimmed = key_config.hash_field.trim();
                    if trimmed.is_empty() {
                        None
                    } else {
                        Some(key_config.hash_field.clone())
                    }
                });

                info!(
                    "🔧 Executing Range coordination for schema: {} with unified keys - hash: {:?}, range: {}",
                    schema.name,
                    hash_expression.as_deref(),
                    range_expression.as_str()
                );

                let mut expressions = vec![("_range_field".to_string(), range_expression)];
                if let Some(hash) = hash_expression.clone() {
                    expressions.push(("_hash_field".to_string(), hash));
                }
                expressions.extend(collected_expressions);
                Ok(expressions)
            },
            |schema, input_values, parsed_chains, expressions, alignment_result| {
                info!(
                    "✅ Range multi-chain field alignment validation passed: {}",
                    schema.name
                );
                Self::execute_multi_chain_with_engine(
                    parsed_chains,
                    expressions,
                    input_values,
                    alignment_result,
                )
            },
        )
    }

    /// Executes HashRange schema pattern.
    ///
    /// # Arguments
    ///
    /// * `schema` - The declarative schema definition
    /// * `input_values` - The input values for the transform
    ///
    /// # Returns
    ///
    /// The result of the HashRange schema execution
    fn execute_hashrange_pattern(
        schema: &DeclarativeSchemaDefinition,
        input_values: &HashMap<String, JsonValue>,
    ) -> Result<JsonValue, SchemaError> {
        let start_time = Instant::now();
        log_schema_execution_start("HashRange", &schema.name, None);

        // Validate schema structure and field alignment
        let validation_timings = validate_hashrange_schema(schema)?;

        // Extract key configuration
        let key_config = Self::extract_hashrange_key_config(schema)?;

        // Execute multi-chain coordination
        let execution_start = Instant::now();
        let result =
            execute_multi_chain_coordination_with_monitoring(schema, input_values, key_config)?;
        let _execution_duration = execution_start.elapsed();

        // Log performance summary
        Self::log_execution_performance(
            "HashRange",
            start_time.elapsed(),
            Some(validation_timings.validation_duration),
        );

        Ok(result)
    }

    /// Common execution pattern used by Single and Range schema types.
    fn execute_with_common_pattern<F, G>(
        schema: &DeclarativeSchemaDefinition,
        input_values: &HashMap<String, JsonValue>,
        schema_type_name: &str,
        prepare_expressions: G,
        custom_logic: F,
    ) -> Result<JsonValue, SchemaError>
    where
        G: FnOnce(
            &DeclarativeSchemaDefinition,
            Vec<(String, String)>,
        ) -> Result<Vec<(String, String)>, SchemaError>,
        F: FnOnce(
            &DeclarativeSchemaDefinition,
            &HashMap<String, JsonValue>,
            &[(String, ParsedChain)],
            &[(String, String)],
            &AlignmentValidationResult,
        ) -> Result<JsonValue, SchemaError>,
    {
        log_schema_execution_start(schema_type_name, &schema.name, None);
        validate_schema_basic(schema)?;
        let collected_expressions = collect_expressions_from_schema(schema);
        let final_expressions = prepare_expressions(schema, collected_expressions)?;

        if final_expressions.is_empty() {
            info!(
                "⚠️ No expressions found for {} schema execution",
                schema_type_name
            );
            return Ok(JsonValue::Object(serde_json::Map::new()));
        }

        info!(
            "📊 Executing {} expressions for {} schema",
            final_expressions.len(),
            schema_type_name
        );
        let parsed_chains = parse_expressions_batch(&final_expressions)?;
        let chains_only: Vec<ParsedChain> = parsed_chains
            .iter()
            .map(|(_, chain)| chain.clone())
            .collect();
        let alignment_result = validate_field_alignment_unified(None, Some(&chains_only))?;
        custom_logic(
            schema,
            input_values,
            &parsed_chains,
            &final_expressions,
            &alignment_result,
        )
    }

    /// Executes multi-chain coordination with ExecutionEngine for Range schemas.
    fn execute_multi_chain_with_engine(
        parsed_chains: &[(String, ParsedChain)],
        expressions: &[(String, String)],
        input_values: &HashMap<String, JsonValue>,
        alignment_result: &AlignmentValidationResult,
    ) -> Result<JsonValue, SchemaError> {
        info!("🚀 Executing multi-chain coordination with ExecutionEngine");
        let input_data = JsonValue::Object(
            input_values
                .iter()
                .map(|(k, v)| (k.clone(), v.clone()))
                .collect(),
        );
        let execution_result =
            execute_chains_with_engine(parsed_chains, alignment_result, input_data)?;
        aggregate_results_unified(
            parsed_chains,
            &execution_result,
            input_values,
            expressions,
            AggregationSchemaType::Range,
        )
    }

    /// Extracts and validates key configuration for HashRange schema.
    fn extract_hashrange_key_config(
        schema: &DeclarativeSchemaDefinition,
    ) -> Result<&crate::schema::types::json_schema::KeyConfig, SchemaError> {
        let key_config = schema.key.as_ref().ok_or_else(|| {
            SchemaError::InvalidTransform(format!(
                "HashRange schema '{}' must have key configuration with hash_field and range_field",
                schema.name
            ))
        })?;

        info!(
            "📊 HashRange key config - hash_field: {}, range_field: {}",
            key_config.hash_field, key_config.range_field
        );

        Ok(key_config)
    }

    /// Logs execution performance summary.
    fn log_execution_performance(
        schema_type: &str,
        total_duration: std::time::Duration,
        validation_duration: Option<std::time::Duration>,
    ) {
        if let Some(validation_duration) = validation_duration {
            info!(
                "⏱️ {} execution completed in {:?} (validation: {:?})",
                schema_type, total_duration, validation_duration
            );
        } else {
            info!(
                "⏱️ {} execution completed in {:?}",
                schema_type, total_duration
            );
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
    ///
    /// # Errors
    ///
    /// Returns an error if the transform is not declarative or if validation fails
    pub fn validate_transform(transform: &Transform) -> Result<(), SchemaError> {
        // Only support declarative transforms
        if !transform.is_declarative() {
            return Err(SchemaError::InvalidTransform(
                "Only declarative transforms are supported by this validator".to_string(),
            ));
        }

        // Validate declarative transform
        let schema = transform.get_declarative_schema().ok_or_else(|| {
            SchemaError::InvalidTransform("Declarative transform must have schema".to_string())
        })?;

        // Validate schema structure
        schema.validate()?;

        // Validate field alignment
        validate_field_alignment(schema)?;

        Ok(())
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
        fields.insert(
            "title".to_string(),
            crate::schema::types::json_schema::FieldDefinition {
                field_type: Some("string".to_string()),
                atom_uuid: Some("input.title".to_string()),
            },
        );

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

        let input_values = HashMap::from([("title".to_string(), json!("Hello World"))]);

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
        fields.insert(
            "name".to_string(),
            crate::schema::types::json_schema::FieldDefinition {
                field_type: Some("string".to_string()),
                atom_uuid: Some("input.name".to_string()),
            },
        );

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
        assert!(
            result.is_ok(),
            "Declarative transform validation should succeed"
        );
    }
}
