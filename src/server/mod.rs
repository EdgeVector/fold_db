pub mod embedded;
pub mod http_server;
pub mod middleware;
pub mod openapi;
pub mod routes;
pub mod static_assets;

pub use embedded::{start_embedded_server, EmbeddedServerHandle};
