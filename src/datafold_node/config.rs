use crate::log_feature;
use crate::logging::features::LogFeature;
use crate::security::SecurityConfig;

use serde::{Deserialize, Serialize};
use std::path::PathBuf;

pub use crate::storage::config::DatabaseConfig;

/// Configuration for a DataFoldNode instance.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NodeConfig {
    /// Database storage configuration
    #[serde(default)]
    pub database: DatabaseConfig,

    // storage_path and its attributes have been removed
    /// Default trust distance for queries when not explicitly specified
    /// Must be greater than 0
    pub default_trust_distance: u32,
    /// Network listening address
    #[serde(default = "default_network_listen_address")]
    pub network_listen_address: String,
    /// Security configuration
    #[serde(default)]
    pub security_config: SecurityConfig,
    /// URL of the schema service (optional, if not provided will load from local directories)
    #[serde(default)]
    pub schema_service_url: Option<String>,
    /// Explicitly provided node public key (Base64)
    #[serde(default)]
    pub public_key: Option<String>,
    /// Explicitly provided node private key (Base64)
    #[serde(default)]
    pub private_key: Option<String>,
}

fn default_network_listen_address() -> String {
    "/ip4/0.0.0.0/tcp/0".to_string()
}

impl Default for NodeConfig {
    fn default() -> Self {
        Self {
            database: DatabaseConfig::default(),
            default_trust_distance: 1,
            network_listen_address: default_network_listen_address(),
            security_config: SecurityConfig::from_env(),
            schema_service_url: None,
            public_key: None,
            private_key: None,
        }
    }
}

impl NodeConfig {
    /// Create a new node configuration with the specified storage path
    pub fn new(storage_path: PathBuf) -> Self {
        Self {
            database: DatabaseConfig::Local { path: storage_path },
            default_trust_distance: 1,
            network_listen_address: default_network_listen_address(),
            security_config: SecurityConfig::from_env(),
            schema_service_url: None,
            public_key: None,
            private_key: None,
        }
    }

    /// Get the effective storage path (from database config)
    pub fn get_storage_path(&self) -> PathBuf {
        match &self.database {
            DatabaseConfig::Local { path } => path.clone(),
            #[cfg(feature = "aws-backend")]
            DatabaseConfig::DynamoDb(_) => PathBuf::from("data"), // Default callback for DynamoDB if local path needed
        }
    }

    /// Set the network listening address
    pub fn with_network_listen_address(mut self, address: &str) -> Self {
        self.network_listen_address = address.to_string();
        self
    }

    /// Set the schema service URL
    pub fn with_schema_service_url(mut self, url: &str) -> Self {
        self.schema_service_url = Some(url.to_string());
        self
    }

    /// Set the node identity keys
    pub fn with_identity(mut self, public_key: &str, private_key: &str) -> Self {
        self.public_key = Some(public_key.to_string());
        self.private_key = Some(private_key.to_string());
        self
    }
}

/// Load a node configuration from the given path or from the `NODE_CONFIG`
/// environment variable.
///
/// If the file does not exist, a default [`NodeConfig`] is returned. When a
/// `port` is provided in this case, the returned config will have its
/// `network_listen_address` set to `"/ip4/0.0.0.0/tcp/<port>"`.
pub fn load_node_config(
    path: Option<&str>,
    port: Option<u16>,
) -> Result<NodeConfig, std::io::Error> {
    use std::fs;

    let config_path = path
        .map(|p| p.to_string())
        .or_else(|| std::env::var("NODE_CONFIG").ok())
        .unwrap_or_else(|| "config/node_config.json".to_string());

    if let Ok(config_str) = fs::read_to_string(&config_path) {
        match serde_json::from_str::<NodeConfig>(&config_str) {
            Ok(cfg) => Ok(cfg),
            Err(e) => {
                log_feature!(
                    LogFeature::HttpServer,
                    error,
                    "Failed to parse node configuration: {}",
                    e
                );
                Err(std::io::Error::new(std::io::ErrorKind::InvalidData, e))
            }
        }
    } else {
        let mut config = NodeConfig::default();

        if let Some(p) = port {
            config.network_listen_address = format!("/ip4/0.0.0.0/tcp/{}", p);
        }
        Ok(config)
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NodeInfo {
    pub id: String,
    pub trust_distance: u32,
}
