//! A DataFold node is a self-contained instance that can store data, process
//! queries and mutations, and communicate with other nodes. Each node has:
//!
//! 1. A local database for storing data
//! 2. A schema system for defining data structure
//! 3. A network layer for communicating with other nodes
//! 4. A TCP server for external client connections
//!
//! Nodes can operate independently or as part of a network, with trust
//! relationships defining how they share and access data.

pub mod config;
mod db;
pub mod error;
mod operation_processor;
pub mod http_server;
pub mod llm_query;
pub mod openapi;
pub mod log_routes;
pub mod node;
pub mod query_routes;
pub mod response_types;
pub mod schema_routes;
pub mod security_routes;
pub mod system_routes;
mod transform_queue;

// Re-export the DataFoldNode struct for easier imports
pub use config::load_node_config;
pub use config::NodeConfig;
pub use http_server::DataFoldHttpServer;
pub use node::DataFoldNode;
pub use operation_processor::OperationProcessor;
