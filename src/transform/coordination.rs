//! Multi-chain coordination for transform execution.
//!
//! This module handles the complex coordination logic for executing multiple
//! transform chains together, particularly for HashRange and Range schemas.

use crate::transform::iterator_stack::chain_parser::ParsedChain;
use crate::transform::iterator_stack::field_alignment::{FieldAlignmentValidator, AlignmentValidationResult};
use crate::transform::iterator_stack::execution_engine::{ExecutionEngine, ExecutionResult};
use crate::transform::shared_utilities::{convert_iterator_stack_error, parse_with_default_retry};
use crate::schema::types::SchemaError;
use log::{info, error};
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
    
    let expressions = collect_all_expressions(schema, key_config)?;
    let parsed_chains = parse_expressions_with_monitoring(&expressions)?;
    let alignment_result = validate_field_alignment(&parsed_chains)?;
    let result = execute_coordination_with_engine(&parsed_chains, input_values, &alignment_result)?;
    
    let total_duration = start_time.elapsed();
    info!("⏱️ Enhanced multi-chain coordination completed in {:?}", total_duration);
    
    Ok(result)
}

/// Collects all expressions from schema and key configuration.
///
/// # Arguments
///
/// * `schema` - The declarative schema definition
/// * `key_config` - The key configuration
///
/// # Returns
///
/// Vector of (field_name, expression) pairs
fn collect_all_expressions(
    schema: &crate::schema::types::json_schema::DeclarativeSchemaDefinition,
    key_config: &crate::schema::types::json_schema::KeyConfig,
) -> Result<Vec<(String, String)>, SchemaError> {
    let mut all_expressions = Vec::new();
    
    // Add key expressions (hash_field and range_field) - these are expressions, not field names
    all_expressions.push(("_hash_field".to_string(), key_config.hash_field.clone()));
    all_expressions.push(("_range_field".to_string(), key_config.range_field.clone()));
    
    // Add regular field expressions from schema
    for (field_name, field_def) in &schema.fields {
        if let Some(atom_uuid_expr) = &field_def.atom_uuid {
            all_expressions.push((field_name.clone(), atom_uuid_expr.clone()));
        }
    }
    
    info!("📊 Coordinating {} expressions for enhanced multi-chain execution", all_expressions.len());
    Ok(all_expressions)
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
    let mut parsed_chains = Vec::new();
    let mut parsing_errors = Vec::new();
    
    for (field_name, expression) in expressions {
        info!("🔗 Parsing expression for field '{}': {}", field_name, expression);
        
        match parse_with_default_retry(expression, field_name) {
            Ok(parsed_chain) => {
                parsed_chains.push((field_name.clone(), parsed_chain));
                info!("✅ Successfully parsed expression for field '{}'", field_name);
            }
            Err(parse_error) => {
                info!("⚠️ Failed to parse expression for field '{}': {}", field_name, parse_error);
                parsing_errors.push((field_name.clone(), expression.clone(), parse_error));
            }
        }
    }
    
    let parsing_duration = parsing_start.elapsed();
    info!("⏱️ Enhanced parsing took: {:?}", parsing_duration);
    
    // Check if we have enough parsed chains to proceed
    if parsed_chains.is_empty() {
        return Err(SchemaError::InvalidField(
            "No expressions could be parsed for enhanced multi-chain coordination".to_string()
        ));
    }
    
    Ok(parsed_chains)
}

/// Validates field alignment across all chains with enhanced monitoring.
///
/// # Arguments
///
/// * `parsed_chains` - Vector of (field_name, ParsedChain) pairs
///
/// # Returns
///
/// Alignment validation result
fn validate_field_alignment(
    parsed_chains: &[(String, ParsedChain)],
) -> Result<AlignmentValidationResult, SchemaError> {
    let validation_start = Instant::now();
    let chains_only: Vec<ParsedChain> = parsed_chains.iter().map(|(_, chain)| chain.clone()).collect();
    let validator = FieldAlignmentValidator::new();
    let alignment_result = validator.validate_alignment(&chains_only)
        .map_err(|err| SchemaError::InvalidField(format!("Alignment validation failed: {}", err)))?;
    let validation_duration = validation_start.elapsed();
    info!("⏱️ Enhanced field alignment validation took: {:?}", validation_duration);
    
    if !alignment_result.valid {
        let error_messages: Vec<String> = alignment_result.errors.iter()
            .map(|err| format!("{:?}: {}", err.error_type, err.message))
            .collect();
        error!("🚨 Enhanced multi-chain field alignment validation failed: {}", error_messages.join("; "));
        return Err(SchemaError::InvalidField(format!(
            "Enhanced multi-chain field alignment validation failed: {}", 
            error_messages.join("; ")
        )));
    }
    
    info!("✅ Enhanced multi-chain field alignment validation passed");
    Ok(alignment_result)
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
    let result = aggregate_execution_results(parsed_chains, &execution_result, input_values)?;
    
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

/// Aggregates execution results from multi-chain execution.
///
/// # Arguments
///
/// * `parsed_chains` - The parsed chains with their field names
/// * `execution_result` - The execution result from the engine
/// * `input_values` - The original input values
///
/// # Returns
///
/// Aggregated result
fn aggregate_execution_results(
    parsed_chains: &[(String, ParsedChain)],
    execution_result: &ExecutionResult,
    input_values: &HashMap<String, JsonValue>,
) -> Result<JsonValue, SchemaError> {
    let aggregation_start = Instant::now();
    let result = crate::transform::aggregation::aggregate_multi_chain_results_enhanced(parsed_chains, execution_result, input_values);
    let aggregation_duration = aggregation_start.elapsed();
    info!("⏱️ Aggregation took: {:?}", aggregation_duration);
    result
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
    
    // Analyze execution quality
    let analysis = analyze_execution_results(execution_result);
    info!("  - Quality score: {:.2}", analysis.quality_score);
    info!("  - Has placeholders: {}", analysis.has_placeholders);
    
    if !analysis.issues.is_empty() {
        info!("  - Issues detected: {}", analysis.issues.join(", "));
    }
}

/// Analyzes execution results for placeholder detection and quality assessment.
fn analyze_execution_results(execution_result: &ExecutionResult) -> crate::transform::aggregation::ExecutionAnalysis {
    let mut analysis = crate::transform::aggregation::ExecutionAnalysis {
        has_placeholders: false,
        quality_score: 1.0,
        issues: Vec::new(),
    };
    
    // Check for placeholder content
    for entry in &execution_result.index_entries {
        if entry.hash_value.to_string().contains("placeholder") || 
           entry.range_value.to_string().contains("placeholder") ||
           entry.hash_value.to_string().contains("null") ||
           entry.range_value.to_string().contains("null") {
            analysis.has_placeholders = true;
            analysis.quality_score -= 0.1;
            analysis.issues.push("Contains placeholder values".to_string());
        }
    }
    
    // Check for warnings
    if !execution_result.warnings.is_empty() {
        analysis.quality_score -= 0.2;
        analysis.issues.push(format!("{} warnings generated", execution_result.warnings.len()));
    }
    
    // Ensure quality score doesn't go below 0
    analysis.quality_score = analysis.quality_score.max(0.0);
    
    analysis
}

