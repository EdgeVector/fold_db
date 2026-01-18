//! System-level operations for Lambda context

use crate::ingestion::IngestionError;
use crate::lambda::logging::{LogEntry, LogLevel, UserLogger};
use serde_json::Value;
use std::collections::HashMap;
use std::time::{SystemTime, UNIX_EPOCH};
use crate::datafold_node::config::DatabaseConfig;

use super::context::LambdaContext;

impl LambdaContext {
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
    ///     let user_id = event.payload["user_id"].as_str().ok_or("Missing user_id")?;
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

    /// Get system status information
    ///
    /// Returns basic system information like uptime and version.
    ///
    /// # Example
    ///
    /// ```ignore
    /// use datafold::lambda::LambdaContext;
    ///
    /// async fn handler() -> Result<(), Box<dyn std::error::Error>> {
    ///     let status = LambdaContext::get_system_status().await?;
    ///     println!("System status: {:?}", status);
    ///     Ok(())
    /// }
    /// ```
    pub async fn get_system_status() -> Result<Value, IngestionError> {
        Ok(serde_json::json!({
            "status": "running",
            "uptime": SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .unwrap_or_default()
                .as_secs(),
            "version": env!("CARGO_PKG_VERSION")
        }))
    }

    /// Get the node's private key
    ///
    /// Returns the private key for the current node.
    ///
    /// # Arguments
    ///
    /// * `user_id` - User ID for node context
    ///
    /// # Example
    ///
    /// ```ignore
    /// use datafold::lambda::LambdaContext;
    ///
    /// async fn handler() -> Result<(), Box<dyn std::error::Error>> {
    ///     let private_key = LambdaContext::get_node_private_key("user_123".to_string()).await?;
    ///     println!("Private key: {}", private_key);
    ///     Ok(())
    /// }
    /// ```
    pub async fn get_node_private_key(user_id: String) -> Result<String, IngestionError> {
        let node_mutex = Self::get_node(&user_id).await?;
        let node = node_mutex.lock().await;
        
        Ok(node.get_node_private_key().to_string())
    }

    /// Get the node's public key
    ///
    /// Returns the public key for the current node.
    ///
    /// # Arguments
    ///
    /// * `user_id` - User ID for node context
    ///
    /// # Example
    ///
    /// ```ignore
    /// use datafold::lambda::LambdaContext;
    ///
    /// async fn handler() -> Result<(), Box<dyn std::error::Error>> {
    ///     let public_key = LambdaContext::get_node_public_key("user_123".to_string()).await?;
    ///     println!("Public key: {}", public_key);
    ///     Ok(())
    /// }
    /// ```
    pub async fn get_node_public_key(user_id: String) -> Result<String, IngestionError> {
        let node_mutex = Self::get_node(&user_id).await?;
        let node = node_mutex.lock().await;
        
        Ok(node.get_node_public_key().to_string())
    }



    /// Reset the database
    ///
    /// **WARNING**: This is a destructive operation that deletes all data.
    /// Only use this in development/testing environments.
    ///
    /// # Arguments
    ///
    /// * `confirm` - Must be true to confirm the reset
    /// * `user_id` - User ID for node context
    ///
    /// # Example
    ///
    /// ```ignore
    /// use datafold::lambda::LambdaContext;
    ///
    /// async fn handler() -> Result<(), Box<dyn std::error::Error>> {
    ///     LambdaContext::reset_database(true, "user_123".to_string()).await?;
    ///     println!("Database reset");
    ///     Ok(())
    /// }
    /// ```
    pub async fn reset_database(confirm: bool, user_id: String) -> Result<(), IngestionError> {
        let ctx = Self::get()?;

        if !confirm {
             return Err(IngestionError::InvalidInput("Reset confirmation required".to_string()));
        }

        // 1. Get processor
        let processor = {
            let node_mutex = Self::get_node(&user_id).await?;
            let node_guard = node_mutex.lock().await;
            crate::datafold_node::OperationProcessor::new(node_guard.clone())
        };

        // 2. Perform reset operations
        processor.perform_database_reset(Some(&user_id)).await
            .map_err(|e| IngestionError::InvalidInput(format!("Database reset failed: {}", e)))?;

        // 3. Invalidate node to force recreation on next request
        ctx.node_manager.invalidate_node(&user_id);
        
        log::info!("Database and schema service reset completed successfully");
        Ok(())
    }

    /// Reset the schema service database
    ///
    /// **WARNING**: This deletes all schemas from the schema service.
    ///
    /// # Arguments
    ///
    /// * `confirm` - Must be true to confirm the reset
    /// * `user_id` - User ID for node context
    ///
    /// # Example
    ///
    /// ```ignore
    /// use datafold::lambda::LambdaContext;
    ///
    /// async fn handler() -> Result<(), Box<dyn std::error::Error>> {
    ///     LambdaContext::reset_schema_service(true, "user_123".to_string()).await?;
    ///     println!("Schema service reset");
    ///     Ok(())
    /// }
    /// ```
    pub async fn reset_schema_service(confirm: bool, user_id: String) -> Result<(), IngestionError> {
        if !confirm {
            return Err(IngestionError::InvalidInput(
                "Reset confirmation required. Set confirm=true".to_string()
            ));
        }

        let node_mutex = Self::get_node(&user_id).await?;
        let node = node_mutex.lock().await;
        let schema_client = node.get_schema_client();

        schema_client.reset_schema_service().await
            .map_err(|e| IngestionError::InvalidInput(format!("Schema service reset failed: {}", e)))?;
        
        log::info!("Schema service database reset completed successfully");
        Ok(())
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
            LogLevel::Info,
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

    /// Get database configuration
    pub async fn get_database_config(user_id: String) -> Result<DatabaseConfig, IngestionError> {
        let node_mutex = Self::get_node(&user_id).await?;
        let node = node_mutex.lock().await;
        Ok(node.config.database.clone())
    }

    /// Update database configuration
    pub async fn update_database_config(new_config: DatabaseConfig, user_id: String) -> Result<(), IngestionError> {
        let ctx = Self::get()?;

        // 1. Get processor
        let processor = {
            let node_mutex = Self::get_node(&user_id).await?;
            let node_guard = node_mutex.lock().await;
            crate::datafold_node::OperationProcessor::new(node_guard.clone())
        };

        // 2. Update configuration
        processor.update_database_configuration(new_config).await
            .map_err(|e| IngestionError::InvalidInput(format!("Failed to update configuration: {}", e)))?;

        // 3. Invalidate node to force recreation with new config
        ctx.node_manager.invalidate_node(&user_id);
        
        Ok(())
    }
}
