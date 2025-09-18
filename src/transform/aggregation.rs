//! Result aggregation for transform execution.
//!
//! This module handles the aggregation of execution results from the ExecutionEngine
//! into the final output format for different schema types.

use crate::transform::iterator_stack::execution_engine::{ExecutionResult, IndexEntry};
use crate::transform::iterator_stack::chain_parser::ParsedChain;
use crate::transform::shared_utilities::resolve_field_value_from_chain;
use crate::schema::types::SchemaError;
use log::info;
use serde_json::Value as JsonValue;
use std::collections::HashMap;
use std::time::Instant;



/// Extracts optimal field value from execution engine entry.
///
/// # Arguments
///
/// * `entry` - The execution engine entry
///
/// # Returns
///
/// The extracted field value
fn extract_optimal_field_value(entry: &crate::transform::iterator_stack::execution_engine::IndexEntry) -> JsonValue {
    // For now, return the hash_value as the primary value
    // This could be enhanced to choose between hash_value and range_value based on context
    serde_json::to_value(&entry.hash_value).unwrap_or(JsonValue::Null)
}

/// Unified result aggregation function that handles all aggregation patterns.
///
/// This function consolidates the duplicate aggregation logic that was previously
/// scattered across multiple executor modules.
///
/// # Arguments
///
/// * `parsed_chains` - The parsed chains with their field names
/// * `execution_result` - The execution result from ExecutionEngine
/// * `input_values` - The original input values for fallback
/// * `all_expressions` - All expressions (including failed parsing attempts)
/// * `schema_type` - The type of schema being processed
///
/// # Returns
///
/// The aggregated result object
pub fn aggregate_results_unified(
    parsed_chains: &[(String, ParsedChain)],
    execution_result: &ExecutionResult,
    input_values: &HashMap<String, JsonValue>,
    all_expressions: &[(String, String)],
    schema_type: SchemaType,
) -> Result<JsonValue, SchemaError> {
    let start_time = Instant::now();
    info!("🔄 Unified aggregation for {:?} schema", schema_type);
    
    let mut result_object = serde_json::Map::new();
    
    if execution_result.index_entries.is_empty() {
        info!("⚠️ ExecutionEngine produced empty results, using direct value resolution");
        process_direct_value_resolution(parsed_chains, input_values, all_expressions, &mut result_object, schema_type)?;
    } else {
        info!("✅ Using ExecutionEngine results with aggregation processing");
        process_execution_result_aggregation(parsed_chains, execution_result, input_values, all_expressions, &mut result_object, schema_type)?;
    }
    
    let result = JsonValue::Object(result_object);
    let duration = start_time.elapsed();
    info!("⏱️ Unified aggregation completed in {:?}", duration);
    Ok(result)
}

/// Schema type for unified aggregation
#[derive(Debug, Clone, Copy)]
pub enum SchemaType {
    Single,
    Range,
    HashRange,
}

