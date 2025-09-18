//! Range schema executor for transform execution.
//!
//! This module handles the execution of Range schema types, including
//! validation, coordination, and multi-chain execution.

use crate::transform::iterator_stack::chain_parser::ParsedChain;
use crate::transform::iterator_stack::field_alignment::AlignmentValidationResult;
use crate::transform::iterator_stack::execution_engine::ExecutionEngine;
use crate::transform::shared_utilities::{
    convert_iterator_stack_error,
    collect_expressions_from_schema_with_keys, parse_expressions_batch,
    validate_schema_basic, log_schema_execution_start
};
use crate::transform::aggregation::{aggregate_results_unified, SchemaType};
use crate::schema::types::SchemaError;
use log::info;
use serde_json::Value as JsonValue;
use std::collections::HashMap;

/// Executes a Range schema type declarative transform.
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
pub fn execute_range_schema(
    schema: &crate::schema::types::json_schema::DeclarativeSchemaDefinition,
    input_values: HashMap<String, JsonValue>,
    range_key: &str,
) -> Result<JsonValue, SchemaError> {
    log_schema_execution_start("Range", &schema.name, Some(range_key));
    
    // Validate schema structure
    validate_schema_basic(schema)?;
    
    // Validate field alignment for declarative transforms (reusing existing validation)
    crate::transform::validation::validate_field_alignment(schema)?;
    
    // Execute range-based coordination (similar to HashRange but simpler)
    execute_range_coordination(schema, &input_values, range_key)
}

/// Executes range-based coordination for Range schemas.
///
/// # Arguments
///
/// * `schema` - The declarative schema definition
/// * `input_values` - The input values for the transform
/// * `range_key` - The range key field name from the schema type
///
/// # Returns
///
/// The result of the range coordination
fn execute_range_coordination(
    schema: &crate::schema::types::json_schema::DeclarativeSchemaDefinition,
    input_values: &HashMap<String, JsonValue>,
    range_key: &str,
) -> Result<JsonValue, SchemaError> {
    info!("🔧 Executing Range coordination for schema: {} with range_key: {}", schema.name, range_key);
    
    // Collect all expressions for Range coordination using unified function
    let key_expressions = vec![("_range_field".to_string(), range_key.to_string())];
    let all_expressions = collect_expressions_from_schema_with_keys(schema, &key_expressions);
    
    info!("📊 Coordinating {} expressions for Range execution", all_expressions.len());
    
    // Use the same multi-chain coordination logic as HashRange
    execute_range_multi_chain_coordination(all_expressions, input_values, schema)
}

/// Executes multi-chain coordination for Range schemas (reuses HashRange logic).
///
/// # Arguments
///
/// * `all_expressions` - All expressions to coordinate
/// * `input_values` - The input values for execution
/// * `schema` - The schema for context
///
/// # Returns
///
/// The coordinated execution result
fn execute_range_multi_chain_coordination(
    all_expressions: Vec<(String, String)>,
    input_values: &HashMap<String, JsonValue>,
    _schema: &crate::schema::types::json_schema::DeclarativeSchemaDefinition,
) -> Result<JsonValue, SchemaError> {
    info!("🚀 Executing Range multi-chain coordination");
    
    // Parse all expressions using unified batch parsing
    let parsed_chains = parse_expressions_batch(&all_expressions)?;
    info!("✅ Successfully parsed {} expressions", parsed_chains.len());
    
    // Validate field alignment using the unified validation function
    let chains_only: Vec<ParsedChain> = parsed_chains.iter().map(|(_, chain)| chain.clone()).collect();
    let alignment_result = crate::transform::validation::validate_field_alignment_unified(
        None, 
        Some(&chains_only)
    )?;
    
    info!("✅ Range multi-chain field alignment validation passed");
    
    // Execute using the same multi-chain engine as HashRange
    execute_multi_chain_with_engine(&parsed_chains, input_values, &alignment_result)
}

/// Executes multi-chain coordination with ExecutionEngine.
///
/// # Arguments
///
/// * `parsed_chains` - The parsed chains with their field names
/// * `input_values` - The input values for execution
/// * `alignment_result` - The alignment validation result
///
/// # Returns
///
/// The coordinated execution result
fn execute_multi_chain_with_engine(
    parsed_chains: &[(String, ParsedChain)],
    input_values: &HashMap<String, JsonValue>,
    alignment_result: &AlignmentValidationResult,
) -> Result<JsonValue, SchemaError> {
    info!("🚀 Executing multi-chain coordination with ExecutionEngine");
    
    // Convert input_values HashMap to JSON object for ExecutionEngine
    let input_data = JsonValue::Object(input_values.iter().map(|(k, v)| (k.clone(), v.clone())).collect());
    
    // Create and execute with ExecutionEngine for all chains
    let mut execution_engine = ExecutionEngine::new();
    let chains_only: Vec<ParsedChain> = parsed_chains.iter().map(|(_, chain)| chain.clone()).collect();
    
    let execution_result = execution_engine.execute_fields(
        &chains_only,
        alignment_result,
        input_data,
    ).map_err(convert_iterator_stack_error)?;
    
    info!("📈 Multi-chain ExecutionEngine produced {} index entries", execution_result.index_entries.len());
    
    // Reconstruct expressions from parsed chains for unified aggregation
    let all_expressions: Vec<(String, String)> = parsed_chains.iter()
        .map(|(field_name, parsed_chain)| (field_name.clone(), parsed_chain.expression.clone()))
        .collect();
    
    // Aggregate results from multi-chain execution using unified aggregation
    aggregate_results_unified(parsed_chains, &execution_result, input_values, &all_expressions, SchemaType::Range)
}

