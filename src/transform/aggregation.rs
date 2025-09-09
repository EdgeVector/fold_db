//! Result aggregation for transform execution.
//!
//! This module handles the aggregation of execution results from the ExecutionEngine
//! into the final output format for different schema types.

use crate::schema::indexing::execution_engine::{ExecutionResult, IndexEntry};
use crate::schema::constants::{HASH_KEY_NAME, RANGE_KEY_NAME};
use crate::schema::indexing::chain_parser::ParsedChain;
use crate::schema::types::SchemaError;
use log::info;
use serde_json::Value as JsonValue;
use std::collections::HashMap;
use std::time::Instant;

/// Analysis result for execution quality assessment
#[derive(Debug)]
pub struct ExecutionAnalysis {
    pub has_placeholders: bool,
    pub quality_score: f64,
    pub issues: Vec<String>,
}

/// Enhanced result aggregation with better error handling and optimization.
///
/// # Arguments
///
/// * `parsed_chains` - The parsed chains with their field names
/// * `execution_result` - The execution result from ExecutionEngine
/// * `input_values` - The original input values for fallback
///
/// # Returns
///
/// The enhanced aggregated result object
pub fn aggregate_multi_chain_results_enhanced(
    parsed_chains: &[(String, ParsedChain)],
    execution_result: &ExecutionResult,
    input_values: &HashMap<String, JsonValue>,
) -> Result<JsonValue, SchemaError> {
    let _start_time = Instant::now();
    info!("🔄 Aggregating results from enhanced multi-chain execution");
    
    // Enhanced placeholder detection with more sophisticated analysis
    let placeholder_analysis = analyze_execution_results(execution_result);
    
    // Check if this is a HashRange schema by looking for key fields
    let has_key_fields = parsed_chains.iter().any(|(field_name, _)| {
        field_name == "_hash_field" || field_name == "_range_field"
    });
    
    if has_key_fields {
        info!("🔑 Detected HashRange schema - creating compound key structure");
        aggregate_hashrange_results(parsed_chains, execution_result, input_values, &placeholder_analysis)
    } else {
        info!("📋 Processing regular schema - using standard aggregation");
        aggregate_regular_results(parsed_chains, execution_result, input_values, &placeholder_analysis)
    }
}

/// Aggregates results for HashRange schemas with compound key structure.
///
/// # Arguments
///
/// * `parsed_chains` - The parsed chains with their field names
/// * `execution_result` - The execution result from ExecutionEngine
/// * `input_values` - The original input values for fallback
/// * `placeholder_analysis` - Analysis of execution results
///
/// # Returns
///
/// The HashRange aggregated result with compound key structure
fn aggregate_hashrange_results(
    parsed_chains: &[(String, ParsedChain)],
    execution_result: &ExecutionResult,
    input_values: &HashMap<String, JsonValue>,
    _placeholder_analysis: &ExecutionAnalysis,
) -> Result<JsonValue, SchemaError> {
    let _start_time = Instant::now();
    info!("🔑 Aggregating HashRange results with compound key structure");
    
    // For HashRange schemas, we need to extract values from ExecutionEngine results
    // and organize them into arrays for each field
    let mut field_arrays: HashMap<String, Vec<JsonValue>> = HashMap::new();
    
    // Initialize arrays for all fields
    for (field_name, _) in parsed_chains.iter() {
        field_arrays.insert(field_name.clone(), Vec::new());
    }
    
    // Extract values from ExecutionEngine index entries
    // The ExecutionEngine produces separate entries for each expression
    // We need to map each field to its corresponding values
    
    // First, collect all entries by expression
    let mut entries_by_expression: HashMap<String, Vec<&IndexEntry>> = HashMap::new();
    for entry in &execution_result.index_entries {
        entries_by_expression.entry(entry.expression.clone()).or_default().push(entry);
    }
    
    println!("DEBUG: Available expressions in execution result: {:?}", entries_by_expression.keys().collect::<Vec<_>>());
    println!("DEBUG: Parsed chains: {:?}", parsed_chains.iter().map(|(name, chain)| (name, &chain.expression)).collect::<Vec<_>>());
    
    // Now map each field to its values
    for (field_name, parsed_chain) in parsed_chains.iter() {
        if field_name != "_hash_field" && field_name != "_range_field" {
            // Find the entries that correspond to this field's expression
            if let Some(entries) = entries_by_expression.get(&parsed_chain.expression) {
                for entry in entries {
                    field_arrays.entry(field_name.clone()).or_default().push(entry.hash_value.clone());
                }
            }
        }
    }
    
    // Handle hash and range fields - only get values from the specific expressions
    for (field_name, parsed_chain) in parsed_chains.iter() {
        if field_name == "_hash_field" {
            // Find entries that correspond to the hash field expression
            if let Some(entries) = entries_by_expression.get(&parsed_chain.expression) {
                for entry in entries {
                    field_arrays.entry("_hash_field".to_string()).or_default().push(entry.hash_value.clone());
                }
            }
        } else if field_name == "_range_field" {
            // Find entries that correspond to the range field expression
            if let Some(entries) = entries_by_expression.get(&parsed_chain.expression) {
                for entry in entries {
                    field_arrays.entry("_range_field".to_string()).or_default().push(entry.hash_value.clone());
                }
            }
        }
    }
    
    // If ExecutionEngine didn't produce results, use fallback resolution
    if execution_result.index_entries.is_empty() {
        info!("⚠️ ExecutionEngine produced no entries, using fallback resolution for HashRange");
        
        for (field_name, parsed_chain) in parsed_chains.iter() {
            let field_value = match resolve_with_enhanced_fallback(parsed_chain, input_values, field_name) {
                Ok(value) => value,
                Err(err) => {
                    info!("⚠️ Enhanced fallback resolution failed for field '{}': {}", field_name, err);
                    JsonValue::Null
                }
            };
            
            field_arrays.entry(field_name.clone()).or_default().push(field_value);
        }
    }
    
    // Create compound key structure with arrays
    let mut result_object = serde_json::Map::new();
    
    // Add hash_key and range_key arrays (converted from _hash_field and _range_field)
    let hash_key_array = field_arrays.remove("_hash_field").unwrap_or_default();
    let range_key_array = field_arrays.remove("_range_field").unwrap_or_default();
    
    result_object.insert(HASH_KEY_NAME.to_string(), JsonValue::Array(hash_key_array));
    result_object.insert(RANGE_KEY_NAME.to_string(), JsonValue::Array(range_key_array));
    
    // Add regular fields as arrays
    for (field_name, field_array) in field_arrays {
        result_object.insert(field_name, JsonValue::Array(field_array));
    }
    
    let result = JsonValue::Object(result_object);
    info!("✨ HashRange aggregation completed: {}", result);
    Ok(result)
}

