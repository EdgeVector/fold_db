//! Executor for declarative transforms.
//!
//! This module provides the high-level interface for applying declarative transforms to field values.
//! It handles the integration with the schema system and manages the execution context.
//!
//! **Note**: This executor only supports declarative transforms. Procedural transforms are not supported.

use crate::schema::types::{SchemaError, Transform, json_schema::DeclarativeSchemaDefinition, schema::SchemaType};
use crate::transform::validation::{validate_hashrange_schema, validate_field_alignment_unified, validate_field_alignment};
use crate::transform::shared_utilities::{
    convert_iterator_stack_error,
    collect_expressions_from_schema, collect_expressions_from_schema_with_keys,
    parse_expressions_batch, modify_expressions_with_input_prefix,
    validate_schema_basic, log_schema_execution_start
};
use crate::transform::coordination::execute_multi_chain_coordination_with_monitoring;
use crate::transform::aggregation::{aggregate_results_unified, SchemaType as AggregationSchemaType};
use crate::transform::iterator_stack::chain_parser::ParsedChain;
use crate::transform::iterator_stack::field_alignment::AlignmentValidationResult;
use crate::transform::iterator_stack::execution_engine::{ExecutionEngine, ExecutionResult};
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
                "Only declarative transforms are supported by this executor".to_string()
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
        
        let schema = transform.get_declarative_schema()
            .ok_or_else(|| SchemaError::InvalidTransform("Transform is not declarative".to_string()))?;
        
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
            SchemaType::Range { range_key } => Self::execute_range_pattern(schema, &input_values, range_key),
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
            |schema, input_values, _parsed_chains, _alignment_result| {
                info!("🚀 Executing Single schema with ExecutionEngine: {}", schema.name);
                
                // Collect all expressions for execution using unified function
                let all_expressions = collect_expressions_from_schema(schema);
                
                if all_expressions.is_empty() {
                    info!("⚠️ No expressions found for Single schema execution");
                    return Ok(JsonValue::Object(serde_json::Map::new()));
                }
                
                info!("📊 Executing {} expressions for Single schema", all_expressions.len());
                
                // Modify expressions to add "input." prefix if needed using unified function
                let modified_expressions = modify_expressions_with_input_prefix(&all_expressions, true);
                
                // Parse all modified expressions using unified batch parsing
                let modified_chains = parse_expressions_batch(&modified_expressions)?;
                
                // Validate field alignment using the unified validation function
                let modified_chains_only: Vec<ParsedChain> = 
                    modified_chains.iter().map(|(_, chain)| chain.clone()).collect();
                let alignment_result = validate_field_alignment_unified(
                    None, 
                    Some(&modified_chains_only)
                )?;
                
                // Structure input data with "input" field containing the actual input values
                let mut root_object = serde_json::Map::new();
                root_object.insert("input".to_string(), JsonValue::Object(input_values.iter().map(|(k, v)| (k.clone(), v.clone())).collect()));
                let input_data = JsonValue::Object(root_object);
                
                // Execute with ExecutionEngine
                let execution_result = Self::setup_execution_engine(&modified_chains, input_data, &alignment_result)?;
                
                // Aggregate results into final output format using unified aggregation
                aggregate_results_unified(&modified_chains, &execution_result, input_values, &modified_expressions, AggregationSchemaType::Single)
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
        range_key: &str,
    ) -> Result<JsonValue, SchemaError> {
        Self::execute_with_common_pattern(
            schema,
            input_values,
            "Range",
            |schema, input_values, _parsed_chains, _alignment_result| {
                info!("🔧 Executing Range coordination for schema: {} with range_key: {}", schema.name, range_key);
                
                // Collect all expressions for Range coordination using unified function
                let key_expressions = vec![("_range_field".to_string(), range_key.to_string())];
                let all_expressions = collect_expressions_from_schema_with_keys(schema, &key_expressions);
                
                info!("📊 Coordinating {} expressions for Range execution", all_expressions.len());
                
                // Parse all expressions using unified batch parsing
                let parsed_chains = parse_expressions_batch(&all_expressions)?;
                info!("✅ Successfully parsed {} expressions", parsed_chains.len());
                
                // Validate field alignment using the unified validation function
                let chains_only: Vec<ParsedChain> = parsed_chains.iter().map(|(_, chain)| chain.clone()).collect();
                let alignment_result = validate_field_alignment_unified(
                    None, 
                    Some(&chains_only)
                )?;
                
                info!("✅ Range multi-chain field alignment validation passed");
                
                // Execute using the same multi-chain engine as HashRange
                Self::execute_multi_chain_with_engine(&parsed_chains, input_values, &alignment_result)
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
        let result = execute_multi_chain_coordination_with_monitoring(schema, input_values, key_config)?;
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
    fn execute_with_common_pattern<F>(
        schema: &DeclarativeSchemaDefinition,
        input_values: &HashMap<String, JsonValue>,
        schema_type_name: &str,
        custom_logic: F,
    ) -> Result<JsonValue, SchemaError>
    where
        F: FnOnce(&DeclarativeSchemaDefinition, &HashMap<String, JsonValue>, Vec<(String, ParsedChain)>, AlignmentValidationResult) -> Result<JsonValue, SchemaError>,
    {
        log_schema_execution_start(schema_type_name, &schema.name, None);
        validate_schema_basic(schema)?;
        let all_expressions = collect_expressions_from_schema(schema);
        
        if all_expressions.is_empty() {
            info!("⚠️ No expressions found for {} schema execution", schema_type_name);
            return Ok(JsonValue::Object(serde_json::Map::new()));
        }
        
        info!("📊 Executing {} expressions for {} schema", all_expressions.len(), schema_type_name);
        let parsed_chains = parse_expressions_batch(&all_expressions)?;
        let chains_only: Vec<ParsedChain> = parsed_chains.iter().map(|(_, chain)| chain.clone()).collect();
        let alignment_result = validate_field_alignment_unified(None, Some(&chains_only))?;
        custom_logic(schema, input_values, parsed_chains, alignment_result)
    }

    /// Sets up and executes with ExecutionEngine.
    fn setup_execution_engine(
        parsed_chains: &[(String, ParsedChain)],
        input_data: JsonValue,
        alignment_result: &AlignmentValidationResult,
    ) -> Result<ExecutionResult, SchemaError> {
        let mut execution_engine = ExecutionEngine::new();
        let chains_only: Vec<ParsedChain> = parsed_chains.iter().map(|(_, chain)| chain.clone()).collect();
        let execution_result = execution_engine.execute_fields(
            &chains_only,
            alignment_result,
            input_data,
        ).map_err(convert_iterator_stack_error)?;
        info!("📈 ExecutionEngine produced {} index entries", execution_result.index_entries.len());
        Ok(execution_result)
    }

    /// Executes multi-chain coordination with ExecutionEngine for Range schemas.
    fn execute_multi_chain_with_engine(
        parsed_chains: &[(String, ParsedChain)],
        input_values: &HashMap<String, JsonValue>,
        alignment_result: &AlignmentValidationResult,
    ) -> Result<JsonValue, SchemaError> {
        info!("🚀 Executing multi-chain coordination with ExecutionEngine");
        let input_data = JsonValue::Object(input_values.iter().map(|(k, v)| (k.clone(), v.clone())).collect());
        let execution_result = Self::setup_execution_engine(parsed_chains, input_data, alignment_result)?;
        let all_expressions: Vec<(String, String)> = parsed_chains.iter()
            .map(|(field_name, parsed_chain)| (field_name.clone(), parsed_chain.expression.clone()))
            .collect();
        aggregate_results_unified(parsed_chains, &execution_result, input_values, &all_expressions, AggregationSchemaType::Range)
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
        
        info!("📊 HashRange key config - hash_field: {}, range_field: {}", 
              key_config.hash_field, key_config.range_field);
        
        Ok(key_config)
    }

    /// Logs execution performance summary.
    fn log_execution_performance(
        schema_type: &str,
        total_duration: std::time::Duration,
        validation_duration: Option<std::time::Duration>,
    ) {
        if let Some(validation_duration) = validation_duration {
            info!("⏱️ {} execution completed in {:?} (validation: {:?})", 
                  schema_type, total_duration, validation_duration);
        } else {
            info!("⏱️ {} execution completed in {:?}", schema_type, total_duration);
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
                "Only declarative transforms are supported by this validator".to_string()
            ));
        }
        
        // Validate declarative transform
        let schema = transform.get_declarative_schema()
            .ok_or_else(|| SchemaError::InvalidTransform("Declarative transform must have schema".to_string()))?;
        
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