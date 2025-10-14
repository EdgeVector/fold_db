//! Schema Service
//! 
//! A standalone HTTP service that provides schema discovery and management.
//! The schema service reads schemas from the available_schemas directory and 
//! provides them via HTTP API to the main DataFold node.

pub mod server;

pub use server::SchemaServiceServer;

