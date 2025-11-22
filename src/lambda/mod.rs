//! Lambda-optimized API
//!
//! This module provides a simplified interface for AWS Lambda functions
//! that eliminates complex initialization and reduces cold start times.
//!
//! # Quick Start
//!
//! ```ignore
//! use datafold::lambda::{LambdaContext, LambdaConfig};
//!
//! #[tokio::main]
//! async fn main() {
//!     // Initialize once during cold start
//!     let config = LambdaConfig::new();
//!     LambdaContext::init(config).await.expect("Failed to initialize");
//!     
//!     // Access node in handler
//!     let node = LambdaContext::node();
//!     // Use node for operations...
//! }
//! ```

use crate::datafold_node::{DataFoldNode, NodeConfig};
use crate::ingestion::core::IngestionRequest;
use crate::ingestion::progress::ProgressService;
use crate::ingestion::simple_service::SimpleIngestionService;
use crate::ingestion::{
    create_progress_tracker, IngestionConfig, IngestionError, IngestionProgress, 
    IngestionResponse, ProgressTracker,
};
use once_cell::sync::OnceCell;
use serde_json::Value;
use std::path::PathBuf;
use std::sync::Arc;

/// Configuration for Lambda context initialization
#[derive(Debug, Clone, Default)]
pub struct LambdaConfig {
    /// Optional custom storage path (defaults to /tmp/folddb)
    pub storage_path: Option<PathBuf>,
    /// Optional schema service URL
    pub schema_service_url: Option<String>,
}

impl LambdaConfig {
    /// Create a new Lambda configuration with defaults
    ///
    /// # Example
    ///
    /// ```ignore
    /// use datafold::lambda::LambdaConfig;
    ///
    /// let config = LambdaConfig::new();
    /// ```
    pub fn new() -> Self {
        Self::default()
    }

    /// Set a custom storage path (default: /tmp/folddb)
    pub fn with_storage_path(mut self, path: PathBuf) -> Self {
        self.storage_path = Some(path);
        self
    }

    /// Set the schema service URL
    pub fn with_schema_service_url(mut self, url: String) -> Self {
        self.schema_service_url = Some(url);
        self
    }
}

/// Lambda context that manages all required state.
///
/// This should be initialized once during Lambda cold start and reused
/// across all invocations to minimize latency.
pub struct LambdaContext {
    node: Arc<tokio::sync::Mutex<DataFoldNode>>,
    progress_tracker: ProgressTracker,
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
        let node = DataFoldNode::new(node_config)
            .map_err(|e| IngestionError::InvalidInput(e.to_string()))?;

        // Create progress tracker
        let progress_tracker = create_progress_tracker();

        let context = LambdaContext {
            node: Arc::new(tokio::sync::Mutex::new(node)),
            progress_tracker,
        };

        LAMBDA_CONTEXT
            .set(context)
            .map_err(|_| IngestionError::configuration_error("Context already initialized"))?;

