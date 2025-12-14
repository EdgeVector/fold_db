//! Lambda context implementation
//!
//! Now supports multi-tenancy via NodeManager.

use crate::datafold_node::{DataFoldNode, NodeConfig};
use crate::datafold_node::llm_query::service::LlmQueryService;
use crate::fold_db_core::FoldDB;
use crate::ingestion::{
    create_progress_tracker, IngestionConfig, IngestionError, ProgressTracker,
};
use crate::lambda::config::{AIConfig, AIProvider, LambdaConfig, LambdaStorage};
use crate::lambda::logging::{LogBridge, Logger};
use crate::lambda::node_manager::NodeManager;
use crate::storage::StorageConfig;
use once_cell::sync::OnceCell;
use std::sync::{Arc, Mutex};

/// Lambda context that manages all required state.
///
/// This should be initialized once during Lambda cold start and reused
/// across all invocations to minimize latency.
pub struct LambdaContext {
    pub(crate) node_manager: Arc<NodeManager>,
    pub(crate) progress_tracker: ProgressTracker,
    pub(crate) llm_service: Option<Arc<LlmQueryService>>,
    pub(crate) logger: Arc<dyn Logger>,
}

static LAMBDA_CONTEXT: OnceCell<LambdaContext> = OnceCell::new();

impl LambdaContext {
    /// Initialize Lambda context with explicit configuration.
    ///
    /// This should be called once during Lambda cold start, before any
    /// handler invocations. The context is stored globally and reused
    /// across invocations for optimal performance.
    ///
    /// # Arguments
    ///
    /// * `config` - Lambda configuration with optional settings
    ///
    /// # Example
    ///
    /// ```ignore
    /// use datafold::lambda::{LambdaContext, LambdaConfig};
    ///
    /// #[tokio::main]
    /// async fn main() {
    ///     let config = LambdaConfig::new(
    ///         StorageConfig::Local { path: PathBuf::from("/tmp/folddb") },
    ///         LambdaLogging::Stdout
    ///     )
    ///         .with_schema_service_url("https://schema.example.com".to_string());
    ///     
    ///     LambdaContext::init(config)
    ///         .await
    ///         .expect("Failed to initialize");
    /// }
    /// ```
    pub async fn init(config: LambdaConfig) -> Result<(), IngestionError> {
        // Initialize NodeManager handles node creation (single or multi-tenant)
        let node_manager = Arc::new(NodeManager::new(config.clone()).await?);

        // Initialize Progress Store based on storage configuration
        let progress_tracker: ProgressTracker = match &config.storage {
            crate::lambda::config::LambdaStorage::Config(crate::storage::StorageConfig::DynamoDb(dynamo_config)) => {
                 use crate::ingestion::progress::DynamoDbProgressStore;
                 
                 let table_name = dynamo_config.tables.process.clone();
                 
                 Arc::new(DynamoDbProgressStore::new(table_name).await
                     .map_err(|e| IngestionError::StorageError(format!("Failed to initialize process table: {}", e)))?)
            },
            _ => {
                // For Local/S3/DbOps, fallback to environment variable or in-memory
                create_progress_tracker().await
            }
        };

        // Initialize AI service if configured
        let llm_service = if let Some(ai_config) = config.ai_config.clone() {
            let ingestion_config = Self::ai_config_to_ingestion_config(ai_config)?;
            match LlmQueryService::new(ingestion_config) {
                Ok(service) => Some(Arc::new(service)),
                Err(e) => {
                    log::warn!("Failed to initialize AI service: {}. AI query methods will not be available.", e);
                    None
                }
            }
        } else {
            None
        };

        // Initialize Logger based on required configuration
        let logger: Arc<dyn Logger> = match config.logging {
            crate::lambda::config::LambdaLogging::DynamoDb => {
                use crate::logging::outputs::dynamodb::DynamoDbLogger;
                // We need to resolve table name.
                // If storage is DynamoDb, we use that. Otherwise we might fail or default?
                // For now, let's try to extract it from storage config if available.
                let table_name = if let crate::lambda::config::LambdaStorage::Config(crate::storage::StorageConfig::DynamoDb(cfg)) = &config.storage {
                    cfg.tables.logs.clone()
                } else {
                    // Fallback or error?
                    // The config.rs comment says "Table name is now taken from ExplicitTables.logs in StorageConfig"
                    // But if we are in DbOps mode or Local mode but request DynamoDb logging, we don't have it.
                    // Let's assume a default or error. Using a safe default for now to satisfy type checker.
                    "datafold-logs".to_string()
                };
                Arc::new(DynamoDbLogger::new(table_name).await)
            },
            crate::lambda::config::LambdaLogging::Stdout => {
                Arc::new(crate::lambda::logging::StdoutLogger::new())
            },
            crate::lambda::config::LambdaLogging::Custom(logger) => logger,
            crate::lambda::config::LambdaLogging::NoOp => {
                Arc::new(crate::lambda::logging::NoOpLogger::new())
            }
        };

        // Bridge Rust's log crate to our custom logger
        // This captures all internal datafold logging (log::info!(), etc.)
        let log_bridge = LogBridge::new(logger.clone());
        if let Err(e) = log::set_boxed_logger(Box::new(log_bridge)) {
            // It's possible the logger was already set (e.g. during tests).
            // This is not a fatal error, so we just log it to stderr and continue.
            eprintln!("Warning: Failed to set logger (probably already initialized): {}", e);
        }
        log::set_max_level(log::LevelFilter::Info);

        let context = LambdaContext {
            node_manager,
            progress_tracker,
            llm_service,
            logger,
        };

        LAMBDA_CONTEXT
            .set(context)
            .map_err(|_| IngestionError::configuration_error("Context already initialized"))?;

        Ok(())
    }

