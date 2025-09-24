//! Shared utilities for declarative transform execution.
//!
//! This module consolidates common functionality used across different
//! executor modules to eliminate code duplication and improve maintainability.

use crate::schema::types::{DeclarativeSchemaDefinition, SchemaError};
use crate::transform::iterator_stack::chain_parser::{ChainParser, ParsedChain};
use crate::transform::iterator_stack::errors::IteratorStackError;
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
        SchemaError::InvalidField(format!(
            "Failed to parse expression '{}': {}",
            expression, err
        ))
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

/// Formats validation errors with standardized message format.
///
/// This function consolidates the duplicate validation error formatting logic that was
/// previously scattered across multiple modules.
///
/// # Arguments
///
/// * `errors` - Vector of validation error messages
/// * `context` - Additional context for the error (e.g., "Field alignment validation")
///
/// # Returns
///
/// Formatted error message
pub fn format_validation_errors(errors: &[String], context: &str) -> String {
    if errors.is_empty() {
        return format!("{} failed: No errors provided", context);
    }

    if errors.len() == 1 {
        format!("{} failed: {}", context, errors[0])
    } else {
        format!("{} failed: {}", context, errors.join("; "))
    }
}

/// Formats parsing errors with standardized message format.
///
/// This function consolidates the duplicate parsing error formatting logic that was
/// previously scattered across multiple modules.
///
/// # Arguments
///
/// * `parsing_errors` - Vector of (field_name, expression, error) tuples
/// * `context` - Additional context for the error (e.g., "Expression parsing")
///
/// # Returns
///
/// Formatted error message
pub fn format_parsing_errors(
    parsing_errors: &[(String, String, SchemaError)],
    context: &str,
) -> String {
    if parsing_errors.is_empty() {
        return format!("{} failed: No parsing errors provided", context);
    }

    let error_messages: Vec<String> = parsing_errors
        .iter()
        .map(|(field, expr, err)| format!("Field '{}' expression '{}': {}", field, expr, err))
        .collect();

    format!(
        "{} failed due to parsing errors: {}",
        context,
        error_messages.join("; ")
    )
}

/// Formats field access errors with standardized message format.
///
/// This function consolidates the duplicate field access error formatting logic that was
/// previously scattered across multiple modules.
///
/// # Arguments
///
/// * `field_name` - The field name that failed to access
/// * `path` - The path that was being accessed
/// * `reason` - The reason for the failure
///
/// # Returns
///
/// Formatted error message
pub fn format_field_access_error(field_name: &str, path: &str, reason: &str) -> String {
    format!(
        "Field access failed for '{}' at path '{}': {}",
        field_name, path, reason
    )
}

/// Formats alignment validation errors with standardized message format.
///
/// This function consolidates the duplicate alignment validation error formatting logic that was
/// previously scattered across validation modules.
///
/// # Arguments
///
/// * `alignment_errors` - Vector of alignment error messages
///
/// # Returns
///
/// Formatted error message
pub fn format_alignment_validation_errors(alignment_errors: &[String]) -> String {
    format_validation_errors(alignment_errors, "Field alignment validation")
}

/// Creates a standardized SchemaError for validation failures.
///
/// This function consolidates the duplicate SchemaError creation logic that was
/// previously scattered across multiple modules.
///
/// # Arguments
///
/// * `errors` - Vector of validation error messages
/// * `context` - Additional context for the error
///
/// # Returns
///
/// Standardized SchemaError
pub fn create_validation_error(errors: &[String], context: &str) -> SchemaError {
    SchemaError::InvalidField(format_validation_errors(errors, context))
}

