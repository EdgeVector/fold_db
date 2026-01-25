use clap::Parser;
use datafold::{
    constants::DEFAULT_HTTP_PORT,
    datafold_node::{load_node_config, DataFoldNode},
    server::http_server::DataFoldHttpServer,
};
use sha2::{Digest, Sha256};
use std::io::{self, Write};

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

    /// User identifier to use for multi-tenancy (will be hashed)
    #[arg(long)]
    user_id: Option<String>,
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
/// * `--schema-service-url <URL>` - URL of the schema service
/// * `--user-id <ID>` - User identifier (will be hashed and used as user_id)
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
    // Parse command-line arguments using clap
    let Cli {
        port: http_port,
        schema_service_url,
        user_id,
    } = Cli::parse();

    // Handle user_id: get from args or prompt
    let user_identifier = match user_id {
        Some(id) => id,
        None => {
            print!("Enter user identifier: ");
            io::stdout().flush()?;
            let mut buffer = String::new();
            io::stdin().read_line(&mut buffer)?;
            buffer.trim().to_string()
        }
    };

    if user_identifier.is_empty() {
        return Err("User identifier cannot be empty".into());
    }

    // Generate hash from identifier (SHA-256, take first 32 chars of hex)
    // This matches the frontend implementation in authSlice.ts
    let mut hasher = Sha256::new();
    hasher.update(user_identifier.as_bytes());
    let result = hasher.finalize();
    let hash_hex = result
        .iter()
        .map(|b| format!("{:02x}", b))
        .collect::<String>();
    let user_hash = hash_hex[0..32].to_string();

    println!(
        "Using user hash: {} (from identifier: '{}')",
        user_hash, user_identifier
    );

    // Load node configuration first to determine backend
    let mut config = load_node_config(None, None)?;

    // Initialize logging system with environment configuration
    #[allow(unused_mut)]
    let mut log_config = datafold::logging::config::LogConfig::from_env().unwrap_or_default();

    // Deterministically generate keys from user_hash (which is 32 hex chars = 16 bytes? NO.
    // The previous code took hash_hex[0..32]. That is 32 hex chars, representing 16 bytes?
    // Wait. SHA256 is 32 bytes. Hex string is 64 chars.
    // hash_hex[0..32] is likely just the first 16 bytes of the hash represented as hex.
    // BUT we need 32 bytes for the secret key seed.
    // Let's use the FULL 32 bytes of the SHA256 hash (result) directly.
    let secret_seed = result.as_slice(); // SHA256 result is [u8; 32] GenericArray
    let keypair = datafold::security::Ed25519KeyPair::from_secret_key(secret_seed)
        .expect("Failed to generate keypair from seed");

    println!("Generated deterministic identity:");
    println!("  Public Key: {}", keypair.public_key_base64());
    println!("  User Hash:  {}", user_hash);

    // If using DynamoDB backend, automatically enable DynamoDB logging
    #[cfg(feature = "aws-backend")]
    if let datafold::datafold_node::config::DatabaseConfig::Cloud(ref mut db_config) =
        config.database
    {
        // Inject the generated user_hash into the DynamoDB config
        // This ensures the node uses our hash instead of generating one from the public key
        db_config.user_id = Some(user_hash.clone());

        // Only enable if not explicitly disabled via env vars
        if std::env::var("DATAFOLD_LOG_DYNAMODB_ENABLED").is_err() {
            log_config.outputs.dynamodb.enabled = true;
            log_config.outputs.dynamodb.table_name = db_config.tables.logs.clone();
            log_config.outputs.dynamodb.region = Some(db_config.region.clone());
        }
    }

    // Inject identity into config
    config.public_key = Some(keypair.public_key_base64());
    config.private_key = Some(keypair.secret_key_base64());

    // Also inject into log config if needed?
    // The previous code for DataFoldHttpServer::new extracts user_id from config.database for logs.
    // So modifying config.database above should be sufficient for DataFoldHttpServer logic.

    if let Err(e) = datafold::logging::LoggingSystem::init_with_config(log_config).await {
        eprintln!("Failed to initialize logging system: {}", e);
    }

    // Set schema service URL if provided
    if let Some(url) = schema_service_url {
        config.schema_service_url = Some(url);
    }

    // Create node (now async!)
    let node = DataFoldNode::new(config).await?;

    // Start the HTTP server
    let bind_address = format!("0.0.0.0:{}", http_port);
    let http_server = DataFoldHttpServer::new(node, &bind_address).await?;

    http_server
        .run()
        .await
        .map_err(|e| Box::new(e) as Box<dyn std::error::Error>)
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
        assert_eq!(cli.user_id, None);
    }

    #[test]
    fn custom_port() {
        let cli = Cli::parse_from(["test", "--port", "8000"]);
        assert_eq!(cli.port, 8000);
    }

    #[test]
    fn custom_user_id() {
        let cli = Cli::parse_from(["test", "--user-id", "alice"]);
        assert_eq!(cli.user_id, Some("alice".to_string()));
    }
}
