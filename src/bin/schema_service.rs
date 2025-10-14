use clap::Parser;
use datafold::{
    constants::DEFAULT_SCHEMA_SERVICE_PORT,
    schema_service::SchemaServiceServer,
};

/// Command line options for the schema service binary.
#[derive(Parser, Debug)]
#[command(author, version, about)]
struct Cli {
    /// Port for the schema service
    #[arg(long, default_value_t = DEFAULT_SCHEMA_SERVICE_PORT)]
    port: u16,
    
    /// Directory containing schema JSON files
    #[arg(long, default_value = "available_schemas")]
    schemas_dir: String,
}

/// Main entry point for the Schema Service.
///
/// This service provides HTTP endpoints for schema discovery and retrieval.
/// It reads schemas from a configured directory and serves them via REST API.
///
/// # Command-Line Arguments
///
/// * `--port <PORT>` - Port for the schema service (default: 9002)
/// * `--schemas-dir <DIR>` - Directory containing schema JSON files (default: available_schemas)
///
/// # Returns
///
/// A `Result` indicating success or failure.
///
/// # Errors
///
/// Returns an error if:
/// * The schema directory cannot be read
/// * The HTTP server cannot be started
#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    datafold::web_logger::init().ok();
    
    // Parse command-line arguments
    let Cli { port, schemas_dir } = Cli::parse();
    
    // Create and run the schema service
    let bind_address = format!("127.0.0.1:{}", port);
    let server = SchemaServiceServer::new(schemas_dir, &bind_address)?;
    
    server.run().await.map_err(|e| Box::new(e) as Box<dyn std::error::Error>)
}

#[cfg(test)]
mod tests {
    use super::Cli;
    use clap::Parser;
    use datafold::constants::DEFAULT_SCHEMA_SERVICE_PORT;
    
    #[test]
    fn defaults() {
        let cli = Cli::parse_from(["test"]);
        assert_eq!(cli.port, DEFAULT_SCHEMA_SERVICE_PORT);
        assert_eq!(cli.schemas_dir, "available_schemas");
    }
    
    #[test]
    fn custom_args() {
        let cli = Cli::parse_from(["test", "--port", "8000", "--schemas-dir", "my_schemas"]);
        assert_eq!(cli.port, 8000);
        assert_eq!(cli.schemas_dir, "my_schemas");
    }
}

