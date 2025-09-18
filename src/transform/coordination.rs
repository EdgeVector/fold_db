//! Multi-chain coordination for transform execution.
//!
//! This module handles the complex coordination logic for executing multiple
//! transform chains together, particularly for HashRange and Range schemas.

use crate::transform::iterator_stack::chain_parser::ParsedChain;
use crate::transform::iterator_stack::field_alignment::AlignmentValidationResult;
use crate::transform::iterator_stack::execution_engine::{ExecutionEngine, ExecutionResult};
use crate::transform::shared_utilities::{
    convert_iterator_stack_error, 
    collect_expressions_from_schema_with_keys, parse_expressions_batch
};
use crate::transform::validation::validate_field_alignment_unified;
use crate::transform::aggregation::{aggregate_results_unified, SchemaType};
use crate::schema::types::SchemaError;
use log::info;
use serde_json::Value as JsonValue;
use std::collections::HashMap;
use std::time::Instant;

/// Executes multi-chain coordination with enhanced monitoring and error recovery.
///
/// # Arguments
///
/// * `schema` - The declarative schema definition
/// * `input_values` - The input values for the transform
/// * `key_config` - The key configuration with hash_field and range_field
///
/// # Returns
///
/// The result of the multi-chain execution with enhanced monitoring
pub fn execute_multi_chain_coordination_with_monitoring(
    schema: &crate::schema::types::json_schema::DeclarativeSchemaDefinition,
    input_values: &HashMap<String, JsonValue>,
    key_config: &crate::schema::types::json_schema::KeyConfig,
) -> Result<JsonValue, SchemaError> {
    let start_time = Instant::now();
    info!("🔗 Starting enhanced multi-chain coordination for HashRange schema");
    
    let key_expressions = vec![
        ("_hash_field".to_string(), key_config.hash_field.clone()),
        ("_range_field".to_string(), key_config.range_field.clone()),
    ];
    let expressions = collect_expressions_from_schema_with_keys(schema, &key_expressions);
    let parsed_chains = parse_expressions_with_monitoring(&expressions)?;
    let chains_only: Vec<ParsedChain> = parsed_chains.iter().map(|(_, chain)| chain.clone()).collect();
    let alignment_result = validate_field_alignment_unified(None, Some(&chains_only))?;
    let result = execute_coordination_with_engine(&parsed_chains, input_values, &alignment_result)?;
    
    let total_duration = start_time.elapsed();
    info!("⏱️ Enhanced multi-chain coordination completed in {:?}", total_duration);
    
    Ok(result)
}


/// Parses expressions using ChainParser with retry mechanism and monitoring.
///
/// # Arguments
///
/// * `expressions` - Vector of (field_name, expression) pairs
///
/// # Returns
///
/// Vector of (field_name, ParsedChain) pairs
fn parse_expressions_with_monitoring(
    expressions: &[(String, String)],
) -> Result<Vec<(String, ParsedChain)>, SchemaError> {
    let parsing_start = Instant::now();
    
    info!("🔗 Starting batch parsing of {} expressions", expressions.len());
    
    // Use unified batch parsing function
    let parsed_chains = parse_expressions_batch(expressions)?;
    
    let parsing_duration = parsing_start.elapsed();
    info!("⏱️ Enhanced parsing took: {:?}", parsing_duration);
    
    // Log parsing statistics
    let success_rate = (parsed_chains.len() as f64 / expressions.len() as f64) * 100.0;
    info!("📊 Parsing completed: {}/{} expressions parsed successfully ({:.1}% success rate)", 
          parsed_chains.len(), expressions.len(), success_rate);
    
    Ok(parsed_chains)
}


/// Executes coordination with ExecutionEngine.
///
/// # Arguments
///
/// * `parsed_chains` - Vector of (field_name, ParsedChain) pairs
/// * `input_values` - The input values for execution
/// * `alignment_result` - The alignment validation result
///
/// # Returns
///
/// Execution result
fn execute_coordination_with_engine(
    parsed_chains: &[(String, ParsedChain)],
    input_values: &HashMap<String, JsonValue>,
    alignment_result: &AlignmentValidationResult,
) -> Result<JsonValue, SchemaError> {
    let execution_start = Instant::now();
    let result = execute_multi_chain_with_engine_enhanced(parsed_chains, input_values, alignment_result)?;
    let execution_duration = execution_start.elapsed();
    info!("⏱️ Enhanced execution took: {:?}", execution_duration);
    Ok(result)
}

