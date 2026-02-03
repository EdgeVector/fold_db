pub mod embedded;
pub mod http_server;
pub mod middleware;
pub mod node_manager;
pub mod openapi;
pub mod routes;
pub mod static_assets;

pub use embedded::{start_embedded_server, EmbeddedServerHandle};
pub use node_manager::{NodeManager, NodeManagerConfig, NodeManagerError};
