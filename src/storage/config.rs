use serde::{Deserialize, Serialize};
use std::env;
use std::path::PathBuf;

/// Error type for configuration parsing
#[derive(Debug)]
pub enum ConfigError {
    MissingVariable(String),
    InvalidValue(String),
}

impl std::fmt::Display for ConfigError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            ConfigError::MissingVariable(var) => write!(f, "Missing environment variable: {}", var),
            ConfigError::InvalidValue(msg) => write!(f, "Invalid configuration value: {}", msg),
        }
    }
}

impl std::error::Error for ConfigError {}

/// Configuration for cloud sync (Exemem encrypted S3 backup)
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct CloudSyncConfig {
    /// Exemem API URL (sync routes at /api/sync/*)
    pub api_url: String,
    /// API key for authentication
    pub api_key: String,
    /// Session token for authenticated API access
    #[serde(default)]
    pub session_token: Option<String>,
    /// User hash derived from public key
    #[serde(default)]
    pub user_hash: Option<String>,
}

/// Storage configuration — always local Sled, optionally with cloud sync.
///
/// Uses a custom deserializer to support both the new format and the legacy
/// `{"type": "local", ...}` / `{"type": "exemem", ...}` JSON formats.
#[derive(Clone, Debug, Serialize)]
pub struct DatabaseConfig {
    /// Path to the local Sled database directory
    pub path: PathBuf,
    /// Optional cloud sync configuration (Exemem encrypted S3 backup)
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub cloud_sync: Option<CloudSyncConfig>,
}

impl Default for DatabaseConfig {
    fn default() -> Self {
        DatabaseConfig {
            path: PathBuf::from("data"),
            cloud_sync: None,
        }
    }
}

impl DatabaseConfig {
    /// Create a local-only config
    pub fn local(path: PathBuf) -> Self {
        DatabaseConfig {
            path,
            cloud_sync: None,
        }
    }

    /// Create a config with cloud sync enabled
    pub fn with_cloud_sync(path: PathBuf, cloud_sync: CloudSyncConfig) -> Self {
        DatabaseConfig {
            path,
            cloud_sync: Some(cloud_sync),
        }
    }

    /// Check if cloud sync is enabled
    pub fn has_cloud_sync(&self) -> bool {
        self.cloud_sync.is_some()
    }

    /// Creates DatabaseConfig from environment variables:
    /// - FOLD_STORAGE_PATH: path for local storage (default: "data")
    /// - FOLD_STORAGE_MODE: "local" (default) or "exemem"
    /// - For exemem mode: EXEMEM_API_URL, EXEMEM_API_KEY
    pub fn from_env() -> Result<Self, ConfigError> {
        let path = env::var("FOLD_STORAGE_PATH")
            .map(PathBuf::from)
            .unwrap_or_else(|_| PathBuf::from("data"));

        let mode = env::var("FOLD_STORAGE_MODE").unwrap_or_else(|_| "local".to_string());

        let cloud_sync = match mode.to_lowercase().as_str() {
            "local" => None,
            "exemem" => {
                let api_url = env::var("EXEMEM_API_URL")
                    .map_err(|_| ConfigError::MissingVariable("EXEMEM_API_URL".to_string()))?;
                let api_key = env::var("EXEMEM_API_KEY")
                    .map_err(|_| ConfigError::MissingVariable("EXEMEM_API_KEY".to_string()))?;
                Some(CloudSyncConfig {
                    api_url,
                    api_key,
                    session_token: env::var("EXEMEM_SESSION_TOKEN").ok(),
                    user_hash: env::var("EXEMEM_USER_HASH").ok(),
                })
            }
            _ => {
                return Err(ConfigError::InvalidValue(format!(
                    "Invalid FOLD_STORAGE_MODE: '{}'. Must be 'local' or 'exemem'",
                    mode
                )))
            }
        };

        Ok(DatabaseConfig { path, cloud_sync })
    }
}

/// Custom deserializer that handles both new format and legacy tagged enum format.
///
/// New format:
/// ```json
/// { "path": "/data", "cloud_sync": { "api_url": "...", "api_key": "..." } }
/// ```
///
/// Legacy formats (auto-migrated):
/// ```json
/// { "type": "local", "path": "/data" }
/// { "type": "exemem", "api_url": "...", "api_key": "..." }
/// ```
impl<'de> Deserialize<'de> for DatabaseConfig {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        let value = serde_json::Value::deserialize(deserializer)?;

