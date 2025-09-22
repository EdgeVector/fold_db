//! Native schema infrastructure built on top of strongly typed transform
//! primitives.
//!
//! The legacy schema system stores definitions as loosely typed JSON
//! structures that are interpreted at runtime. The native schema module
//! replaces that indirection with Rust data structures powered by the
//! native transform types introduced in NTS-1. Schemas constructed through
//! this module are validated eagerly and stored inside a concurrent
//! registry that guarantees type safety for downstream consumers.

pub mod errors;
pub mod registry;
pub mod schema;

pub use errors::{KeyConfigError, NativeSchemaError, RegistryError, SchemaValidationError};
pub use registry::NativeSchemaRegistry;
pub use schema::{KeyConfig, NativeSchema};
