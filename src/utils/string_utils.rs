//! String-specific error handling utilities.
//!
//! This module provides safe alternatives to `.unwrap()` calls when working
//! with strings, particularly for character operations and string parsing.

use crate::schema::types::SchemaError;

/// Utility for safe string operations
pub struct StringUtils;

impl StringUtils {
    /// Safely get the first character of a string
    pub fn first_char(s: &str, context: &str) -> Result<char, SchemaError> {
        s.chars()
            .next()
            .ok_or_else(|| SchemaError::InvalidData(format!("Empty string: {}", context)))
    }

    /// Safely check if string starts with numeric character
    pub fn starts_with_numeric(s: &str) -> bool {
        s.chars().next().is_some_and(|c| c.is_numeric())
    }

    /// Safely check if string starts with alphabetic character
    pub fn starts_with_alphabetic(s: &str) -> bool {
        s.chars().next().is_some_and(|c| c.is_alphabetic())
    }

    /// Safely get a substring with bounds checking
    pub fn safe_substring<'a>(
        s: &'a str,
        start: usize,
        end: usize,
        context: &str,
    ) -> Result<&'a str, SchemaError> {
        if start > s.len() || end > s.len() || start > end {
            return Err(SchemaError::InvalidData(format!(
                "Invalid substring bounds [{}, {}] for string of length {} in {}",
                start,
                end,
                s.len(),
                context
            )));
        }
        Ok(&s[start..end])
    }

    /// Safely parse a string to a number
    pub fn parse_number<T>(s: &str, context: &str) -> Result<T, SchemaError>
    where
        T: std::str::FromStr,
        T::Err: std::fmt::Display,
    {
        s.parse().map_err(|e| {
            SchemaError::InvalidData(format!(
                "Failed to parse '{}' as number in {}: {}",
                s, context, e
            ))
        })
    }

    /// Safely split string and get specific part
    pub fn split_and_get<'a>(
        s: &'a str,
        delimiter: char,
        index: usize,
        context: &str,
    ) -> Result<&'a str, SchemaError> {
        s.split(delimiter).nth(index).ok_or_else(|| {
            SchemaError::InvalidData(format!(
                "No part {} found when splitting '{}' by '{}' in {}",
                index, s, delimiter, context
            ))
        })
    }

    /// Safely trim and validate non-empty
    pub fn trim_non_empty<'a>(s: &'a str, context: &str) -> Result<&'a str, SchemaError> {
        let trimmed = s.trim();
        if trimmed.is_empty() {
            Err(SchemaError::InvalidData(format!(
                "String is empty after trimming in {}",
                context
            )))
        } else {
            Ok(trimmed)
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_first_char() {
        assert_eq!(StringUtils::first_char("hello", "test").unwrap(), 'h');
        assert!(StringUtils::first_char("", "test").is_err());
    }

    #[test]
    fn test_starts_with_checks() {
        assert!(StringUtils::starts_with_numeric("123abc"));
        assert!(!StringUtils::starts_with_numeric("abc123"));
        assert!(!StringUtils::starts_with_numeric(""));

        assert!(StringUtils::starts_with_alphabetic("abc123"));
        assert!(!StringUtils::starts_with_alphabetic("123abc"));
        assert!(!StringUtils::starts_with_alphabetic(""));
    }

    #[test]
    fn test_safe_substring() {
        let s = "hello world";
        assert_eq!(
            StringUtils::safe_substring(s, 0, 5, "test").unwrap(),
            "hello"
        );
        assert_eq!(
            StringUtils::safe_substring(s, 6, 11, "test").unwrap(),
            "world"
        );
        assert!(StringUtils::safe_substring(s, 0, 20, "test").is_err());
        assert!(StringUtils::safe_substring(s, 5, 3, "test").is_err());
    }

    #[test]
    #[allow(clippy::approx_constant)]
    fn test_parse_number() {
        assert_eq!(StringUtils::parse_number::<i32>("42", "test").unwrap(), 42);
        assert_eq!(
            StringUtils::parse_number::<f64>("3.14", "test").unwrap(),
            3.14
        );
        assert!(StringUtils::parse_number::<i32>("not_a_number", "test").is_err());
    }

    #[test]
    fn test_split_and_get() {
        let s = "a,b,c,d";
        assert_eq!(StringUtils::split_and_get(s, ',', 0, "test").unwrap(), "a");
        assert_eq!(StringUtils::split_and_get(s, ',', 2, "test").unwrap(), "c");
        assert!(StringUtils::split_and_get(s, ',', 10, "test").is_err());
    }

    #[test]
    fn test_trim_non_empty() {
        assert_eq!(
            StringUtils::trim_non_empty("  hello  ", "test").unwrap(),
            "hello"
        );
        assert!(StringUtils::trim_non_empty("   ", "test").is_err());
        assert!(StringUtils::trim_non_empty("", "test").is_err());
    }
}
