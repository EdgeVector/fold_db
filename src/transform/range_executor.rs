//! Range schema executor for transform execution.
//!
//! This module handles the execution of Range schema types, including
//! validation, coordination, and multi-chain execution.

use crate::transform::iterator_stack::chain_parser::ParsedChain;
use crate::transform::iterator_stack::field_alignment::AlignmentValidationResult;
use crate::transform::iterator_stack::execution_engine::{ExecutionEngine, ExecutionResult};
use crate::transform::shared_utilities::{
    convert_iterator_stack_error, resolve_field_value_from_chain,
    collect_expressions_from_schema_with_keys, parse_expressions_batch
};
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
    info!("🔧 Executing Range schema: {} with range_key: {}", schema.name, range_key);
    
    // Validate schema structure
    schema.validate()?;
    
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
    
    // Aggregate results from multi-chain execution
    aggregate_multi_chain_results(parsed_chains, &execution_result, input_values)
}

/// Aggregates results from multi-chain execution into final output format.
///
/// # Arguments
///
/// * `parsed_chains` - The parsed chains with their field names
/// * `execution_result` - The execution result from ExecutionEngine
/// * `input_values` - The original input values for fallback
///
/// # Returns
///
/// The aggregated result object
fn aggregate_multi_chain_results(
    parsed_chains: &[(String, ParsedChain)],
    execution_result: &ExecutionResult,
    input_values: &HashMap<String, JsonValue>,
) -> Result<JsonValue, SchemaError> {
    info!("🔄 Aggregating results from multi-chain execution");
    
    let mut result_object = serde_json::Map::new();
    
    if execution_result.index_entries.is_empty() {
        info!("⚠️ ExecutionEngine produced empty results, using fallback resolution");
        
        // Fallback resolution for empty results
        for (field_name, parsed_chain) in parsed_chains {
            let field_value = match resolve_field_value(parsed_chain, input_values, field_name) {
                Ok(value) => value,
                Err(err) => {
                    info!("⚠️ Fallback resolution failed for field '{}': {}", field_name, err);
                    JsonValue::Null
                }
            };
            
            // Special handling for key fields (don't include in final output)
            if !field_name.starts_with('_') {
                result_object.insert(field_name.clone(), field_value);
            }
        }
    } else {
        info!("✅ Using ExecutionEngine results for multi-chain coordination");
        
        // Process results from ExecutionEngine
        for (i, (field_name, _)) in parsed_chains.iter().enumerate() {
            if let Some(entry) = execution_result.index_entries.get(i) {
                let field_value = serde_json::to_value(&entry.hash_value).unwrap_or(JsonValue::Null);
                
                // Special handling for key fields (don't include in final output)
                if !field_name.starts_with('_') {
                    result_object.insert(field_name.clone(), field_value);
                }
            } else {
                // No entry for this field, use null
                if !field_name.starts_with('_') {
                    result_object.insert(field_name.clone(), JsonValue::Null);
                }
            }
        }
    }
    
    let result = JsonValue::Object(result_object);
    info!("✨ Range aggregation completed: {}", result);
    Ok(result)
}

/// Resolves field value from parsed chain with fallback.
///
/// # Arguments
///
/// * `parsed_chain` - The parsed chain to resolve
/// * `input_values` - The input values for fallback
/// * `field_name` - The field name for context
///
/// # Returns
///
/// Resolved field value or error
fn resolve_field_value(
    parsed_chain: &ParsedChain,
    input_values: &HashMap<String, JsonValue>,
    field_name: &str,
) -> Result<JsonValue, SchemaError> {
    // Use shared utility for field resolution
    resolve_field_value_from_chain(parsed_chain, input_values, field_name)
}
