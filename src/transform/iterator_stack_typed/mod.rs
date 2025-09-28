//! Parallel typed iterator engine (no JSON structures internally)
//!
//! This module provides a side-by-side implementation of the iterator engine
//! that operates on typed inputs without using serde_json::Value for internal
//! representation. It can be wired in later to replace the legacy engine.

pub mod types;
pub mod engine;
pub mod adapter;

#[cfg(test)]
mod tests;


