//! API boundary modules bridging external JSON contracts with native data structures.

pub mod json_boundary;

pub use json_boundary::{JsonBoundaryError, JsonBoundaryLayer, JsonBoundarySchema};
