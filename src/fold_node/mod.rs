//! A FoldDB node is a self-contained instance that can store data, process
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
pub mod llm_query;
pub mod node;
mod operation_processor;
pub mod response_types;
pub mod schema_client;
// Re-export the FoldNode struct for easier imports
pub use crate::server::{start_embedded_server, start_embedded_server_lazy, EmbeddedServerHandle};
pub use config::load_node_config;
pub use config::NodeConfig;
pub use node::FileIngestionRecord;
pub use node::FoldNode;
pub use node::MutationOutcome;
pub use operation_processor::OperationProcessor;
pub use schema_client::SchemaServiceClient;
