//! Error types for schema indexing iterator stack operations

use std::fmt;

/// Errors that can occur during iterator stack operations
#[derive(Debug, Clone, PartialEq)]
pub enum IteratorStackError {
    /// Invalid chain syntax in expression
    InvalidChainSyntax { expression: String, reason: String },
    /// Incompatible fan-out depths between fields
    IncompatibleFanoutDepths {
        field1: String,
        depth1: usize,
        field2: String,
        depth2: usize,
    },
    /// Fields fan out on incomparable branches (cartesian product)
    CartesianFanoutError {
        field1: String,
        branch1: String,
        field2: String,
        branch2: String,
    },
    /// Field requires a reducer but none was provided
    ReducerRequired {
        field: String,
        current_depth: usize,
        max_depth: usize,
    },
    /// Invalid iterator chain structure
    InvalidIteratorChain { chain: String, reason: String },
    /// Ambiguous fan-out on different branches
    AmbiguousFanoutDifferentBranches { branches: Vec<String> },
    /// Iterator depth exceeds maximum allowed
    MaxDepthExceeded {
        current_depth: usize,
        max_depth: usize,
    },
    /// Field alignment validation failed
    FieldAlignmentError { field: String, reason: String },
    /// Runtime execution error
    ExecutionError { message: String },
}

impl fmt::Display for IteratorStackError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            IteratorStackError::InvalidChainSyntax { expression, reason } => {
                write!(
                    f,
                    "Invalid chain syntax in expression '{}': {}",
                    expression, reason
                )
            }
            IteratorStackError::IncompatibleFanoutDepths {
                field1,
                depth1,
                field2,
                depth2,
            } => {
                write!(
                    f,
                    "Incompatible fan-out depths: field '{}' at depth {} vs field '{}' at depth {}",
                    field1, depth1, field2, depth2
                )
            }
            IteratorStackError::CartesianFanoutError {
                field1,
                branch1,
                field2,
                branch2,
            } => {
                write!(
                    f,
                    "Cartesian fan-out error: field '{}' on branch '{}' vs field '{}' on branch '{}'",
                    field1, branch1, field2, branch2
                )
            }
            IteratorStackError::ReducerRequired {
                field,
                current_depth,
                max_depth,
            } => {
                write!(
                    f,
                    "Field '{}' at depth {} exceeds max depth {} and requires a reducer",
                    field, current_depth, max_depth
                )
            }
            IteratorStackError::InvalidIteratorChain { chain, reason } => {
                write!(f, "Invalid iterator chain '{}': {}", chain, reason)
            }
            IteratorStackError::AmbiguousFanoutDifferentBranches { branches } => {
                write!(
                    f,
                    "Ambiguous fan-out on different branches: {}",
                    branches.join(", ")
                )
            }
            IteratorStackError::MaxDepthExceeded {
                current_depth,
                max_depth,
            } => {
                write!(
                    f,
                    "Iterator depth {} exceeds maximum allowed depth {}",
                    current_depth, max_depth
                )
            }
            IteratorStackError::FieldAlignmentError { field, reason } => {
                write!(f, "Field alignment error for '{}': {}", field, reason)
            }
            IteratorStackError::ExecutionError { message } => {
                write!(f, "Execution error: {}", message)
            }
        }
    }
}

impl std::error::Error for IteratorStackError {}

/// Result type for iterator stack operations
pub type IteratorStackResult<T> = Result<T, IteratorStackError>;

/// Constants for iterator stack limits
pub mod constants {
    /// Maximum iterator depth allowed
    pub const MAX_ITERATOR_DEPTH: usize = 10;

    /// Maximum number of fields in a single schema
    pub const MAX_FIELDS_PER_SCHEMA: usize = 100;

    /// Maximum chain expression length
    pub const MAX_CHAIN_EXPRESSION_LENGTH: usize = 1000;
}
