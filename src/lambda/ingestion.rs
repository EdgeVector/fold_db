//! Ingestion operations for Lambda context

use crate::ingestion::core::IngestionRequest;
use crate::ingestion::progress::ProgressService;
use crate::ingestion::simple_service::SimpleIngestionService;
use crate::ingestion::{IngestionConfig, IngestionError, IngestionProgress, IngestionResponse};
use serde_json::Value;

use super::context::LambdaContext;

impl LambdaContext {
    /// Validate JSON data for ingestion without processing
    ///
    /// Checks if the JSON data is valid for ingestion.
    ///
    /// # Arguments
    ///
    /// * `json_data` - The JSON data to validate
    ///
    /// # Example
    ///
    /// ```ignore
    /// use datafold::lambda::LambdaContext;
    /// use serde_json::json;
    ///
    /// async fn handler() -> Result<(), Box<dyn std::error::Error>> {
    ///     let data = json!({"key": "value"});
    ///     LambdaContext::validate_json(data).await?;
    ///     println!("JSON is valid");
    ///     Ok(())
    /// }
    /// ```
    pub async fn validate_json(json_data: Value) -> Result<(), IngestionError> {
        let config = IngestionConfig::from_env()?;
        let service = SimpleIngestionService::new(config)?;
        service.validate_input(&json_data)
    }

    /// Get ingestion service status
    ///
    /// Returns whether the ingestion service is configured and enabled.
    ///
    /// # Example
    ///
    /// ```ignore
    /// use datafold::lambda::LambdaContext;
    ///
    /// async fn handler() -> Result<(), Box<dyn std::error::Error>> {
    ///     let status = LambdaContext::get_ingestion_status().await?;
    ///     println!("Ingestion enabled: {:?}", status);
    ///     Ok(())
    /// }
    /// ```
    pub async fn get_ingestion_status() -> Result<Value, IngestionError> {
        let config = IngestionConfig::from_env_allow_empty();
        let is_configured = config.is_ready();
        
        Ok(serde_json::json!({
            "enabled": config.enabled,
            "configured": is_configured,
            "provider": format!("{:?}", config.provider),
        }))
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

    /// Get all active ingestion progress
    ///
    /// Returns all current ingestion operations and their progress.
    ///
    /// # Example
    ///
    /// ```ignore
    /// use datafold::lambda::LambdaContext;
    ///
    /// async fn handler() -> Result<(), Box<dyn std::error::Error>> {
    ///     let all_progress = LambdaContext::get_all_progress()?;
    ///     println!("Active ingestions: {}", all_progress.len());
    ///     Ok(())
    /// }
    /// ```
    pub fn get_all_progress() -> Result<Vec<IngestionProgress>, IngestionError> {
        let ctx = Self::get()?;
        let tracker = ctx.progress_tracker.lock().map_err(|_| {
            IngestionError::InvalidInput("Failed to lock progress tracker".to_string())
        })?;
        Ok(tracker.values().cloned().collect())
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
        user_id: String,
    ) -> Result<String, IngestionError> {
        let ctx = Self::get()?;
        let node = Self::get_node(&user_id).await?; // Use user-specific node
        let progress_tracker = ctx.progress_tracker.clone();

        // Generate unique progress ID
        let progress_id = uuid::Uuid::new_v4().to_string();

        // Start progress tracking
        let progress_service = ProgressService::new(progress_tracker);
        progress_service.start_progress(progress_id.clone());

        // Load ingestion config
        let config = IngestionConfig::from_env()?;



        // Clone for background task
        let progress_id_clone = progress_id.clone();
        let json_data_clone = json_data.clone();
        let pub_key_clone = pub_key.clone();
        let user_id_clone = user_id.clone();

        // Spawn background ingestion task
        tokio::spawn(async move {
            use crate::lambda::logging::run_with_user;
            
            run_with_user(&user_id_clone, async move {
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

                // Create ingestion request
                let request = IngestionRequest {
                    data: json_data_clone,
                    auto_execute: Some(auto_execute),
                    trust_distance: Some(trust_distance),
                    pub_key: Some(pub_key_clone),
                    source_file_name: None,
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
            }).await;
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
        user_id: String,
    ) -> Result<IngestionResponse, IngestionError> {
        let ctx = Self::get()?;
        let node = Self::get_node(&user_id).await?; // Use user-specific node
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
