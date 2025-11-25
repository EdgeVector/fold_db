//! Lambda context implementation

use crate::datafold_node::{DataFoldNode, NodeConfig};
use crate::datafold_node::llm_query::service::LlmQueryService;
use crate::datafold_node::OperationProcessor;
use crate::fold_db_core::query::records_from_field_map;
use crate::ingestion::core::IngestionRequest;
use crate::ingestion::progress::ProgressService;
use crate::ingestion::simple_service::SimpleIngestionService;
use crate::ingestion::{
    create_progress_tracker, IngestionConfig, IngestionError, IngestionProgress, 
    IngestionResponse, ProgressTracker,
};
use crate::lambda::config::{AIConfig, AIProvider, LambdaConfig};
use crate::lambda::logging::{LogBridge, LogEntry, Logger, NoOpLogger, UserLogger};
use crate::lambda::types::{
    AIQueryResponse, CompleteQueryResponse, ConversationMessage, FollowupRequest, 
    FollowupResponse, QueryContext, QueryPlanInfo,
};
use once_cell::sync::OnceCell;
use serde_json::Value;
use std::sync::Arc;
use std::time::{SystemTime, UNIX_EPOCH};

/// Lambda context that manages all required state.
///
/// This should be initialized once during Lambda cold start and reused
/// across all invocations to minimize latency.
pub struct LambdaContext {
    node: Arc<tokio::sync::Mutex<DataFoldNode>>,
    progress_tracker: ProgressTracker,
    llm_service: Option<Arc<LlmQueryService>>,
    logger: Arc<dyn Logger>,
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

        // Use provided logger or default to NoOpLogger
        let logger = config.logger.unwrap_or_else(|| Arc::new(NoOpLogger));

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

    /// Create a user-scoped logger
    ///
    /// Returns a logger that automatically includes the user_id in all log entries.
    ///
    /// # Example
    ///
    /// ```ignore
    /// use datafold::lambda::LambdaContext;
    ///
    /// async fn handler(event: LambdaEvent<Value>) -> Result<Value, Error> {
    ///     let user_id = event.payload["user_id"].as_str().unwrap_or("anonymous");
    ///     let logger = LambdaContext::create_logger(user_id)?;
    ///     
    ///     logger.info("request_started", "Processing your request").await?;
    ///     // Your business logic...
    ///     logger.info("request_completed", "Request completed successfully").await?;
    ///     
    ///     Ok(json!({ "statusCode": 200 }))
    /// }
    /// ```
    pub fn create_logger(user_id: &str) -> Result<UserLogger, IngestionError> {
        let ctx = Self::get()?;
        Ok(UserLogger::new(user_id.to_string(), ctx.logger.clone()))
    }

