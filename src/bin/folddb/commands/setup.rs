use crate::error::CliError;
use dialoguer::{Input, Select};
use fold_db::fold_node::config::NodeConfig;
use fold_db::security::{Ed25519KeyPair, SecurityConfig};
use fold_db::storage::DatabaseConfig;
use serde::{Deserialize, Serialize};
use std::fs;
use std::path::{Path, PathBuf};

const DEFAULT_SCHEMA_SERVICE_URL: &str =
    "https://y0q3m6vk75.execute-api.us-west-2.amazonaws.com";

#[derive(Serialize, Deserialize)]
struct NodeIdentity {
    private_key: String,
    public_key: String,
}

/// Check whether a persisted node identity exists at `config/node_identity.json`.
pub fn identity_file_exists() -> bool {
    Path::new("config/node_identity.json").exists()
}

/// Run the interactive setup wizard.
///
/// Returns a fully populated `NodeConfig` with identity keys embedded.
pub fn run_setup_wizard() -> Result<NodeConfig, CliError> {
    eprintln!();
    eprintln!("Welcome to FoldDB setup!");
    eprintln!();

    // --- Backend selection ---
    let backends = &["Local (Sled - embedded, runs on this machine)", "Exemem Cloud (remote API-backed storage)"];
    let backend_idx = Select::new()
        .with_prompt("Storage backend")
        .items(backends)
        .default(0)
        .interact()
        .map_err(|e| CliError::new(format!("Selection cancelled: {}", e)))?;

    let database = match backend_idx {
        0 => {
            let default_path = dirs::home_dir()
                .map(|h| h.join(".folddb").join("data"))
                .unwrap_or_else(|| PathBuf::from("data"));

            let data_dir: String = Input::new()
                .with_prompt("Data directory")
                .default(default_path.to_string_lossy().to_string())
                .interact_text()
                .map_err(|e| CliError::new(format!("Input cancelled: {}", e)))?;

            DatabaseConfig::Local {
                path: PathBuf::from(data_dir),
            }
        }
        1 => {
            let api_url: String = Input::new()
                .with_prompt("Exemem API URL")
                .interact_text()
                .map_err(|e| CliError::new(format!("Input cancelled: {}", e)))?;

            let api_key: String = Input::new()
                .with_prompt("Exemem API key")
                .interact_text()
                .map_err(|e| CliError::new(format!("Input cancelled: {}", e)))?;

            DatabaseConfig::Exemem { api_url, api_key }
        }
        _ => unreachable!(),
    };

    // --- Schema service URL ---
    let schema_url: String = Input::new()
        .with_prompt("Schema service URL")
        .default(DEFAULT_SCHEMA_SERVICE_URL.to_string())
        .interact_text()
        .map_err(|e| CliError::new(format!("Input cancelled: {}", e)))?;

    // --- Generate identity ---
    eprintln!();
    eprint!("Generating node identity...");
    let keypair = Ed25519KeyPair::generate()
        .map_err(|e| CliError::new(format!("Failed to generate keypair: {}", e)))?;

    let identity = NodeIdentity {
        private_key: keypair.secret_key_base64(),
        public_key: keypair.public_key_base64(),
    };

    // Persist identity
    let config_dir = Path::new("config");
    if !config_dir.exists() {
        fs::create_dir_all(config_dir)
            .map_err(|e| CliError::new(format!("Failed to create config dir: {}", e)))?;
    }
    let identity_json = serde_json::to_string_pretty(&identity)
        .map_err(|e| CliError::new(format!("Failed to serialize identity: {}", e)))?;
    fs::write(config_dir.join("node_identity.json"), &identity_json)
        .map_err(|e| CliError::new(format!("Failed to write node_identity.json: {}", e)))?;
    eprintln!(" done.");

    // --- Build NodeConfig ---
    let config = NodeConfig {
        database,
        network_listen_address: "/ip4/0.0.0.0/tcp/0".to_string(),
        security_config: SecurityConfig::from_env(),
        schema_service_url: Some(schema_url),
        public_key: Some(identity.public_key),
        private_key: Some(identity.private_key),
    };

    // Persist config
    let config_json = serde_json::to_string_pretty(&config)
        .map_err(|e| CliError::new(format!("Failed to serialize config: {}", e)))?;
    fs::write(config_dir.join("node_config.json"), &config_json)
        .map_err(|e| CliError::new(format!("Failed to write node_config.json: {}", e)))?;

    eprintln!("Config saved to config/node_config.json");
    eprintln!();

    Ok(config)
}