/// Unified direct value resolution for empty execution results.
///
/// When the ExecutionEngine produces no results, this function directly resolves
/// field values from input data using chain parsing or dotted path resolution.
///
/// # Arguments
///
/// * `parsed_chains` - The parsed chains with their field names
/// * `input_values` - The original input values for direct resolution
/// * `all_expressions` - All expressions (including failed parsing attempts)
/// * `result_object` - The result object to populate
/// * `schema_type` - The type of schema being processed
///
/// # Returns
///
/// Result indicating success or failure
fn process_direct_value_resolution(
    parsed_chains: &[(String, ParsedChain)],
    input_values: &HashMap<String, JsonValue>,
    all_expressions: &[(String, String)],
    result_object: &mut serde_json::Map<String, JsonValue>,
    schema_type: SchemaType,
) -> Result<(), SchemaError> {
    for (field_name, expression) in all_expressions {
        let field_value = if let Some((_, parsed_chain)) = parsed_chains.iter().find(|(name, _)| name == field_name) {
            // Field was successfully parsed, use chain resolution
            match resolve_field_value_from_chain(parsed_chain, input_values, field_name) {
                Ok(value) => value,
                Err(err) => {
                    info!("⚠️ Chain resolution failed for field '{}': {}", field_name, err);
                    JsonValue::Null
                }
            }
        } else {
            // Field failed to parse, try direct dotted path resolution
            match crate::transform::shared_utilities::resolve_dotted_path(expression, input_values) {
                Ok(value) => value,
                Err(err) => {
                    info!("⚠️ Direct dotted path resolution failed for field '{}': {}", field_name, err);
                    JsonValue::Null
                }
            }
        };
        
        // Handle field inclusion based on schema type
        match schema_type {
            SchemaType::HashRange => {
                // For HashRange schemas, convert internal fields to public fields
                // Note: For direct resolution, we create single values, not arrays
                // Arrays are only created when ExecutionEngine produces multiple IndexEntry objects
                match field_name.as_str() {
                    "_hash_field" => {
                        result_object.insert("hash_key".to_string(), field_value);
                    }
                    "_range_field" => {
                        result_object.insert("range_key".to_string(), field_value);
                    }
                    _ => {
                        // Regular fields are included as-is
                        result_object.insert(field_name.clone(), field_value);
                    }
                }
            }
            _ => {
                // For other schema types, filter out internal fields
                if !field_name.starts_with('_') {
                    result_object.insert(field_name.clone(), field_value);
                }
            }
        }
    }
    Ok(())
}