        // Check if this is the legacy tagged format
        if let Some(type_tag) = value.get("type").and_then(|v| v.as_str()) {
            match type_tag {
                "local" => {
                    let path = value
                        .get("path")
                        .and_then(|v| v.as_str())
                        .map(PathBuf::from)
                        .unwrap_or_else(|| PathBuf::from("data"));
                    Ok(DatabaseConfig {
                        path,
                        cloud_sync: None,
                    })
                }
                "exemem" => {
                    let api_url = value
                        .get("api_url")
                        .and_then(|v| v.as_str())
                        .unwrap_or_default()
                        .to_string();
                    let api_key = value
                        .get("api_key")
                        .and_then(|v| v.as_str())
                        .unwrap_or_default()
                        .to_string();
                    let session_token = value
                        .get("session_token")
                        .and_then(|v| v.as_str())
                        .map(|s| s.to_string());
                    let user_hash = value
                        .get("user_hash")
                        .and_then(|v| v.as_str())
                        .map(|s| s.to_string());

                    let path = std::env::var("FOLD_STORAGE_PATH")
                        .map(PathBuf::from)
                        .unwrap_or_else(|_| PathBuf::from("data"));

                    Ok(DatabaseConfig {
                        path,
                        cloud_sync: Some(CloudSyncConfig {
                            api_url,
                            api_key,
                            session_token,
                            user_hash,
                        }),
                    })
                }
                other => Err(serde::de::Error::custom(format!(
                    "Unknown database type: '{}'. Supported: 'local', 'exemem'",
                    other
                ))),
            }
        } else {
            // New format: direct struct fields
            let path = value
                .get("path")
                .and_then(|v| v.as_str())
                .map(PathBuf::from)
                .unwrap_or_else(|| PathBuf::from("data"));

            let cloud_sync = value
                .get("cloud_sync")
                .and_then(|v| {
                    serde_json::from_value::<CloudSyncConfig>(v.clone()).ok()
                });

            Ok(DatabaseConfig { path, cloud_sync })
        }
    }
}

/// Upload storage configuration for uploaded files
#[derive(Debug, Clone)]
pub enum UploadStorageConfig {
    /// Local filesystem storage (default)
    Local { path: PathBuf },
}

impl Default for UploadStorageConfig {
    fn default() -> Self {
        UploadStorageConfig::Local {
            path: PathBuf::from("data/uploads"),
        }
    }
}

impl UploadStorageConfig {
    /// Creates UploadStorageConfig from environment variables:
    /// - FOLD_UPLOAD_STORAGE_MODE: "local" (defaults to "local")
    /// - FOLD_UPLOAD_PATH: Path for local storage (defaults to "data/uploads")
    pub fn from_env() -> Result<Self, ConfigError> {
        let mode = env::var("FOLD_UPLOAD_STORAGE_MODE").unwrap_or_else(|_| "local".to_string());

        match mode.as_str() {
            "local" => {
                let path = env::var("FOLD_UPLOAD_PATH")
                    .map(PathBuf::from)
                    .unwrap_or_else(|_| PathBuf::from("data/uploads"));
                Ok(UploadStorageConfig::Local { path })
            }

            _ => Err(ConfigError::InvalidValue(format!(
                "Invalid FOLD_UPLOAD_STORAGE_MODE: {}. Must be 'local'",
                mode
            ))),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_deserialize_new_format() {
        let json = r#"{"path": "/data/db", "cloud_sync": {"api_url": "https://api.example.com", "api_key": "key123"}}"#;
        let config: DatabaseConfig = serde_json::from_str(json).unwrap();
        assert_eq!(config.path, PathBuf::from("/data/db"));
        assert!(config.cloud_sync.is_some());
        let sync = config.cloud_sync.unwrap();
        assert_eq!(sync.api_url, "https://api.example.com");
        assert_eq!(sync.api_key, "key123");
    }

    #[test]
    fn test_deserialize_new_format_no_sync() {
        let json = r#"{"path": "/data/db"}"#;
        let config: DatabaseConfig = serde_json::from_str(json).unwrap();
        assert_eq!(config.path, PathBuf::from("/data/db"));
        assert!(config.cloud_sync.is_none());
    }

    #[test]
    fn test_deserialize_legacy_local() {
        let json = r#"{"type": "local", "path": "/data/db"}"#;
        let config: DatabaseConfig = serde_json::from_str(json).unwrap();
        assert_eq!(config.path, PathBuf::from("/data/db"));
        assert!(config.cloud_sync.is_none());
    }

    #[test]
    fn test_deserialize_legacy_exemem() {
        let json = r#"{"type": "exemem", "api_url": "https://api.example.com", "api_key": "key123"}"#;
        let config: DatabaseConfig = serde_json::from_str(json).unwrap();
        assert!(config.cloud_sync.is_some());
        let sync = config.cloud_sync.unwrap();
        assert_eq!(sync.api_url, "https://api.example.com");
        assert_eq!(sync.api_key, "key123");
    }

    #[test]
    fn test_serialize_roundtrip() {
        let config = DatabaseConfig::with_cloud_sync(
            PathBuf::from("/data"),
            CloudSyncConfig {
                api_url: "https://api.example.com".to_string(),
                api_key: "key123".to_string(),
                session_token: None,
                user_hash: None,
            },
        );
        let json = serde_json::to_string(&config).unwrap();
        let deserialized: DatabaseConfig = serde_json::from_str(&json).unwrap();
        assert_eq!(deserialized.path, config.path);
        assert!(deserialized.cloud_sync.is_some());
    }

    #[test]
    fn test_default() {
        let config = DatabaseConfig::default();
        assert_eq!(config.path, PathBuf::from("data"));
        assert!(config.cloud_sync.is_none());
    }
}
