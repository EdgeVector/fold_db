//! Validation utilities to consolidate duplicate validation patterns across the codebase
//!
//! This module provides common validation functions that were previously duplicated
//! throughout the codebase as .is_empty() checks and similar patterns.

use crate::schema::types::SchemaError;

/// Validation utilities for common patterns
pub struct ValidationUtils;

impl ValidationUtils {
    /// Validates that a string is not empty, returning a SchemaError if it is
    pub fn require_non_empty_string(value: &str, field_name: &str) -> Result<(), SchemaError> {
        if value.is_empty() {
            return Err(SchemaError::InvalidField(format!(
                "{} cannot be empty",
                field_name
            )));
        }
        Ok(())
    }

    /// Validates that a collection is not empty, returning a SchemaError if it is
    pub fn require_non_empty_collection<T>(
        collection: &[T],
        field_name: &str,
    ) -> Result<(), SchemaError> {
        if collection.is_empty() {
            return Err(SchemaError::InvalidField(format!(
                "{} cannot be empty",
                field_name
            )));
        }
        Ok(())
    }

    /// Validates that an option contains a value
    pub fn require_some<'a, T>(
        option: &'a Option<T>,
        field_name: &str,
    ) -> Result<&'a T, SchemaError> {
        option
            .as_ref()
            .ok_or_else(|| SchemaError::InvalidField(format!("{} is required", field_name)))
    }

    /// Validates that a numeric value is positive
    pub fn require_positive(value: f64, field_name: &str) -> Result<(), SchemaError> {
        if value <= 0.0 {
            return Err(SchemaError::InvalidField(format!(
                "{} must be positive",
                field_name
            )));
        }
        Ok(())
    }

    /// Validates API key format (common pattern in ingestion configs)
    pub fn require_valid_api_key(api_key: &str, service_name: &str) -> Result<(), SchemaError> {
        Self::require_non_empty_string(api_key, &format!("{} API key", service_name))?;

        // Basic API key format validation
        if api_key.len() < 10 {
            return Err(SchemaError::InvalidField(format!(
                "{} API key appears to be too short",
                service_name
            )));
        }

        Ok(())
    }

    /// Validates field name format (schema.field)
    pub fn require_valid_field_name(field_name: &str) -> Result<(), SchemaError> {
        Self::require_non_empty_string(field_name, "Field name")?;

        if !field_name.contains('.') {
            return Err(SchemaError::InvalidField(
                "Field name must be in format 'schema.field'".to_string(),
            ));
        }

        let parts: Vec<&str> = field_name.split('.').collect();
        if parts.len() != 2 {
            return Err(SchemaError::InvalidField(
                "Field name must be in format 'schema.field'".to_string(),
            ));
        }

        Self::require_non_empty_string(parts[0], "Schema name")?;
        Self::require_non_empty_string(parts[1], "Field name")?;

        Ok(())
    }

    /// Validates schema name format with restrictive rules
    pub fn require_valid_schema_name(schema_name: &str) -> Result<(), SchemaError> {
        // Check for empty string
        Self::require_non_empty_string(schema_name, "Schema name")?;

        // Check for whitespace-only strings
        if schema_name.trim().is_empty() {
            return Err(SchemaError::InvalidField(
                "Schema name cannot be whitespace only".to_string(),
            ));
        }

        // Check length constraints
        if schema_name.len() < 3 {
            return Err(SchemaError::InvalidField(
                "Schema name must be at least 3 characters long".to_string(),
            ));
        }

        if schema_name.len() > 64 {
            return Err(SchemaError::InvalidField(
                "Schema name cannot exceed 64 characters".to_string(),
            ));
        }

        // Check for reserved words
        let reserved_words = [
            "system", "admin", "root", "default", "internal", "temp", "test",
        ];
        if reserved_words.contains(&schema_name.to_lowercase().as_str()) {
            return Err(SchemaError::InvalidField(format!(
                "Schema name '{}' is reserved and cannot be used",
                schema_name
            )));
        }

        // Check for invalid characters and patterns
        if schema_name
            .chars()
            .any(|c| c.is_control() || c == '\n' || c == '\r')
        {
            return Err(SchemaError::InvalidField(
                "Schema name cannot contain control characters".to_string(),
            ));
        }

        // Must start with a letter
        if !schema_name.chars().next().unwrap().is_alphabetic() {
            return Err(SchemaError::InvalidField(
                "Schema name must start with a letter".to_string(),
            ));
        }

        // Can only contain letters, numbers, and underscores
        if !schema_name.chars().all(|c| c.is_alphanumeric() || c == '_') {
            return Err(SchemaError::InvalidField(
                "Schema name can only contain letters, numbers, and underscores".to_string(),
            ));
        }

        // Cannot end with underscore
        if schema_name.ends_with('_') {
            return Err(SchemaError::InvalidField(
                "Schema name cannot end with an underscore".to_string(),
            ));
        }

        // Cannot contain consecutive underscores
        if schema_name.contains("__") {
            return Err(SchemaError::InvalidField(
                "Schema name cannot contain consecutive underscores".to_string(),
            ));
        }

        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_require_non_empty_string() {
        // Valid case
        assert!(ValidationUtils::require_non_empty_string("valid", "test").is_ok());

        // Invalid case
        assert!(ValidationUtils::require_non_empty_string("", "test").is_err());
    }

    #[test]
    fn test_require_valid_field_name() {
        // Valid cases
        assert!(ValidationUtils::require_valid_field_name("Schema.field").is_ok());

        // Invalid cases
        assert!(ValidationUtils::require_valid_field_name("").is_err());
        assert!(ValidationUtils::require_valid_field_name("no_dot").is_err());
        assert!(ValidationUtils::require_valid_field_name("too.many.dots").is_err());
        assert!(ValidationUtils::require_valid_field_name(".field").is_err());
        assert!(ValidationUtils::require_valid_field_name("schema.").is_err());
    }

    #[test]
    fn test_require_positive() {
        // Valid case
        assert!(ValidationUtils::require_positive(1.0, "test").is_ok());

        // Invalid cases
        assert!(ValidationUtils::require_positive(0.0, "test").is_err());
        assert!(ValidationUtils::require_positive(-1.0, "test").is_err());
    }

    #[test]
    fn test_require_valid_schema_name() {
        // Valid cases
        assert!(ValidationUtils::require_valid_schema_name("ValidSchema").is_ok());
        assert!(ValidationUtils::require_valid_schema_name("valid_schema").is_ok());
        assert!(ValidationUtils::require_valid_schema_name("schema123").is_ok());
        assert!(ValidationUtils::require_valid_schema_name("MySchema").is_ok());

        // Invalid cases - empty
        assert!(ValidationUtils::require_valid_schema_name("").is_err());

        // Invalid cases - whitespace only
        assert!(ValidationUtils::require_valid_schema_name("   ").is_err());
        assert!(ValidationUtils::require_valid_schema_name("\t\n").is_err());

        // Invalid cases - too short
        assert!(ValidationUtils::require_valid_schema_name("ab").is_err());
        assert!(ValidationUtils::require_valid_schema_name("a").is_err());

        // Invalid cases - too long
        let long_name = "a".repeat(65);
        assert!(ValidationUtils::require_valid_schema_name(&long_name).is_err());

        // Invalid cases - reserved words
        assert!(ValidationUtils::require_valid_schema_name("system").is_err());
        assert!(ValidationUtils::require_valid_schema_name("admin").is_err());
        assert!(ValidationUtils::require_valid_schema_name("root").is_err());
        assert!(ValidationUtils::require_valid_schema_name("default").is_err());
        assert!(ValidationUtils::require_valid_schema_name("internal").is_err());
        assert!(ValidationUtils::require_valid_schema_name("temp").is_err());
        assert!(ValidationUtils::require_valid_schema_name("test").is_err());

        // Invalid cases - control characters
        assert!(ValidationUtils::require_valid_schema_name("schema\x00").is_err());
        assert!(ValidationUtils::require_valid_schema_name("schema\n").is_err());
        assert!(ValidationUtils::require_valid_schema_name("schema\r").is_err());

        // Invalid cases - doesn't start with letter
        assert!(ValidationUtils::require_valid_schema_name("123schema").is_err());
        assert!(ValidationUtils::require_valid_schema_name("_schema").is_err());
        assert!(ValidationUtils::require_valid_schema_name("-schema").is_err());

        // Invalid cases - invalid characters
        assert!(ValidationUtils::require_valid_schema_name("schema-name").is_err());
        assert!(ValidationUtils::require_valid_schema_name("schema.name").is_err());
        assert!(ValidationUtils::require_valid_schema_name("schema name").is_err());
        assert!(ValidationUtils::require_valid_schema_name("schema@name").is_err());
        assert!(ValidationUtils::require_valid_schema_name("schema#name").is_err());

        // Invalid cases - ends with underscore
        assert!(ValidationUtils::require_valid_schema_name("schema_").is_err());

        // Invalid cases - consecutive underscores
        assert!(ValidationUtils::require_valid_schema_name("schema__name").is_err());
        assert!(ValidationUtils::require_valid_schema_name("schema___name").is_err());
    }
}