/// Unified execution result aggregation for successful execution results.
///
/// When the ExecutionEngine produces IndexEntry results, this function aggregates
/// them into the final result object, handling schema-specific field mapping and
/// array creation for HashRange schemas.
///
/// # Arguments
///
/// * `parsed_chains` - The parsed chains with their field names
/// * `execution_result` - The execution result from ExecutionEngine
/// * `input_values` - The original input values for fallback resolution
/// * `all_expressions` - All expressions (including failed parsing attempts)
/// * `result_object` - The result object to populate
/// * `schema_type` - The type of schema being processed
///
/// # Returns
///
/// Result indicating success or failure
fn process_execution_result_aggregation(
    parsed_chains: &[(String, ParsedChain)],
    execution_result: &ExecutionResult,
    input_values: &HashMap<String, JsonValue>,
    all_expressions: &[(String, String)],
    result_object: &mut serde_json::Map<String, JsonValue>,
    schema_type: SchemaType,
) -> Result<(), SchemaError> {
    match schema_type {
        SchemaType::HashRange => {
            // For HashRange schemas, we need to collect all values into arrays
            let mut field_arrays: HashMap<String, Vec<JsonValue>> = HashMap::new();
            
            // Initialize arrays for all fields
            for (field_name, _) in parsed_chains.iter() {
                field_arrays.insert(field_name.clone(), Vec::new());
            }
            
            // Collect all entries by expression (multiple entries per expression)
            let mut entries_by_expression: HashMap<String, Vec<&IndexEntry>> = HashMap::new();
            for entry in &execution_result.index_entries {
                entries_by_expression.entry(entry.expression.clone()).or_default().push(entry);
            }
            
            // Extract values from ExecutionEngine index entries for each field
            for (field_name, parsed_chain) in parsed_chains.iter() {
                if let Some(entries) = entries_by_expression.get(&parsed_chain.expression) {
                    for entry in entries {
                        let field_value = extract_optimal_field_value(entry);
                        
                        // Handle hash and range fields by storing them with their internal names
                        // for later conversion to public names
                        if field_name == "_hash_field" {
                            field_arrays.entry("_hash_field".to_string()).or_default().push(field_value);
                        } else if field_name == "_range_field" {
                            field_arrays.entry("_range_field".to_string()).or_default().push(field_value);
                        } else {
                            // Regular fields
                            field_arrays.entry(field_name.clone()).or_default().push(field_value);
                        }
                    }
                }
            }
            
            // Create compound key structure with arrays
            let hash_key_array = field_arrays.remove("_hash_field").unwrap_or_default();
            let range_key_array = field_arrays.remove("_range_field").unwrap_or_default();
            
            result_object.insert("hash_key".to_string(), JsonValue::Array(hash_key_array));
            result_object.insert("range_key".to_string(), JsonValue::Array(range_key_array));
            
            // Add regular fields as arrays
            for (field_name, field_array) in field_arrays {
                result_object.insert(field_name, JsonValue::Array(field_array));
            }
        }
        _ => {
            // For Single and Range schemas, use single values
            let mut entries_by_expression: HashMap<String, &IndexEntry> = HashMap::new();
            for entry in &execution_result.index_entries {
                entries_by_expression.insert(entry.expression.clone(), entry);
            }
            
            // Process all fields, including those that failed to parse
            for (field_name, expression) in all_expressions {
                let field_value = if let Some((_, parsed_chain)) = parsed_chains.iter().find(|(name, _)| name == field_name) {
                    // Field was successfully parsed, check if it was executed
                    if let Some(entry) = entries_by_expression.get(&parsed_chain.expression) {
                        extract_optimal_field_value(entry)
                    } else {
                        // Parsed but not executed, use fallback
                        match resolve_field_value_from_chain(parsed_chain, input_values, field_name) {
                            Ok(value) => value,
                            Err(err) => {
                                info!("⚠️ Fallback resolution failed for field '{}': {}", field_name, err);
                                JsonValue::Null
                            }
                        }
                    }
                } else {
                    // Field failed to parse, try direct dotted path resolution
                    match crate::transform::shared_utilities::resolve_dotted_path(expression, input_values) {
                        Ok(value) => value,
                        Err(err) => {
                            info!("⚠️ Direct dotted path resolution failed for field '{}': {}", field_name, err);
                            JsonValue::Null
                        }
                    }
                };
                
                // For other schema types, filter out internal fields
                if !field_name.starts_with('_') {
                    result_object.insert(field_name.clone(), field_value);
                }
            }
        }
    }
    
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::transform::iterator_stack::chain_parser::ParsedChain;
    use crate::transform::iterator_stack::execution_engine::{ExecutionResult, IndexEntry};
    use serde_json::json;
    
    #[test]
    fn test_aggregate_results_unified_empty_execution() {
        let parsed_chains = vec![
            ("field1".to_string(), ParsedChain {
                operations: vec![
                    crate::transform::iterator_stack::chain_parser::ChainOperation::FieldAccess("input".to_string()),
                    crate::transform::iterator_stack::chain_parser::ChainOperation::FieldAccess("field1".to_string()),
                ],
                expression: "input.field1".to_string(),
                depth: 0,
                branch: "main".to_string(),
                scopes: vec![],
            }),
        ];
        
        let execution_result = ExecutionResult {
            index_entries: vec![],
            statistics: crate::transform::iterator_stack::execution_engine::core::ExecutionStatistics {
                total_entries: 0,
                items_per_depth: HashMap::new(),
                memory_usage_bytes: 0,
                cache_hits: 0,
                cache_misses: 0,
            },
            warnings: vec![],
        };
        
        let input_values = HashMap::from([
            ("input".to_string(), json!({
                "field1": "value1"
            })),
        ]);
        
        let all_expressions = vec![
            ("field1".to_string(), "input.field1".to_string()),
        ];
        
        let result = aggregate_results_unified(
            &parsed_chains,
            &execution_result,
            &input_values,
            &all_expressions,
            SchemaType::Single,
        );
        
        assert!(result.is_ok());
        let result_value = result.unwrap();
        assert_eq!(result_value["field1"], json!("value1"));
    }
    
    #[test]
    fn test_aggregate_results_unified_with_execution() {
        let parsed_chains = vec![
            ("field1".to_string(), ParsedChain {
                operations: vec![
                    crate::transform::iterator_stack::chain_parser::ChainOperation::FieldAccess("input".to_string()),
                    crate::transform::iterator_stack::chain_parser::ChainOperation::FieldAccess("field1".to_string()),
                ],
                expression: "input.field1".to_string(),
                depth: 0,
                branch: "main".to_string(),
                scopes: vec![],
            }),
        ];
        
        let execution_result = ExecutionResult {
            index_entries: vec![
                IndexEntry {
                    expression: "input.field1".to_string(),
                    hash_value: json!("executed_value1"),
                    range_value: json!("range_value1"),
                    atom_uuid: "test-uuid".to_string(),
                    metadata: HashMap::new(),
                },
            ],
            statistics: crate::transform::iterator_stack::execution_engine::core::ExecutionStatistics {
                total_entries: 1,
                items_per_depth: HashMap::new(),
                memory_usage_bytes: 0,
                cache_hits: 0,
                cache_misses: 0,
            },
            warnings: vec![],
        };
        
        let input_values = HashMap::from([
            ("input".to_string(), json!({
                "field1": "fallback_value1"
            })),
        ]);
        
        let all_expressions = vec![
            ("field1".to_string(), "input.field1".to_string()),
        ];
        
        let result = aggregate_results_unified(
            &parsed_chains,
            &execution_result,
            &input_values,
            &all_expressions,
            SchemaType::Single,
        );
        
        assert!(result.is_ok());
        let result_value = result.unwrap();
        assert_eq!(result_value["field1"], json!("executed_value1"));
    }
    
    #[test]
    fn test_aggregate_results_unified_key_field_filtering() {
        let parsed_chains = vec![
            ("_hash_field".to_string(), ParsedChain {
                operations: vec![
                    crate::transform::iterator_stack::chain_parser::ChainOperation::FieldAccess("input".to_string()),
                    crate::transform::iterator_stack::chain_parser::ChainOperation::FieldAccess("hash".to_string()),
                ],
                expression: "input.hash".to_string(),
                depth: 0,
                branch: "main".to_string(),
                scopes: vec![],
            }),
            ("field1".to_string(), ParsedChain {
                operations: vec![
                    crate::transform::iterator_stack::chain_parser::ChainOperation::FieldAccess("input".to_string()),
                    crate::transform::iterator_stack::chain_parser::ChainOperation::FieldAccess("field1".to_string()),
                ],
                expression: "input.field1".to_string(),
                depth: 0,
                branch: "main".to_string(),
                scopes: vec![],
            }),
        ];
        
        let execution_result = ExecutionResult {
            index_entries: vec![],
            statistics: crate::transform::iterator_stack::execution_engine::core::ExecutionStatistics {
                total_entries: 0,
                items_per_depth: HashMap::new(),
                memory_usage_bytes: 0,
                cache_hits: 0,
                cache_misses: 0,
            },
            warnings: vec![],
        };
        
        let input_values = HashMap::from([
            ("input".to_string(), json!({
                "hash": "hash_value",
                "field1": "value1"
            })),
        ]);
        
        let all_expressions = vec![
            ("_hash_field".to_string(), "input.hash".to_string()),
            ("field1".to_string(), "input.field1".to_string()),
        ];
        
        let result = aggregate_results_unified(
            &parsed_chains,
            &execution_result,
            &input_values,
            &all_expressions,
            SchemaType::HashRange,
        );
        
        assert!(result.is_ok());
        let result_value = result.unwrap();
        
        // Key fields should not be included in final output
        assert!(!result_value.as_object().unwrap().contains_key("_hash_field"));
        assert!(result_value.as_object().unwrap().contains_key("field1"));
        assert_eq!(result_value["field1"], json!("value1"));
    }
    
    #[test]
    fn test_schema_type_enum() {
        assert_eq!(format!("{:?}", SchemaType::Single), "Single");
        assert_eq!(format!("{:?}", SchemaType::Range), "Range");
        assert_eq!(format!("{:?}", SchemaType::HashRange), "HashRange");
    }
}

