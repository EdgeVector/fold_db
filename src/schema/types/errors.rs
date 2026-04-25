use serde::Serialize;
use std::fmt;

#[derive(Debug, Clone, Serialize)]
pub enum SchemaError {
    NotFound(String),
    InvalidField(String),
    InvalidPermission(String),
    InvalidTransform(String),
    InvalidData(String),
    PermissionDenied(String),
    /// Transform exceeded its fuel budget (MDT-E). Carried separately from
    /// `InvalidTransform` so the view resolver can recognize a fuel trap
    /// and surface a `gas exceeded` cause string without parsing the
    /// inner message — `max_gas` must fail identically on every device,
    /// so mis-classifying a fuel trap as a generic execution error would
    /// hide the canonical failure shape from the audit log.
    TransformGasExceeded {
        input_size: u64,
    },
}

impl fmt::Display for SchemaError {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match self {
            Self::NotFound(msg) => write!(f, "Schema not found: {msg}"),
            Self::InvalidField(msg) => write!(f, "Invalid field: {msg}"),
            Self::InvalidPermission(msg) => write!(f, "Invalid permission: {msg}"),
            Self::InvalidTransform(msg) => write!(f, "Invalid transform: {msg}"),
            Self::InvalidData(msg) => write!(f, "Invalid data: {msg}"),
            Self::PermissionDenied(msg) => write!(f, "Permission denied: {msg}"),
            Self::TransformGasExceeded { input_size } => {
                write!(f, "Transform gas exceeded (input_size={input_size})")
            }
        }
    }
}

impl std::error::Error for SchemaError {}

impl From<sled::Error> for SchemaError {
    fn from(error: sled::Error) -> Self {
        SchemaError::InvalidData(format!("Database error: {}", error))
    }
}

impl From<crate::messaging::MessageBusError> for SchemaError {
    fn from(error: crate::messaging::MessageBusError) -> Self {
        SchemaError::InvalidData(format!("Message bus error: {}", error))
    }
}

impl From<crate::storage::StorageError> for SchemaError {
    fn from(error: crate::storage::StorageError) -> Self {
        SchemaError::InvalidData(error.to_string())
    }
}
