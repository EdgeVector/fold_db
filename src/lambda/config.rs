//! Configuration types for Lambda context

use crate::db_operations::DbOperationsV2;
use crate::lambda::logging::Logger;
use crate::storage::{StorageConfig, DynamoDbConfig};
use std::sync::Arc;

/// Storage configuration for Lambda - either use StorageConfig or provide a pre-created DbOperationsV2
#[derive(Clone)]
pub enum LambdaStorage {
    /// Use StorageConfig to create DbOperationsV2 automatically (Local or S3)
    Config(StorageConfig),
    /// Use a pre-created DbOperationsV2 instance (allows any backend implementation)
    DbOps(Arc<DbOperationsV2>),
    /// Use DynamoDB with specific configuration
    DynamoDb(DynamoDbConfig),
}



/// Configuration for Lambda logging
#[derive(Clone)]
pub enum LambdaLogging {
    /// Use DynamoDB for logging (recommended for multi-tenant)
    DynamoDb { table_name: String },
    /// Use stdout for logging (good for development/single-tenant)
    Stdout,
    /// Use a custom logger implementation
    Custom(Arc<dyn Logger>),
    /// Disable logging (not recommended)
    NoOp,
}

// LambdaProcess enum removed

impl std::fmt::Debug for LambdaLogging {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::DynamoDb { table_name } => f.debug_struct("DynamoDb").field("table_name", table_name).finish(),
            Self::Stdout => write!(f, "Stdout"),
            Self::Custom(_) => write!(f, "Custom(<logger>)"),
            Self::NoOp => write!(f, "NoOp"),
        }
    }
}

/// Configuration for Lambda context initialization
#[derive(Clone)]
pub struct LambdaConfig {
    /// Required storage configuration - either StorageConfig or pre-created DbOperationsV2
    pub storage: LambdaStorage,
    /// Required logging configuration
    pub logging: LambdaLogging,
    /// Optional schema service URL
    pub schema_service_url: Option<String>,
    /// Optional AI configuration for query capabilities
    pub ai_config: Option<AIConfig>,
    // logger field removed, moved to logging enum
}

impl std::fmt::Debug for LambdaConfig {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("LambdaConfig")
            .field("storage", &match &self.storage {
                LambdaStorage::Config(cfg) => format!("Config({:?})", cfg),
                LambdaStorage::DbOps(_) => "DbOps(<pre-created>)".to_string(),
                LambdaStorage::DynamoDb(config) => format!("DynamoDb({:?})", config),
            })
            .field("schema_service_url", &self.schema_service_url)
            .field("ai_config", &self.ai_config)
            .field("logging", &self.logging)
            .finish()
    }
}

// Note: Default is not implemented because storage_config is required
// Use LambdaConfig::new() or LambdaConfig::with_storage_config() instead

/// AI Provider types
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum AIProvider {
    OpenRouter,
    Ollama,
}

/// Configuration for AI query functionality
#[derive(Debug, Clone)]
pub struct AIConfig {
    pub provider: AIProvider,
    pub openrouter: Option<OpenRouterConfig>,
    pub ollama: Option<OllamaConfig>,
    pub timeout_seconds: u64,
    pub max_retries: u32,
}

/// OpenRouter configuration
#[derive(Debug, Clone)]
pub struct OpenRouterConfig {
    pub api_key: String,
    pub model: String,
    pub base_url: Option<String>,
}

/// Ollama configuration
#[derive(Debug, Clone)]
pub struct OllamaConfig {
    pub base_url: String,
    pub model: String,
}

impl LambdaConfig {
    /// Create a new Lambda configuration with StorageConfig (Local or S3) and Logging.
    ///
    /// # Example
    ///
    /// ```ignore
    /// use datafold::lambda::{LambdaConfig, LambdaLogging};
    /// use datafold::StorageConfig;
    ///
    ///     StorageConfig::Local { path: PathBuf::from("/tmp/folddb") },
    ///     LambdaLogging::Stdout
    /// );
    /// ```
    pub fn new(storage_config: StorageConfig, logging: LambdaLogging) -> Self {
        Self {
            storage: LambdaStorage::Config(storage_config),
            logging,
            schema_service_url: None,
            ai_config: None,
        }
    }

