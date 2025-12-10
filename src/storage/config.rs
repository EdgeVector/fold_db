use std::path::PathBuf;
use std::env;
use serde::{Deserialize, Serialize};



/// Configuration for DynamoDB storage
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct DynamoDbConfig {
    /// AWS Region
    pub region: String,
    /// Table naming configuration
    pub table_config: TableConfig,
    /// If true, tables will be automatically created if missing.
    pub auto_create: bool,
    /// Optional user_id for multi-tenant isolation
    #[serde(default)]
    pub user_id: Option<String>,
}

/// DynamoDB Table Naming Configuration
#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(tag = "type", content = "value")]
pub enum TableConfig {
    /// Tables are named "{prefix}-{namespace}"
    #[serde(rename = "prefix")]
    Prefix(String),
    /// Exact names provided for each namespace
    #[serde(rename = "explicit")]
    Explicit(ExplicitTables),
}

/// Explicit table names for all required namespaces
#[derive(Clone, Debug, Default, Serialize, Deserialize)]
pub struct ExplicitTables {
    pub main: String,
    pub metadata: String,
    pub permissions: String,
    pub transforms: String,
    pub orchestrator: String,
    pub schema_states: String,
    pub schemas: String,
    pub public_keys: String,
    pub transform_queue: String,
    /// Native index table
    pub native_index: String,
    /// Process tracking table (ingestion, backfills)
    ///
    /// This table is used to track long-running operations.
    /// If using `TableConfig::Prefix`, this is automatically set to `{prefix}-process`.
    /// If using `TableConfig::Explicit`, you must provide this table name.
    ///
    /// NOTE: If using DynamoDB storage, this table MUST exist or initialization will fail.
    pub process: String,
}

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



/// Storage configuration enum for different backends
#[derive(Clone, Debug, Serialize, Deserialize)]
pub enum StorageConfig {
    /// Local filesystem storage
    #[serde(rename = "local")]
    Local {
        /// Path to the local database file/directory
        path: PathBuf,
    },
}

impl Default for StorageConfig {
    fn default() -> Self {
        StorageConfig::Local {
            path: PathBuf::from("data"),
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
    /// - DATAFOLD_UPLOAD_STORAGE_MODE: "local" or "s3" (defaults to "local")
    /// - DATAFOLD_UPLOAD_PATH: Path for local storage (defaults to "data/uploads")
    /// - DATAFOLD_UPLOAD_S3_BUCKET: S3 bucket for uploads (required if mode=s3)
    /// - DATAFOLD_UPLOAD_S3_REGION: AWS region (required if mode=s3)
    /// - DATAFOLD_UPLOAD_S3_PREFIX: S3 prefix/folder (defaults to "uploads")
    pub fn from_env() -> Result<Self, ConfigError> {
        let mode = env::var("DATAFOLD_UPLOAD_STORAGE_MODE")
            .unwrap_or_else(|_| "local".to_string());

        match mode.as_str() {
            "local" => {
                let path = env::var("DATAFOLD_UPLOAD_PATH")
                    .map(PathBuf::from)
                    .unwrap_or_else(|_| PathBuf::from("data/uploads"));
                Ok(UploadStorageConfig::Local { path })
            }

            _ => Err(ConfigError::InvalidValue(format!(
                "Invalid DATAFOLD_UPLOAD_STORAGE_MODE: {}. Must be 'local' or 's3'",
                mode
            ))),
        }
    }
}

impl StorageConfig {
    /// Creates StorageConfig from environment variables:
    /// - DATAFOLD_STORAGE_MODE: "local" (default) or "s3"
    /// - DATAFOLD_STORAGE_PATH: path for local storage (default: "data")
    /// - For S3 mode, uses S3Config::from_env()
    pub fn from_env() -> Result<Self, ConfigError> {
        let mode = env::var("DATAFOLD_STORAGE_MODE")
            .unwrap_or_else(|_| "local".to_string());

        match mode.to_lowercase().as_str() {

            "local" => {
                let path = env::var("DATAFOLD_STORAGE_PATH")
                    .map(PathBuf::from)
                    .unwrap_or_else(|_| PathBuf::from("data"));
                Ok(StorageConfig::Local { path })
            }
            _ => Err(ConfigError::InvalidValue(
                format!("Invalid DATAFOLD_STORAGE_MODE: '{}'. Must be 'local' or 's3'", mode)
            )),
        }
    }
}

