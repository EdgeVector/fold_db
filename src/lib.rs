//! # FoldDB Core Library
//!
//! This library implements the core database functionality of the Fold distributed data platform.
//! It provides schema-based data storage and query with distributed networking capabilities.
//!
//! ## Core Components
//!
//! * `atom` - Atomic data storage units that form the foundation of the database
//! * `db_operations` - Database operation handlers
//! * `error` - Error types and handling
//! * `fold_db_core` - Core database functionality
//! * `schema` - Schema definition, validation, and execution
//! * `security` - Cryptographic key management and signing
//! * `storage` - Storage backend abstraction (Sled, DynamoDB)
//!
//! ## Architecture
//!
//! Fold uses a distributed architecture where each node can store and process data
//! according to defined schemas. Nodes can communicate with each other to share and
//! replicate data, with permissions controlling access to different schemas and operations.
//!
//! The system is built around the concept of schemas that define the structure of data
//! and the operations that can be performed on it. Each schema has fields with associated
//! permissions and payment requirements.

pub mod atom;
pub mod constants;
pub mod crypto;
pub mod db_operations;
pub mod error;
pub mod fold_db_core;
pub mod logging;
pub mod progress;
pub mod schema;
pub mod security;
pub mod schema_service;
pub mod storage;
pub mod sync;
pub mod testing_utils;
pub mod view;

// Re-export main types for convenience
pub use error::{FoldDbError, FoldDbResult};
pub use fold_db_core::FoldDB;

// Re-export schema types
pub use schema::types::operations::MutationType;
pub use schema::SchemaState;

// Re-export storage types
pub use storage::DatabaseConfig;
