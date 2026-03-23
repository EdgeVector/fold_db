//! Schema Service
//!
//! A standalone schema registry that provides schema discovery, deduplication,
//! semantic similarity matching, field canonicalization, view management,
//! and a Global Transform Registry.

pub mod classify;
mod state_expansion;
mod state_fields;
mod state_matching;
mod state_transforms;
pub mod state;
pub mod transform_resolver;
pub mod types;
