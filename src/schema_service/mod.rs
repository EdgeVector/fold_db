//! Schema Service
//!
//! A standalone HTTP service that provides schema discovery and management.
//! The schema service loads schemas from a sled database on startup and
//! provides them via HTTP API to the main FoldDB node.

mod routes;
pub mod server;
pub mod state;
pub mod types;

pub use server::SchemaServiceServer;
