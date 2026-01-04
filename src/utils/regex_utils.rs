//! Regex-specific error handling utilities.
//!
//! This module provides safe alternatives to `.unwrap()` calls when working
//! with regular expressions, particularly for regex compilation and matching.

use crate::schema::types::SchemaError;
use regex::Regex;

/// Utility for safe regex operations
pub struct RegexUtils;

impl RegexUtils {
    /// Safely compile a regex pattern with context
    pub fn compile_with_context(pattern: &str, context: &str) -> Result<Regex, SchemaError> {
        Regex::new(pattern).map_err(|e| {
            SchemaError::InvalidData(format!(
                "Regex compilation failed in {}: {} - Pattern: {}",
                context, e, pattern
            ))
        })
    }

    /// Safely compile a commonly used regex pattern
    pub fn compile_cross_reference_pattern() -> Result<Regex, SchemaError> {
        Self::compile_with_context(
            r"([A-Za-z0-9_]+)\.([A-Za-z0-9_]+)",
            "cross-reference pattern compilation",
        )
    }

    /// Safely compile an identifier pattern
    pub fn compile_identifier_pattern() -> Result<Regex, SchemaError> {
        Self::compile_with_context(
            r"^[A-Za-z_][A-Za-z0-9_]*$",
            "identifier pattern compilation",
        )
    }

    /// Safely get the first capture group from a match
    pub fn first_capture<'t>(
        captures: regex::Captures<'t>,
        context: &str,
    ) -> Result<&'t str, SchemaError> {
        captures.get(1).map(|m| m.as_str()).ok_or_else(|| {
            SchemaError::InvalidData(format!("No first capture group found: {}", context))
        })
    }

    /// Safely get a specific capture group from a match
    pub fn get_capture<'t>(
        captures: regex::Captures<'t>,
        index: usize,
        context: &str,
    ) -> Result<&'t str, SchemaError> {
        captures.get(index).map(|m| m.as_str()).ok_or_else(|| {
            SchemaError::InvalidData(format!("No capture group {} found: {}", index, context))
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_regex_compilation() {
        let result = RegexUtils::compile_with_context(r"\d+", "test pattern");
        assert!(result.is_ok());

        let bad_result = RegexUtils::compile_with_context(r"[", "invalid pattern");
        assert!(bad_result.is_err());
    }

    #[test]
    fn test_cross_reference_pattern() {
        let regex = RegexUtils::compile_cross_reference_pattern().unwrap();
        assert!(regex.is_match("schema.field"));
        assert!(!regex.is_match("invalid"));
    }

    #[test]
    fn test_capture_groups() {
        let regex = RegexUtils::compile_cross_reference_pattern().unwrap();
        let captures = regex.captures("schema.field").unwrap();

        let first = RegexUtils::first_capture(captures, "test").unwrap();
        assert_eq!(first, "schema");

        let captures = regex.captures("schema.field").unwrap();
        let second = RegexUtils::get_capture(captures, 2, "test").unwrap();
        assert_eq!(second, "field");
    }
}
