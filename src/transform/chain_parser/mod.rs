//! Chain syntax parser for iterator stack expressions
//!
//! Parses expressions like `blogpost.map().content.split_by_word().map()` and
//! tracks iterator depths and branch structures.

pub mod errors;
pub mod parser;
pub mod types;
pub mod validation;

#[cfg(test)]
mod tests;

pub use errors::*;
pub use parser::*;
pub use types::*;
