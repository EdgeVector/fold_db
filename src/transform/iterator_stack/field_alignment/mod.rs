//! Field alignment validation for iterator stack model
//!
//! Ensures all fields are properly aligned relative to the deepest iterator
//! using 1:1, broadcast, and reduced alignment rules.

pub mod types;
pub mod validator;
pub mod optimization;

pub use types::*;
