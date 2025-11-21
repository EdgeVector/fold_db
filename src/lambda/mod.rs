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
use crate::ingestion::{create_progress_tracker, IngestionError, IngestionProgress, ProgressTracker};
use once_cell::sync::OnceCell;
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
}