    /// Create a new Lambda configuration with a pre-created DbOperationsV2 and Logging.
    /// 
    /// This allows you to use any storage backend implementation (DynamoDB, custom, etc.)
    /// by creating DbOperationsV2 yourself.
    ///
    /// # Example
    ///
    /// ```ignore
    /// use datafold::lambda::{LambdaConfig, LambdaLogging};
    /// use datafold::db_operations::DbOperationsV2;
    /// use std::sync::Arc;
    ///
    /// // Create your DbOperationsV2 with any backend
    /// let db_ops = Arc::new(DbOperationsV2::from_dynamodb(client, table, Some(user_id)).await?);
    /// // Create your DbOperationsV2 with any backend
    /// let db_ops = Arc::new(DbOperationsV2::from_dynamodb(client, table, Some(user_id)).await?);
    /// let config = LambdaConfig::with_db_ops(
    ///     db_ops, 
    ///     LambdaLogging::DynamoDb { table_name: "logs".into() }
    /// );
    /// ```
    pub fn with_db_ops(db_ops: Arc<DbOperationsV2>, logging: LambdaLogging) -> Self {
        Self {
            storage: LambdaStorage::DbOps(db_ops),
            logging,
            schema_service_url: None,
            ai_config: None,
        }
    }

    /// Set the storage configuration (replaces existing storage)
    ///
    /// # Example
    ///
    /// ```ignore
    /// use datafold::lambda::{LambdaConfig, LambdaLogging};
    /// use datafold::{StorageConfig, S3Config};
    ///
    /// let s3_config = S3Config::new(
    ///     "my-bucket".to_string(),
    ///     "us-west-2".to_string(),
    ///     "folddb".to_string()
    /// );
    /// let config = LambdaConfig::new(
    ///     StorageConfig::Local { path: PathBuf::from("/tmp") },
    ///     LambdaLogging::Stdout
    /// ).with_storage_config(StorageConfig::S3 { config: s3_config });
    /// ```
    pub fn with_storage_config(mut self, storage_config: StorageConfig) -> Self {
        self.storage = LambdaStorage::Config(storage_config);
        self
    }

    /// Set the schema service URL
    pub fn with_schema_service_url(mut self, url: String) -> Self {
        self.schema_service_url = Some(url);
        self
    }

    /// Enable AI query functionality with OpenRouter
    ///
    /// # Example
    ///
    /// ```ignore
    /// use datafold::lambda::{LambdaConfig, LambdaLogging};
    /// use datafold::StorageConfig;
    /// use std::path::PathBuf;
    ///
    /// let storage = StorageConfig::Local { path: PathBuf::from("/tmp/folddb") };
    /// let logging = LambdaLogging::Stdout;
    /// let config = LambdaConfig::new(storage, logging)
    ///     .with_openrouter(
    ///         "sk-or-v1-...".to_string(),
    ///         "anthropic/claude-3.5-sonnet".to_string()
    ///     );
    /// ```
    pub fn with_openrouter(mut self, api_key: String, model: String) -> Self {
        self.ai_config = Some(AIConfig {
            provider: AIProvider::OpenRouter,
            openrouter: Some(OpenRouterConfig {
                api_key,
                model,
                base_url: None,
            }),
            ollama: None,
            timeout_seconds: 120,
            max_retries: 3,
        });
        self
    }

    /// Enable AI query functionality with Ollama
    ///
    /// # Example
    ///
    /// ```ignore
    /// use datafold::lambda::{LambdaConfig, LambdaLogging};
    /// use datafold::StorageConfig;
    /// use std::path::PathBuf;
    ///
    /// let storage = StorageConfig::Local { path: PathBuf::from("/tmp/folddb") };
    /// let logging = LambdaLogging::Stdout;
    /// let config = LambdaConfig::new(storage, logging)
    ///     .with_ollama(
    ///         "http://localhost:11434".to_string(),
    ///         "llama2".to_string()
    ///     );
    /// ```
    pub fn with_ollama(mut self, base_url: String, model: String) -> Self {
        self.ai_config = Some(AIConfig {
            provider: AIProvider::Ollama,
            openrouter: None,
            ollama: Some(OllamaConfig {
                base_url,
                model,
            }),
            timeout_seconds: 120,
            max_retries: 3,
        });
        self
    }

