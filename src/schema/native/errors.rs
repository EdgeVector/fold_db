use crate::transform::native::{FieldDefinitionError, FieldType};
use thiserror::Error;

/// Validation failures emitted while normalizing schema key configuration.
#[derive(Debug, Error, PartialEq, Eq)]
pub enum KeyConfigError {
    /// A key field was blank or contained only whitespace characters.
    #[error("{field} cannot be empty")]
    EmptyField { field: &'static str },
    /// Hash and range expressions must point at distinct fields.
    #[error("hash and range fields must be different")]
    DuplicateHashAndRange,
}

/// Errors produced while constructing or mutating a native schema definition.
#[derive(Debug, Error, PartialEq, Eq)]
pub enum NativeSchemaError {
    /// Schema name failed validation.
    #[error("invalid schema name '{name}': {reason}")]
    InvalidName { name: String, reason: String },
    /// An attempt was made to add the same field twice.
    #[error("duplicate field '{field_name}' in schema '{schema}'")]
    DuplicateField { schema: String, field_name: String },
    /// A field definition violated validation invariants.
    #[error("invalid field definition for schema '{schema}'")]
    InvalidFieldDefinition {
        schema: String,
        #[source]
        source: FieldDefinitionError,
    },
    /// Key configuration referenced a field that does not exist.
    #[error("schema '{schema}' is missing required key field '{field_name}'")]
    MissingKeyField { schema: String, field_name: String },
    /// Key configuration failed structural validation.
    #[error("invalid key configuration for schema '{schema}'")]
    InvalidKeyConfig {
        schema: String,
        #[source]
        source: KeyConfigError,
    },
}

/// Validation errors returned when verifying a data record against a schema.
#[derive(Debug, Error, Clone, PartialEq, Eq)]
pub enum SchemaValidationError {
    /// Required field not present in the record payload.
    #[error("required field '{field_name}' is missing")]
    RequiredFieldMissing { field_name: String },
    /// Field present but with a value that does not match the declared type.
    #[error("field '{field_name}' has unexpected type (expected {expected:?}, got {actual:?})")]
    TypeMismatch {
        field_name: String,
        expected: Box<FieldType>,
        actual: Box<FieldType>,
    },
    /// Record contained a field that does not belong to the schema.
    #[error("record contains unknown field '{field_name}'")]
    UnexpectedField { field_name: String },
}

/// Errors emitted by the schema registry when schema lifecycle operations fail.
#[derive(Debug, Error)]
pub enum RegistryError {
    /// A schema with the same name already exists.
    #[error("schema '{name}' already exists in registry")]
    SchemaExists { name: String },
    /// Requested schema could not be found.
    #[error("schema '{name}' not found in registry")]
    SchemaNotFound { name: String },
    /// Provided schema failed validation prior to registration.
    #[error("invalid schema '{name}' provided to registry")]
    InvalidSchema {
        name: String,
        #[source]
        source: NativeSchemaError,
    },
}
