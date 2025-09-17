//! Single schema executor for declarative transforms.
//!
//! This module handles execution of Single schema types, which are the simplest
//! form of declarative transforms without range semantics or complex indexing.

use crate::schema::types::{SchemaError, json_schema::DeclarativeSchemaDefinition};
use crate::transform::shared_utilities::{
    convert_iterator_stack_error, resolve_field_value_from_chain,
    collect_expressions_from_schema, parse_expressions_batch, modify_expressions_with_input_prefix
};
use log::info;
use serde_json::Value as JsonValue;
use std::collections::HashMap;

/// Executes a Single schema declarative transform.
///
/// # Arguments
///
/// * `schema` - The declarative schema definition
/// * `input_values` - The input values for execution
///
/// # Returns
///
/// The execution result
pub fn execute_single_schema(
    schema: &DeclarativeSchemaDefinition,
    input_values: HashMap<String, JsonValue>,
) -> Result<JsonValue, SchemaError> {
    info!("🚀 Executing Single schema: {}", schema.name);
    
    // Use ExecutionEngine for consistent execution across all field types
    execute_with_execution_engine(schema, &input_values)
}

/// Executes Single schema using ExecutionEngine for consistent behavior.
///
/// # Arguments
///
/// * `schema` - The declarative schema definition
/// * `input_values` - The input values for execution
///
/// # Returns
///
/// The execution result
fn execute_with_execution_engine(
    schema: &DeclarativeSchemaDefinition,
    input_values: &HashMap<String, JsonValue>,
) -> Result<JsonValue, SchemaError> {
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
    let modified_chains_only: Vec<crate::transform::iterator_stack::chain_parser::ParsedChain> = 
        modified_chains.iter().map(|(_, chain)| chain.clone()).collect();
    let alignment_result = crate::transform::validation::validate_field_alignment_unified(
        None, 
        Some(&modified_chains_only)
    )?;
    
    // Structure input data with "input" field containing the actual input values
    let mut root_object = serde_json::Map::new();
    root_object.insert("input".to_string(), JsonValue::Object(input_values.iter().map(|(k, v)| (k.clone(), v.clone())).collect()));
    let input_data = JsonValue::Object(root_object);
    let mut execution_engine = crate::transform::iterator_stack::execution_engine::ExecutionEngine::new();
    
    let execution_result = execution_engine.execute_fields(
        &modified_chains.iter().map(|(_, chain)| chain.clone()).collect::<Vec<_>>(),
        &alignment_result,
        input_data,
    ).map_err(convert_iterator_stack_error)?;
    
    // Aggregate results into final output format
    aggregate_single_results(&modified_chains, &execution_result, input_values, &modified_expressions)
}

