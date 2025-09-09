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
    
    // Parse all field expressions for multi-chain coordination
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
    
    // Parse all expressions using ChainParser with retry mechanism
    let parsing_start = Instant::now();
    let mut parsed_chains = Vec::new();
    let mut parsing_errors = Vec::new();
    
    for (field_name, expression) in &all_expressions {
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
    
    // Validate field alignment across all chains with enhanced monitoring
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
    
    // Execute multi-chain coordination with ExecutionEngine and enhanced monitoring
    let execution_start = Instant::now();
    let result = execute_multi_chain_with_engine_enhanced(&parsed_chains, input_values, &alignment_result);
    let execution_duration = execution_start.elapsed();
    
    let total_duration = start_time.elapsed();
    info!("⏱️ Enhanced multi-chain coordination completed in {:?} (parsing: {:?}, validation: {:?}, execution: {:?})", 
          total_duration, parsing_duration, validation_duration, execution_duration);
    
    result
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
    
    // Convert input_values HashMap to JSON object for ExecutionEngine
    let conversion_start = Instant::now();
    let input_data = JsonValue::Object(input_values.iter().map(|(k, v)| (k.clone(), v.clone())).collect());
    let conversion_duration = conversion_start.elapsed();
    info!("⏱️ Input data conversion took: {:?}", conversion_duration);
    
    // Create and execute with ExecutionEngine for all chains with enhanced monitoring
    let engine_start = Instant::now();
    let mut execution_engine = ExecutionEngine::new();
    let chains_only: Vec<ParsedChain> = parsed_chains.iter().map(|(_, chain)| chain.clone()).collect();
    
    let execution_result = execution_engine.execute_fields(
        &chains_only,
        alignment_result,
        input_data,
    ).map_err(convert_iterator_stack_error)?;
    
    let engine_duration = engine_start.elapsed();
    info!("⏱️ ExecutionEngine execution took: {:?}", engine_duration);
    
    info!("📈 Enhanced ExecutionEngine produced {} index entries, {} warnings", 
          execution_result.index_entries.len(), execution_result.warnings.len());
    
    // Log execution statistics for advanced monitoring
    log_execution_statistics(&execution_result);
    
    // Aggregate results from multi-chain execution with enhanced error handling
    let aggregation_start = Instant::now();
    let result = crate::transform::aggregation::aggregate_multi_chain_results_enhanced(parsed_chains, &execution_result, input_values);
    let aggregation_duration = aggregation_start.elapsed();
    
    let total_duration = start_time.elapsed();
    info!("⏱️ Enhanced multi-chain execution completed in {:?} (conversion: {:?}, engine: {:?}, aggregation: {:?})", 
          total_duration, conversion_duration, engine_duration, aggregation_duration);
    
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

