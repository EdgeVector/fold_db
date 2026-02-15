//! Embedded server functionality for running FoldDB in desktop applications.
//!
//! This module provides an embeddable version of the FoldDB HTTP server that can
//! be integrated into desktop applications (e.g., Tauri, Electron) without blocking
//! the main thread.

use super::http_server::FoldHttpServer;
use super::node_manager::{NodeManager, NodeManagerConfig};
use crate::fold_node::FoldNode;
use crate::error::FoldDbResult;
use std::sync::Arc;
use tokio::task::JoinHandle;

/// Handle to a running embedded server.
///
/// This handle can be used to manage the lifecycle of an embedded FoldDB server.
pub struct EmbeddedServerHandle {
    /// The join handle for the server task
    task_handle: JoinHandle<FoldDbResult<()>>,
    /// The bind address
    bind_address: String,
}

impl EmbeddedServerHandle {
    /// Get the bind address of the server.
    pub fn bind_address(&self) -> &str {
        &self.bind_address
    }

    /// Check if the server is still running.
    pub fn is_running(&self) -> bool {
        !self.task_handle.is_finished()
    }

    /// Wait for the server to finish (blocks until server stops).
    pub async fn wait(self) -> FoldDbResult<()> {
        self.task_handle
            .await
            .map_err(|e| crate::error::FoldDbError::Other(format!("Server task panicked: {}", e)))?
    }

    /// Abort the server task.
    pub fn abort(&self) {
        self.task_handle.abort();
    }
}

/// Start an embedded FoldDB HTTP server in a background task.
///
/// This function creates and starts a FoldDB HTTP server without blocking the
/// current thread. It's designed for use in desktop applications where the server
/// needs to run alongside a UI.
///
/// # Arguments
///
/// * `node` - The FoldNode instance to use
/// * `port` - The port to bind to (e.g., 9001)
///
/// # Returns
///
/// Returns an `EmbeddedServerHandle` that can be used to manage the server.
///
/// # Example
///
/// ```no_run
/// use std::path::PathBuf;
/// use fold_db::fold_node::{FoldNode, start_embedded_server};
/// use fold_db::fold_node::config::NodeConfig;
///
/// #[tokio::main]
/// async fn main() -> Result<(), Box<dyn std::error::Error>> {
///     // Build a NodeConfig and create the node with the current API:
///     let config = NodeConfig::new(PathBuf::from("./data"));
///     let node = FoldNode::new(config).await?;
///     let handle = start_embedded_server(node, 9001).await?;
///
///     println!("Server running on {}", handle.bind_address());
///
///     // Do other work...
///
///     // When done:
///     handle.abort();
///     Ok(())
/// }
/// ```
pub async fn start_embedded_server(
    node: FoldNode,
    port: u16,
) -> FoldDbResult<EmbeddedServerHandle> {
    let bind_address = format!("127.0.0.1:{}", port);

    // Wrap the node in a NodeManager for compatibility with the HTTP server
    // For embedded single-user scenarios, we use a default user ID
    let node_manager_config = NodeManagerConfig {
        base_config: node.config.clone(),
    };
    let node_manager = NodeManager::new(node_manager_config);

    // Pre-populate the NodeManager with the provided node using a default embedded user
    node_manager.set_node("embedded_user", node).await;

    let server = FoldHttpServer::new(node_manager, &bind_address).await?;

    let address = bind_address.clone();
    let task_handle = tokio::spawn(async move { server.run().await });

    Ok(EmbeddedServerHandle {
        task_handle,
        bind_address: address,
    })
}

/// Start an embedded FoldDB HTTP server with a shared node reference.
///
/// This variant is useful when you need to keep a reference to the node
/// for other purposes while also running the server.
///
/// # Arguments
///
/// * `node` - Arc-wrapped RwLock-wrapped FoldNode
/// * `port` - The port to bind to
///
/// # Returns
///
/// Returns an `EmbeddedServerHandle` that can be used to manage the server.
pub async fn start_embedded_server_shared(
    node: Arc<tokio::sync::RwLock<FoldNode>>,
    port: u16,
) -> FoldDbResult<EmbeddedServerHandle> {
    let node_instance = {
        let guard = node.read().await;
        // Clone the node since FoldNode implements Clone
        guard.clone()
    };

    start_embedded_server(node_instance, port).await
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::tempdir;

    #[tokio::test]
    async fn test_embedded_server_starts() {
        // Create a temporary directory for the test database
        let temp_dir = tempdir().unwrap();

        // Create a config with a mock schema service URL
        let mut config =
            crate::fold_node::config::NodeConfig::new(temp_dir.path().to_path_buf());
        config.schema_service_url = Some("mock://test".to_string());

        let keypair = crate::security::Ed25519KeyPair::generate().unwrap();
        config.public_key = Some(keypair.public_key_base64());
        config.private_key = Some(keypair.secret_key_base64());

        // Create the node
        let node = FoldNode::new(config).await.unwrap();

        // Use a random high port to avoid conflicts
        use rand::Rng;
        let port = rand::thread_rng().gen_range(50000..60000);

        let handle = start_embedded_server(node, port).await.unwrap();

        // Verify the server is running
        assert!(handle.is_running());

        // Verify the bind address
        assert_eq!(handle.bind_address(), format!("127.0.0.1:{}", port));

        // Clean up
        handle.abort();
    }
}
