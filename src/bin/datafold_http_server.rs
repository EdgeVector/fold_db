use clap::Parser;
use datafold::{
    constants::DEFAULT_HTTP_PORT,
    datafold_node::{load_node_config, DataFoldHttpServer, DataFoldNode},
};

/// Command line options for the HTTP server binary.
#[derive(Parser, Debug)]
#[command(author, version, about)]
struct Cli {
    /// Port for the HTTP server
    #[arg(long, default_value_t = DEFAULT_HTTP_PORT)]
    port: u16,
    
    /// Schema service URL (if provided, node will fetch schemas from this service)
    #[arg(long)]
    schema_service_url: Option<String>,
}

/// Main entry point for the DataFold HTTP server.
///
/// This function starts a DataFold HTTP server that serves the UI and provides
/// REST API endpoints for schemas, queries, and mutations. It initializes the node,
/// loads configuration, and starts the HTTP server.
///
/// # Command-Line Arguments
///
/// * `--port <PORT>` - Port for the HTTP server (default: 9001)
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
/// * The HTTP server cannot be started
#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    datafold::web_logger::init().ok();

    // Parse command-line arguments using clap
    let Cli { port: http_port, schema_service_url } = Cli::parse();

    // Load node configuration
    let mut config = load_node_config(None, None)?;
    
    // Set schema service URL if provided
    if let Some(url) = schema_service_url {
        config.schema_service_url = Some(url);
    }

    // Create node
    let node = DataFoldNode::new(config)?;

    // Start the HTTP server
    let bind_address = format!("127.0.0.1:{}", http_port);
    let http_server = DataFoldHttpServer::new(node, &bind_address).await?;

    http_server.run().await.map_err(|e| Box::new(e) as Box<dyn std::error::Error>)
}

#[cfg(test)]
mod tests {
    use super::Cli;
    use clap::Parser;
    use datafold::constants::DEFAULT_HTTP_PORT;

    #[test]
    fn defaults() {
        let cli = Cli::parse_from(["test"]);
        assert_eq!(cli.port, DEFAULT_HTTP_PORT);
    }

    #[test]
    fn custom_port() {
        let cli = Cli::parse_from(["test", "--port", "8000"]);
        assert_eq!(cli.port, 8000);
    }
}
