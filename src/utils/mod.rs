//! Comprehensive error handling utilities for the DataFold system.
//!
//! This module provides utilities to eliminate unsafe `.unwrap()` calls
//! and implement robust error handling patterns across the codebase.

pub mod error_factory;
pub mod http_errors;
pub mod iterator_utils;
pub mod parser_utils;
pub mod regex_utils;
pub mod string_utils;

use crate::schema::types::SchemaError;

/// Utility trait for safe unwrapping with context
pub trait SafeUnwrap<T> {
    /// Safely unwrap with a custom error message
    fn safe_unwrap(self, context: &str) -> Result<T, SchemaError>;
}

impl<T> SafeUnwrap<T> for Option<T> {
    fn safe_unwrap(self, context: &str) -> Result<T, SchemaError> {
        self.ok_or_else(|| SchemaError::InvalidData(format!("Unexpected None: {}", context)))
    }
}

impl<T, E: std::fmt::Display> SafeUnwrap<T> for Result<T, E> {
    fn safe_unwrap(self, context: &str) -> Result<T, SchemaError> {
        self.map_err(|e| SchemaError::InvalidData(format!("{}: {}", context, e)))
    }
}

/// Utility for safe iterator operations
pub struct SafeIterator;

impl SafeIterator {
    /// Safely get the next item from an iterator with context
    pub fn next_with_context<T, I>(iter: &mut I, context: &str) -> Result<T, SchemaError>
    where
        I: Iterator<Item = T>,
    {
        iter.next()
            .ok_or_else(|| SchemaError::InvalidData(format!("Iterator exhausted: {}", context)))
    }

    /// Safely get the first item from an iterator with context
    pub fn first_with_context<T, I>(mut iter: I, context: &str) -> Result<T, SchemaError>
    where
        I: Iterator<Item = T>,
    {
        iter.next()
            .ok_or_else(|| SchemaError::InvalidData(format!("Empty iterator: {}", context)))
    }
}

/// Utility for safe string operations
pub struct SafeString;

impl SafeString {
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
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_safe_unwrap_option() {
        let some_val: Option<i32> = Some(42);
        let none_val: Option<i32> = None;

        assert_eq!(some_val.safe_unwrap("test context").unwrap(), 42);
        assert!(none_val.safe_unwrap("test context").is_err());
    }

    #[test]
    fn test_safe_iterator() {
        let mut iter = vec![1, 2, 3].into_iter();
        assert_eq!(
            SafeIterator::next_with_context(&mut iter, "test").unwrap(),
            1
        );

        let empty_iter = std::iter::empty::<i32>();
        assert!(SafeIterator::first_with_context(empty_iter, "test").is_err());
    }

    #[test]
    fn test_safe_string() {
        assert_eq!(SafeString::first_char("hello", "test").unwrap(), 'h');
        assert!(SafeString::first_char("", "test").is_err());
        assert!(SafeString::starts_with_numeric("123abc"));
        assert!(!SafeString::starts_with_numeric("abc123"));
    }
}
