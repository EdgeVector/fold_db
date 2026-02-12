use clap::Parser;
use fold_db::{
    constants::DEFAULT_HTTP_PORT,
    fold_node::load_node_config,
    server::{
        http_server::FoldHttpServer,
        node_manager::{NodeManager, NodeManagerConfig},
    },
};

/// Command line options for the HTTP server binary.
///
/// The HTTP server is now stateless - it accepts any user_hash from the
/// X-User-Hash header on each request, matching the Lambda implementation.
#[derive(Parser, Debug)]
#[command(
    author,
    version,
    about = "Stateless DataFold HTTP Server - user identity comes from X-User-Hash header on each request"
)]
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
/// This is a STATELESS HTTP server - user identity comes from the X-User-Hash
/// header on each incoming request, just like the Lambda implementation.
///
/// # Architecture
///
/// The server uses lazy per-user node initialization:
/// - On startup: Only configuration is loaded, no DynamoDB access
/// - On first request for a user: Node is created with user context
/// - Subsequent requests: Node is cached and reused
///
/// This aligns with Lambda's multi-tenant architecture and avoids issues
/// with DynamoDB access before user context is available.
///
/// # Command-Line Arguments
///
/// * `--port <PORT>` - Port for the HTTP server (default: 9001)
/// * `--schema-service-url <URL>` - URL of the schema service
///
/// # Environment Variables
///
/// * `NODE_CONFIG` - Path to the node configuration file (default: config/node_config.json)
///
/// # Client-Side Authentication
///
/// The UI generates a user_hash from the user identifier using:
///   user_hash = SHA256(user_id)[0:32] (first 32 hex characters)
///
/// This hash is sent with every request in the X-User-Hash header.
/// The server uses this to isolate user data in task-local storage.
///
/// # Returns
///
/// A `Result` indicating success or failure.
///
/// # Errors
///
/// Returns an error if:
/// * The configuration file cannot be read or parsed
/// * The HTTP server cannot be started
#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Parse command-line arguments using clap
    let Cli {
        port: http_port,
        schema_service_url,
    } = Cli::parse();

    println!("┌─────────────────────────────────────────────────────────┐");
    println!("│         DataFold HTTP Server (Stateless Mode)          │");
    println!("├─────────────────────────────────────────────────────────┤");
    println!("│  User identity comes from X-User-Hash header per-request  │");
    println!("│  Client generates: user_hash = SHA256(user_id)[0:32]   │");
    println!("│                                                         │");
    println!("│  Nodes are created lazily on first request per user.   │");
    println!("│  No DynamoDB access during startup.                    │");
    println!("└─────────────────────────────────────────────────────────┘");

    // Load node configuration
    let mut config = load_node_config(None, None)?;

    // Initialize logging system with environment configuration
    #[allow(unused_mut)]
    let mut log_config = fold_db::logging::config::LogConfig::from_env().unwrap_or_default();

    // If using DynamoDB backend, enable DynamoDB logging
    #[cfg(feature = "aws-backend")]
    if let fold_db::fold_node::config::DatabaseConfig::Cloud(ref mut db_config) =
        config.database
    {
        // Note: user_id is NOT set here - it comes from per-request headers
        // The middleware will inject it via task-local context

        // Only enable DynamoDB logging if not explicitly disabled
        if std::env::var("DATAFOLD_LOG_DYNAMODB_ENABLED").is_err() {
            log_config.outputs.dynamodb.enabled = true;
            log_config.outputs.dynamodb.table_name = db_config.tables.logs.clone();
            log_config.outputs.dynamodb.region = Some(db_config.region.clone());
        }
    }

    if let Err(e) = fold_db::logging::LoggingSystem::init_with_config(log_config).await {
        eprintln!("Failed to initialize logging system: {}", e);
    }

    // Set schema service URL if provided
    if let Some(url) = schema_service_url {
        println!("Schema service URL: {}", url);
        config.schema_service_url = Some(url);
    }

    // Create NodeManager instead of a single node
    // Nodes will be created lazily per-user on first request
    let node_manager_config = NodeManagerConfig {
        base_config: config,
    };
    let node_manager = NodeManager::new(node_manager_config);

    println!("\n✅ NodeManager initialized (nodes created lazily per-user)");

    // Start the HTTP server
    let bind_address = format!("0.0.0.0:{}", http_port);
    println!("Starting server on http://localhost:{}", http_port);
    println!("Waiting for authenticated requests with X-User-Hash header...\n");

    let http_server = FoldHttpServer::new(node_manager, &bind_address).await?;

    http_server
        .run()
        .await
        .map_err(|e| Box::new(e) as Box<dyn std::error::Error>)
}

#[cfg(test)]
mod tests {
    use super::Cli;
    use clap::Parser;
    use fold_db::constants::DEFAULT_HTTP_PORT;

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

    #[test]
    fn with_schema_service() {
        let cli = Cli::parse_from(["test", "--schema-service-url", "http://localhost:9002"]);
        assert_eq!(
            cli.schema_service_url,
            Some("http://localhost:9002".to_string())
        );
    }
}
