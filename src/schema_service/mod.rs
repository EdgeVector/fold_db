//! Schema Service
//!
//! A standalone schema registry that provides schema discovery, deduplication,
//! semantic similarity matching, field canonicalization, and view management.

mod state_expansion;
mod state_fields;
mod state_matching;
pub mod state;
pub mod types;
