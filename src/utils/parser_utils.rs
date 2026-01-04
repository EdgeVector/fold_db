//! Parser-specific error handling utilities.
//!
//! This module provides safe alternatives to `.unwrap()` calls commonly found
//! in parser code, particularly for handling parse tree navigation.

use crate::schema::types::SchemaError;

/// Utility for safe parser operations
pub struct ParserUtils;

impl ParserUtils {
    /// Create a parse error with context
    pub fn parse_error(context: &str, details: &str) -> SchemaError {
        SchemaError::InvalidField(format!("Parse error in {}: {}", context, details))
    }

    /// Create an iterator exhausted error
    pub fn iterator_exhausted_error(context: &str) -> SchemaError {
        SchemaError::InvalidField(format!("Iterator exhausted: {}", context))
    }

    /// Create a no pairs found error
    pub fn no_pairs_error(context: &str) -> SchemaError {
        SchemaError::InvalidField(format!("No pairs found: {}", context))
    }

    /// Create a no inner pairs error
    pub fn no_inner_pairs_error(context: &str) -> SchemaError {
        SchemaError::InvalidField(format!("No inner pairs found: {}", context))
    }

    /// Create an unexpected rule error
    pub fn unexpected_rule_error(expected: &str, found: &str, context: &str) -> SchemaError {
        SchemaError::InvalidField(format!(
            "Expected {} but found {} in {}",
            expected, found, context
        ))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_error_creation() {
        let error = ParserUtils::parse_error("test context", "test details");
        assert!(error.to_string().contains("test context"));
        assert!(error.to_string().contains("test details"));
    }

    #[test]
    fn test_iterator_exhausted_error() {
        let error = ParserUtils::iterator_exhausted_error("test iterator");
        assert!(error.to_string().contains("Iterator exhausted"));
        assert!(error.to_string().contains("test iterator"));
    }
}
