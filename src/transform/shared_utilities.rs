//! Shared utilities for declarative transform execution.
//!
//! This module consolidates common functionality used across different
//! executor modules to eliminate code duplication and improve maintainability.

use crate::schema::types::SchemaError;
use crate::transform::chain_parser::{ChainParser, ParsedChain};

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
