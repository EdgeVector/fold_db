//! Core managers responsible for different aspects of data management
//!
//! This module contains all the core manager components that handle
//! specific aspects of the FoldDB system:
//! - Atom management (storage and retrieval)

pub mod atom;

pub use atom::AtomManager;
