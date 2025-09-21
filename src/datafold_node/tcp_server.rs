use crate::datafold_node::DataFoldNode;
use crate::error::FoldDbResult;
use crate::log_feature;
use crate::logging::features::LogFeature;
use std::sync::Arc;
use tokio::net::TcpListener;
use tokio::sync::Mutex;

/// TCP server for the DataFold node.
///
/// TcpServer provides a TCP-based interface for external clients to interact
/// with a DataFold node. It handles connection management, request parsing,
/// and response formatting.
///
/// # Features
///
/// * Connection handling for multiple clients
/// * JSON-based request/response protocol
/// * Request forwarding to other nodes
/// * Error handling and recovery
///
/// # Examples
///
/// ```rust,no_run
/// use datafold::datafold_node::{DataFoldNode, NodeConfig, TcpServer};
/// use datafold::error::FoldDbResult;
/// use std::path::PathBuf;
///
/// #[tokio::main]
/// async fn main() -> FoldDbResult<()> {
///     // Create a node first
///     let config = NodeConfig {
///         storage_path: PathBuf::from("data"),
///         default_trust_distance: 1,
///         network_listen_address: "/ip4/127.0.0.1/tcp/0".to_string(),
///         security_config: datafold::security::SecurityConfig::default(),
///     };
///     let node = DataFoldNode::new(config)?;
///     
///     // Create a new TCP server on port 9000
///     let tcp_server = TcpServer::new(node, 9000).await?;
///
///     // Run the server (this will block until the server is stopped)
///     tcp_server.run().await?;
///     
///     Ok(())
/// }
/// ```
pub struct TcpServer {
    /// The DataFold node
    node: Arc<Mutex<DataFoldNode>>,
    /// The TCP listener
    listener: TcpListener,
}

impl TcpServer {
    /// Create a new TCP server.
    ///
    /// This method creates a new TCP server that listens on the specified port.
    /// It binds to 127.0.0.1 (localhost) and starts listening for incoming connections.
    /// The server uses the provided DataFoldNode to process client requests.
    ///
    /// # Arguments
    ///
    /// * `node` - The DataFoldNode instance to use for processing requests
    /// * `port` - The port number to listen on
    ///
    /// # Returns
    ///
    /// A `FoldDbResult` containing the new TcpServer instance.
    ///
    /// # Errors
    ///
    /// Returns a `FoldDbError` if:
    /// * There is an error binding to the specified port
    /// * The port is already in use
    /// * There is insufficient permission to bind to the port
    ///
    /// # Examples
    ///
    /// ```rust,no_run
    /// use datafold::datafold_node::{DataFoldNode, NodeConfig, TcpServer};
    /// use datafold::error::FoldDbResult;
    /// use std::path::PathBuf;
    ///
    /// #[tokio::main]
    /// async fn main() -> FoldDbResult<()> {
    ///     let config = NodeConfig {
    ///         storage_path: PathBuf::from("data"),
    ///         default_trust_distance: 1,
    ///         network_listen_address: "/ip4/127.0.0.1/tcp/0".to_string(),
    ///         security_config: datafold::security::SecurityConfig::default(),
    ///     };
    ///     let node = DataFoldNode::new(config)?;
    ///     let tcp_server = TcpServer::new(node, 9000).await?;
    ///     Ok(())
    /// }
    /// ```
    pub async fn new(node: DataFoldNode, port: u16) -> FoldDbResult<Self> {
        let addr = format!("127.0.0.1:{}", port);
        let listener = TcpListener::bind(&addr).await?;
        log_feature!(
            LogFeature::TcpServer,
            info,
            "TCP server listening on {}",
            addr
        );

        // Register this node's address with the network if available
        if let Ok(mut net) = node.get_network_mut().await {
            net.register_node_address(node.get_node_id(), addr.clone());
        }

        Ok(Self {
            node: Arc::new(Mutex::new(node)),
            listener,
        })
    }

    /// Run the TCP server.
    ///
    /// This method starts the TCP server and begins accepting client connections.
    /// It runs in an infinite loop, spawning a new task for each client connection.
    /// Each connection is handled independently, allowing multiple clients to
    /// connect simultaneously.
    ///
    /// # Returns
    ///
    /// A `FoldDbResult` indicating success or failure.
    ///
    /// # Errors
    ///
    /// Returns a `FoldDbError` if:
    /// * There is an error accepting a connection
    /// * There is an error creating a new task
    ///
    /// # Examples
    ///
    /// ```rust,no_run
    /// use datafold::datafold_node::{DataFoldNode, NodeConfig, TcpServer};
    /// use datafold::error::FoldDbResult;
    /// use std::path::PathBuf;
    ///
    /// #[tokio::main]
    /// async fn main() -> FoldDbResult<()> {
    ///     let config = NodeConfig {
    ///         storage_path: PathBuf::from("data"),
    ///         default_trust_distance: 1,
    ///         network_listen_address: "/ip4/127.0.0.1/tcp/0".to_string(),
    ///         security_config: datafold::security::SecurityConfig::default(),
    ///     };
    ///     let node = DataFoldNode::new(config)?;
    ///     let tcp_server = TcpServer::new(node, 9000).await?;
    ///     tcp_server.run().await?;
    ///     Ok(())
    /// }
    /// ```
    pub async fn run(&self) -> FoldDbResult<()> {
        log_feature!(LogFeature::TcpServer, info, "TCP server running...");

        loop {
            let (socket, _) = self.listener.accept().await?;
            log_feature!(LogFeature::TcpServer, info, "New client connected");

            // Clone the node reference for the new connection
            let node_clone = self.node.clone();

            // Spawn a new task to handle the connection
            tokio::spawn(async move {
                if let Err(e) = Self::handle_connection(socket, node_clone).await {
                    log_feature!(
                        LogFeature::TcpServer,
                        error,
                        "Error handling connection: {}",
                        e
                    );
                }
            });
        }
    }
}
