//! Iterator-specific error handling utilities.
//!
//! This module provides safe alternatives to `.unwrap()` calls when working
//! with iterators, particularly for getting next items safely.

use crate::schema::types::SchemaError;

/// Utility for safe iterator operations
pub struct IteratorUtils;

impl IteratorUtils {
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

    /// Safely collect iterator into Vec with size limit
    pub fn collect_with_limit<T, I>(
        iter: I,
        limit: usize,
        context: &str,
    ) -> Result<Vec<T>, SchemaError>
    where
        I: Iterator<Item = T>,
    {
        let mut result = Vec::new();
        for (i, item) in iter.enumerate() {
            if i >= limit {
                return Err(SchemaError::InvalidData(format!(
                    "Iterator exceeded limit {} in {}",
                    limit, context
                )));
            }
            result.push(item);
        }
        Ok(result)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_next_with_context() {
        let mut iter = vec![1, 2, 3].into_iter();
        assert_eq!(
            IteratorUtils::next_with_context(&mut iter, "test").unwrap(),
            1
        );
        assert_eq!(
            IteratorUtils::next_with_context(&mut iter, "test").unwrap(),
            2
        );
        assert_eq!(
            IteratorUtils::next_with_context(&mut iter, "test").unwrap(),
            3
        );
        assert!(IteratorUtils::next_with_context(&mut iter, "test").is_err());
    }

    #[test]
    fn test_first_with_context() {
        let iter = vec![1, 2, 3].into_iter();
        assert_eq!(IteratorUtils::first_with_context(iter, "test").unwrap(), 1);

        let empty_iter = std::iter::empty::<i32>();
        assert!(IteratorUtils::first_with_context(empty_iter, "test").is_err());
    }

    #[test]
    fn test_collect_with_limit() {
        let iter = vec![1, 2, 3].into_iter();
        let result = IteratorUtils::collect_with_limit(iter, 5, "test").unwrap();
        assert_eq!(result, vec![1, 2, 3]);

        let iter = vec![1, 2, 3, 4, 5, 6].into_iter();
        assert!(IteratorUtils::collect_with_limit(iter, 3, "test").is_err());
    }
}
