use clap::Parser;
use fold_db::{constants::DEFAULT_SCHEMA_SERVICE_PORT, schema_service::SchemaServiceServer};

#[cfg(feature = "aws-backend")]
use fold_db::storage::CloudConfig;

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
///
/// # Storage Modes
///
/// The service supports two storage modes:
/// 1. **Local Sled Storage (Default)**: Uses local sled database
/// 2. **DynamoDB Storage (Serverless)**: Uses DynamoDB with no locking needed!
///
/// # Command-Line Arguments
///
/// * `--port <PORT>` - Port for the schema service (default: 9002)
/// * `--db-path <PATH>` - Path to the sled database for local storage (default: schema_registry)
///
/// # Environment Variables (DynamoDB Mode)
///
/// To enable DynamoDB storage, set the following environment variables:
/// * `FOLD_DYNAMODB_TABLE` - DynamoDB table name (required for DynamoDB mode)
/// * `FOLD_DYNAMODB_REGION` - AWS region (required for DynamoDB mode)
///
/// If DynamoDB environment variables are set, DynamoDB storage will be used automatically.
/// **No distributed locking needed** - topology hashes ensure idempotent writes!
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
/// * DynamoDB configuration is invalid (when using DynamoDB mode)
#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    fold_db::logging::LoggingSystem::init_default().await.ok();

    // Parse command-line arguments
    let Cli { port, db_path } = Cli::parse();

    let bind_address = format!("127.0.0.1:{}", port);

    // Check if DynamoDB configuration is available from environment
    #[cfg(feature = "aws-backend")]
    let server = if let Ok(dynamodb_config) = CloudConfig::from_env() {
        println!("🚀 Schema service starting with DynamoDB storage");
        println!(
            "   Tables: {} (main), {} (schemas), etc.",
            dynamodb_config.tables.main, dynamodb_config.tables.schemas
        );
        println!("   Region: {}", dynamodb_config.region);
        println!("   ✨ No locking needed - topology hashes ensure idempotent writes!");

        SchemaServiceServer::new_with_cloud(dynamodb_config, &bind_address).await?
    } else {
        println!("🚀 Schema service starting with local sled storage");
        println!("   Database path: {}", db_path);

        SchemaServiceServer::new(db_path, &bind_address)?
    };

    #[cfg(not(feature = "aws-backend"))]
    let server = {
        println!("🚀 Schema service starting with local sled storage");
        println!("   Database path: {}", db_path);

        SchemaServiceServer::new(db_path, &bind_address)?
    };

    println!("✅ Schema service listening on {}", bind_address);

    server
        .run()
        .await
        .map_err(|e| Box::new(e) as Box<dyn std::error::Error>)
}

#[cfg(test)]
mod tests {
    use super::Cli;
    use clap::Parser;
    use fold_db::constants::DEFAULT_SCHEMA_SERVICE_PORT;

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
