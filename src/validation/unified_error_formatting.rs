//! Unified error formatting utilities.
//!
//! This module consolidates all error formatting patterns that were previously duplicated
//! across the codebase, particularly SchemaError formatting patterns.

use crate::schema::types::SchemaError;

/// Unified error formatting for field validation errors.
///
/// This function consolidates the duplicate SchemaError::InvalidField formatting patterns
/// that were previously scattered across multiple modules.
///
/// # Arguments
///
/// * `message` - The error message template
/// * `args` - Arguments to format into the message
///
/// # Returns
///
/// Formatted SchemaError::InvalidField
pub fn invalid_field_error(message: &str, args: &[&dyn std::fmt::Display]) -> SchemaError {
    SchemaError::InvalidField(format_field_error_message(message, args))
}

/// Unified error formatting for data validation errors.
///
/// This function consolidates the duplicate SchemaError::InvalidData formatting patterns.
///
/// # Arguments
///
/// * `message` - The error message template
/// * `args` - Arguments to format into the message
///
/// # Returns
///
/// Formatted SchemaError::InvalidData
pub fn invalid_data_error(message: &str, args: &[&dyn std::fmt::Display]) -> SchemaError {
    SchemaError::InvalidData(format_field_error_message(message, args))
}

/// Unified error formatting for transform validation errors.
///
/// This function consolidates the duplicate SchemaError::InvalidTransform formatting patterns.
///
/// # Arguments
///
/// * `message` - The error message template
/// * `args` - Arguments to format into the message
///
/// # Returns
///
/// Formatted SchemaError::InvalidTransform
pub fn invalid_transform_error(message: &str, args: &[&dyn std::fmt::Display]) -> SchemaError {
    SchemaError::InvalidTransform(format_field_error_message(message, args))
}

/// Internal function to format error messages with arguments.
///
/// This function handles the actual formatting logic for error messages,
/// supporting both simple string messages and formatted messages with arguments.
fn format_field_error_message(message: &str, args: &[&dyn std::fmt::Display]) -> String {
    if args.is_empty() {
        message.to_string()
    } else {
        // For now, we'll use a simple approach. In a more sophisticated implementation,
        // we could support format strings with placeholders like "Field '{}' is invalid"
        // and replace them with the provided arguments.
        let mut result = message.to_string();
        for arg in args {
            result = result.replace("{}", &arg.to_string());
        }
        result
    }
}

/// Convenience macros for common error patterns.
///
/// These macros provide a more ergonomic way to create common error patterns
/// without having to manually construct argument arrays.
/// Creates an InvalidField error with a simple message.
#[macro_export]
macro_rules! invalid_field {
    ($msg:expr) => {
        $crate::validation::unified_error_formatting::invalid_field_error($msg, &[])
    };
}

/// Creates an InvalidField error with formatted arguments.
#[macro_export]
macro_rules! invalid_field_fmt {
    ($msg:expr, $($arg:expr),*) => {
        $crate::validation::unified_error_formatting::invalid_field_error($msg, &[$(&&$arg),*])
    };
}

/// Creates an InvalidData error with a simple message.
#[macro_export]
macro_rules! invalid_data {
    ($msg:expr) => {
        $crate::validation::unified_error_formatting::invalid_data_error($msg, &[])
    };
}

/// Creates an InvalidData error with formatted arguments.
#[macro_export]
macro_rules! invalid_data_fmt {
    ($msg:expr, $($arg:expr),*) => {
        $crate::validation::unified_error_formatting::invalid_data_error($msg, &[$(&&$arg),*])
    };
}

/// Creates an InvalidTransform error with a simple message.
#[macro_export]
macro_rules! invalid_transform {
    ($msg:expr) => {
        $crate::validation::unified_error_formatting::invalid_transform_error($msg, &[])
    };
}

/// Creates an InvalidTransform error with formatted arguments.
#[macro_export]
macro_rules! invalid_transform_fmt {
    ($msg:expr, $($arg:expr),*) => {
        $crate::validation::unified_error_formatting::invalid_transform_error($msg, &[$(&&$arg),*])
    };
}

/// Common error message templates.
///
/// This module provides standardized error message templates to ensure consistency
/// across the codebase.
pub mod templates {
    /// Field validation error templates
    pub mod field {
        pub const MISSING_PROPERTY: &str = "Field '{}' must have at least one property defined (atom_uuid or field_type)";
        pub const EMPTY_ATOM_UUID: &str = "Field '{}' atom_uuid cannot be empty";
        pub const INVALID_ATOM_UUID_FORMAT: &str = "Field '{}' atom_uuid expression '{}' cannot start or end with a dot";
        pub const CONSECUTIVE_DOTS: &str = "Field '{}' atom_uuid expression '{}' cannot contain consecutive dots";
        pub const EMPTY_FIELD_TYPE: &str = "Field '{}' field_type cannot be empty";
        pub const FIELD_TYPE_TOO_LONG: &str = "Field '{}' field_type '{}' is too long (max 100 characters)";
        pub const INVALID_FIELD_TYPE_CHARS: &str = "Field '{}' field_type '{}' contains invalid characters";
        pub const EMPTY_FIELD_NAME: &str = "Field name cannot be empty";
        pub const INVALID_FIELD_MAPPER: &str = "Field {} has invalid field mapper: empty key or value";
        pub const POSITIVE_MULTIPLIER: &str = "Field {} base_multiplier must be positive";
        pub const NON_ZERO_MIN_PAYMENT: &str = "Field {} min_payment cannot be zero";
    }

