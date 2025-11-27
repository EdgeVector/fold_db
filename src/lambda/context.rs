//! Lambda context implementation

use crate::datafold_node::{DataFoldNode, NodeConfig};
use crate::datafold_node::llm_query::service::LlmQueryService;
use crate::ingestion::{
    create_progress_tracker, IngestionConfig, IngestionError, ProgressTracker,
};
use crate::lambda::config::{AIConfig, AIProvider, LambdaConfig};
use crate::lambda::logging::{LogBridge, Logger};
use once_cell::sync::OnceCell;
use std::sync::Arc;

/// Lambda context that manages all required state.
///
/// This should be initialized once during Lambda cold start and reused
/// across all invocations to minimize latency.
pub struct LambdaContext {
    pub(crate) node: Arc<tokio::sync::Mutex<DataFoldNode>>,
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
    ///     let config = LambdaConfig::new()
    ///         .with_schema_service_url("https://schema.example.com".to_string());
    ///     
    ///     LambdaContext::init(config)
    ///         .await
    ///         .expect("Failed to initialize");
    /// }
    /// ```
    pub async fn init(config: LambdaConfig) -> Result<(), IngestionError> {
        // Use custom storage path or default to /tmp/folddb
        let storage_path = config
            .storage_path
            .unwrap_or_else(|| std::env::temp_dir().join("folddb"));

        std::fs::create_dir_all(&storage_path)
            .map_err(|e| IngestionError::StorageError(e.to_string()))?;

        // Initialize node config
        let mut node_config = NodeConfig::new(storage_path);

        // Set schema service URL if provided
        if let Some(schema_url) = config.schema_service_url {
            node_config = node_config.with_schema_service_url(&schema_url);
        }

        // Create DataFold node
        let node = DataFoldNode::new(node_config).await
            .map_err(|e| IngestionError::InvalidInput(e.to_string()))?;

        // Create progress tracker
        let progress_tracker = create_progress_tracker();

        // Initialize AI service if configured
        let llm_service = if let Some(ai_config) = config.ai_config {
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

        // Logger is required - fail if not provided
        let logger = config.logger.ok_or_else(|| {
            IngestionError::configuration_error(
                "Logger is required for LambdaContext. Add .with_logger(Arc::new(StdoutLogger)) to your LambdaConfig. See LAMBDA_LOGGING_QUICKSTART.md for details."
            )
        })?;

        // Bridge Rust's log crate to our custom logger
        // This captures all internal datafold logging (log::info!(), etc.)
        let log_bridge = LogBridge::new(logger.clone());
        log::set_boxed_logger(Box::new(log_bridge))
            .map_err(|e| IngestionError::configuration_error(format!("Failed to set logger: {}", e)))?;
        log::set_max_level(log::LevelFilter::Info);

        let context = LambdaContext {
            node: Arc::new(tokio::sync::Mutex::new(node)),
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

    /// Get a reference to the DataFold node.
    ///
    /// Use this to access the node for custom operations.
    ///
    /// # Example
    ///
    /// ```ignore
    /// use datafold::lambda::LambdaContext;
    ///
    /// async fn handler() -> Result<(), Box<dyn std::error::Error>> {
    ///     let node = LambdaContext::node()?;
    ///     let node_guard = node.lock().await;
    ///     // Use node...
    ///     Ok(())
    /// }
    /// ```
    pub fn node() -> Result<Arc<tokio::sync::Mutex<DataFoldNode>>, IngestionError> {
        Ok(Self::get()?.node.clone())
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
}