/// Aggregates results for regular schemas (non-HashRange).
///
/// # Arguments
///
/// * `parsed_chains` - The parsed chains with their field names
/// * `execution_result` - The execution result from ExecutionEngine
/// * `input_values` - The original input values for fallback
/// * `placeholder_analysis` - Analysis of execution results
///
/// # Returns
///
/// The regular aggregated result object
fn aggregate_regular_results(
    parsed_chains: &[(String, ParsedChain)],
    execution_result: &ExecutionResult,
    input_values: &HashMap<String, JsonValue>,
    placeholder_analysis: &ExecutionAnalysis,
) -> Result<JsonValue, SchemaError> {
    let start_time = Instant::now();
    info!("📋 Aggregating regular schema results");
    
    let mut result_object = serde_json::Map::new();
    
    if placeholder_analysis.has_placeholders || execution_result.index_entries.is_empty() {
        info!("⚠️ ExecutionEngine produced placeholder/empty results, using enhanced fallback resolution");
        
        // Enhanced fallback with better error handling
        let fallback_start = Instant::now();
        for (field_name, parsed_chain) in parsed_chains {
            let field_value = match resolve_with_enhanced_fallback(parsed_chain, input_values, field_name) {
                Ok(value) => value,
                Err(err) => {
                    info!("⚠️ Enhanced fallback resolution failed for field '{}': {}", field_name, err);
                    JsonValue::Null
                }
            };
            
            // Special handling for key fields (don't include in final output)
            if !field_name.starts_with('_') {
                result_object.insert(field_name.clone(), field_value);
            }
        }
        let fallback_duration = fallback_start.elapsed();
        info!("⏱️ Enhanced fallback resolution took: {:?}", fallback_duration);
    } else {
        info!("✅ Using ExecutionEngine results for enhanced multi-chain coordination");
        
        // Enhanced result processing with optimization
        let processing_start = Instant::now();
        for (i, (field_name, _)) in parsed_chains.iter().enumerate() {
            if let Some(entry) = execution_result.index_entries.get(i) {
                let field_value = extract_optimal_field_value(entry);
                
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
        let processing_duration = processing_start.elapsed();
        info!("⏱️ Enhanced result processing took: {:?}", processing_duration);
    }
    
    let result = JsonValue::Object(result_object);
    let total_duration = start_time.elapsed();
    info!("✨ Regular aggregation completed in {:?}: {}", total_duration, result);
    Ok(result)
}

/// Analyzes execution results for placeholder detection and quality assessment.
pub fn analyze_execution_results(execution_result: &ExecutionResult) -> ExecutionAnalysis {
    let mut analysis = ExecutionAnalysis {
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

/// Resolves field values with enhanced fallback mechanisms.
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
fn resolve_with_enhanced_fallback(
    parsed_chain: &ParsedChain,
    input_values: &HashMap<String, JsonValue>,
    field_name: &str,
) -> Result<JsonValue, SchemaError> {
    // Try to resolve using the parsed chain
    match resolve_parsed_chain_simple(parsed_chain, input_values) {
        Ok(value) => Ok(value),
        Err(err) => {
            info!("⚠️ Chain resolution failed for field '{}', using fallback: {}", field_name, err);
            
            // Enhanced fallback: try to extract value from input based on field name
            if let Some(input_value) = input_values.get(field_name) {
                Ok(input_value.clone())
            } else {
                // Ultimate fallback: return null
                info!("⚠️ No fallback value found for field '{}', using null", field_name);
                Ok(JsonValue::Null)
            }
        }
    }
}

/// Extracts optimal field value from execution engine entry.
///
/// # Arguments
///
/// * `entry` - The execution engine entry
///
/// # Returns
///
/// The extracted field value
fn extract_optimal_field_value(entry: &crate::schema::indexing::execution_engine::IndexEntry) -> JsonValue {
    // For now, return the hash_value as the primary value
    // This could be enhanced to choose between hash_value and range_value based on context
    serde_json::to_value(&entry.hash_value).unwrap_or(JsonValue::Null)
}

/// Resolves parsed chain with simple fallback logic.
///
/// # Arguments
///
/// * `parsed_chain` - The parsed chain to resolve
/// * `input_values` - The input values for resolution
///
/// # Returns
///
/// Resolved value or error
fn resolve_parsed_chain_simple(
    parsed_chain: &ParsedChain,
    input_values: &HashMap<String, JsonValue>,
) -> Result<JsonValue, SchemaError> {
    // Extract simple path from operations for basic field access
    let simple_path = extract_simple_path_from_operations(&parsed_chain.operations);
    
    if simple_path.is_empty() {
        return Err(SchemaError::InvalidField("No simple path found in parsed chain".to_string()));
    }
    
    // Try to resolve the simple path
    resolve_dotted_path(&simple_path, input_values)
}

/// Extracts simple path from chain operations.
///
/// # Arguments
///
/// * `operations` - The chain operations
///
/// # Returns
///
/// The extracted simple path
fn extract_simple_path_from_operations(operations: &[crate::schema::indexing::chain_parser::ChainOperation]) -> String {
    let mut path_parts = Vec::new();
    
    for operation in operations {
        match operation {
            crate::schema::indexing::chain_parser::ChainOperation::FieldAccess(field_name) => {
                path_parts.push(field_name.clone());
            }
            _ => {
                // For complex operations, we can't extract a simple path
                return String::new();
            }
        }
    }
    
    path_parts.join(".")
}

/// Resolves dotted path in input values.
///
/// # Arguments
///
/// * `path` - The dotted path to resolve
/// * `input_values` - The input values to search in
///
/// # Returns
///
/// Resolved value or error
fn resolve_dotted_path(path: &str, input_values: &HashMap<String, JsonValue>) -> Result<JsonValue, SchemaError> {
    let parts: Vec<&str> = path.split('.').collect();
    
    if parts.is_empty() {
        return Err(SchemaError::InvalidField("Empty path provided".to_string()));
    }
    
    // Start with the root value
    let mut current_value = input_values.get(parts[0])
        .ok_or_else(|| SchemaError::InvalidField(format!("Field '{}' not found", parts[0])))?
        .clone();
    
    // Navigate through the path
    for part in parts.iter().skip(1) {
        if let JsonValue::Object(obj) = current_value {
            current_value = obj.get(*part)
                .ok_or_else(|| SchemaError::InvalidField(format!("Field '{}' not found in path '{}'", part, path)))?
                .clone();
        } else if let JsonValue::Array(arr) = current_value {
            if let Ok(index) = part.parse::<usize>() {
                current_value = arr.get(index)
                    .ok_or_else(|| SchemaError::InvalidField(format!("Index '{}' out of bounds in path '{}'", index, path)))?
                    .clone();
            } else {
                return Err(SchemaError::InvalidField(format!("Invalid array index '{}' in path '{}'", part, path)));
            }
        } else {
            return Err(SchemaError::InvalidField(format!("Cannot access '{}' on non-object/non-array value in path '{}'", part, path)));
        }
    }
    
    Ok(current_value)
}