/// Creates a standardized SchemaError for parsing failures.
///
/// This function consolidates the duplicate SchemaError creation logic that was
/// previously scattered across multiple modules.
///
/// # Arguments
///
/// * `parsing_errors` - Vector of (field_name, expression, error) tuples
/// * `context` - Additional context for the error
///
/// # Returns
///
/// Standardized SchemaError
pub fn create_parsing_error(
    parsing_errors: &[(String, String, SchemaError)],
    context: &str,
) -> SchemaError {
    SchemaError::InvalidField(format_parsing_errors(parsing_errors, context))
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
pub fn resolve_dotted_path(
    path: &str,
    input_values: &HashMap<String, JsonValue>,
) -> Result<JsonValue, SchemaError> {
    let parts: Vec<&str> = path.split('.').collect();

    if parts.is_empty() {
        return Err(SchemaError::InvalidField("Empty path provided".to_string()));
    }

    // Start with the root value
    let mut current_value = input_values
        .get(parts[0])
        .ok_or_else(|| SchemaError::InvalidField(format!("Field '{}' not found", parts[0])))?
        .clone();

    // Navigate through the path
    for part in parts.iter().skip(1) {
        if let JsonValue::Object(obj) = current_value {
            current_value = obj
                .get(*part)
                .ok_or_else(|| {
                    SchemaError::InvalidField(format!(
                        "Field '{}' not found in path '{}'",
                        part, path
                    ))
                })?
                .clone();
        } else if let JsonValue::Array(arr) = current_value {
            if let Ok(index) = part.parse::<usize>() {
                current_value = arr
                    .get(index)
                    .ok_or_else(|| {
                        SchemaError::InvalidField(format!(
                            "Index '{}' out of bounds in path '{}'",
                            index, path
                        ))
                    })?
                    .clone();
            } else {
                return Err(SchemaError::InvalidField(format!(
                    "Invalid array index '{}' in path '{}'",
                    part, path
                )));
            }
        } else {
            return Err(SchemaError::InvalidField(format!(
                "Cannot access '{}' on non-object/non-array value in path '{}'",
                part, path
            )));
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
pub fn extract_simple_path_from_operations(
    operations: &[crate::transform::iterator_stack::chain_parser::ChainOperation],
) -> String {
    let mut path_parts = Vec::new();

    for operation in operations {
        match operation {
            crate::transform::iterator_stack::chain_parser::ChainOperation::FieldAccess(
                field_name,
            ) => {
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
        return Err(SchemaError::InvalidField(format!(
            "No simple path found in parsed chain for field '{}'",
            field_name
        )));
    }

    // Try to resolve the simple path
    resolve_dotted_path(&simple_path, input_values)
}

/// Enhanced parsing with retry mechanism for better error recovery.
/// Parses multiple expressions in batch with unified error handling.
///
/// This function consolidates the duplicate batch parsing logic that was previously
/// scattered across multiple executor modules.
///
/// # Arguments
///
/// * `expressions` - Vector of (field_name, expression) pairs to parse
///
/// # Returns
///
/// Vector of (field_name, ParsedChain) pairs for successfully parsed expressions
pub fn parse_expressions_batch(
    expressions: &[(String, String)],
) -> Result<Vec<(String, ParsedChain)>, SchemaError> {
    let mut parsed_chains = Vec::new();
    let mut parsing_errors = Vec::new();

    for (field_name, expression) in expressions {
        match parse_atom_uuid_expression(expression) {
            Ok(parsed_chain) => {
                parsed_chains.push((field_name.clone(), parsed_chain));
            }
            Err(err) => {
                parsing_errors.push((field_name.clone(), expression.clone(), err));
            }
        }
    }

    // Log warnings for failed expressions but don't fail the entire batch
    if !parsing_errors.is_empty() {
        let error_messages: Vec<String> = parsing_errors
            .iter()
            .map(|(field, expr, err)| format!("Field '{}' expression '{}': {}", field, expr, err))
            .collect();
        log::warn!(
            "⚠️ {} expressions failed to parse (will use fallback): {}",
            parsing_errors.len(),
            error_messages.join("; ")
        );
    }

    Ok(parsed_chains)
}

/// Collects all expressions from a schema definition.
///
/// This function consolidates the duplicate expression collection logic that was
/// previously scattered across multiple executor modules.
///
/// # Arguments
///
/// * `schema` - The declarative schema definition
///
/// # Returns
///
/// Vector of (field_name, expression) pairs
pub fn collect_expressions_from_schema(
    schema: &DeclarativeSchemaDefinition,
) -> Vec<(String, String)> {
    let mut all_expressions = Vec::new();

    for (field_name, field_def) in &schema.fields {
        if let Some(atom_uuid_expr) = &field_def.field_expression {
            all_expressions.push((field_name.clone(), atom_uuid_expr.clone()));
        }
    }

    all_expressions
}

/// Collects expressions from schema with additional key expressions.
///
/// This function consolidates the duplicate expression collection logic that was
/// previously scattered across coordination and range executor modules.
///
/// # Arguments
///
/// * `schema` - The declarative schema definition
/// * `key_expressions` - Additional key expressions to include (e.g., hash_field, range_field)
///
/// # Returns
///
/// Vector of (field_name, expression) pairs
pub fn collect_expressions_from_schema_with_keys(
    schema: &DeclarativeSchemaDefinition,
    key_expressions: &[(String, String)],
) -> Vec<(String, String)> {
    let mut all_expressions = Vec::new();

    // Add key expressions first
    all_expressions.extend(key_expressions.iter().cloned());

    // Add regular field expressions from schema
    all_expressions.extend(collect_expressions_from_schema(schema));

    all_expressions
}

/// Modifies expressions to add input prefix if needed.
///
/// This function consolidates the duplicate expression modification logic that was
/// previously scattered across executor modules.
///
/// # Arguments
///
/// * `expressions` - Vector of (field_name, expression) pairs
/// * `add_input_prefix` - Whether to add "input." prefix to expressions that don't have it
///
/// # Returns
///
/// Vector of (field_name, modified_expression) pairs
pub fn modify_expressions_with_input_prefix(
    expressions: &[(String, String)],
    add_input_prefix: bool,
) -> Vec<(String, String)> {
    if !add_input_prefix {
        return expressions.to_vec();
    }

    expressions
        .iter()
        .map(|(field_name, expression)| {
            let modified_expression = if expression.starts_with("input.") {
                expression.clone()
            } else {
                format!("input.{}", expression)
            };
            (field_name.clone(), modified_expression)
        })
        .collect()
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
        let input_values =
            HashMap::from([("user".to_string(), json!({"name": "John", "age": 30}))]);

        let result = resolve_dotted_path("user.name", &input_values);
        assert!(result.is_ok());
        assert_eq!(result.unwrap(), json!("John"));
    }

    #[test]
    fn test_resolve_dotted_path_nested() {
        let input_values =
            HashMap::from([("user".to_string(), json!({"profile": {"name": "John"}}))]);

        let result = resolve_dotted_path("user.profile.name", &input_values);
        assert!(result.is_ok());
        assert_eq!(result.unwrap(), json!("John"));
    }

    #[test]
    fn test_resolve_dotted_path_array() {
        let input_values =
            HashMap::from([("items".to_string(), json!(["first", "second", "third"]))]);

        let result = resolve_dotted_path("items.1", &input_values);
        assert!(result.is_ok());
        assert_eq!(result.unwrap(), json!("second"));
    }

    #[test]
    fn test_resolve_dotted_path_not_found() {
        let input_values = HashMap::from([("user".to_string(), json!({"name": "John"}))]);

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

    #[test]
    fn test_parse_expressions_batch_success() {
        let expressions = vec![
            ("field1".to_string(), "input.value1".to_string()),
            ("field2".to_string(), "input.value2".to_string()),
        ];

        let result = parse_expressions_batch(&expressions);
        assert!(result.is_ok());
        assert_eq!(result.unwrap().len(), 2);
    }

    #[test]
    fn test_parse_expressions_batch_failure() {
        let expressions = vec![
            ("field1".to_string(), "input.value1".to_string()),
            ("field2".to_string(), "invalid..syntax".to_string()),
        ];

        let result = parse_expressions_batch(&expressions);
        assert!(result.is_ok());

        // Should return only the successfully parsed expressions
        let parsed_chains = result.unwrap();
        assert_eq!(parsed_chains.len(), 1);
        assert_eq!(parsed_chains[0].0, "field1");
    }

    #[test]
    fn test_modify_expressions_with_input_prefix() {
        let expressions = vec![
            ("field1".to_string(), "input.value1".to_string()),
            ("field2".to_string(), "value2".to_string()),
        ];

        let modified = modify_expressions_with_input_prefix(&expressions, true);
        assert_eq!(modified[0].1, "input.value1"); // Already has prefix
        assert_eq!(modified[1].1, "input.value2"); // Added prefix

        let unmodified = modify_expressions_with_input_prefix(&expressions, false);
        assert_eq!(unmodified[0].1, "input.value1"); // No change
        assert_eq!(unmodified[1].1, "value2"); // No change
    }

    #[test]
    fn test_format_validation_errors_single() {
        let errors = vec!["Field 'name' is required".to_string()];
        let result = format_validation_errors(&errors, "Schema validation");
        assert_eq!(result, "Schema validation failed: Field 'name' is required");
    }

    #[test]
    fn test_format_validation_errors_multiple() {
        let errors = vec![
            "Field 'name' is required".to_string(),
            "Field 'age' must be positive".to_string(),
        ];
        let result = format_validation_errors(&errors, "Schema validation");
        assert_eq!(
            result,
            "Schema validation failed: Field 'name' is required; Field 'age' must be positive"
        );
    }

    #[test]
    fn test_format_validation_errors_empty() {
        let errors = vec![];
        let result = format_validation_errors(&errors, "Schema validation");
        assert_eq!(result, "Schema validation failed: No errors provided");
    }

    #[test]
    fn test_format_parsing_errors() {
        let parsing_errors = vec![
            (
                "field1".to_string(),
                "input.value1".to_string(),
                SchemaError::InvalidField("Parse error".to_string()),
            ),
            (
                "field2".to_string(),
                "invalid..syntax".to_string(),
                SchemaError::InvalidField("Syntax error".to_string()),
            ),
        ];
        let result = format_parsing_errors(&parsing_errors, "Expression parsing");
        assert!(result.contains("Expression parsing failed due to parsing errors"));
        assert!(result.contains("Field 'field1' expression 'input.value1'"));
        assert!(result.contains("Field 'field2' expression 'invalid..syntax'"));
    }

    #[test]
    fn test_format_field_access_error() {
        let result = format_field_access_error("user", "user.profile.name", "Field not found");
        assert_eq!(
            result,
            "Field access failed for 'user' at path 'user.profile.name': Field not found"
        );
    }

    #[test]
    fn test_format_alignment_validation_errors() {
        let errors = vec!["Fields have incompatible depths".to_string()];
        let result = format_alignment_validation_errors(&errors);
        assert_eq!(
            result,
            "Field alignment validation failed: Fields have incompatible depths"
        );
    }

    #[test]
    fn test_create_validation_error() {
        let errors = vec!["Field 'name' is required".to_string()];
        let result = create_validation_error(&errors, "Schema validation");
        match result {
            SchemaError::InvalidField(msg) => {
                assert_eq!(msg, "Schema validation failed: Field 'name' is required");
            }
            _ => panic!("Expected InvalidField error"),
        }
    }

    #[test]
    fn test_create_parsing_error() {
        let parsing_errors = vec![(
            "field1".to_string(),
            "input.value1".to_string(),
            SchemaError::InvalidField("Parse error".to_string()),
        )];
        let result = create_parsing_error(&parsing_errors, "Expression parsing");
        match result {
            SchemaError::InvalidField(msg) => {
                assert!(msg.contains("Expression parsing failed due to parsing errors"));
                assert!(msg.contains("Field 'field1' expression 'input.value1'"));
            }
            _ => panic!("Expected InvalidField error"),
        }
    }
}