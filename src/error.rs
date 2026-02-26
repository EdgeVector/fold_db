use crate::schema::types::SchemaError;
use std::fmt;
use std::io;

/// Unified error type for the entire application.
///
/// This error type centralizes all possible errors that can occur in the application,
/// providing a consistent interface for error handling and propagation.
///
/// Each variant represents a specific category of errors, with associated context
/// to help with debugging and error reporting.
#[derive(Debug)]
pub enum FoldDbError {
    /// Errors related to schema operations
    Schema(SchemaError),

    /// Errors related to database operations
    Database(String),

    /// Errors related to permission checks
    Permission(String),

    /// Errors related to configuration
    Config(String),

    /// Errors related to IO operations
    Io(io::Error),

    /// Errors related to serialization/deserialization
    Serialization(String),

    /// Errors related to security operations
    SecurityError(String),

    /// Other errors that don't fit into the above categories
    Other(String),
}

impl fmt::Display for FoldDbError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Schema(err) => write!(f, "Schema error: {}", err),
            Self::Database(msg) => write!(f, "Database error: {}", msg),
            Self::Permission(msg) => write!(f, "Permission error: {}", msg),
            Self::Config(msg) => write!(f, "Configuration error: {}", msg),
            Self::Io(err) => write!(f, "IO error: {}", err),
            Self::Serialization(msg) => write!(f, "Serialization error: {}", msg),
            Self::SecurityError(msg) => write!(f, "Security error: {}", msg),
            Self::Other(msg) => write!(f, "Error: {}", msg),
        }
    }
}

impl std::error::Error for FoldDbError {}

/// Conversion from SchemaError to FoldDbError
impl From<SchemaError> for FoldDbError {
    fn from(error: SchemaError) -> Self {
        FoldDbError::Schema(error)
    }
}

/// Conversion from io::Error to FoldDbError
impl From<io::Error> for FoldDbError {
    fn from(error: io::Error) -> Self {
        FoldDbError::Io(error)
    }
}

/// Conversion from serde_json::Error to FoldDbError
impl From<serde_json::Error> for FoldDbError {
    fn from(error: serde_json::Error) -> Self {
        FoldDbError::Serialization(error.to_string())
    }
}

/// Conversion from sled::Error to FoldDbError
impl From<sled::Error> for FoldDbError {
    fn from(error: sled::Error) -> Self {
        FoldDbError::Database(error.to_string())
    }
}

/// Result type alias for operations that can result in a FoldDbError
pub type FoldDbResult<T> = Result<T, FoldDbError>;