/// Executes multi-chain coordination with enhanced ExecutionEngine.
///
/// # Arguments
///
/// * `parsed_chains` - The parsed chains with their field names
/// * `input_values` - The input values for execution
/// * `alignment_result` - The alignment validation result
///
/// # Returns
///
/// The enhanced execution result
fn execute_multi_chain_with_engine_enhanced(
    parsed_chains: &[(String, ParsedChain)],
    input_values: &HashMap<String, JsonValue>,
    alignment_result: &AlignmentValidationResult,
) -> Result<JsonValue, SchemaError> {
    let start_time = Instant::now();
    info!("🚀 Executing enhanced multi-chain coordination with ExecutionEngine");
    
    let input_data = convert_input_values_to_json(input_values)?;
    let execution_result = execute_with_engine(parsed_chains, &input_data, alignment_result)?;
    log_execution_statistics(&execution_result);
    // Reconstruct expressions from parsed chains for unified aggregation
    let all_expressions: Vec<(String, String)> = parsed_chains.iter()
        .map(|(field_name, parsed_chain)| (field_name.clone(), parsed_chain.expression.clone()))
        .collect();
    let result = aggregate_results_unified(parsed_chains, &execution_result, input_values, &all_expressions, SchemaType::HashRange)?;
    
    let total_duration = start_time.elapsed();
    info!("⏱️ Enhanced multi-chain execution completed in {:?}", total_duration);
    
    Ok(result)
}

/// Converts input values HashMap to JSON object for ExecutionEngine.
///
/// # Arguments
///
/// * `input_values` - The input values HashMap
///
/// # Returns
///
/// JSON object representation of input values
fn convert_input_values_to_json(input_values: &HashMap<String, JsonValue>) -> Result<JsonValue, SchemaError> {
    let conversion_start = Instant::now();
    let input_data = JsonValue::Object(input_values.iter().map(|(k, v)| (k.clone(), v.clone())).collect());
    let conversion_duration = conversion_start.elapsed();
    info!("⏱️ Input data conversion took: {:?}", conversion_duration);
    Ok(input_data)
}

/// Executes chains with ExecutionEngine.
///
/// # Arguments
///
/// * `parsed_chains` - The parsed chains with their field names
/// * `input_data` - The input data as JSON
/// * `alignment_result` - The alignment validation result
///
/// # Returns
///
/// Execution result from the engine
fn execute_with_engine(
    parsed_chains: &[(String, ParsedChain)],
    input_data: &JsonValue,
    alignment_result: &AlignmentValidationResult,
) -> Result<ExecutionResult, SchemaError> {
    let engine_start = Instant::now();
    let mut execution_engine = ExecutionEngine::new();
    let chains_only: Vec<ParsedChain> = parsed_chains.iter().map(|(_, chain)| chain.clone()).collect();
    
    let execution_result = execution_engine.execute_fields(
        &chains_only,
        alignment_result,
        input_data.clone(),
    ).map_err(convert_iterator_stack_error)?;
    
    let engine_duration = engine_start.elapsed();
    info!("⏱️ ExecutionEngine execution took: {:?}", engine_duration);
    
    info!("📈 Enhanced ExecutionEngine produced {} index entries, {} warnings", 
          execution_result.index_entries.len(), execution_result.warnings.len());
    
    Ok(execution_result)
}


/// Logs execution statistics for advanced monitoring.
///
/// # Arguments
///
/// * `execution_result` - The execution result to analyze
fn log_execution_statistics(execution_result: &ExecutionResult) {
    info!("📊 Execution Statistics:");
    info!("  - Index entries produced: {}", execution_result.index_entries.len());
    info!("  - Warnings generated: {}", execution_result.warnings.len());
    
    if !execution_result.warnings.is_empty() {
        for warning in &execution_result.warnings {
            info!("  - Warning: {:?}", warning);
        }
    }
    
}


