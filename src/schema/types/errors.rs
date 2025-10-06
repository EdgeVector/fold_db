use serde::Serialize;
use std::fmt;

#[derive(Debug, Clone, Serialize)]
pub enum SchemaError {
    NotFound(String),
    InvalidField(String),
    InvalidPermission(String),
    InvalidTransform(String),
    InvalidData(String),
    InvalidDSL(String),
    MappingError(String),
}

impl fmt::Display for SchemaError {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match self {
            Self::NotFound(msg) => write!(f, "Schema not found: {msg}"),
            Self::InvalidField(msg) => write!(f, "Invalid field: {msg}"),
            Self::InvalidPermission(msg) => write!(f, "Invalid permission: {msg}"),
            Self::InvalidTransform(msg) => write!(f, "Invalid transform: {msg}"),
            Self::InvalidData(msg) => write!(f, "Invalid data: {msg}"),
            Self::InvalidDSL(msg) => write!(f, "Invalid DSL: {msg}"),
            Self::MappingError(msg) => write!(f, "Mapping error: {msg}"),
        }
    }
}

impl std::error::Error for SchemaError {}

impl From<sled::Error> for SchemaError {
    fn from(error: sled::Error) -> Self {
        SchemaError::InvalidData(format!("Database error: {}", error))
    }
}

impl From<crate::fold_db_core::infrastructure::message_bus::MessageBusError> for SchemaError {
    fn from(error: crate::fold_db_core::infrastructure::message_bus::MessageBusError) -> Self {
        SchemaError::InvalidData(format!("Message bus error: {}", error))
    }
}
