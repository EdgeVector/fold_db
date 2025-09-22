//! Native transform data structures.
//!
//! This module hosts strongly typed building blocks that replace the
//! historical reliance on `serde_json::Value` within the transform
//! pipeline. Upcoming tasks extend these primitives into field
//! definitions and transform specifications.

pub mod field_definition;
pub mod types;

pub use field_definition::{FieldDefinition, FieldDefinitionError};
pub use types::{FieldType, FieldValue};
