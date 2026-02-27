//! Chain syntax parser for iterator stack expressions
//!
//! Parses expressions like `blogpost.content.split_by_word()` and
//! tracks iterator depths and branch structures.

pub mod errors;
pub mod parser;
pub mod types;

#[cfg(test)]
mod tests;

pub use errors::*;
pub use parser::*;
pub use types::*;
