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
use crate::datafold_node::llm_query::{session::SessionManager, service::LlmQueryService};
use crate::datafold_node::OperationProcessor;
use crate::fold_db_core::query::records_from_field_map;
use crate::ingestion::core::IngestionRequest;
use crate::ingestion::progress::ProgressService;
use crate::ingestion::simple_service::SimpleIngestionService;
use crate::ingestion::{
    create_progress_tracker, IngestionConfig, IngestionError, IngestionProgress, 
    IngestionResponse, ProgressTracker,
};
use once_cell::sync::OnceCell;
use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::path::PathBuf;
use std::sync::Arc;
use std::time::{SystemTime, UNIX_EPOCH};

/// Configuration for Lambda context initialization
#[derive(Debug, Clone, Default)]
pub struct LambdaConfig {
    /// Optional custom storage path (defaults to /tmp/folddb)
    pub storage_path: Option<PathBuf>,
    /// Optional schema service URL
    pub schema_service_url: Option<String>,
    /// Optional AI configuration for query capabilities
    pub ai_config: Option<AIConfig>,
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

/// Context for stateless follow-up queries
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct QueryContext {
    pub original_query: String,
    pub query_results: Vec<Value>,
    pub conversation_history: Vec<ConversationMessage>,
    pub query_plan: Option<QueryPlanInfo>,
}

/// Conversation message for context
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ConversationMessage {
    pub role: String,
    pub content: String,
    pub timestamp: u64,
}

/// Query plan information
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct QueryPlanInfo {
    pub schema_name: String,
    pub fields: Vec<String>,
    pub filter_type: Option<String>,
    pub reasoning: String,
}

/// Response from AI query
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AIQueryResponse {
    pub ai_interpretation: String,
    pub raw_results: Vec<Value>,
    pub context: QueryContext,
}

/// Complete query response with planning details
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CompleteQueryResponse {
    pub query_plan: QueryPlanInfo,
    pub results: Vec<Value>,
    pub summary: Option<String>,
    pub context: QueryContext,
}

/// Request for follow-up question
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FollowupRequest {
    pub context: QueryContext,
    pub question: String,
}

/// Response from follow-up
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FollowupResponse {
    pub answer: String,
    pub executed_new_query: bool,
    pub context: QueryContext,
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

    /// Enable AI query functionality with OpenRouter
    ///
    /// # Example
    ///
    /// ```ignore
    /// use datafold::lambda::LambdaConfig;
    ///
    /// let config = LambdaConfig::new()
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
    /// use datafold::lambda::LambdaConfig;
    ///
    /// let config = LambdaConfig::new()
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

/// Lambda context that manages all required state.
///
/// This should be initialized once during Lambda cold start and reused
/// across all invocations to minimize latency.
pub struct LambdaContext {
    node: Arc<tokio::sync::Mutex<DataFoldNode>>,
    progress_tracker: ProgressTracker,
    llm_service: Option<Arc<LlmQueryService>>,
    session_manager: Arc<SessionManager>,
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

        // Create session manager
        let session_manager = Arc::new(SessionManager::new());

        let context = LambdaContext {
            node: Arc::new(tokio::sync::Mutex::new(node)),
            progress_tracker,
            llm_service,
            session_manager,
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

    /// Execute an AI-native index query using semantic search
    ///
    /// This is the simplest method - provide a natural language query and get
    /// AI-interpreted results. Fully stateless.
    ///
    /// # Arguments
    ///
    /// * `query` - Natural language query
    ///
    /// # Example
    ///
    /// ```ignore
    /// use datafold::lambda::LambdaContext;
    ///
    /// async fn handler() -> Result<(), Box<dyn std::error::Error>> {
    ///     let response = LambdaContext::ai_query("Find all electronics products").await?;
    ///     println!("AI says: {}", response.ai_interpretation);
    ///     println!("Found {} results", response.raw_results.len());
    ///     Ok(())
    /// }
    /// ```
    pub async fn ai_query(query: &str) -> Result<AIQueryResponse, IngestionError> {
        let ctx = Self::get()?;
        
        // Check if AI is configured
        let service = ctx.llm_service.as_ref()
            .ok_or_else(|| IngestionError::configuration_error(
                "AI query not configured. Initialize LambdaContext with ai_config using .with_openrouter() or .with_ollama()."
            ))?;

        // Get available schemas
        let schemas = {
            let node = ctx.node.lock().await;
            let db_guard = node.get_fold_db()
                .map_err(|e| IngestionError::InvalidInput(format!("Failed to access database: {}", e)))?;
            db_guard.schema_manager.get_schemas_with_states()
                .map_err(|e| IngestionError::InvalidInput(format!("Failed to get schemas: {}", e)))?
        };

        // Execute AI-native index query workflow
        let (ai_interpretation, raw_results) = {
            let node = ctx.node.lock().await;
            let db_ops = node.get_fold_db()
                .map_err(|e| IngestionError::InvalidInput(format!("Failed to access database: {}", e)))?
                .get_db_ops();
            drop(node); // Drop lock before await
            
            service.execute_ai_native_index_query_with_results(query, &schemas, &db_ops).await
                .map_err(|e| IngestionError::InvalidInput(format!("AI query failed: {}", e)))?
        };

        // Convert results to JSON
        let results_as_json: Vec<Value> = raw_results
            .into_iter()
            .map(|result| serde_json::to_value(result).unwrap_or(serde_json::json!({})))
            .collect();

        // Build context for potential follow-ups
        let context = QueryContext {
            original_query: query.to_string(),
            query_results: results_as_json.clone(),
            conversation_history: vec![
                ConversationMessage {
                    role: "user".to_string(),
                    content: query.to_string(),
                    timestamp: SystemTime::now()
                        .duration_since(UNIX_EPOCH)
                        .unwrap_or_default()
                        .as_secs(),
                },
                ConversationMessage {
                    role: "assistant".to_string(),
                    content: ai_interpretation.clone(),
                    timestamp: SystemTime::now()
                        .duration_since(UNIX_EPOCH)
                        .unwrap_or_default()
                        .as_secs(),
                },
            ],
            query_plan: None,
        };

        Ok(AIQueryResponse {
            ai_interpretation,
            raw_results: results_as_json,
            context,
        })
    }

    /// Run complete AI query workflow: analyze + execute + summarize
    ///
    /// This method handles the full workflow internally and waits for any
    /// necessary backfills to complete before returning results. Fully stateless.
    ///
    /// # Arguments
    ///
    /// * `query` - Natural language query
    ///
    /// # Example
    ///
    /// ```ignore
    /// use datafold::lambda::LambdaContext;
    ///
    /// async fn handler() -> Result<(), Box<dyn std::error::Error>> {
    ///     let response = LambdaContext::run_ai_query("Show recent blog posts").await?;
    ///     println!("Found {} records", response.results.len());
    ///     if let Some(summary) = response.summary {
    ///         println!("Summary: {}", summary);
    ///     }
    ///     Ok(())
    /// }
    /// ```
    pub async fn run_ai_query(query: &str) -> Result<CompleteQueryResponse, IngestionError> {
        let ctx = Self::get()?;
        
        // Check if AI is configured
        let service = ctx.llm_service.as_ref()
            .ok_or_else(|| IngestionError::configuration_error(
                "AI query not configured. Initialize LambdaContext with ai_config using .with_openrouter() or .with_ollama()."
            ))?;

        // Get available schemas
        let schemas = {
            let node = ctx.node.lock().await;
            let db_guard = node.get_fold_db()
                .map_err(|e| IngestionError::InvalidInput(format!("Failed to access database: {}", e)))?;
            db_guard.schema_manager.get_schemas_with_states()
                .map_err(|e| IngestionError::InvalidInput(format!("Failed to get schemas: {}", e)))?
        };

        // Analyze query with LLM
        let query_plan = service.analyze_query(query, &schemas).await
            .map_err(|e| IngestionError::InvalidInput(format!("Failed to analyze query: {}", e)))?;

        // Execute the query
        let node_arc = Arc::clone(&ctx.node);
        let processor = OperationProcessor::new(node_arc);
        let results = match processor.execute_query_map(query_plan.query.clone()).await {
            Ok(result_map) => {
                let records_map = records_from_field_map(&result_map);
                records_map
                    .into_iter()
                    .map(|(key, record)| serde_json::json!({"key": key, "fields": record.fields, "metadata": record.metadata}))
                    .collect::<Vec<Value>>()
            }
            Err(e) => {
                return Err(IngestionError::InvalidInput(format!("Failed to execute query: {}", e)));
            }
        };

        // Summarize results with LLM
        let summary = service.summarize_results(query, &results).await.ok();

        // Build query plan info
        let filter_type = query_plan.query.filter.as_ref().map(|f| format!("{:?}", f));
        let query_plan_info = QueryPlanInfo {
            schema_name: query_plan.query.schema_name.clone(),
            fields: query_plan.query.fields.clone(),
            filter_type,
            reasoning: query_plan.reasoning.clone(),
        };

        // Build context for follow-ups
        let mut conversation_history = vec![
            ConversationMessage {
                role: "user".to_string(),
                content: query.to_string(),
                timestamp: SystemTime::now()
                    .duration_since(UNIX_EPOCH)
                    .unwrap_or_default()
                    .as_secs(),
            },
        ];

        if let Some(ref s) = summary {
            conversation_history.push(ConversationMessage {
                role: "assistant".to_string(),
                content: s.clone(),
                timestamp: SystemTime::now()
                    .duration_since(UNIX_EPOCH)
                    .unwrap_or_default()
                    .as_secs(),
            });
        }

        let context = QueryContext {
            original_query: query.to_string(),
            query_results: results.clone(),
            conversation_history,
            query_plan: Some(query_plan_info.clone()),
        };

        Ok(CompleteQueryResponse {
            query_plan: query_plan_info,
            results,
            summary,
            context,
        })
    }

    /// Ask a follow-up question about previous query results
    ///
    /// Completely stateless - client provides full context from previous query.
    ///
    /// # Arguments
    ///
    /// * `request` - Follow-up request with context and question
    ///
    /// # Example
    ///
    /// ```ignore
    /// use datafold::lambda::{LambdaContext, FollowupRequest};
    ///
    /// async fn handler() -> Result<(), Box<dyn std::error::Error>> {
    ///     // First query
    ///     let response1 = LambdaContext::run_ai_query("Show all products").await?;
    ///     
    ///     // Follow-up question
    ///     let followup = LambdaContext::ask_followup(FollowupRequest {
    ///         context: response1.context,
    ///         question: "Which are electronics?".to_string(),
    ///     }).await?;
    ///     
    ///     println!("Answer: {}", followup.answer);
    ///     Ok(())
    /// }
    /// ```
    pub async fn ask_followup(request: FollowupRequest) -> Result<FollowupResponse, IngestionError> {
        let ctx = Self::get()?;
        
        // Check if AI is configured
        let service = ctx.llm_service.as_ref()
            .ok_or_else(|| IngestionError::configuration_error(
                "AI query not configured. Initialize LambdaContext with ai_config using .with_openrouter() or .with_ollama()."
            ))?;

        let context = request.context;
        let question = request.question;

        // Get available schemas
        let schemas = {
            let node = ctx.node.lock().await;
            let db_guard = node.get_fold_db()
                .map_err(|e| IngestionError::InvalidInput(format!("Failed to access database: {}", e)))?;
            db_guard.schema_manager.get_schemas_with_states()
                .map_err(|e| IngestionError::InvalidInput(format!("Failed to get schemas: {}", e)))?
        };

        // Convert conversation history to Message format
        let conversation_history: Vec<crate::datafold_node::llm_query::types::Message> = context
            .conversation_history
            .iter()
            .map(|msg| crate::datafold_node::llm_query::types::Message {
                role: msg.role.clone(),
                content: msg.content.clone(),
                timestamp: SystemTime::UNIX_EPOCH + std::time::Duration::from_secs(msg.timestamp),
            })
            .collect();

        // Analyze if follow-up needs a new query
        let analysis = service
            .analyze_followup_question(
                &context.original_query,
                &context.query_results,
                &question,
                &schemas,
            )
            .await
            .map_err(|e| IngestionError::InvalidInput(format!("Failed to analyze followup: {}", e)))?;

        let mut combined_results = context.query_results.clone();
        let mut executed_new_query = false;

        // If a new query is needed, execute it
        if analysis.needs_query {
            if let Some(new_query) = analysis.query {
                executed_new_query = true;
                let node_arc = Arc::clone(&ctx.node);
                let processor = OperationProcessor::new(node_arc);
                match processor.execute_query_map(new_query).await {
                    Ok(result_map) => {
                        let records_map = records_from_field_map(&result_map);
                        combined_results = records_map
                            .into_iter()
                            .map(|(key, record)| serde_json::json!({"key": key, "fields": record.fields, "metadata": record.metadata}))
                            .collect();
                    }
                    Err(e) => {
                        log::warn!("Failed to execute followup query: {}", e);
                    }
                }
            }
        }

        // Get answer from AI
        let answer = service
            .answer_question(
                &context.original_query,
                &combined_results,
                &conversation_history,
                &question,
            )
            .await
            .map_err(|e| IngestionError::InvalidInput(format!("Failed to get answer: {}", e)))?;

        // Build updated context
        let mut updated_conversation = context.conversation_history.clone();
        updated_conversation.push(ConversationMessage {
            role: "user".to_string(),
            content: question.clone(),
            timestamp: SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .unwrap_or_default()
                .as_secs(),
        });
        updated_conversation.push(ConversationMessage {
            role: "assistant".to_string(),
            content: answer.clone(),
            timestamp: SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .unwrap_or_default()
                .as_secs(),
        });

        let updated_context = QueryContext {
            original_query: context.original_query,
            query_results: combined_results,
            conversation_history: updated_conversation,
            query_plan: context.query_plan,
        };

        Ok(FollowupResponse {
            answer,
            executed_new_query,
            context: updated_context,
        })
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
            ai_config: None,
        };
        
        assert_eq!(config.storage_path, Some(path));
        assert_eq!(config.schema_service_url, Some(url));
        assert!(config.ai_config.is_none());
    }

    // Note: Context initialization tests are in integration tests
    // since OnceCell can only be initialized once per test run

    #[test]
    fn test_lambda_config_with_openrouter() {
        let config = LambdaConfig::new()
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
        let config = LambdaConfig::new()
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
        let config = LambdaConfig::new()
            .with_storage_path(PathBuf::from("/tmp/test"))
            .with_schema_service_url("https://schema.example.com".to_string())
            .with_openrouter("key".to_string(), "model".to_string());
        
        assert_eq!(config.storage_path, Some(PathBuf::from("/tmp/test")));
        assert_eq!(config.schema_service_url, Some("https://schema.example.com".to_string()));
        assert!(config.ai_config.is_some());
    }

    #[test]
    fn test_query_context_serialization() {
        let context = QueryContext {
            original_query: "test query".to_string(),
            query_results: vec![serde_json::json!({"key": "value"})],
            conversation_history: vec![
                ConversationMessage {
                    role: "user".to_string(),
                    content: "Hello".to_string(),
                    timestamp: 1234567890,
                }
            ],
            query_plan: None,
        };
        
        // Should serialize and deserialize without errors
        let json = serde_json::to_string(&context).unwrap();
        let deserialized: QueryContext = serde_json::from_str(&json).unwrap();
        
        assert_eq!(deserialized.original_query, "test query");
        assert_eq!(deserialized.conversation_history.len(), 1);
        assert_eq!(deserialized.conversation_history[0].role, "user");
    }

    #[test]
    fn test_followup_request_serialization() {
        let context = QueryContext {
            original_query: "original".to_string(),
            query_results: vec![],
            conversation_history: vec![],
            query_plan: None,
        };
        
        let request = FollowupRequest {
            context,
            question: "follow-up question".to_string(),
        };
        
        let json = serde_json::to_string(&request).unwrap();
        let deserialized: FollowupRequest = serde_json::from_str(&json).unwrap();
        
        assert_eq!(deserialized.question, "follow-up question");
        assert_eq!(deserialized.context.original_query, "original");
    }

    #[test]
    fn test_ai_query_response_structure() {
        let context = QueryContext {
            original_query: "test".to_string(),
            query_results: vec![],
            conversation_history: vec![],
            query_plan: None,
        };
        
        let response = AIQueryResponse {
            ai_interpretation: "AI response".to_string(),
            raw_results: vec![serde_json::json!({"test": "data"})],
            context,
        };
        
        assert_eq!(response.ai_interpretation, "AI response");
        assert_eq!(response.raw_results.len(), 1);
        assert_eq!(response.context.original_query, "test");
    }

    #[test]
    fn test_complete_query_response_structure() {
        let query_plan = QueryPlanInfo {
            schema_name: "TestSchema".to_string(),
            fields: vec!["field1".to_string(), "field2".to_string()],
            filter_type: Some("HashKey".to_string()),
            reasoning: "Test reasoning".to_string(),
        };
        
        let context = QueryContext {
            original_query: "test".to_string(),
            query_results: vec![],
            conversation_history: vec![],
            query_plan: Some(query_plan.clone()),
        };
        
        let response = CompleteQueryResponse {
            query_plan,
            results: vec![],
            summary: Some("Test summary".to_string()),
            context,
        };
        
        assert_eq!(response.query_plan.schema_name, "TestSchema");
        assert_eq!(response.summary, Some("Test summary".to_string()));
        assert!(response.context.query_plan.is_some());
    }

    #[test]
    fn test_conversation_message_timestamp() {
        let msg = ConversationMessage {
            role: "assistant".to_string(),
            content: "response".to_string(),
            timestamp: 1700000000,
        };
        
        assert_eq!(msg.timestamp, 1700000000);
        assert_eq!(msg.role, "assistant");
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

    #[test]
    fn test_query_plan_info_serialization() {
        let plan = QueryPlanInfo {
            schema_name: "BlogPost".to_string(),
            fields: vec!["title".to_string(), "content".to_string()],
            filter_type: Some("RangePrefix".to_string()),
            reasoning: "Using BlogPost schema for efficiency".to_string(),
        };
        
        let json = serde_json::to_string(&plan).unwrap();
        let deserialized: QueryPlanInfo = serde_json::from_str(&json).unwrap();
        
        assert_eq!(deserialized.schema_name, "BlogPost");
        assert_eq!(deserialized.fields.len(), 2);
        assert_eq!(deserialized.filter_type, Some("RangePrefix".to_string()));
    }

    #[test]
    fn test_followup_response_with_new_query() {
        let context = QueryContext {
            original_query: "original".to_string(),
            query_results: vec![serde_json::json!({"new": "data"})],
            conversation_history: vec![
                ConversationMessage {
                    role: "user".to_string(),
                    content: "question".to_string(),
                    timestamp: 1234567890,
                },
                ConversationMessage {
                    role: "assistant".to_string(),
                    content: "answer".to_string(),
                    timestamp: 1234567891,
                },
            ],
            query_plan: None,
        };
        
        let response = FollowupResponse {
            answer: "Here's the answer".to_string(),
            executed_new_query: true,
            context,
        };
        
        assert_eq!(response.executed_new_query, true);
        assert_eq!(response.context.conversation_history.len(), 2);
        assert_eq!(response.context.query_results.len(), 1);
    }
}
