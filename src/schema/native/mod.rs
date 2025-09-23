//! Native schema representation with strongly typed field definitions.
//!
//! This module provides the building blocks for the native schema registry
//! introduced in the NTS-2 workstream. Schemas are composed from
//! [`crate::transform::native::FieldDefinition`] instances and keep track of the
//! key configuration that governs how records are addressed inside the
//! datastore. The registry itself will be implemented by follow-up tasks.

mod schema;

pub use schema::{
    KeyConfig, NativeSchema, NativeSchemaBuilder, NativeSchemaError, SchemaValidationError,
};