    /// Range schema error templates
    pub mod range {
        pub const RANGE_KEY_NOT_FOUND: &str = "RangeSchema range_key '{}' must be one of the schema's fields";
        pub const RANGE_KEY_MISSING: &str = "RangeSchema '{}' range_key field '{}' does not exist in the schema";
        pub const RANGE_KEY_WRONG_TYPE: &str = "RangeSchema '{}' has range_key field '{}' that is a {} field, but range_key must be a Range field";
        pub const NON_RANGE_FIELD: &str = "RangeSchema '{}' contains {} field '{}', but ALL fields must be Range fields. Consider using a regular Schema (not RangeSchema) if you need {} fields, or convert '{}' to a Range field to maintain RangeSchema consistency";
        pub const EMPTY_SCHEMA: &str = "RangeSchema '{}' must contain at least the range_key field '{}'";
        pub const JSON_RANGE_KEY_MISSING: &str = "JSON RangeSchema '{}' is missing the range_key field '{}'. The range_key must be defined as a field in the schema";
        pub const JSON_RANGE_KEY_WRONG_TYPE: &str = "JSON RangeSchema '{}' has range_key field '{}' defined as {:?} field, but it must be a Range field";
        pub const JSON_NON_RANGE_FIELD: &str = "JSON RangeSchema '{}' contains {} field '{}', but ALL fields must be Range fields. Consider using a regular Schema (not RangeSchema) if you need {} fields, or change '{}' to field_type: \"Range\" to maintain RangeSchema consistency";
    }

    /// Transform error templates
    pub mod transform {
        pub const INVALID_OUTPUT_FORMAT: &str = "Transform output field must be in format 'schema.field'";
        pub const INVALID_INPUT_FORMAT: &str = "Transform input field '{}' must be in format 'schema.field'";
        pub const INVALID_INPUT_FORMAT_DETAILED: &str = "Invalid input format {} for field {}";
        pub const SELF_REFERENCE: &str = "Transform input {} cannot reference its own field";
        pub const UNKNOWN_FIELD: &str = "Input {} references unknown field";
        pub const SCHEMA_NOT_FOUND: &str = "Schema {} not found for input {}";
        pub const OUTPUT_SCHEMA_NOT_FOUND: &str = "Schema {} not found for output {}.{}";
        pub const OUTPUT_FIELD_NOT_FOUND: &str = "Output field {} not found in schema {}";
        pub const OUTPUT_MISMATCH: &str = "Transform output {} does not match field name {}";
        pub const PARSE_FAILED: &str = "Failed to parse expression '{}': {}";
        pub const ITERATOR_STACK_ERROR: &str = "Iterator stack error: {}";
    }

    /// Mutation error templates
    pub mod mutation {
        pub const MISSING_RANGE_FIELD: &str = "Range schema mutation for '{}' is missing required range field '{}'. All range schema mutations must provide a value for the range field";
        pub const NULL_RANGE_FIELD: &str = "Range schema mutation for '{}' has null value for range field '{}'. Range field must have a valid value";
        pub const EMPTY_RANGE_FIELD: &str = "Range schema mutation for '{}' has empty string value for range field '{}'. Range field must have a non-empty value";
        pub const NON_RANGE_FIELD_IN_SCHEMA: &str = "Range schema '{}' contains {} field '{}', but all fields must be RangeFields";
        pub const NON_RANGE_FIELD_IN_MUTATION: &str = "All fields in a RangeSchema must be rangeFields. Field '{}' is not a rangeField";
        pub const INVALID_VALUE_FORMAT: &str = "Value for field '{}' must be an object containing the range_key '{}'";
        pub const MISSING_RANGE_KEY: &str = "Value for field '{}' must contain the range_key '{}'";
        pub const MISMATCHED_RANGE_KEYS: &str = "All range_key values must match for RangeSchema. Field '{}' has a different value";
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_invalid_field_error_simple() {
        let error = invalid_field_error("Simple error message", &[]);
        assert!(matches!(error, SchemaError::InvalidField(_)));
        assert!(error.to_string().contains("Simple error message"));
    }

    #[test]
    fn test_invalid_field_error_with_args() {
        let field_name = "test_field";
        let error = invalid_field_error("Field '{}' is invalid", &[&field_name]);
        assert!(matches!(error, SchemaError::InvalidField(_)));
        assert!(error.to_string().contains("test_field"));
    }

    #[test]
    fn test_invalid_data_error() {
        let error = invalid_data_error("Data validation failed", &[]);
        assert!(matches!(error, SchemaError::InvalidData(_)));
        assert!(error.to_string().contains("Data validation failed"));
    }

    #[test]
    fn test_invalid_transform_error() {
        let error = invalid_transform_error("Transform validation failed", &[]);
        assert!(matches!(error, SchemaError::InvalidTransform(_)));
        assert!(error.to_string().contains("Transform validation failed"));
    }

    #[test]
    fn test_macro_invalid_field() {
        let error = invalid_field!("Simple field error");
        assert!(matches!(error, SchemaError::InvalidField(_)));
        assert!(error.to_string().contains("Simple field error"));
    }

    #[test]
    fn test_macro_invalid_field_fmt() {
        let field_name = "test_field";
        let error = invalid_field_fmt!("Field '{}' is invalid", field_name);
        assert!(matches!(error, SchemaError::InvalidField(_)));
        assert!(error.to_string().contains("test_field"));
    }

    #[test]
    fn test_macro_invalid_data() {
        let error = invalid_data!("Simple data error");
        assert!(matches!(error, SchemaError::InvalidData(_)));
        assert!(error.to_string().contains("Simple data error"));
    }

    #[test]
    fn test_macro_invalid_transform() {
        let error = invalid_transform!("Simple transform error");
        assert!(matches!(error, SchemaError::InvalidTransform(_)));
        assert!(error.to_string().contains("Simple transform error"));
    }
}