    /// Query logs for a specific user
    ///
    /// Returns logs from the configured logger backend, if the logger supports querying.
    ///
    /// # Example
    ///
    /// ```ignore
    /// use datafold::lambda::LambdaContext;
    ///
    /// async fn get_user_logs() -> Result<(), Box<dyn std::error::Error>> {
    ///     let logs = LambdaContext::query_logs(
    ///         "user_123",
    ///         Some(100),  // limit
    ///         None        // from_timestamp
    ///     ).await?;
    ///     
    ///     for log in logs {
    ///         println!("{}: {} - {}", log.timestamp, log.event_type, log.message);
    ///     }
    ///     Ok(())
    /// }
    /// ```
    pub async fn query_logs(
        user_id: &str,
        limit: Option<usize>,
        from_timestamp: Option<i64>,
    ) -> Result<Vec<LogEntry>, IngestionError> {
        let ctx = Self::get()?;
        ctx.logger.query(user_id, limit, from_timestamp).await
            .map_err(|e| IngestionError::InvalidInput(format!("Failed to query logs: {}", e)))
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

    /// Execute a query and return results
    ///
    /// This is for regular (non-AI) queries where you know the schema and fields.
    ///
    /// # Arguments
    ///
    /// * `query` - Query specification with schema name, fields, and optional filter
    ///
    /// # Example
    ///
    /// ```ignore
    /// use datafold::lambda::LambdaContext;
    /// use datafold::schema::types::Query;
    ///
    /// async fn handler() -> Result<(), Box<dyn std::error::Error>> {
    ///     let query = Query {
    ///         schema_name: "users".to_string(),
    ///         fields: vec!["name".to_string(), "email".to_string()],
    ///         filter: None,
    ///     };
    ///     
    ///     let results = LambdaContext::query(query).await?;
    ///     println!("Found {} records", results.len());
    ///     Ok(())
    /// }
    /// ```
    pub async fn query(query: crate::schema::types::Query) -> Result<Vec<Value>, IngestionError> {
        let ctx = Self::get()?;
        let node_arc = Arc::clone(&ctx.node);
        let processor = OperationProcessor::new(node_arc);
        
        match processor.execute_query_map(query).await {
            Ok(result_map) => {
                let records_map = records_from_field_map(&result_map);
                let results: Vec<Value> = records_map
                    .into_iter()
                    .map(|(key, record)| serde_json::json!({
                        "key": key,
                        "fields": record.fields,
                        "metadata": record.metadata
                    }))
                    .collect();
                Ok(results)
            }
            Err(e) => Err(IngestionError::InvalidInput(format!("Query failed: {}", e))),
        }
    }

    /// Execute a single mutation
    ///
    /// Creates a new record or updates an existing one.
    ///
    /// # Arguments
    ///
    /// * `mutation` - Mutation specification with schema, keys, fields, and values
    ///
    /// # Example
    ///
    /// ```ignore
    /// use datafold::lambda::LambdaContext;
    /// use datafold::schema::types::Mutation;
    /// use serde_json::json;
    ///
    /// async fn handler() -> Result<(), Box<dyn std::error::Error>> {
    ///     let mutation = Mutation {
    ///         schema_name: "users".to_string(),
    ///         keys_and_values: vec![("id".to_string(), json!("user123"))],
    ///         fields_and_values: vec![
    ///             ("name".to_string(), json!("Alice")),
    ///             ("email".to_string(), json!("alice@example.com")),
    ///         ],
    ///         trust_distance: 0,
    ///         pub_key: "default".to_string(),
    ///     };
    ///     
    ///     let mutation_id = LambdaContext::execute_mutation(mutation).await?;
    ///     println!("Mutation ID: {}", mutation_id);
    ///     Ok(())
    /// }
    /// ```
    pub async fn execute_mutation(mutation: crate::schema::types::Mutation) -> Result<String, IngestionError> {
        let ctx = Self::get()?;
        let node = ctx.node.lock().await;
        
        node.mutate_batch(vec![mutation])
            .map_err(|e| IngestionError::InvalidInput(format!("Mutation failed: {}", e)))?
            .into_iter()
            .next()
            .ok_or_else(|| IngestionError::InvalidInput("No mutation ID returned".to_string()))
    }

    /// Execute multiple mutations in a batch
    ///
    /// More efficient than calling `execute_mutation()` multiple times.
    ///
    /// # Arguments
    ///
    /// * `mutations` - Vector of mutations to execute
    ///
    /// # Example
    ///
    /// ```ignore
    /// use datafold::lambda::LambdaContext;
    /// use datafold::schema::types::Mutation;
    /// use serde_json::json;
    ///
    /// async fn handler() -> Result<(), Box<dyn std::error::Error>> {
    ///     let mutations = vec![
    ///         Mutation {
    ///             schema_name: "users".to_string(),
    ///             keys_and_values: vec![("id".to_string(), json!("user1"))],
    ///             fields_and_values: vec![("name".to_string(), json!("Alice"))],
    ///             trust_distance: 0,
    ///             pub_key: "default".to_string(),
    ///         },
    ///         Mutation {
    ///             schema_name: "users".to_string(),
    ///             keys_and_values: vec![("id".to_string(), json!("user2"))],
    ///             fields_and_values: vec![("name".to_string(), json!("Bob"))],
    ///             trust_distance: 0,
    ///             pub_key: "default".to_string(),
    ///         },
    ///     ];
    ///     
    ///     let mutation_ids = LambdaContext::execute_mutations(mutations).await?;
    ///     println!("Created {} mutations", mutation_ids.len());
    ///     Ok(())
    /// }
    /// ```
    pub async fn execute_mutations(mutations: Vec<crate::schema::types::Mutation>) -> Result<Vec<String>, IngestionError> {
        let ctx = Self::get()?;
        let node = ctx.node.lock().await;
        
        node.mutate_batch(mutations)
            .map_err(|e| IngestionError::InvalidInput(format!("Batch mutations failed: {}", e)))
    }

    /// List all schemas with their states
    ///
    /// Returns schemas along with their approval/pending states.
    ///
    /// # Example
    ///
    /// ```ignore
    /// use datafold::lambda::LambdaContext;
    ///
    /// async fn handler() -> Result<(), Box<dyn std::error::Error>> {
    ///     let schemas = LambdaContext::list_schemas().await?;
    ///     
    ///     for schema in schemas {
    ///         println!("Schema: {} - State: {:?}", schema.schema.name, schema.state);
    ///     }
    ///     Ok(())
    /// }
    /// ```
    pub async fn list_schemas() -> Result<Vec<crate::schema::SchemaWithState>, IngestionError> {
        let ctx = Self::get()?;
        let node = ctx.node.lock().await;
        let db_guard = node.get_fold_db()
            .map_err(|e| IngestionError::InvalidInput(format!("Failed to access database: {}", e)))?;
        
        db_guard.schema_manager.get_schemas_with_states()
            .map_err(|e| IngestionError::InvalidInput(format!("Failed to list schemas: {}", e)))
    }

    /// Approve a schema
    ///
    /// Approves a schema if it's not already approved (idempotent).
    ///
    /// # Arguments
    ///
    /// * `schema_name` - Name of the schema to approve
    ///
    /// # Example
    ///
    /// ```ignore
    /// use datafold::lambda::LambdaContext;
    ///
    /// async fn handler() -> Result<(), Box<dyn std::error::Error>> {
    ///     LambdaContext::approve_schema("users").await?;
    ///     println!("Schema approved");
    ///     Ok(())
    /// }
    /// ```
    pub async fn approve_schema(schema_name: &str) -> Result<(), IngestionError> {
        let ctx = Self::get()?;
        let node = ctx.node.lock().await;
        let db_guard = node.get_fold_db()
            .map_err(|e| IngestionError::InvalidInput(format!("Failed to access database: {}", e)))?;
        
        db_guard.schema_manager.approve(schema_name)
            .map_err(|e| IngestionError::InvalidInput(format!("Failed to approve schema: {}", e)))
    }

    /// Get the state of a schema
    ///
    /// # Arguments
    ///
    /// * `schema_name` - Name of the schema
    ///
    /// # Returns
    ///
    /// Returns `Some(SchemaState)` if the schema exists, or `None` if not found.
    ///
    /// # Example
    ///
    /// ```ignore
    /// use datafold::lambda::LambdaContext;
    ///
    /// async fn handler() -> Result<(), Box<dyn std::error::Error>> {
    ///     if let Some(state) = LambdaContext::get_schema_state("users").await? {
    ///         println!("Schema state: {:?}", state);
    ///     }
    ///     Ok(())
    /// }
    /// ```
    pub async fn get_schema_state(schema_name: &str) -> Result<Option<crate::schema::SchemaState>, IngestionError> {
        let ctx = Self::get()?;
        let node = ctx.node.lock().await;
        let db_guard = node.get_fold_db()
            .map_err(|e| IngestionError::InvalidInput(format!("Failed to access database: {}", e)))?;
        
        let states = db_guard.schema_manager.get_schema_states()
            .map_err(|e| IngestionError::InvalidInput(format!("Failed to get schema states: {}", e)))?;
        
        Ok(states.get(schema_name).copied())
    }

    /// List all registered transforms
    ///
    /// Returns a map of transform IDs to their definitions.
    ///
    /// # Example
    ///
    /// ```ignore
    /// use datafold::lambda::LambdaContext;
    ///
    /// async fn handler() -> Result<(), Box<dyn std::error::Error>> {
    ///     let transforms = LambdaContext::list_transforms().await?;
    ///     
    ///     for (id, transform) in transforms {
    ///         println!("Transform: {} - Schema: {}", id, transform.get_schema_name());
    ///     }
    ///     Ok(())
    /// }
    /// ```
    pub async fn list_transforms() -> Result<std::collections::HashMap<String, crate::schema::types::Transform>, IngestionError> {
        let ctx = Self::get()?;
        let node = ctx.node.lock().await;
        
        node.list_transforms()
            .map_err(|e| IngestionError::InvalidInput(format!("Failed to list transforms: {}", e)))
    }

    /// Test the logger with all log levels and features
    ///
    /// This is a diagnostic endpoint that tests all logger functionality.
    /// Useful for verifying your logger implementation is working correctly.
    ///
    /// # Arguments
    ///
    /// * `user_id` - User ID to use for logging tests
    ///
    /// # Returns
    ///
    /// A JSON object with test results
    ///
    /// # Example
    ///
    /// ```ignore
    /// use datafold::lambda::LambdaContext;
    ///
    /// async fn handler() -> Result<(), Box<dyn std::error::Error>> {
    ///     let result = LambdaContext::test_logger("test_user_123").await?;
    ///     println!("Logger test results: {}", result);
    ///     Ok(())
    /// }
    /// ```
    pub async fn test_logger(user_id: &str) -> Result<Value, IngestionError> {
        use std::collections::HashMap;
        
        let logger = Self::create_logger(user_id)?;
        let mut results = Vec::new();

        // Test 1: INFO level
        logger.info("test_info", "Testing INFO level logging")
            .await
            .map_err(|e| IngestionError::InvalidInput(format!("INFO test failed: {}", e)))?;
        results.push("INFO level test passed");

        // Test 2: ERROR level
        logger.error("test_error", "Testing ERROR level logging")
            .await
            .map_err(|e| IngestionError::InvalidInput(format!("ERROR test failed: {}", e)))?;
        results.push("ERROR level test passed");

        // Test 3: WARN level
        logger.warn("test_warn", "Testing WARN level logging")
            .await
            .map_err(|e| IngestionError::InvalidInput(format!("WARN test failed: {}", e)))?;
        results.push("WARN level test passed");

        // Test 4: DEBUG level
        logger.debug("test_debug", "Testing DEBUG level logging")
            .await
            .map_err(|e| IngestionError::InvalidInput(format!("DEBUG test failed: {}", e)))?;
        results.push("DEBUG level test passed");

        // Test 5: TRACE level
        logger.trace("test_trace", "Testing TRACE level logging")
            .await
            .map_err(|e| IngestionError::InvalidInput(format!("TRACE test failed: {}", e)))?;
        results.push("TRACE level test passed");

        // Test 6: Metadata logging
        let mut metadata = HashMap::new();
        metadata.insert("test_key".to_string(), "test_value".to_string());
        metadata.insert("event_id".to_string(), "12345".to_string());
        metadata.insert("status".to_string(), "success".to_string());

        logger.log(
            crate::lambda::logging::LogLevel::Info,
            "test_metadata",
            "Testing logging with custom metadata",
            Some(metadata),
        )
        .await
        .map_err(|e| IngestionError::InvalidInput(format!("Metadata test failed: {}", e)))?;
        results.push("Metadata logging test passed");

        // Test 7: Rapid-fire logging
        for i in 0..5 {
            logger.info(
                &format!("rapid_test_{}", i),
                &format!("Rapid fire log message {}", i),
            )
            .await
            .map_err(|e| IngestionError::InvalidInput(format!("Rapid test failed: {}", e)))?;
        }
        results.push("Rapid-fire logging test passed (5 messages)");

        // Test 8: User ID verification
        let logger_user_id = logger.user_id();
        if logger_user_id == user_id {
            logger.info(
                "user_id_verified",
                &format!("User ID correctly set to: {}", logger_user_id),
            )
            .await
            .map_err(|e| IngestionError::InvalidInput(format!("User ID test failed: {}", e)))?;
            results.push("User ID verification passed");
        } else {
            return Err(IngestionError::InvalidInput(format!(
                "User ID mismatch: expected {}, got {}",
                user_id, logger_user_id
            )));
        }

        // Test 9: Workflow simulation
        logger.info("workflow_started", "Beginning test workflow").await
            .map_err(|e| IngestionError::InvalidInput(format!("Workflow test failed: {}", e)))?;
        logger.debug("workflow_step_1", "Processing step 1").await
            .map_err(|e| IngestionError::InvalidInput(format!("Workflow test failed: {}", e)))?;
        logger.debug("workflow_step_2", "Processing step 2").await
            .map_err(|e| IngestionError::InvalidInput(format!("Workflow test failed: {}", e)))?;
        logger.debug("workflow_step_3", "Processing step 3").await
            .map_err(|e| IngestionError::InvalidInput(format!("Workflow test failed: {}", e)))?;
        logger.info("workflow_completed", "Workflow completed successfully").await
            .map_err(|e| IngestionError::InvalidInput(format!("Workflow test failed: {}", e)))?;
        results.push("Workflow logging test passed");

        Ok(serde_json::json!({
            "success": true,
            "user_id": user_id,
            "tests_run": results.len(),
            "results": results,
            "message": "All logger tests passed successfully",
            "note": "Check your configured logger backend (CloudWatch, DynamoDB, etc.) for log entries"
        }))
    }
}
