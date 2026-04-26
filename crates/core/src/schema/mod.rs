//! # Schema System
//!
//! The schema module defines the structure and behavior of data in the FoldDB system.
//! Schemas define fields, their types, permissions, and payment requirements.
//!
//! ## Components
//!
//! * `core` - Core schema functionality including loading, validation, and field mapping
//! * `types` - Schema-related data structures and type definitions
//!
//! ## Architecture
//!
//! Schemas in FoldDB define the structure of data and the operations that can be
//! performed on it. Each schema has a name and a set of fields, each with its own
//! type, permissions, and payment requirements.
//!
//! The schema system supports field mapping between schemas, allowing fields from
//! one schema to reference fields in another. This creates a graph-like structure
//! of related data across schemas.
//!
//! Schemas are loaded from JSON definitions, validated, and then used to process
//! queries and mutations against the database.

// Internal modules
pub mod core;
pub mod field_mapper;
pub mod interpreter;
pub mod persistence;
pub mod schema_types;
pub mod types;

// Public re-exports
pub use core::SchemaCore;
pub use field_mapper::FieldMapperService;
pub use interpreter::SchemaInterpreter;
pub use schema_types::{SchemaState, SchemaWithState};
pub use types::{Schema, SchemaError};

/// Public prelude module containing types needed by tests and external code
pub mod prelude {
    pub use super::SchemaCore;
}
