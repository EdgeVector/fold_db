//! Schema Service
//!
//! A standalone HTTP service that provides schema discovery and management.
//! The schema service loads schemas from a sled database on startup and
//! provides them via HTTP API to the main FoldDB node.

pub mod server;

pub use server::SchemaServiceServer;
