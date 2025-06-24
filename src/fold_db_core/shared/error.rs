//! # FoldDbCore Error Handling
//!
//! Unified error handling for the fold_db_core module, providing structured error
//! information that preserves context while enabling better debugging.
//!
//! This error system is designed to:
//! - Provide specific error variants for different operation types
//! - Preserve error context and enable error chaining
//! - Support conversion from common underlying error types
//! - Enable consistent error handling across fold_db_core modules

use thiserror::Error;

/// Unified error type for fold_db_core operations.
///
/// This error type consolidates all possible errors that can occur within the
/// fold_db_core module, providing structured error information with context
/// for better debugging and error handling.
#[derive(Error, Debug)]
pub enum FoldDbCoreError {
    // ========== Atom and Reference Management Errors ==========
    /// An atom with the specified UUID was not found
    #[error("Atom not found: {id}")]
    AtomNotFound { id: String },

    /// A molecule with the specified UUID was not found
    #[error("Molecule not found: {aref_uuid}")]
    MoleculeNotFound { aref_uuid: String },

    /// Molecule type mismatch (e.g., expected Collection but found Range)
    #[error("Molecule type mismatch for {aref_uuid}: expected {expected}, found {actual}")]
    MoleculeTypeMismatch {
        aref_uuid: String,
        expected: String,
        actual: String,
    },

    /// Ghost UUID detected - molecule_uuid points to non-existent Molecule
    #[error("Ghost UUID detected: field {field_name} has molecule_uuid {uuid} but no corresponding Molecule exists")]
    GhostUuidDetected { field_name: String, uuid: String },

    // ========== Field Operation Errors ==========
    /// A field with the specified name was not found in the schema
    #[error("Field not found: {field_name} in schema {schema_name}")]
    FieldNotFound {
        field_name: String,
        schema_name: String,
    },

    /// Invalid field operation (e.g., wrong operation for field type)
    #[error("Invalid field operation: {operation} on field {field_name} of type {field_type}")]
    InvalidFieldOperation {
        operation: String,
        field_name: String,
        field_type: String,
    },

    /// Field validation failed
    #[error("Field validation failed for {field_name}: {reason}")]
    FieldValidationFailed { field_name: String, reason: String },

    // ========== Schema Operation Errors ==========
    /// A schema with the specified name was not found
    #[error("Schema not found: {schema_name}")]
    SchemaNotFound { schema_name: String },

    /// Schema validation failed
    #[error("Schema validation failed for {schema_name}: {reason}")]
    SchemaValidationFailed {
        schema_name: String,
        reason: String,
    },

    /// Range schema specific error
    #[error("Range schema error for {schema_name}: {reason}")]
    RangeSchemaError {
        schema_name: String,
        reason: String,
    },

    // ========== Database Operation Errors ==========
    /// Database operation failed
    #[error("Database operation failed: {operation} - {reason}")]
    DatabaseError { operation: String, reason: String },

    /// Serialization error
    #[error("Serialization error: {context} - {reason}")]
    SerializationError { context: String, reason: String },

    /// Deserialization error
    #[error("Deserialization error: {context} - {reason}")]
    DeserializationError { context: String, reason: String },

    // ========== Concurrency Errors ==========
    /// Lock acquisition failed
    #[error("Lock error: failed to acquire {lock_type} lock for {resource}")]
    LockError { lock_type: String, resource: String },

    /// Concurrency conflict detected
    #[error("Concurrency error: {operation} failed due to {reason}")]
    ConcurrencyError { operation: String, reason: String },

    // ========== Transform Operation Errors ==========
    /// Transform with the specified name was not found
    #[error("Transform not found: {transform_name}")]
    TransformNotFound { transform_name: String },

    /// Transform execution failed
    #[error("Transform execution failed: {transform_name} - {reason}")]
    TransformExecutionFailed {
        transform_name: String,
        reason: String,
    },

    // ========== Permission Errors ==========
    /// Permission denied for the requested operation
    #[error("Permission denied: {operation} on {resource} for {subject}")]
    PermissionDenied {
        operation: String,
        resource: String,
        subject: String,
    },

    // ========== Generic Fallback ==========
    /// Operation failed with a generic error
    #[error("Operation failed: {operation} - {reason}")]
    OperationFailed { operation: String, reason: String },
}

impl FoldDbCoreError {
    // ========== Constructor Helper Functions ==========

    /// Create an AtomNotFound error
    pub fn atom_not_found(id: impl Into<String>) -> Self {
        Self::AtomNotFound { id: id.into() }
    }

    /// Create a MoleculeNotFound error
    pub fn molecule_not_found(aref_uuid: impl Into<String>) -> Self {
        Self::MoleculeNotFound {
            aref_uuid: aref_uuid.into(),
        }
    }

    /// Create a MoleculeTypeMismatch error
    pub fn molecule_type_mismatch(
        aref_uuid: impl Into<String>,
        expected: impl Into<String>,
        actual: impl Into<String>,
    ) -> Self {
        Self::MoleculeTypeMismatch {
            aref_uuid: aref_uuid.into(),
            expected: expected.into(),
            actual: actual.into(),
        }
    }