    /// Set custom AI configuration
    pub fn with_ai_config(mut self, config: AIConfig) -> Self {
        self.ai_config = Some(config);
        self
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::storage::StorageConfig;
    use std::path::PathBuf;

    #[test]
    fn test_lambda_config_creation() {
        let storage_config = StorageConfig::Local { path: PathBuf::from("/tmp/folddb") };
        let config = LambdaConfig::new(storage_config, LambdaLogging::Stdout);
        assert!(config.schema_service_url.is_none());
    }

    #[test]
    fn test_lambda_config_with_storage_config() {
        let storage_config1 = StorageConfig::Local { path: PathBuf::from("/tmp/test1") };
        let storage_config2 = StorageConfig::Local { path: PathBuf::from("/tmp/test2") };
        let config = LambdaConfig::new(storage_config1.clone(), LambdaLogging::Stdout)
            .with_storage_config(storage_config2.clone());
        
        match &config.storage {
            LambdaStorage::Config(StorageConfig::Local { path }) => {
                assert_eq!(path, &PathBuf::from("/tmp/test2"));
            }
            _ => panic!("Expected Local storage config"),
        }
    }

    #[test]
    fn test_lambda_config_with_schema_service_url() {
        let storage_config = StorageConfig::Local { path: PathBuf::from("/tmp/folddb") };
        let url = "https://schema.example.com".to_string();
        let config = LambdaConfig::new(storage_config, LambdaLogging::Stdout).with_schema_service_url(url.clone());
        assert_eq!(config.schema_service_url, Some(url));
    }

    #[test]
    fn test_lambda_config_builder_pattern() {
        let storage_config = StorageConfig::Local { path: PathBuf::from("/tmp/test") };
        let url = "https://schema.example.com".to_string();
        
        let config = LambdaConfig::new(storage_config, LambdaLogging::Stdout)
            .with_schema_service_url(url.clone());
        
        assert_eq!(config.schema_service_url, Some(url));
    }

    #[test]
    fn test_lambda_config_debug_impl() {
        let storage_config = StorageConfig::Local { path: PathBuf::from("/tmp/test") };
        let config = LambdaConfig::new(storage_config, LambdaLogging::Stdout);
        
        let debug_str = format!("{:?}", config);
        assert!(debug_str.contains("LambdaConfig"));
    }

    #[test]
    fn test_lambda_config_clone() {
        let storage_config = StorageConfig::Local { path: PathBuf::from("/tmp/test") };
        let config1 = LambdaConfig::new(storage_config, LambdaLogging::Stdout)
            .with_schema_service_url("https://example.com".to_string());
        
        let config2 = config1.clone();
        
        assert_eq!(config1.schema_service_url, config2.schema_service_url);
    }

    #[test]
    fn test_lambda_config_with_both_options() {
        let storage_config = StorageConfig::Local { path: PathBuf::from("/tmp/lambda_test") };
        let url = "https://schema.service.com".to_string();
        
        let config = LambdaConfig {
            storage: LambdaStorage::Config(storage_config.clone()),
            logging: LambdaLogging::NoOp,
            schema_service_url: Some(url.clone()),
            ai_config: None,
        };
        
        assert_eq!(config.schema_service_url, Some(url));
        assert!(config.ai_config.is_none());
    }

    #[test]
    fn test_lambda_config_with_openrouter() {
        let storage_config = StorageConfig::Local { path: PathBuf::from("/tmp/folddb") };
        let config = LambdaConfig::new(storage_config, LambdaLogging::Stdout)
            .with_openrouter(
                "test-key".to_string(),
                "test-model".to_string()
            );
        
        assert!(config.ai_config.is_some());
        let ai_config = config.ai_config.unwrap();
        assert_eq!(ai_config.provider, AIProvider::OpenRouter);
        assert!(ai_config.openrouter.is_some());
        assert_eq!(ai_config.openrouter.unwrap().api_key, "test-key");
    }

    #[test]
    fn test_lambda_config_with_ollama() {
        let storage_config = StorageConfig::Local { path: PathBuf::from("/tmp/folddb") };
        let config = LambdaConfig::new(storage_config, LambdaLogging::Stdout)
            .with_ollama(
                "http://localhost:11434".to_string(),
                "llama2".to_string()
            );
        
        assert!(config.ai_config.is_some());
        let ai_config = config.ai_config.unwrap();
        assert_eq!(ai_config.provider, AIProvider::Ollama);
        assert!(ai_config.ollama.is_some());
        assert_eq!(ai_config.ollama.unwrap().base_url, "http://localhost:11434");
    }

    #[test]
    fn test_lambda_config_builder_chain() {
        let storage_config = StorageConfig::Local { path: PathBuf::from("/tmp/test") };
        let config = LambdaConfig::new(storage_config, LambdaLogging::Stdout)
            .with_schema_service_url("https://schema.example.com".to_string())
            .with_openrouter("key".to_string(), "model".to_string());
        
        assert_eq!(config.schema_service_url, Some("https://schema.example.com".to_string()));
        assert!(config.ai_config.is_some());
    }

    #[test]
    fn test_ai_config_custom_timeout_retries() {
        let config = AIConfig {
            provider: AIProvider::OpenRouter,
            openrouter: Some(OpenRouterConfig {
                api_key: "test".to_string(),
                model: "test-model".to_string(),
                base_url: Some("https://custom.url".to_string()),
            }),
            ollama: None,
            timeout_seconds: 300,
            max_retries: 10,
        };
        
        assert_eq!(config.timeout_seconds, 300);
        assert_eq!(config.max_retries, 10);
        assert_eq!(config.openrouter.as_ref().unwrap().base_url, Some("https://custom.url".to_string()));
    }
}
