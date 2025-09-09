//! Shared utilities for declarative transform execution.
//!
//! This module consolidates common functionality used across different
//! executor modules to eliminate code duplication and improve maintainability.

use crate::transform::iterator_stack::chain_parser::{ChainParser, ParsedChain};
use crate::transform::iterator_stack::errors::IteratorStackError;
use crate::schema::types::SchemaError;
use serde_json::Value as JsonValue;
use std::collections::HashMap;

/// Parses atom UUID expressions using ChainParser.
///
/// This function consolidates the duplicate parsing logic that was previously
/// scattered across multiple executor modules.
///
/// # Arguments
///
/// * `expression` - The expression to parse
///
/// # Returns
///
/// The parsed chain or error
pub fn parse_atom_uuid_expression(expression: &str) -> Result<ParsedChain, SchemaError> {
    let parser = ChainParser::new();
    parser.parse(expression).map_err(|err| {
        SchemaError::InvalidField(format!("Failed to parse expression '{}': {}", expression, err))
    })
}

/// Converts IteratorStackError to SchemaError.
///
/// This function consolidates the duplicate error conversion logic that was
/// previously scattered across multiple executor modules.
///
/// # Arguments
///
/// * `error` - The iterator stack error to convert
///
/// # Returns
///
/// Converted schema error
pub fn convert_iterator_stack_error(error: IteratorStackError) -> SchemaError {
    SchemaError::InvalidField(format!("Iterator stack error: {}", error))
}

/// Resolves a dotted path in input values.
///
/// This function consolidates the duplicate path resolution logic that was
/// previously scattered across multiple executor modules.
///
/// # Arguments
///
/// * `path` - The dotted path to resolve (e.g., "user.profile.name")
/// * `input_values` - The input values to search in
///
/// # Returns
///
/// Resolved value or error
pub fn resolve_dotted_path(path: &str, input_values: &HashMap<String, JsonValue>) -> Result<JsonValue, SchemaError> {
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

/// Extracts a simple path from chain operations.
///
/// This function consolidates the duplicate path extraction logic that was
/// previously scattered across multiple executor modules.
///
/// # Arguments
///
/// * `operations` - The chain operations to extract path from
///
/// # Returns
///
/// The extracted simple path (e.g., "user.profile.name")
pub fn extract_simple_path_from_operations(operations: &[crate::transform::iterator_stack::chain_parser::ChainOperation]) -> String {
    let mut path_parts = Vec::new();
    
    for operation in operations {
        match operation {
            crate::transform::iterator_stack::chain_parser::ChainOperation::FieldAccess(field_name) => {
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

/// Resolves field value from parsed chain with fallback mechanisms.
///
/// This function consolidates the duplicate field resolution logic that was
/// previously scattered across multiple executor modules.
///
/// # Arguments
///
/// * `parsed_chain` - The parsed chain to resolve
/// * `input_values` - The input values for fallback
/// * `field_name` - The field name for context (used in error messages)
///
/// # Returns
///
/// Resolved field value or error
pub fn resolve_field_value_from_chain(
    parsed_chain: &ParsedChain,
    input_values: &HashMap<String, JsonValue>,
    field_name: &str,
) -> Result<JsonValue, SchemaError> {
    // Extract simple path from operations for basic field access
    let simple_path = extract_simple_path_from_operations(&parsed_chain.operations);
    
    if simple_path.is_empty() {
        return Err(SchemaError::InvalidField(format!("No simple path found in parsed chain for field '{}'", field_name)));
    }
    
    // Try to resolve the simple path
    resolve_dotted_path(&simple_path, input_values)
}

/// Enhanced parsing with retry mechanism for better error recovery.
///
/// This function consolidates the retry logic that was previously only in
/// the coordination module.
///
/// # Arguments
///
/// * `expression` - The expression to parse
/// * `field_name` - The field name for context
/// * `max_retries` - Maximum number of retry attempts (default: 2)
///
/// # Returns
///
/// The parsed chain with retry logic
pub fn parse_with_retry(expression: &str, field_name: &str, max_retries: u32) -> Result<ParsedChain, SchemaError> {
    for attempt in 1..=max_retries {
        match parse_atom_uuid_expression(expression) {
            Ok(parsed_chain) => return Ok(parsed_chain),
            Err(err) => {
                if attempt == max_retries {
                    return Err(SchemaError::InvalidField(format!(
                        "Failed to parse expression '{}' for field '{}' after {} attempts: {}", 
                        expression, field_name, max_retries, err
                    )));
                }
                log::info!("⚠️ Parse attempt {} failed for field '{}', retrying: {}", attempt, field_name, err);
            }
        }
    }
    
    unreachable!("Retry loop should have returned or errored")
}

/// Default retry parsing with standard retry count.
pub fn parse_with_default_retry(expression: &str, field_name: &str) -> Result<ParsedChain, SchemaError> {
    parse_with_retry(expression, field_name, 2)
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn test_parse_atom_uuid_expression_simple() {
        let result = parse_atom_uuid_expression("input.field");
        assert!(result.is_ok());
    }

    #[test]
    fn test_parse_atom_uuid_expression_invalid() {
        let result = parse_atom_uuid_expression("invalid..syntax");
        assert!(result.is_err());
    }

    #[test]
    fn test_resolve_dotted_path_simple() {
        let input_values = HashMap::from([
            ("user".to_string(), json!({"name": "John", "age": 30})),
        ]);
        
        let result = resolve_dotted_path("user.name", &input_values);
        assert!(result.is_ok());
        assert_eq!(result.unwrap(), json!("John"));
    }

    #[test]
    fn test_resolve_dotted_path_nested() {
        let input_values = HashMap::from([
            ("user".to_string(), json!({"profile": {"name": "John"}})),
        ]);
        
        let result = resolve_dotted_path("user.profile.name", &input_values);
        assert!(result.is_ok());
        assert_eq!(result.unwrap(), json!("John"));
    }

    #[test]
    fn test_resolve_dotted_path_array() {
        let input_values = HashMap::from([
            ("items".to_string(), json!(["first", "second", "third"])),
        ]);
        
        let result = resolve_dotted_path("items.1", &input_values);
        assert!(result.is_ok());
        assert_eq!(result.unwrap(), json!("second"));
    }

    #[test]
    fn test_resolve_dotted_path_not_found() {
        let input_values = HashMap::from([
            ("user".to_string(), json!({"name": "John"})),
        ]);
        
        let result = resolve_dotted_path("user.age", &input_values);
        assert!(result.is_err());
    }

    #[test]
    fn test_extract_simple_path_from_operations() {
        use crate::transform::iterator_stack::chain_parser::ChainOperation;
        
        let operations = vec![
            ChainOperation::FieldAccess("user".to_string()),
            ChainOperation::FieldAccess("profile".to_string()),
            ChainOperation::FieldAccess("name".to_string()),
        ];
        
        let path = extract_simple_path_from_operations(&operations);
        assert_eq!(path, "user.profile.name");
    }

    #[test]
    fn test_extract_simple_path_empty() {
        let operations = vec![];
        let path = extract_simple_path_from_operations(&operations);
        assert_eq!(path, "");
    }
}