    /// Create a GhostUuidDetected error
    pub fn ghost_uuid_detected(
        field_name: impl Into<String>,
        uuid: impl Into<String>,
    ) -> Self {
        Self::GhostUuidDetected {
            field_name: field_name.into(),
            uuid: uuid.into(),
        }
    }

    /// Create a FieldNotFound error
    pub fn field_not_found(
        field_name: impl Into<String>,
        schema_name: impl Into<String>,
    ) -> Self {
        Self::FieldNotFound {
            field_name: field_name.into(),
            schema_name: schema_name.into(),
        }
    }

    /// Create an InvalidFieldOperation error
    pub fn invalid_field_operation(
        operation: impl Into<String>,
        field_name: impl Into<String>,
        field_type: impl Into<String>,
    ) -> Self {
        Self::InvalidFieldOperation {
            operation: operation.into(),
            field_name: field_name.into(),
            field_type: field_type.into(),
        }
    }

    /// Create a FieldValidationFailed error
    pub fn field_validation_failed(
        field_name: impl Into<String>,
        reason: impl Into<String>,
    ) -> Self {
        Self::FieldValidationFailed {
            field_name: field_name.into(),
            reason: reason.into(),
        }
    }

    /// Create a SchemaNotFound error
    pub fn schema_not_found(schema_name: impl Into<String>) -> Self {
        Self::SchemaNotFound {
            schema_name: schema_name.into(),
        }
    }

    /// Create a SchemaValidationFailed error
    pub fn schema_validation_failed(
        schema_name: impl Into<String>,
        reason: impl Into<String>,
    ) -> Self {
        Self::SchemaValidationFailed {
            schema_name: schema_name.into(),
            reason: reason.into(),
        }
    }

    /// Create a RangeSchemaError
    pub fn range_schema_error(
        schema_name: impl Into<String>,
        reason: impl Into<String>,
    ) -> Self {
        Self::RangeSchemaError {
            schema_name: schema_name.into(),
            reason: reason.into(),
        }
    }

    /// Create a DatabaseError
    pub fn database_error(
        operation: impl Into<String>,
        reason: impl Into<String>,
    ) -> Self {
        Self::DatabaseError {
            operation: operation.into(),
            reason: reason.into(),
        }
    }

    /// Create a SerializationError
    pub fn serialization_error(
        context: impl Into<String>,
        reason: impl Into<String>,
    ) -> Self {
        Self::SerializationError {
            context: context.into(),
            reason: reason.into(),
        }
    }

    /// Create a DeserializationError
    pub fn deserialization_error(
        context: impl Into<String>,
        reason: impl Into<String>,
    ) -> Self {
        Self::DeserializationError {
            context: context.into(),
            reason: reason.into(),
        }
    }

    /// Create a LockError
    pub fn lock_error(lock_type: impl Into<String>, resource: impl Into<String>) -> Self {
        Self::LockError {
            lock_type: lock_type.into(),
            resource: resource.into(),
        }
    }

    /// Create a ConcurrencyError
    pub fn concurrency_error(
        operation: impl Into<String>,
        reason: impl Into<String>,
    ) -> Self {
        Self::ConcurrencyError {
            operation: operation.into(),
            reason: reason.into(),
        }
    }

    /// Create a TransformNotFound error
    pub fn transform_not_found(transform_name: impl Into<String>) -> Self {
        Self::TransformNotFound {
            transform_name: transform_name.into(),
        }
    }

    /// Create a TransformExecutionFailed error
    pub fn transform_execution_failed(
        transform_name: impl Into<String>,
        reason: impl Into<String>,
    ) -> Self {
        Self::TransformExecutionFailed {
            transform_name: transform_name.into(),
            reason: reason.into(),
        }
    }

    /// Create a PermissionDenied error
    pub fn permission_denied(
        operation: impl Into<String>,
        resource: impl Into<String>,
        subject: impl Into<String>,
    ) -> Self {
        Self::PermissionDenied {
            operation: operation.into(),
            resource: resource.into(),
            subject: subject.into(),
        }
    }

    /// Create an OperationFailed error
    pub fn operation_failed(
        operation: impl Into<String>,
        reason: impl Into<String>,
    ) -> Self {
        Self::OperationFailed {
            operation: operation.into(),
            reason: reason.into(),
        }
    }
}

// ========== Conversion Traits ==========

/// Convert from sled::Error to FoldDbCoreError
impl From<sled::Error> for FoldDbCoreError {
    fn from(error: sled::Error) -> Self {
        Self::DatabaseError {
            operation: "sled_operation".to_string(),
            reason: error.to_string(),
        }
    }
}

