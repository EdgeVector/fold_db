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
    
    /// Path to the sled database for storing schemas
    #[arg(long, default_value = "schema_registry")]
    db_path: String,
}

/// Main entry point for the Schema Service.
///
/// This service provides HTTP endpoints for schema discovery and retrieval.
/// It stores schemas in a sled database and serves them via REST API.
///
/// # Command-Line Arguments
///
/// * `--port <PORT>` - Port for the schema service (default: 9002)
/// * `--db-path <PATH>` - Path to the sled database for storing schemas (default: schema_registry)
///
/// # Returns
///
/// A `Result` indicating success or failure.
///
/// # Errors
///
/// Returns an error if:
/// * The database cannot be opened
/// * The HTTP server cannot be started
#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    datafold::web_logger::init().ok();
    
    // Parse command-line arguments
    let Cli { port, db_path } = Cli::parse();
    
    // Create and run the schema service
    let bind_address = format!("127.0.0.1:{}", port);
    let server = SchemaServiceServer::new(db_path, &bind_address)?;
    
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
        assert_eq!(cli.db_path, "schema_registry");
    }
    
    #[test]
    fn custom_args() {
        let cli = Cli::parse_from(["test", "--port", "8000", "--db-path", "my_schema_db"]);
        assert_eq!(cli.port, 8000);
        assert_eq!(cli.db_path, "my_schema_db");
    }
}

