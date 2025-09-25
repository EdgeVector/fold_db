use datafold::{
    constants::DEFAULT_P2P_PORT,
    datafold_node::{load_node_config, DataFoldNode},
};

use clap::Parser;
use log::info;

/// Command line options for the datafold node binary.
#[derive(Parser, Debug)]
#[command(author, version, about)]
struct Cli {
    /// Port for the P2P network
    #[arg(long, default_value_t = DEFAULT_P2P_PORT)]
    port: u16,

    /// Port for the TCP server
    #[arg(long, default_value_t = DEFAULT_P2P_PORT)]
    tcp_port: u16,
}

/// Main entry point for the DataFold Node server.
///
/// This function starts a DataFold node server that listens for incoming
/// connections on the specified ports. It initializes the node, loads
/// configuration, sets up the network layer, and starts the TCP server.
///
/// # Command-Line Arguments
///
/// * `--port <PORT>` - Port for the P2P network (default: 9000)
/// * `--tcp-port <PORT>` - Port for the TCP server (default: 9000)
///
/// # Environment Variables
///
/// * `NODE_CONFIG` - Path to the node configuration file (default: config/node_config.json)
///
/// # Returns
///
/// A `Result` indicating success or failure.
///
/// # Errors
///
/// Returns an error if:
/// * The configuration file cannot be read or parsed
/// * The node cannot be initialized
/// * The network layer cannot be initialized
/// * The TCP server cannot be started
#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    datafold::web_logger::init().ok();
    info!("Starting DataFold Node...");

    // Parse command-line arguments using clap
    let Cli { port, tcp_port: _ } = Cli::parse();

    // Load node configuration
    let config = load_node_config(None, Some(port))?;
    info!("Config loaded successfully");

    // Load or initialize node
    info!("Loading DataFold Node...");
    let node = DataFoldNode::load(config).await?;
    info!("Node loaded successfully");

    // Schemas are loaded from disk during node initialization
    info!("Previously loaded schemas are available");

    // Print node ID for connecting
    info!("Node ID: {}", node.get_node_id());
    info!("Node initialized successfully");

    // TODO: Network and TCP server functionality needs to be implemented
    info!("Network and TCP server functionality is not yet implemented");

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::Cli;
    use clap::Parser;
    use datafold::constants::DEFAULT_P2P_PORT;

    #[test]
    fn defaults() {
        let cli = Cli::parse_from(["test"]);
        assert_eq!(cli.port, DEFAULT_P2P_PORT);
        assert_eq!(cli.tcp_port, DEFAULT_P2P_PORT);
    }

    #[test]
    fn custom_values() {
        let cli = Cli::parse_from(["test", "--port", "8000", "--tcp-port", "8001"]);
        assert_eq!(cli.port, 8000);
        assert_eq!(cli.tcp_port, 8001);
    }
}