        Ok(())
    }

    /// Get the global Lambda context.
    ///
    /// Returns an error if the context has not been initialized.
    fn get() -> Result<&'static LambdaContext, IngestionError> {
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

    /// Get ingestion progress by ID.
    ///
    /// # Arguments
    ///
    /// * `progress_id` - The progress ID from an ingestion operation
    ///
    /// # Returns
    ///
    /// Returns `Some(IngestionProgress)` if found, or `None` if the ID is not found.
    ///
    /// # Example
    ///
    /// ```ignore
    /// use datafold::lambda::LambdaContext;
    ///
    /// async fn check_progress(progress_id: &str) -> Result<(), Box<dyn std::error::Error>> {
    ///     if let Some(progress) = LambdaContext::get_progress(progress_id)? {
    ///         println!("Current step: {:?}", progress.current_step);
    ///         println!("Completed: {}", progress.completed);
    ///     }
    ///     Ok(())
    /// }
    /// ```
    pub fn get_progress(progress_id: &str) -> Result<Option<IngestionProgress>, IngestionError> {
        let ctx = Self::get()?;
        let tracker = ctx.progress_tracker.lock().map_err(|_| {
            IngestionError::InvalidInput("Failed to lock progress tracker".to_string())
        })?;
        Ok(tracker.get(progress_id).cloned())
    }

    /// Ingest JSON data asynchronously (returns immediately with progress_id)
    ///
    /// This function processes JSON data in the background and returns a progress_id
    /// that can be used to track the ingestion status.
    ///
    /// # Arguments
    ///
    /// * `json_data` - The JSON data to ingest (array of objects or single object)
    /// * `auto_execute` - Whether to execute mutations after generation
    /// * `trust_distance` - Trust distance for mutations (default: 0)
    /// * `pub_key` - Public key for mutations (default: "default")
    ///
    /// # Example
    ///
    /// ```ignore
    /// use datafold::lambda::LambdaContext;
    /// use serde_json::json;
    ///
    /// async fn handler() -> Result<(), Box<dyn std::error::Error>> {
    ///     let data = json!([
    ///         {"id": 1, "name": "Alice"},
    ///         {"id": 2, "name": "Bob"}
    ///     ]);
    ///     
    ///     let progress_id = LambdaContext::ingest_json(data, true, 0, "default".to_string()).await?;
    ///     
    ///     println!("Started ingestion: {}", progress_id);
    ///     Ok(())
    /// }
    /// ```
    pub async fn ingest_json(
        json_data: Value,
        auto_execute: bool,
        trust_distance: u32,
        pub_key: String,
    ) -> Result<String, IngestionError> {
        let ctx = Self::get()?;
        let node = ctx.node.clone();
        let progress_tracker = ctx.progress_tracker.clone();

        // Generate unique progress ID
        let progress_id = uuid::Uuid::new_v4().to_string();

        // Start progress tracking
        let progress_service = ProgressService::new(progress_tracker);
        progress_service.start_progress(progress_id.clone());

        // Load ingestion config
        let config = IngestionConfig::from_env()?;

        // Create ingestion request
        let request = IngestionRequest {
            data: json_data,
            auto_execute: Some(auto_execute),
            trust_distance: Some(trust_distance),
            pub_key: Some(pub_key),
            source_file_name: None,
        };

        // Clone for background task
        let progress_id_clone = progress_id.clone();

        // Spawn background ingestion task
        tokio::spawn(async move {
            // Create ingestion service
            let service = match SimpleIngestionService::new(config) {
                Ok(service) => service,
                Err(e) => {
                    progress_service.fail_progress(
                        &progress_id_clone,
                        format!("Failed to create ingestion service: {}", e),
                    );
                    return;
                }
            };

            // Process ingestion
            match service
                .process_json_with_node_and_progress(
                    request,
                    node,
                    &progress_service,
                    progress_id_clone.clone(),
                )
                .await
            {
                Ok(_) => {}
                Err(e) => {
                    progress_service.fail_progress(
                        &progress_id_clone,
                        format!("Ingestion failed: {}", e),
                    );
                }
            }
        });

        Ok(progress_id)
    }

    /// Ingest JSON data synchronously (waits for completion)
    ///
    /// This function processes JSON data and waits for completion before returning.
    /// Use this when you need the full ingestion results immediately.
    ///
    /// # Arguments
    ///
    /// * `json_data` - The JSON data to ingest (array of objects or single object)
    /// * `auto_execute` - Whether to execute mutations after generation
    /// * `trust_distance` - Trust distance for mutations (default: 0)
    /// * `pub_key` - Public key for mutations (default: "default")
    ///
    /// # Example
    ///
    /// ```ignore
    /// use datafold::lambda::LambdaContext;
    /// use serde_json::json;
    ///
    /// async fn handler() -> Result<(), Box<dyn std::error::Error>> {
    ///     let data = json!([
    ///         {"id": 1, "name": "Alice"},
    ///         {"id": 2, "name": "Bob"}
    ///     ]);
    ///     
    ///     let response = LambdaContext::ingest_json_sync(data, true, 0, "default".to_string()).await?;
    ///     
    ///     println!("Ingested {} mutations", response.mutations_executed);
    ///     Ok(())
    /// }
    /// ```
    pub async fn ingest_json_sync(
        json_data: Value,
        auto_execute: bool,
        trust_distance: u32,
        pub_key: String,
    ) -> Result<IngestionResponse, IngestionError> {
        let ctx = Self::get()?;
        let node = ctx.node.clone();
        let progress_tracker = ctx.progress_tracker.clone();

        // Generate unique progress ID
        let progress_id = uuid::Uuid::new_v4().to_string();

        // Start progress tracking
        let progress_service = ProgressService::new(progress_tracker);
        progress_service.start_progress(progress_id.clone());

        // Load ingestion config
        let config = IngestionConfig::from_env()?;

        // Create ingestion service
        let service = SimpleIngestionService::new(config)?;

        // Create ingestion request
        let request = IngestionRequest {
            data: json_data,
            auto_execute: Some(auto_execute),
            trust_distance: Some(trust_distance),
            pub_key: Some(pub_key),
            source_file_name: None,
        };

        // Process synchronously
        service
            .process_json_with_node_and_progress(request, node, &progress_service, progress_id)
            .await
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_lambda_config_creation() {
        let config = LambdaConfig::new();
        assert!(config.storage_path.is_none());
        assert!(config.schema_service_url.is_none());
    }

    #[test]
    fn test_lambda_config_default() {
        let config = LambdaConfig::default();
        assert!(config.storage_path.is_none());
        assert!(config.schema_service_url.is_none());
    }

    #[test]
    fn test_lambda_config_with_storage_path() {
        let path = PathBuf::from("/tmp/custom_path");
        let config = LambdaConfig::new().with_storage_path(path.clone());
        assert_eq!(config.storage_path, Some(path));
    }

    #[test]
    fn test_lambda_config_with_schema_service_url() {
        let url = "https://schema.example.com".to_string();
        let config = LambdaConfig::new().with_schema_service_url(url.clone());
        assert_eq!(config.schema_service_url, Some(url));
    }

    #[test]
    fn test_lambda_config_builder_pattern() {
        let path = PathBuf::from("/tmp/test");
        let url = "https://schema.example.com".to_string();
        
        let config = LambdaConfig::new()
            .with_storage_path(path.clone())
            .with_schema_service_url(url.clone());
        
        assert_eq!(config.storage_path, Some(path));
        assert_eq!(config.schema_service_url, Some(url));
    }

    #[test]
    fn test_lambda_config_debug_impl() {
        let config = LambdaConfig::new()
            .with_storage_path(PathBuf::from("/tmp/test"));
        
        let debug_str = format!("{:?}", config);
        assert!(debug_str.contains("LambdaConfig"));
    }

    #[test]
    fn test_lambda_config_clone() {
        let config1 = LambdaConfig::new()
            .with_storage_path(PathBuf::from("/tmp/test"))
            .with_schema_service_url("https://example.com".to_string());
        
        let config2 = config1.clone();
        
        assert_eq!(config1.storage_path, config2.storage_path);
        assert_eq!(config1.schema_service_url, config2.schema_service_url);
    }

    #[test]
    fn test_lambda_config_with_both_options() {
        let path = PathBuf::from("/tmp/lambda_test");
        let url = "https://schema.service.com".to_string();
        
        let config = LambdaConfig {
            storage_path: Some(path.clone()),
            schema_service_url: Some(url.clone()),
        };
        
        assert_eq!(config.storage_path, Some(path));
        assert_eq!(config.schema_service_url, Some(url));
    }

    // Note: Context initialization tests are in integration tests
    // since OnceCell can only be initialized once per test run
}
