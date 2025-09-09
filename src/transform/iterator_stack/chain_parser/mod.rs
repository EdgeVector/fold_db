//! Chain syntax parser for iterator stack expressions
//!
//! Parses expressions like `blogpost.map().content.split_by_word().map()` and
//! tracks iterator depths and branch structures.

pub mod types;
pub mod parser;
pub mod validation;

pub use types::*;
pub use parser::*;