    /// Convert AIConfig to IngestionConfig for LLM service
    fn ai_config_to_ingestion_config(ai_config: AIConfig) -> Result<IngestionConfig, IngestionError> {
        use crate::ingestion::config::{AIProvider as IngestionAIProvider, OllamaConfig as IngestionOllamaConfig, OpenRouterConfig as IngestionOpenRouterConfig};

        let provider = match ai_config.provider {
            AIProvider::OpenRouter => IngestionAIProvider::OpenRouter,
            AIProvider::Ollama => IngestionAIProvider::Ollama,
        };

        let openrouter = ai_config.openrouter.map(|cfg| IngestionOpenRouterConfig {
            api_key: cfg.api_key,
            model: cfg.model,
            base_url: cfg.base_url.unwrap_or_else(|| "https://openrouter.ai/api/v1".to_string()),
        });

        let ollama = ai_config.ollama.map(|cfg| IngestionOllamaConfig {
            base_url: cfg.base_url,
            model: cfg.model,
        });

        Ok(IngestionConfig {
            provider,
            openrouter: openrouter.unwrap_or_default(),
            ollama: ollama.unwrap_or_default(),
            enabled: true,
            max_retries: ai_config.max_retries,
            timeout_seconds: ai_config.timeout_seconds,
            auto_execute_mutations: false,  // Not used for AI queries
            default_trust_distance: 0,      // Not used for AI queries
        })
    }

    /// Get the global Lambda context.
    ///
    /// Returns an error if the context has not been initialized.
    pub(crate) fn get() -> Result<&'static LambdaContext, IngestionError> {
        LAMBDA_CONTEXT.get().ok_or_else(|| {
            IngestionError::configuration_error(
                "Lambda context not initialized. Call LambdaContext::init() first.",
            )
        })
    }

    /// Get a reference to the DataFold node for the default user.
    ///
    /// Use this to access the node for custom operations in single-tenant mode.
    /// For multi-tenant mode, use `get_node(user_id)`.
    ///
    /// # Example
    ///
    /// ```ignore
    /// use datafold::lambda::LambdaContext;
    ///
    /// async fn handler() -> Result<(), Box<dyn std::error::Error>> {
    ///     let node = LambdaContext::node().await?;
    ///     let node_guard = node.lock().await;
    ///     // Use node...
    ///     Ok(())
    /// }
    /// ```
    pub async fn node() -> Result<Arc<tokio::sync::Mutex<DataFoldNode>>, IngestionError> {
        Self::get()?.node_manager.get_node("default").await
    }

    /// Get a reference to the DataFold node for a specific user.
    ///
    /// Required for multi-tenant deployments using DynamoDB.
    pub async fn get_node(user_id: &str) -> Result<Arc<tokio::sync::Mutex<DataFoldNode>>, IngestionError> {
        Self::get()?.node_manager.get_node(user_id).await
    }

    /// Get a reference to the progress tracker.
    ///
    /// Use this to track ingestion progress.
    ///
    /// # Example
    ///
    /// ```ignore
    /// use datafold::lambda::LambdaContext;
    ///
    /// async fn handler() -> Result<(), Box<dyn std::error::Error>> {
    ///     let tracker = LambdaContext::progress_tracker()?;
    ///     // Use tracker...
    ///     Ok(())
    /// }
    /// ```
    pub fn progress_tracker() -> Result<ProgressTracker, IngestionError> {
        Ok(Self::get()?.progress_tracker.clone())
    }

    /// Get a user-scoped logger.
    ///
    /// This logger will automatically attach the user_id to all logs.
    /// Use this in multi-tenant handlers to ensure logs are correctly attributed.
    pub fn get_user_logger(user_id: &str) -> Result<crate::lambda::logging::UserLogger, IngestionError> {
        let ctx = Self::get()?;
        Ok(crate::lambda::logging::UserLogger::new(
            user_id.to_string(),
            ctx.logger.clone(),
        ))
    }
}
