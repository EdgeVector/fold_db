//! # Schema Indexing Iterator Stack Model
//!
//! This module implements the iterator stack model for schema indexing that handles
//! fan-out using a stack of iterators (scopes). Each field expression is evaluated
//! within this stacked scope, with the field containing the deepest active iterator
//! determining the number of output rows.
//!
//! ## Components
//!
//! * `chain_parser` - Parse chain syntax expressions like `blogpost.map().content.split_by_word().map()`
//! * `iterator_stack` - Manage iterator depths and scope contexts
//! * `field_alignment` - Validate field alignment rules (1:1, broadcast, reduced)
//! * `execution_engine` - Runtime execution engine for broadcasting and emission
//! * `errors` - Error types for iterator stack operations

pub mod chain_parser;
pub mod stack;
pub mod field_alignment;
pub mod execution_engine;
pub mod errors;

pub use chain_parser::*;
pub use stack::*;
pub use field_alignment::*;
pub use execution_engine::*;
pub use errors::*;