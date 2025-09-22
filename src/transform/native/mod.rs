//! Native transform data structures.
//!
//! This module hosts strongly typed building blocks that replace the
//! historical reliance on `serde_json::Value` within the transform
//! pipeline. Upcoming tasks extend these primitives into field
//! definitions and transform specifications.

pub mod types;

pub use types::{FieldType, FieldValue};