/// Aggregates results from ExecutionEngine into final output format for Single schemas.
///
/// # Arguments
///
/// * `parsed_chains` - The parsed chains with their field names
/// * `execution_result` - The execution result from ExecutionEngine
/// * `input_values` - The original input values for fallback
/// * `all_expressions` - All expressions including those that failed to parse
///
/// # Returns
///
/// The aggregated result object
fn aggregate_single_results(
    parsed_chains: &[(String, crate::transform::iterator_stack::chain_parser::ParsedChain)],
    execution_result: &crate::transform::iterator_stack::execution_engine::ExecutionResult,
    input_values: &HashMap<String, JsonValue>,
    all_expressions: &[(String, String)],
) -> Result<JsonValue, SchemaError> {
    info!("🔄 Aggregating results from Single schema execution");
    info!("📊 ExecutionEngine produced {} index entries", execution_result.index_entries.len());
    
    let mut result_object = serde_json::Map::new();
    
    if execution_result.index_entries.is_empty() {
        info!("⚠️ ExecutionEngine produced empty results, using fallback resolution");
        
        // Fallback resolution for empty results
        for (field_name, expression) in all_expressions {
            // Check if this field was successfully parsed
            if let Some((_, parsed_chain)) = parsed_chains.iter().find(|(name, _)| name == field_name) {
                let field_value = match resolve_field_value_from_chain(parsed_chain, input_values, field_name) {
                    Ok(value) => value,
                    Err(err) => {
                        info!("⚠️ Fallback resolution failed for field '{}': {}", field_name, err);
                        JsonValue::Null
                    }
                };
                result_object.insert(field_name.clone(), field_value);
            } else {
                // Field failed to parse, try direct dotted path resolution
                let field_value = match crate::transform::shared_utilities::resolve_dotted_path(expression, input_values) {
                    Ok(value) => value,
                    Err(err) => {
                        info!("⚠️ Direct dotted path resolution failed for field '{}': {}", field_name, err);
                        JsonValue::Null
                    }
                };
                result_object.insert(field_name.clone(), field_value);
            }
        }
    } else {
        info!("✅ Using ExecutionEngine results for Single schema");
        
        // Process results from ExecutionEngine - map by expression, not by index
        let mut entries_by_expression: HashMap<String, &crate::transform::iterator_stack::execution_engine::IndexEntry> = HashMap::new();
        for entry in &execution_result.index_entries {
            entries_by_expression.insert(entry.expression.clone(), entry);
        }
        
        // Process all fields, including those that failed to parse
        for (field_name, expression) in all_expressions {
            // Check if this field was successfully parsed and executed
            if let Some((_, parsed_chain)) = parsed_chains.iter().find(|(name, _)| name == field_name) {
                if let Some(entry) = entries_by_expression.get(&parsed_chain.expression) {
                    let field_value = serde_json::to_value(&entry.hash_value).unwrap_or(JsonValue::Null);
                    result_object.insert(field_name.clone(), field_value);
                } else {
                    // No entry for this field, use null
                    result_object.insert(field_name.clone(), JsonValue::Null);
                }
            } else {
                // Field failed to parse, try direct dotted path resolution
                info!("🔍 Trying direct dotted path resolution for field '{}' with expression '{}'", field_name, expression);
                let field_value = match crate::transform::shared_utilities::resolve_dotted_path(expression, input_values) {
                    Ok(value) => {
                        info!("✅ Direct dotted path resolution succeeded for field '{}': {}", field_name, value);
                        value
                    },
                    Err(err) => {
                        info!("⚠️ Direct dotted path resolution failed for field '{}': {}", field_name, err);
                        JsonValue::Null
                    }
                };
                result_object.insert(field_name.clone(), field_value);
            }
        }
    }
    
    let result = JsonValue::Object(result_object);
    info!("✨ Single schema aggregation completed: {}", result);
    Ok(result)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::schema::types::json_schema::DeclarativeSchemaDefinition;
    use crate::schema::types::schema::SchemaType;
    use serde_json::json;

    #[test]
    fn test_execute_single_schema_simple() {
        // Create a simple Single schema for testing
        let mut fields = std::collections::HashMap::new();
        fields.insert("title".to_string(), crate::schema::types::json_schema::FieldDefinition {
            field_type: Some("string".to_string()),
            atom_uuid: Some("input.title".to_string()),
        });
        fields.insert("count".to_string(), crate::schema::types::json_schema::FieldDefinition {
            field_type: Some("number".to_string()),
            atom_uuid: Some("input.count".to_string()),
        });
        
        let schema = DeclarativeSchemaDefinition {
            name: "test_schema".to_string(),
            schema_type: SchemaType::Single,
            fields,
            key: None,
        };
        
        let input_values = HashMap::from([
            ("title".to_string(), json!("Hello World")),
            ("count".to_string(), json!(42)),
        ]);
        
        let result = execute_single_schema(&schema, input_values);
        
        match result {
            Ok(json_result) => {
                assert!(json_result.is_object());
                let obj = json_result.as_object().unwrap();
                assert_eq!(obj.get("title").unwrap(), "Hello World");
                assert_eq!(obj.get("count").unwrap(), 42);
            }
            Err(err) => {
                panic!("Single schema execution failed: {}", err);
            }
        }
    }

    #[test]
    fn test_execute_single_schema_missing_field() {
        // Test handling of missing input fields
        let mut fields = std::collections::HashMap::new();
        fields.insert("title".to_string(), crate::schema::types::json_schema::FieldDefinition {
            field_type: Some("string".to_string()),
            atom_uuid: Some("input.title".to_string()),
        });
        fields.insert("missing".to_string(), crate::schema::types::json_schema::FieldDefinition {
            field_type: Some("string".to_string()),
            atom_uuid: Some("input.missing".to_string()),
        });
        
        let schema = DeclarativeSchemaDefinition {
            name: "test_schema".to_string(),
            schema_type: SchemaType::Single,
            fields,
            key: None,
        };
        
        let input_values = HashMap::from([
            ("title".to_string(), json!("Hello World")),
        ]);
        
        let result = execute_single_schema(&schema, input_values);
        
        match result {
            Ok(json_result) => {
                assert!(json_result.is_object());
                let obj = json_result.as_object().unwrap();
                assert_eq!(obj.get("title").unwrap(), "Hello World");
                assert_eq!(*obj.get("missing").unwrap(), json!(null));
            }
            Err(err) => {
                panic!("Single schema execution failed: {}", err);
            }
        }
    }

    #[test]
    fn test_execute_single_schema_complex_expression() {
        // Test handling of complex expressions (should return null)
        let mut fields = std::collections::HashMap::new();
        fields.insert("simple".to_string(), crate::schema::types::json_schema::FieldDefinition {
            field_type: Some("string".to_string()),
            atom_uuid: Some("input.simple".to_string()),
        });
        fields.insert("complex".to_string(), crate::schema::types::json_schema::FieldDefinition {
            field_type: Some("string".to_string()),
            atom_uuid: Some("input.data.map().filter()".to_string()),
        });
        
        let schema = DeclarativeSchemaDefinition {
            name: "test_schema".to_string(),
            schema_type: SchemaType::Single,
            fields,
            key: None,
        };
        
        let input_values = HashMap::from([
            ("simple".to_string(), json!("Hello World")),
        ]);
        
        let result = execute_single_schema(&schema, input_values);
        
        match result {
            Ok(json_result) => {
                assert!(json_result.is_object());
                let obj = json_result.as_object().unwrap();
                assert_eq!(obj.get("simple").unwrap(), "Hello World");
                assert_eq!(*obj.get("complex").unwrap(), json!(null));
            }
            Err(err) => {
                panic!("Single schema execution failed: {}", err);
            }
        }
    }
}
