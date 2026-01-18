//! Configuration types for Lambda context
//!
//! storage types now use DatabaseConfig instead of outdated StorageConfig

use crate::db_operations::DbOperations;
use crate::lambda::logging::Logger;
use crate::storage::DatabaseConfig;
use std::sync::Arc;

/// Storage configuration for Lambda - either use DatabaseConfig or provide a pre-created DbOperations
#[derive(Clone)]
pub enum LambdaStorage {
    /// Use DatabaseConfig to create DbOperations automatically (Local, S3, or DynamoDB)
    Config(DatabaseConfig),
    /// Use a pre-created DbOperations instance (allows any backend implementation)
    DbOps(Arc<DbOperations>),
}

/// Configuration for Lambda logging
#[derive(Clone)]
pub enum LambdaLogging {
    /// Use DynamoDB for logging (recommended for multi-tenant)
    /// Table name is now taken from ExplicitTables.logs in DatabaseConfig (if applicable)
    DynamoDb,
    /// Use stdout for logging (good for development/single-tenant)
    Stdout,
    /// Use a custom logger implementation
    Custom(Arc<dyn Logger>),
    /// Disable logging (not recommended)
    NoOp,
}

impl std::fmt::Debug for LambdaLogging {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::DynamoDb => write!(f, "DynamoDb"),
            Self::Stdout => write!(f, "Stdout"),
            Self::Custom(_) => write!(f, "Custom(<logger>)"),
            Self::NoOp => write!(f, "NoOp"),
        }
    }
}

/// Configuration for Lambda context initialization
#[derive(Clone)]
pub struct LambdaConfig {
    /// Required storage configuration - either DatabaseConfig or pre-created DbOperations
    pub storage: LambdaStorage,
    /// Required logging configuration
    pub logging: LambdaLogging,
    /// Optional schema service URL
    pub schema_service_url: Option<String>,
    /// Optional AI configuration for query capabilities
    pub ai_config: Option<AIConfig>,
}

impl std::fmt::Debug for LambdaConfig {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("LambdaConfig")
            .field(
                "storage",
                &match &self.storage {
                    LambdaStorage::Config(cfg) => format!("Config({:?})", cfg),
                    LambdaStorage::DbOps(_) => "DbOps(<pre-created>)".to_string(),
                },
            )
            .field("schema_service_url", &self.schema_service_url)
            .field("ai_config", &self.ai_config)
            .field("logging", &self.logging)
            .finish()
    }
}

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
    /// Create a new Lambda configuration with DatabaseConfig (Local or S3) and Logging.
    ///
    /// # Example
    ///
    /// ```ignore
    /// use datafold::lambda::{LambdaConfig, LambdaLogging};
    /// use datafold::DatabaseConfig;
    ///
    ///     DatabaseConfig::Local { path: PathBuf::from("/tmp/folddb") },
    ///     LambdaLogging::Stdout
    /// );
    /// ```
    pub fn new(storage_config: DatabaseConfig, logging: LambdaLogging) -> Self {
        Self {
            storage: LambdaStorage::Config(storage_config),
            logging,
            schema_service_url: None,
            ai_config: None,
        }
    }

    /// Create a new Lambda configuration with a pre-created DbOperations and Logging.
    ///
    /// This allows you to use any storage backend implementation (DynamoDB, custom, etc.)
    /// by creating DbOperations yourself.
    ///
    /// # Example
    ///
    /// ```ignore
    /// use datafold::lambda::{LambdaConfig, LambdaLogging};
    /// use datafold::db_operations::DbOperations;
    /// use std::sync::Arc;
    ///
    /// // Create your DbOperations with any backend
    /// let db_ops = Arc::new(DbOperations::from_dynamodb(client, table, Some(user_id)).await?);
    /// let config = LambdaConfig::with_db_ops(
    ///     db_ops,
    ///     LambdaLogging::DynamoDb { table_name: "logs".into() }
    /// );
    /// ```
    pub fn with_db_ops(db_ops: Arc<DbOperations>, logging: LambdaLogging) -> Self {
        Self {
            storage: LambdaStorage::DbOps(db_ops),
            logging,
            schema_service_url: None,
            ai_config: None,
        }
    }

    /// Set the storage configuration (replaces existing storage)
    pub fn with_storage_config(mut self, storage_config: DatabaseConfig) -> Self {
        self.storage = LambdaStorage::Config(storage_config);
        self
    }

    /// Set the schema service URL
    pub fn with_schema_service_url(mut self, url: String) -> Self {
        self.schema_service_url = Some(url);
        self
    }

    /// Enable AI query functionality with OpenRouter
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
    pub fn with_ollama(mut self, base_url: String, model: String) -> Self {
        self.ai_config = Some(AIConfig {
            provider: AIProvider::Ollama,
            openrouter: None,
            ollama: Some(OllamaConfig { base_url, model }),
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
    use crate::storage::DatabaseConfig;
    use std::path::PathBuf;

    #[test]
    fn test_lambda_config_creation() {
        let storage_config = DatabaseConfig::Local {
            path: PathBuf::from("/tmp/folddb"),
        };
        let config = LambdaConfig::new(storage_config, LambdaLogging::Stdout);
        assert!(config.schema_service_url.is_none());
    }

    #[test]
    fn test_lambda_config_with_storage_config() {
        let storage_config1 = DatabaseConfig::Local {
            path: PathBuf::from("/tmp/test1"),
        };
        let storage_config2 = DatabaseConfig::Local {
            path: PathBuf::from("/tmp/test2"),
        };
        let config = LambdaConfig::new(storage_config1.clone(), LambdaLogging::Stdout)
            .with_storage_config(storage_config2.clone());

        match &config.storage {
            LambdaStorage::Config(DatabaseConfig::Local { path }) => {
                assert_eq!(path, &PathBuf::from("/tmp/test2"));
            }
            _ => panic!("Expected Local storage config"),
        }
    }

    #[test]
    fn test_lambda_config_with_schema_service_url() {
        let storage_config = DatabaseConfig::Local {
            path: PathBuf::from("/tmp/folddb"),
        };
        let url = "https://schema.example.com".to_string();
        let config = LambdaConfig::new(storage_config, LambdaLogging::Stdout)
            .with_schema_service_url(url.clone());
        assert_eq!(config.schema_service_url, Some(url));
    }
}