/// Convert from serde_json::Error to FoldDbCoreError
impl From<serde_json::Error> for FoldDbCoreError {
    fn from(error: serde_json::Error) -> Self {
        if error.is_syntax() || error.is_data() {
            Self::DeserializationError {
                context: "json_deserialization".to_string(),
                reason: error.to_string(),
            }
        } else {
            Self::SerializationError {
                context: "json_serialization".to_string(),
                reason: error.to_string(),
            }
        }
    }
}

/// Convert from SchemaError to FoldDbCoreError
impl From<crate::schema::types::SchemaError> for FoldDbCoreError {
    fn from(error: crate::schema::types::SchemaError) -> Self {
        use crate::schema::types::SchemaError;
        match error {
            SchemaError::NotFound(msg) => Self::SchemaNotFound { schema_name: msg },
            SchemaError::InvalidField(msg) => Self::FieldValidationFailed {
                field_name: "unknown".to_string(),
                reason: msg,
            },
            SchemaError::InvalidPermission(msg) => Self::PermissionDenied {
                operation: "schema_access".to_string(),
                resource: "schema".to_string(),
                subject: msg,
            },
            SchemaError::InvalidTransform(msg) => Self::TransformExecutionFailed {
                transform_name: "unknown".to_string(),
                reason: msg,
            },
            SchemaError::InvalidData(msg) => Self::SchemaValidationFailed {
                schema_name: "unknown".to_string(),
                reason: msg,
            },
            SchemaError::InvalidDSL(msg) => Self::TransformExecutionFailed {
                transform_name: "dsl_transform".to_string(),
                reason: msg,
            },
            SchemaError::MappingError(msg) => Self::OperationFailed {
                operation: "schema_mapping".to_string(),
                reason: msg,
            },
        }
    }
}

/// Convert from Box<dyn std::error::Error> to FoldDbCoreError
impl From<Box<dyn std::error::Error>> for FoldDbCoreError {
    fn from(error: Box<dyn std::error::Error>) -> Self {
        Self::OperationFailed {
            operation: "generic_operation".to_string(),
            reason: error.to_string(),
        }
    }
}

/// Result type alias for fold_db_core operations
pub type FoldDbCoreResult<T> = Result<T, FoldDbCoreError>;

// ========== Common Error Pattern Helpers ==========

impl FoldDbCoreError {
    /// Helper for common lock timeout scenarios
    pub fn mutex_lock_timeout(resource: impl Into<String>) -> Self {
        Self::lock_error("mutex", resource)
    }

    /// Helper for common rwlock timeout scenarios
    pub fn rwlock_timeout(resource: impl Into<String>) -> Self {
        Self::lock_error("rwlock", resource)
    }

    /// Helper for field type validation errors
    pub fn field_type_validation_error(
        field_name: impl Into<String>,
        expected_type: impl Into<String>,
        actual_type: impl Into<String>,
    ) -> Self {
        Self::field_validation_failed(
            field_name,
            format!(
                "Expected field type {}, found {}",
                expected_type.into(),
                actual_type.into()
            ),
        )
    }

    /// Helper for range schema validation errors
    pub fn range_key_validation_error(
        schema_name: impl Into<String>,
        field_name: impl Into<String>,
        issue: impl Into<String>,
    ) -> Self {
        Self::range_schema_error(
            schema_name,
            format!("Range key validation failed for field {}: {}", 
                field_name.into(), issue.into()),
        )
    }

    /// Helper for atom ref creation failures
    pub fn molecule_creation_failed(
        field_name: impl Into<String>,
        reason: impl Into<String>,
    ) -> Self {
        Self::invalid_field_operation(
            "create_molecule".to_string(),
            field_name,
            format!("Molecule creation failed: {}", reason.into()),
        )
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_error_creation_helpers() {
        let error = FoldDbCoreError::atom_not_found("test-uuid");
        assert!(error.to_string().contains("test-uuid"));

        let error = FoldDbCoreError::field_not_found("test_field", "test_schema");
        assert!(error.to_string().contains("test_field"));
        assert!(error.to_string().contains("test_schema"));

        let error = FoldDbCoreError::mutex_lock_timeout("test_resource");
        assert!(error.to_string().contains("mutex"));
        assert!(error.to_string().contains("test_resource"));
    }

    #[test]
    fn test_error_conversions() {
        // Test sled::Error conversion
        let sled_error = sled::Error::Unsupported("test error".to_string());
        let core_error: FoldDbCoreError = sled_error.into();
        match core_error {
            FoldDbCoreError::DatabaseError { operation, reason } => {
                assert_eq!(operation, "sled_operation");
                assert!(reason.contains("test error"));
            }
            _ => panic!("Expected DatabaseError"),
        }

        // Test serde_json::Error conversion
        let json_str = r#"{"invalid": json}"#;
        let json_error = serde_json::from_str::<serde_json::Value>(json_str).unwrap_err();
        let core_error: FoldDbCoreError = json_error.into();
        match core_error {
            FoldDbCoreError::DeserializationError { context, .. } => {
                assert_eq!(context, "json_deserialization");
            }
            _ => panic!("Expected DeserializationError"),
        }
    }
}