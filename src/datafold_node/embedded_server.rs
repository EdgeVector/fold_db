//! Embedded server functionality for running DataFold in desktop applications.
//!
//! This module provides an embeddable version of the DataFold HTTP server that can
//! be integrated into desktop applications (e.g., Tauri, Electron) without blocking
//! the main thread.

use super::{DataFoldHttpServer, DataFoldNode};
use crate::error::FoldDbResult;
use std::sync::Arc;
use tokio::task::JoinHandle;

/// Handle to a running embedded server.
///
/// This handle can be used to manage the lifecycle of an embedded DataFold server.
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
        self.task_handle.await.map_err(|e| {
            crate::error::FoldDbError::Other(format!("Server task panicked: {}", e))
        })?
    }

    /// Abort the server task.
    pub fn abort(&self) {
        self.task_handle.abort();
    }
}

/// Start an embedded DataFold HTTP server in a background task.
///
/// This function creates and starts a DataFold HTTP server without blocking the
/// current thread. It's designed for use in desktop applications where the server
/// needs to run alongside a UI.
///
/// # Arguments
///
/// * `node` - The DataFoldNode instance to use
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
/// use datafold::datafold_node::{DataFoldNode, start_embedded_server};
/// use datafold::datafold_node::config::NodeConfig;
///
/// #[tokio::main]
/// async fn main() -> Result<(), Box<dyn std::error::Error>> {
///     // Build a NodeConfig and create the node with the current API:
///     let config = NodeConfig::new(PathBuf::from("./data"));
///     let node = DataFoldNode::new(config).await?;
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
    node: DataFoldNode,
    port: u16,
) -> FoldDbResult<EmbeddedServerHandle> {
    let bind_address = format!("127.0.0.1:{}", port);
    let server = DataFoldHttpServer::new(node, &bind_address).await?;
    
    let address = bind_address.clone();
    let task_handle = tokio::spawn(async move {
        server.run().await
    });

    Ok(EmbeddedServerHandle {
        task_handle,
        bind_address: address,
    })
}

/// Start an embedded DataFold HTTP server with a shared node reference.
///
/// This variant is useful when you need to keep a reference to the node
/// for other purposes while also running the server.
///
/// # Arguments
///
/// * `node` - Arc-wrapped Mutex-wrapped DataFoldNode
/// * `port` - The port to bind to
///
/// # Returns
///
/// Returns an `EmbeddedServerHandle` that can be used to manage the server.
pub async fn start_embedded_server_shared(
    node: Arc<tokio::sync::Mutex<DataFoldNode>>,
    port: u16,
) -> FoldDbResult<EmbeddedServerHandle> {
    let node_instance = {
        let guard = node.lock().await;
        // Clone the node since DataFoldNode implements Clone
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
        let mut config = crate::datafold_node::config::NodeConfig::new(temp_dir.path().to_path_buf());
        config.schema_service_url = Some("mock://test".to_string());
        
        // Create the node
        let node = DataFoldNode::new(config).await.unwrap();
        
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

