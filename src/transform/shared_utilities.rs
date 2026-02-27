//! Shared utilities for declarative transform execution.
//!
//! This module consolidates common functionality used across different
//! executor modules to eliminate code duplication and improve maintainability.

use crate::schema::types::SchemaError;
use crate::transform::chain_parser::{ChainParser, ParsedChain};
use serde_json::Value as JsonValue;
use std::collections::HashMap;

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

/// Resolves field value from parsed chain with fallback mechanisms.
///
/// This function consolidates the duplicate field resolution logic that was
/// previously scattered across multiple executor modules.
///
/// # Arguments
///
/// * `parsed_chain` - The parsed chain to resolve
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
        match ChainParser::new().parse(expression) {
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
