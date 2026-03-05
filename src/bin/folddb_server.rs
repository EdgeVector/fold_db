use clap::Parser;
use fold_db::{
    constants::{DEFAULT_HTTP_PORT, DEFAULT_SCHEMA_SERVICE_URL},
    fold_node::load_node_config,
    server::{
        http_server::FoldHttpServer,
        node_manager::{NodeManager, NodeManagerConfig},
    },
};
use std::path::PathBuf;

/// Command line options for the HTTP server binary.
///
/// The HTTP server is now stateless - it accepts any user_hash from the
/// X-User-Hash header on each request, matching the Lambda implementation.
#[derive(Parser, Debug)]
#[command(
    author,
    version,
    about = "FoldDB Server — run locally, open the UI at http://localhost:9001"
)]
struct Cli {
    /// Port for the HTTP server
    #[arg(long, default_value_t = DEFAULT_HTTP_PORT)]
    port: u16,

    /// Data directory (default: ~/.folddb/data)
    #[arg(long)]
    data_dir: Option<PathBuf>,

    /// Schema service URL (default: production schema service)
    #[arg(long)]
    schema_service_url: Option<String>,

    /// Run in demo mode with isolated data/config directories
    #[arg(long)]
    demo: bool,
}

/// Resolve the default data directory: ~/.folddb/data (or ~/.folddb/demo-data in demo mode)
fn default_data_dir(demo: bool) -> PathBuf {
    let subdir = if demo { "demo-data" } else { "data" };
    dirs::home_dir()
        .unwrap_or_else(|| PathBuf::from("."))
        .join(".folddb")
        .join(subdir)
}

/// Resolve the default config directory: ~/.folddb/config (or ~/.folddb/demo-config in demo mode)
fn default_config_dir(demo: bool) -> PathBuf {
    let subdir = if demo { "demo-config" } else { "config" };
    dirs::home_dir()
        .unwrap_or_else(|| PathBuf::from("."))
        .join(".folddb")
        .join(subdir)
}

/// Check if a user-provided or env-var config file exists.
fn config_file_exists() -> bool {
    let path = std::env::var("NODE_CONFIG")
        .unwrap_or_else(|_| "config/node_config.json".to_string());
    std::path::Path::new(&path).exists()
}

/// Main entry point for the FoldDB HTTP server.
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
/// # Command-Line Arguments
///
/// * `--port <PORT>` - Port for the HTTP server (default: 9001)
/// * `--data-dir <PATH>` - Data directory (default: ~/.folddb/data)
/// * `--schema-service-url <URL>` - URL of the schema service
///
/// # Environment Variables
///
/// * `NODE_CONFIG` - Path to the node configuration file (default: config/node_config.json)
#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let Cli {
        port: http_port,
        data_dir,
        schema_service_url,
        demo,
    } = Cli::parse();

    // Load node configuration
    let mut config = load_node_config(None, None)?;
    let has_config_file = config_file_exists();

    // When no config file exists, apply zero-config defaults so the binary
    // works out of the box after a fresh install.
    if !has_config_file {
        let data_path = data_dir.unwrap_or_else(|| default_data_dir(demo));
        let config_path = default_config_dir(demo);

        std::fs::create_dir_all(&data_path)?;
        std::fs::create_dir_all(&config_path)?;

        config.database = fold_db::storage::config::DatabaseConfig::Local {
            path: data_path.clone(),
        };
        config.schema_service_url =
            Some(schema_service_url.unwrap_or_else(|| DEFAULT_SCHEMA_SERVICE_URL.to_string()));

        // Let ingestion config saves go to ~/.folddb/config
        std::env::set_var("FOLD_CONFIG_DIR", &config_path);

        let label = if demo { "FoldDB Server [DEMO]" } else { "FoldDB Server" };
        println!("{}", label);
        println!("  Data:   {}", data_path.display());
        println!("  Config: {}", config_path.display());
        println!("  UI:     http://localhost:{}", http_port);
        println!();
    } else {
        // Config file exists — honour explicit CLI overrides only
        if let Some(dir) = data_dir {
            config.database = fold_db::storage::config::DatabaseConfig::Local { path: dir };
        }
        if let Some(url) = schema_service_url {
            config.schema_service_url = Some(url);
        }

        // Ensure FOLD_CONFIG_DIR is set so ingestion config can be saved
        let config_path = default_config_dir(demo);
        std::fs::create_dir_all(&config_path)?;
        std::env::set_var("FOLD_CONFIG_DIR", &config_path);

        println!("FoldDB Server (config file detected)");
        println!("  Data:   {}", config.get_storage_path().display());
        if let Some(ref url) = config.schema_service_url {
            println!("  Schema: {}", url);
        }
        println!("  UI:     http://localhost:{}", http_port);
        println!();
    }

    // Initialize logging system with environment configuration
    #[allow(unused_mut)]
    let mut log_config = fold_db::logging::config::LogConfig::from_env().unwrap_or_default();

    // If using DynamoDB backend, enable DynamoDB logging
    #[cfg(feature = "aws-backend")]
    if let fold_db::fold_node::config::DatabaseConfig::Cloud(ref mut db_config) =
        config.database
    {
        if std::env::var("FOLD_LOG_DYNAMODB_ENABLED").is_err() {
            log_config.outputs.dynamodb.enabled = true;
            log_config.outputs.dynamodb.table_name = db_config.tables.logs.clone();
            log_config.outputs.dynamodb.region = Some(db_config.region.clone());
        }
    }

    if let Err(e) = fold_db::logging::LoggingSystem::init_with_config(log_config).await {
        eprintln!("Failed to initialize logging system: {}", e);
    }

    // Create NodeManager — nodes are created lazily per-user on first request
    let node_manager_config = NodeManagerConfig {
        base_config: config,
    };
    let node_manager = NodeManager::new(node_manager_config);

    // Start the HTTP server
    let bind_address = format!("0.0.0.0:{}", http_port);
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
        assert!(cli.data_dir.is_none());
        assert!(cli.schema_service_url.is_none());
    }

    #[test]
    fn custom_port() {
        let cli = Cli::parse_from(["test", "--port", "8000"]);
        assert_eq!(cli.port, 8000);
    }

    #[test]
    fn with_data_dir() {
        let cli = Cli::parse_from(["test", "--data-dir", "/tmp/folddb"]);
        assert_eq!(
            cli.data_dir,
            Some(std::path::PathBuf::from("/tmp/folddb"))
        );
    }

    #[test]
    fn with_schema_service() {
        let cli = Cli::parse_from(["test", "--schema-service-url", "http://localhost:9002"]);
        assert_eq!(
            cli.schema_service_url,
            Some("http://localhost:9002".to_string())
        );
    }

    #[test]
    fn demo_flag() {
        let cli = Cli::parse_from(["test", "--demo"]);
        assert!(cli.demo);
    }

    #[test]
    fn demo_flag_default_false() {
        let cli = Cli::parse_from(["test"]);
        assert!(!cli.demo);
    }

    #[test]
    fn demo_data_dir() {
        let normal = super::default_data_dir(false);
        let demo = super::default_data_dir(true);
        assert!(normal.ends_with("data"));
        assert!(demo.ends_with("demo-data"));
    }

    #[test]
    fn demo_config_dir() {
        let normal = super::default_config_dir(false);
        let demo = super::default_config_dir(true);
        assert!(normal.ends_with("config"));
        assert!(demo.ends_with("demo-config"));
    }
}
