use serde::{Deserialize, Serialize};
use std::env;
use std::path::PathBuf;

/// Configuration for Cloud storage
#[cfg(feature = "aws-backend")]
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct CloudConfig {
    /// AWS Region
    pub region: String,
    /// Explicit table names for all required namespaces
    pub tables: ExplicitTables,
    /// If true, tables will be automatically created if missing.
    pub auto_create: bool,
    /// Optional user_id for multi-tenant isolation
    #[serde(default)]
    pub user_id: Option<String>,
    /// Optional S3 bucket for file storage (uploads/ingestion)
    /// When set, files will be stored in S3 instead of local filesystem
    #[serde(default)]
    pub file_storage_bucket: Option<String>,
}

#[cfg(feature = "aws-backend")]
impl CloudConfig {
    /// Create config from environment variables.
    ///
    /// Required environment variables:
    /// - `DATAFOLD_DYNAMODB_REGION`: AWS region
    /// - `DATAFOLD_DYNAMODB_TABLE_PREFIX`: Prefix for table names (tables will be named `{prefix}-{namespace}`)
    ///
    /// Optional:
    /// - `DATAFOLD_DYNAMODB_USER_ID`: User ID for multi-tenant isolation
    /// - `DATAFOLD_S3_FILE_STORAGE_BUCKET`: S3 bucket for file storage (uploads/ingestion)
    pub fn from_env() -> Result<Self, ConfigError> {
        let region = env::var("DATAFOLD_DYNAMODB_REGION")
            .map_err(|_| ConfigError::MissingVariable("DATAFOLD_DYNAMODB_REGION".to_string()))?;

        let table_prefix = env::var("DATAFOLD_DYNAMODB_TABLE_PREFIX")
            .or_else(|_| env::var("DATAFOLD_DYNAMODB_TABLE")) // Backward compatibility
            .map_err(|_| {
                ConfigError::MissingVariable("DATAFOLD_DYNAMODB_TABLE_PREFIX".to_string())
            })?;

        // Generate explicit table names from prefix
        let tables = ExplicitTables::from_prefix(&table_prefix);

        Ok(Self {
            region,
            tables,
            auto_create: true,
            user_id: env::var("DATAFOLD_DYNAMODB_USER_ID").ok(),
            file_storage_bucket: env::var("DATAFOLD_S3_FILE_STORAGE_BUCKET").ok(),
        })
    }
}

/// Explicit table names for all required DynamoDB namespaces.
///
/// All 12 tables must be explicitly configured:
/// - `main`: Primary data storage
/// - `metadata`: Metadata storage  
/// - `permissions`: Node/schema permission mappings
/// - `transforms`: Schema transformations
/// - `orchestrator`: Orchestration state
/// - `schema_states`: Schema state tracking
/// - `schemas`: Schema definitions
/// - `public_keys`: Public key storage
/// - `transform_queue`: Transform processing queue
/// - `native_index`: Native index data
/// - `process`: Process tracking (ingestion, backfills)
/// - `logs`: System logs
#[cfg(feature = "aws-backend")]
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
    /// NOTE: If using DynamoDB storage, this table MUST exist or initialization will fail.
    pub process: String,
    /// System logs table
    pub logs: String,
    /// Idempotency tracking table (mutation content hashes)
    pub idempotency: String,
}

#[cfg(feature = "aws-backend")]
impl ExplicitTables {
    /// Create ExplicitTables from a prefix.
    /// Tables are named `{prefix}-{namespace}`.
    pub fn from_prefix(prefix: &str) -> Self {
        Self {
            main: format!("{}-main", prefix),
            metadata: format!("{}-metadata", prefix),
            permissions: format!("{}-node_id_schema_permissions", prefix),
            transforms: format!("{}-transforms", prefix),
            orchestrator: format!("{}-orchestrator_state", prefix),
            schema_states: format!("{}-schema_states", prefix),
            schemas: format!("{}-schemas", prefix),
            public_keys: format!("{}-public_keys", prefix),
            transform_queue: format!("{}-transform_queue_tree", prefix),
            native_index: format!("{}-native_index", prefix),
            process: format!("{}-process", prefix),
            logs: format!("{}-logs", prefix),
            idempotency: format!("{}-idempotency", prefix),
        }
    }
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
#[serde(tag = "type")]
pub enum DatabaseConfig {
    /// Local filesystem storage
    #[serde(rename = "local")]
    Local {
        /// Path to the local database file/directory
        path: PathBuf,
    },
    /// Cloud storage (DynamoDB etc)
    #[cfg(feature = "aws-backend")]
    #[serde(rename = "cloud")]
    Cloud(Box<CloudConfig>),
    /// Exemem Cloud storage (remote API-backed)
    #[serde(rename = "exemem")]
    Exemem {
        /// Exemem Storage API URL
        api_url: String,
        /// API key for authentication
        api_key: String,
    },
}

impl Default for DatabaseConfig {
    fn default() -> Self {
        DatabaseConfig::Local {
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
        let mode = env::var("DATAFOLD_UPLOAD_STORAGE_MODE").unwrap_or_else(|_| "local".to_string());

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

impl DatabaseConfig {
    /// Creates DatabaseConfig from environment variables:
    /// - DATAFOLD_STORAGE_MODE: "local" (default) or "s3"
    /// - DATAFOLD_STORAGE_PATH: path for local storage (default: "data")
    /// - For S3 mode, uses S3Config::from_env()
    pub fn from_env() -> Result<Self, ConfigError> {
        let mode = env::var("DATAFOLD_STORAGE_MODE").unwrap_or_else(|_| "local".to_string());

        match mode.to_lowercase().as_str() {
            "local" => {
                let path = env::var("DATAFOLD_STORAGE_PATH")
                    .map(PathBuf::from)
                    .unwrap_or_else(|_| PathBuf::from("data"));
                Ok(DatabaseConfig::Local { path })
            }
            #[cfg(feature = "aws-backend")]
            "cloud" | "dynamodb" => {
                let config = CloudConfig::from_env()?;
                Ok(DatabaseConfig::Cloud(Box::new(config)))
            }
            "exemem" => {
                let api_url = env::var("EXEMEM_API_URL")
                    .map_err(|_| ConfigError::MissingVariable("EXEMEM_API_URL".to_string()))?;
                let api_key = env::var("EXEMEM_API_KEY")
                    .map_err(|_| ConfigError::MissingVariable("EXEMEM_API_KEY".to_string()))?;
                Ok(DatabaseConfig::Exemem { api_url, api_key })
            }
            _ => Err(ConfigError::InvalidValue(format!(
                "Invalid DATAFOLD_STORAGE_MODE: '{}'. Must be 'local', 'exemem'{}",
                mode,
                if cfg!(feature = "aws-backend") {
                    ", or 'dynamodb'"
                } else {
                    ""
                }
            ))),
        }
    }
}
