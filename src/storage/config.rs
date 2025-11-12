use std::path::PathBuf;
use std::env;

/// S3 configuration for remote storage
#[derive(Debug, Clone)]
pub struct S3Config {
    /// S3 bucket name
    pub bucket: String,
    /// AWS region (e.g., "us-west-2")
    pub region: String,
    /// Prefix/path within the bucket (e.g., "production/folddb")
    pub prefix: String,
    /// Local cache path (defaults to /tmp/folddb-data for Lambda)
    pub local_path: PathBuf,
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

impl S3Config {
    pub fn new(bucket: String, region: String, prefix: String) -> Self {
        Self {
            bucket,
            region,
            prefix,
            local_path: PathBuf::from("/tmp/folddb-data"),
        }
    }

    pub fn with_local_path(mut self, path: PathBuf) -> Self {
        self.local_path = path;
        self
    }

    /// Creates S3Config from environment variables:
    /// - DATAFOLD_S3_BUCKET (required)
    /// - DATAFOLD_S3_REGION (required)
    /// - DATAFOLD_S3_PREFIX (optional, defaults to "folddb")
    /// - DATAFOLD_S3_LOCAL_PATH (optional, defaults to "/tmp/folddb-data")
    pub fn from_env() -> Result<Self, ConfigError> {
        let bucket = env::var("DATAFOLD_S3_BUCKET")
            .map_err(|_| ConfigError::MissingVariable("DATAFOLD_S3_BUCKET".to_string()))?;
        
        let region = env::var("DATAFOLD_S3_REGION")
            .map_err(|_| ConfigError::MissingVariable("DATAFOLD_S3_REGION".to_string()))?;
        
        let prefix = env::var("DATAFOLD_S3_PREFIX")
            .unwrap_or_else(|_| "folddb".to_string());
        
        let local_path = env::var("DATAFOLD_S3_LOCAL_PATH")
            .map(PathBuf::from)
            .unwrap_or_else(|_| PathBuf::from("/tmp/folddb-data"));

        Ok(Self {
            bucket,
            region,
            prefix,
            local_path,
        })
    }
}

/// Storage configuration enum for different backends
#[derive(Debug, Clone)]
pub enum StorageConfig {
    /// Local Sled storage (default)
    Local { path: PathBuf },
    /// S3-backed storage with local cache
    S3 { config: S3Config },
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
    /// S3 storage for uploads
    S3 { 
        bucket: String, 
        region: String,
        prefix: String,
    },
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
            "s3" => {
                let bucket = env::var("DATAFOLD_UPLOAD_S3_BUCKET")
                    .map_err(|_| ConfigError::MissingVariable("DATAFOLD_UPLOAD_S3_BUCKET".to_string()))?;
                
                let region = env::var("DATAFOLD_UPLOAD_S3_REGION")
                    .map_err(|_| ConfigError::MissingVariable("DATAFOLD_UPLOAD_S3_REGION".to_string()))?;
                
                let prefix = env::var("DATAFOLD_UPLOAD_S3_PREFIX")
                    .unwrap_or_else(|_| "uploads".to_string());

                Ok(UploadStorageConfig::S3 {
                    bucket,
                    region,
                    prefix,
                })
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
            "s3" => {
                let s3_config = S3Config::from_env()?;
                Ok(StorageConfig::S3 { config: s3_config })
            }
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

